//! Slack App integration for OpenRustClaw.
//!
//! Features:
//! - Messages in channels
//! - Direct messages
//! - Slash commands
//! - Interactive components (buttons, modals)
//! - File sharing
//! - App Home
//! - Socket Mode or HTTP mode
//! - Event handling

use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{SlackConfig, SlackMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Slack channel implementation.
pub struct SlackChannel {
    config: SlackConfig,
    client: Client,
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
}

impl SlackChannel {
    /// Create a new Slack channel with the given configuration.
    pub fn new(config: SlackConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Slack allows ~10+ requests per second for most endpoints)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            client: Client::new(),
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
        }
    }

    /// Convert Slack mrkdwn to standard markdown
    #[allow(dead_code)]
    fn slack_to_markdown(text: &str) -> String {
        // Slack uses special mrkdwn syntax
        let mut result = text.to_string();

        // Convert user mentions
        result = result.replace("<@", "@").replace(">", "");

        // Convert special mentions
        result = result.replace("<!channel>", "@channel");
        result = result.replace("<!here>", "@here");
        result = result.replace("<!everyone>", "@everyone");

        result
    }

    /// Convert markdown to Slack mrkdwn
    fn markdown_to_slack(text: &str) -> String {
        // Convert standard markdown to Slack mrkdwn
        let mut result = text.to_string();

        // Convert bold
        result = result.replace("**", "*");

        result
    }

    /// Parse blocks from metadata
    fn parse_blocks(metadata: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
        metadata.get("blocks")?.as_array().cloned()
    }

    fn api_base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://slack.com/api".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn workspace_allowed(&self, workspace_id: Option<&str>) -> bool {
        if self.config.allowed_workspaces.is_empty() {
            return true;
        }

        workspace_id
            .map(|id| self.config.allowed_workspaces.contains(&id.to_string()))
            .unwrap_or(false)
    }

    /// Create an HTTP event handler for Slack Events API requests.
    pub fn event_handler(&self) -> SlackEventHandler {
        SlackEventHandler::new(self.config.clone(), self.incoming_tx.clone())
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn platform(&self) -> Platform {
        Platform::Slack
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "slack".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get channel from metadata
        let channel_id = msg
            .metadata
            .get("slack_channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Missing channel in metadata".to_string(),
            })?
            .to_string();

        // Format content for Slack
        let formatted_content = Self::markdown_to_slack(&msg.content);

        // Check for thread_ts
        let thread_ts = msg
            .metadata
            .get("slack_thread_ts")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        // Check for blocks in metadata
        let blocks = Self::parse_blocks(&msg.metadata);
        let workspace_id = msg.metadata.get("slack_team_id").and_then(|v| v.as_str());
        if !self.workspace_allowed(workspace_id) {
            return Err(ChannelError::PermissionDenied {
                platform: "slack".to_string(),
                message: format!(
                    "workspace {} is not allowed",
                    workspace_id.unwrap_or("<missing>")
                ),
            }
            .into());
        }

        let update_ts = msg
            .metadata
            .get("slack_update_ts")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        let reaction = msg
            .metadata
            .get("slack_reaction")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        let reaction_target_ts = msg
            .metadata
            .get("slack_target_ts")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let endpoint = if reaction.is_some() {
            "reactions.add"
        } else if update_ts.is_some() {
            "chat.update"
        } else {
            "chat.postMessage"
        };
        let url = format!("{}/{}", self.api_base_url(), endpoint);
        let mut payload = serde_json::json!({
            "channel": channel_id,
            "text": formatted_content,
        });
        if let Some(thread_ts) = thread_ts {
            payload["thread_ts"] = serde_json::json!(thread_ts);
        }
        if let Some(blocks) = blocks {
            payload["blocks"] = serde_json::json!(blocks);
        }
        if let Some(ts) = update_ts {
            payload["ts"] = serde_json::json!(ts);
        }
        if let (Some(name), Some(ts)) = (reaction, reaction_target_ts) {
            payload = serde_json::json!({
                "channel": channel_id,
                "timestamp": ts,
                "name": name,
            });
        }

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ChannelError::RateLimited {
                platform: "slack".to_string(),
                retry_after_secs: None,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack send failed")
                    .to_string(),
            }
            .into());
        }

        debug!("Slack message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "slack".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Slack API...");

        // Validate token
        if self.config.token.is_empty() {
            return Err(ChannelError::Config {
                platform: "slack".to_string(),
                message: "Slack bot token is required".to_string(),
            }
            .into());
        }

        // In a full implementation, this would:
        // 1. Create a slack_morphism client
        // 2. Test authentication with auth.test
        // 3. Start Socket Mode listener or HTTP webhook handler
        // 4. Handle events: app_mention, message, slash_command, block_actions, view_submission

        match self.config.mode {
            SlackMode::SocketMode => {
                if self.config.app_token.is_none() {
                    warn!("Socket Mode enabled but no app_token provided");
                }
            }
            SlackMode::Http => {}
        }

        let url = format!("{}/auth.test", self.api_base_url());
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "slack".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack auth probe failed")
                    .to_string(),
            }
            .into());
        }

        let team_id = body.get("team_id").and_then(|v| v.as_str());
        if !self.workspace_allowed(team_id) {
            return Err(ChannelError::PermissionDenied {
                platform: "slack".to_string(),
                message: format!(
                    "workspace {} is not allowed",
                    team_id.unwrap_or("<missing>")
                ),
            }
            .into());
        }

        *self.is_connected.write().await = true;
        info!(mode = ?self.config.mode, "Slack channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Slack...");

        *self.is_connected.write().await = false;

        info!("Slack channel disconnected");
        Ok(())
    }
}

/// Handle Slack Events API HTTP requests and enqueue normalized incoming messages.
pub struct SlackEventHandler {
    config: SlackConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl SlackEventHandler {
    pub fn new(config: SlackConfig, incoming_tx: mpsc::Sender<IncomingMessage>) -> Self {
        Self {
            config,
            incoming_tx,
        }
    }

    pub async fn handle_event(
        &self,
        body: &[u8],
        timestamp: Option<&str>,
        signature: Option<&str>,
    ) -> Result<Option<serde_json::Value>> {
        self.verify_signature(body, timestamp, signature)?;

        let event: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: format!("Failed to parse Slack event: {}", e),
            })?;

        match event.get("type").and_then(|value| value.as_str()) {
            Some("url_verification") => Ok(Some(serde_json::json!({
                "challenge": event
                    .get("challenge")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
            }))),
            Some("event_callback") => {
                self.handle_event_callback(&event).await?;
                Ok(None)
            }
            Some(other) => {
                debug!(event_type = %other, "Ignoring unsupported Slack event");
                Ok(None)
            }
            None => Err(ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack event payload missing type".to_string(),
            }
            .into()),
        }
    }

    fn verify_signature(
        &self,
        body: &[u8],
        timestamp: Option<&str>,
        signature: Option<&str>,
    ) -> Result<()> {
        let Some(secret) = self.config.signing_secret.as_deref() else {
            return Ok(());
        };

        let timestamp = timestamp.ok_or_else(|| ChannelError::AuthFailed {
            platform: "slack".to_string(),
            message: "Missing X-Slack-Request-Timestamp header".to_string(),
        })?;
        let signature = signature.ok_or_else(|| ChannelError::AuthFailed {
            platform: "slack".to_string(),
            message: "Missing X-Slack-Signature header".to_string(),
        })?;

        let request_age = chrono::Utc::now().timestamp()
            - timestamp
                .parse::<i64>()
                .map_err(|e| ChannelError::AuthFailed {
                    platform: "slack".to_string(),
                    message: format!("Invalid Slack request timestamp: {}", e),
                })?;

        if request_age.abs() > 60 * 5 {
            return Err(ChannelError::AuthFailed {
                platform: "slack".to_string(),
                message: "Slack request timestamp is outside the allowed window".to_string(),
            }
            .into());
        }

        let body_str = std::str::from_utf8(body).map_err(|e| ChannelError::InvalidFormat {
            platform: "slack".to_string(),
            message: format!("Slack request body is not valid UTF-8: {}", e),
        })?;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|e| {
            ChannelError::AuthFailed {
                platform: "slack".to_string(),
                message: format!("Invalid Slack signing secret: {}", e),
            }
        })?;
        mac.update(format!("v0:{timestamp}:{body_str}").as_bytes());
        let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        if expected != signature {
            return Err(ChannelError::AuthFailed {
                platform: "slack".to_string(),
                message: "Slack request signature verification failed".to_string(),
            }
            .into());
        }

        Ok(())
    }

    async fn handle_event_callback(&self, payload: &serde_json::Value) -> Result<()> {
        let team_id = payload.get("team_id").and_then(|value| value.as_str());
        if !self.workspace_allowed(team_id) {
            return Err(ChannelError::PermissionDenied {
                platform: "slack".to_string(),
                message: format!(
                    "workspace {} is not allowed",
                    team_id.unwrap_or("<missing>")
                ),
            }
            .into());
        }

        let event = payload
            .get("event")
            .and_then(|value| value.as_object())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack event callback missing event payload".to_string(),
            })?;

        let event_type = event
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if !matches!(event_type, "message" | "app_mention") {
            return Ok(());
        }

        if event_type == "message"
            && event
                .get("subtype")
                .and_then(|value| value.as_str())
                .is_some()
        {
            return Ok(());
        }

        let user_id = event
            .get("user")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack message event missing user".to_string(),
            })?;
        let channel_id = event
            .get("channel")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack message event missing channel".to_string(),
            })?;
        let content = event
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            return Ok(());
        }

        let mut metadata = serde_json::json!({
            "slack_channel": channel_id,
            "slack_is_group": !channel_id.starts_with('D'),
            "slack_bot_mentioned": event_type == "app_mention",
        });
        if let Some(team_id) = team_id {
            metadata["slack_team_id"] = serde_json::json!(team_id);
        }
        if let Some(thread_ts) = event.get("thread_ts").and_then(|value| value.as_str()) {
            metadata["slack_thread_ts"] = serde_json::json!(thread_ts);
        }
        if let Some(event_ts) = event.get("event_ts").and_then(|value| value.as_str()) {
            metadata["slack_event_ts"] = serde_json::json!(event_ts);
        }

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            content: Self::normalize_slack_text(&content),
            platform: Platform::Slack,
            metadata,
        };

        self.incoming_tx
            .send(incoming)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "slack".to_string(),
                message: format!("Failed to enqueue Slack incoming message: {}", e),
            })?;

        Ok(())
    }

    fn normalize_slack_text(text: &str) -> String {
        SlackChannel::slack_to_markdown(text)
    }

    fn workspace_allowed(&self, workspace_id: Option<&str>) -> bool {
        if self.config.allowed_workspaces.is_empty() {
            return true;
        }

        workspace_id
            .map(|id| self.config.allowed_workspaces.contains(&id.to_string()))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_to_markdown() {
        let text = "Hello <@U123> check this";
        let result = SlackChannel::slack_to_markdown(text);
        assert!(result.contains("@U123"));
    }

    #[test]
    fn test_markdown_to_slack() {
        let text = "Hello **world**";
        let result = SlackChannel::markdown_to_slack(text);
        assert_eq!(result, "Hello *world*");
    }

    #[tokio::test]
    async fn test_connect_and_send_slack_message() {
        use wiremock::matchers::{bearer_token, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth.test"))
            .and(bearer_token("xoxb-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "team_id": "T123"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat.postMessage"))
            .and(bearer_token("xoxb-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "ts": "1.23"
            })))
            .mount(&server)
            .await;

        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: Some(server.uri()),
            app_token: None,
            signing_secret: None,
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        };
        let mut channel = SlackChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello slack".to_string(),
                metadata: serde_json::json!({
                    "slack_channel": "C123",
                    "slack_team_id": "T123"
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_url_verification_event_returns_challenge() {
        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: None,
            app_token: None,
            signing_secret: None,
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec![],
            app_home_enabled: true,
        };
        let channel = SlackChannel::new(config);
        let handler = channel.event_handler();

        let response = handler
            .handle_event(
                br#"{"type":"url_verification","challenge":"abc123"}"#,
                None,
                None,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(response["challenge"], "abc123");
    }

    #[tokio::test]
    async fn test_message_event_enqueues_incoming_message() {
        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: None,
            app_token: None,
            signing_secret: None,
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        };
        let channel = SlackChannel::new(config);
        let handler = channel.event_handler();

        handler
            .handle_event(
                br#"{
                    "type":"event_callback",
                    "team_id":"T123",
                    "event":{
                        "type":"message",
                        "user":"U123",
                        "text":"Hello <@U999>",
                        "channel":"C456",
                        "thread_ts":"171234.000100",
                        "event_ts":"171234.000200"
                    }
                }"#,
                None,
                None,
            )
            .await
            .unwrap();

        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.platform, Platform::Slack);
        assert_eq!(incoming.user_id, "U123");
        assert_eq!(incoming.content, "Hello @U999");
        assert_eq!(incoming.metadata["slack_channel"], "C456");
        assert_eq!(incoming.metadata["slack_team_id"], "T123");
        assert_eq!(incoming.metadata["slack_thread_ts"], "171234.000100");
    }

    #[tokio::test]
    async fn test_signature_verification_rejects_invalid_request() {
        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: None,
            app_token: None,
            signing_secret: Some("secret".to_string()),
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec![],
            app_home_enabled: true,
        };
        let channel = SlackChannel::new(config);
        let handler = channel.event_handler();
        let timestamp = chrono::Utc::now().timestamp().to_string();

        let error = handler
            .handle_event(
                br#"{"type":"url_verification","challenge":"abc123"}"#,
                Some(&timestamp),
                Some("v0=invalid"),
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("signature"));
    }
}
