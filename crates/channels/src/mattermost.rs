//! Mattermost bot integration for OpenRustClaw.
//!
//! Shipped scope:
//! - Slash command / outgoing webhook ingress
//! - Bot-token REST outbound posts
//! - Thread replies via root_id
//! - Local file-reference upload via `/api/v4/files`

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use reqwest::multipart::{Form, Part};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::info;
use url::form_urlencoded;
use uuid::Uuid;

use openrustclaw_core::config::MattermostConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

#[derive(Debug)]
pub struct MattermostChannel {
    config: MattermostConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: RwLock<bool>,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Default)]
struct MattermostEvent {
    token: Option<String>,
    user_id: Option<String>,
    user_name: Option<String>,
    channel_id: Option<String>,
    channel_name: Option<String>,
    team_id: Option<String>,
    text: Option<String>,
    trigger_word: Option<String>,
    post_id: Option<String>,
    root_id: Option<String>,
    channel_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MattermostWebhookHandler {
    channel: Arc<MattermostChannel>,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl MattermostChannel {
    pub fn new(config: MattermostConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).expect("non-zero quota")),
        );

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
            is_connected: RwLock::new(false),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build mattermost client"),
        }
    }

    pub fn webhook_handler(&self) -> MattermostWebhookHandler {
        MattermostWebhookHandler {
            channel: Arc::new(Self::new(self.config.clone())),
            incoming_tx: self.incoming_tx.clone(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/api/v4{}",
            self.config.server_url.trim_end_matches('/'),
            path
        )
    }

    fn event_from_json(value: &serde_json::Value) -> MattermostEvent {
        let post = value.get("post");
        MattermostEvent {
            token: value
                .get("token")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            user_id: value
                .get("user_id")
                .or_else(|| post.and_then(|p| p.get("user_id")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            user_name: value
                .get("user_name")
                .or_else(|| value.get("username"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            channel_id: value
                .get("channel_id")
                .or_else(|| post.and_then(|p| p.get("channel_id")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            channel_name: value
                .get("channel_name")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            team_id: value
                .get("team_id")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            text: value
                .get("text")
                .or_else(|| value.get("message"))
                .or_else(|| post.and_then(|p| p.get("message")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            trigger_word: value
                .get("trigger_word")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            post_id: value
                .get("post_id")
                .or_else(|| post.and_then(|p| p.get("id")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            root_id: value
                .get("root_id")
                .or_else(|| post.and_then(|p| p.get("root_id")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            channel_type: value
                .get("channel_type")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
        }
    }

    fn event_from_form(body: &[u8]) -> MattermostEvent {
        let mut values = HashMap::new();
        for (key, value) in form_urlencoded::parse(body) {
            values.insert(key.into_owned(), value.into_owned());
        }
        MattermostEvent {
            token: values.get("token").cloned(),
            user_id: values.get("user_id").cloned(),
            user_name: values.get("user_name").cloned(),
            channel_id: values.get("channel_id").cloned(),
            channel_name: values.get("channel_name").cloned(),
            team_id: values.get("team_id").cloned(),
            text: values.get("text").cloned(),
            trigger_word: values.get("trigger_word").cloned(),
            post_id: values.get("post_id").cloned(),
            root_id: values.get("root_id").cloned(),
            channel_type: values.get("channel_type").cloned(),
        }
    }

    fn event_from_bytes(content_type: Option<&str>, body: &[u8]) -> Result<MattermostEvent> {
        if content_type
            .map(|value| value.starts_with("application/x-www-form-urlencoded"))
            .unwrap_or(false)
        {
            return Ok(Self::event_from_form(body));
        }

        let json: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "mattermost".to_string(),
                message: format!("Failed to decode Mattermost payload: {}", e),
            })?;
        Ok(Self::event_from_json(&json))
    }

    fn strip_trigger_word(text: &str, trigger_word: Option<&str>) -> String {
        if let Some(trigger) = trigger_word {
            let trimmed = text.trim();
            let trigger_trimmed = trigger.trim();
            if let Some(rest) = trimmed.strip_prefix(trigger_trimmed) {
                return rest.trim_start().to_string();
            }
        }
        text.to_string()
    }

    fn is_group_event(event: &MattermostEvent) -> bool {
        if let Some(channel_type) = event.channel_type.as_deref() {
            return !matches!(channel_type, "D");
        }
        event
            .channel_name
            .as_deref()
            .map(|name| !name.starts_with('@'))
            .unwrap_or(true)
    }

    fn bot_mentioned(&self, event: &MattermostEvent) -> bool {
        if event
            .trigger_word
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            return true;
        }
        let Some(bot_username) = self.config.bot_username.as_deref() else {
            return false;
        };
        let Some(text) = event.text.as_deref() else {
            return false;
        };
        text.contains(&format!("@{}", bot_username))
    }

    async fn upload_files(
        &self,
        channel_id: &str,
        metadata: &serde_json::Value,
    ) -> Result<Vec<String>> {
        let refs = metadata
            .get("file_references")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        if refs.is_empty() {
            return Ok(Vec::new());
        }

        let mut form = Form::new().text("channel_id", channel_id.to_string());
        let mut attached = false;
        for entry in refs {
            let Some(local_path) = entry
                .get("local_path")
                .or_else(|| entry.get("path"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            else {
                continue;
            };
            let bytes =
                tokio::fs::read(local_path)
                    .await
                    .map_err(|e| ChannelError::SendFailed {
                        platform: "mattermost".to_string(),
                        message: format!(
                            "Failed to read Mattermost upload file '{}': {}",
                            local_path, e
                        ),
                    })?;
            let filename = entry
                .get("name")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    std::path::Path::new(local_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                })
                .unwrap_or("file.bin")
                .to_string();
            let part = Part::bytes(bytes).file_name(filename);
            form = form.part("files", part);
            attached = true;
        }

        if !attached {
            return Ok(Vec::new());
        }

        let response = self
            .http
            .post(self.api_url("/files"))
            .bearer_auth(&self.config.bot_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "mattermost".to_string(),
                message: format!("Mattermost file upload failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "mattermost".to_string(),
                message: format!("Mattermost file upload failed: {}", body),
            }
            .into());
        }

        let json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "mattermost".to_string(),
                    message: format!("Failed to parse Mattermost upload response: {}", e),
                })?;

        Ok(json
            .get("file_infos")
            .and_then(|value| value.as_array())
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.get("id").and_then(|value| value.as_str()))
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn send_post(&self, msg: &OutgoingMessage) -> Result<()> {
        let channel_id = msg
            .metadata
            .get("mattermost_channel_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "mattermost".to_string(),
                message: "Missing mattermost_channel_id in metadata".to_string(),
            })?;
        let root_id = msg
            .metadata
            .get("mattermost_root_id")
            .or_else(|| msg.metadata.get("mattermost_post_id"))
            .and_then(|value| value.as_str());
        let file_ids = self.upload_files(channel_id, &msg.metadata).await?;

        let mut payload = serde_json::json!({
            "channel_id": channel_id,
            "message": msg.content,
        });
        if let Some(root_id) = root_id {
            payload["root_id"] = serde_json::json!(root_id);
        }
        if !file_ids.is_empty() {
            payload["file_ids"] = serde_json::json!(file_ids);
        }

        let response = self
            .http
            .post(self.api_url("/posts"))
            .bearer_auth(&self.config.bot_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "mattermost".to_string(),
                message: format!("Mattermost post send failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "mattermost".to_string(),
                message: format!("Mattermost post send failed: {}", body),
            }
            .into());
        }

        Ok(())
    }
}

impl MattermostWebhookHandler {
    pub async fn handle_request(
        &self,
        content_type: Option<&str>,
        body: &[u8],
    ) -> Result<serde_json::Value> {
        let event = MattermostChannel::event_from_bytes(content_type, body)?;

        if let Some(expected) = self.channel.config.webhook_token.as_deref() {
            if event.token.as_deref() != Some(expected) {
                return Err(ChannelError::AuthFailed {
                    platform: "mattermost".to_string(),
                    message: "Invalid Mattermost webhook token".to_string(),
                }
                .into());
            }
        }

        if !self.channel.config.allowlist.is_empty() {
            let user_allowed = event
                .user_id
                .as_ref()
                .map(|value| self.channel.config.allowlist.contains(value))
                .unwrap_or(false)
                || event
                    .user_name
                    .as_ref()
                    .map(|value| self.channel.config.allowlist.contains(value))
                    .unwrap_or(false);
            if !user_allowed {
                return Ok(serde_json::json!({"status": "ignored"}));
            }
        }

        if !self.channel.config.allowed_channels.is_empty()
            && !event
                .channel_id
                .as_ref()
                .map(|value| self.channel.config.allowed_channels.contains(value))
                .unwrap_or(false)
        {
            return Ok(serde_json::json!({"status": "ignored"}));
        }

        let text = event.text.clone().unwrap_or_default();
        let stripped = MattermostChannel::strip_trigger_word(&text, event.trigger_word.as_deref());
        let is_group = MattermostChannel::is_group_event(&event);
        let bot_mentioned = self.channel.bot_mentioned(&event);

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: event
                .user_id
                .clone()
                .or_else(|| event.user_name.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            content: stripped,
            platform: Platform::Mattermost,
            metadata: serde_json::json!({
                "mattermost_channel_id": event.channel_id,
                "mattermost_channel_name": event.channel_name,
                "mattermost_team_id": event.team_id,
                "mattermost_user_name": event.user_name,
                "mattermost_trigger_word": event.trigger_word,
                "mattermost_post_id": event.post_id,
                "mattermost_root_id": event.root_id,
                "mattermost_is_group": is_group,
                "mattermost_bot_mentioned": bot_mentioned,
            }),
        };

        let _ = self.incoming_tx.send(incoming).await;
        Ok(serde_json::json!({"status": "ok"}))
    }
}

#[async_trait]
impl Channel for MattermostChannel {
    fn platform(&self) -> Platform {
        Platform::Mattermost
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        self.rate_limiter.until_ready().await;
        self.send_post(&msg).await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "mattermost".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }
        if self.config.server_url.is_empty() || self.config.bot_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "mattermost".to_string(),
                message: "server_url and bot_token are required".to_string(),
            }
            .into());
        }
        *self.is_connected.write().await = true;
        info!("Mattermost channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.is_connected.write().await = false;
        info!("Mattermost channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(server: &MockServer) -> MattermostConfig {
        MattermostConfig {
            enabled: true,
            server_url: server.uri(),
            bot_token: "test-token".to_string(),
            webhook_path: "/webhooks/mattermost".to_string(),
            webhook_token: Some("hook-token".to_string()),
            bot_username: Some("claw".to_string()),
            allowlist: Vec::new(),
            allowed_channels: Vec::new(),
            rate_limit_requests_per_second: 10,
        }
    }

    #[tokio::test]
    async fn test_webhook_handler_routes_form_payload() {
        let server = MockServer::start().await;
        let channel = MattermostChannel::new(test_config(&server));
        let handler = channel.webhook_handler();
        let body = b"token=hook-token&user_id=user-1&user_name=alice&channel_id=channel-1&channel_name=town-square&team_id=team-1&text=%40claw+hello+there&trigger_word=%40claw";

        handler
            .handle_request(Some("application/x-www-form-urlencoded"), body)
            .await
            .unwrap();

        let incoming = channel.receive().await.expect("incoming");
        assert_eq!(incoming.platform, Platform::Mattermost);
        assert_eq!(incoming.user_id, "user-1");
        assert_eq!(incoming.content, "hello there");
        assert_eq!(incoming.metadata["mattermost_channel_id"], "channel-1");
        assert_eq!(incoming.metadata["mattermost_team_id"], "team-1");
        assert_eq!(incoming.metadata["mattermost_bot_mentioned"], true);
        assert_eq!(incoming.metadata["mattermost_is_group"], true);
    }

    #[tokio::test]
    async fn test_send_uploads_local_files_and_replies_in_thread() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/files"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "file_infos": [{"id": "file-1"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "post-1"
            })))
            .mount(&server)
            .await;

        let temp_root =
            std::env::temp_dir().join(format!("orc-mattermost-upload-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_root).await.unwrap();
        let file_path = temp_root.join("report.txt");
        tokio::fs::write(&file_path, b"mattermost").await.unwrap();

        let mut channel = MattermostChannel::new(test_config(&server));
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "thread reply".to_string(),
                metadata: serde_json::json!({
                    "mattermost_channel_id": "channel-1",
                    "mattermost_post_id": "root-1",
                    "file_references": [{
                        "local_path": file_path,
                        "name": "report.txt"
                    }]
                }),
            })
            .await
            .unwrap();

        let _ = tokio::fs::remove_file(&file_path).await;
        let _ = tokio::fs::remove_dir_all(&temp_root).await;
    }
}
