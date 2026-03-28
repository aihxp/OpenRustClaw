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

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use governor::{Quota, RateLimiter};
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
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
    socket_task: Mutex<Option<JoinHandle<()>>>,
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
            socket_task: Mutex::new(None),
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

    fn parse_file_references(metadata: &serde_json::Value) -> Vec<(String, String)> {
        metadata
            .get("file_references")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| {
                if let Some(url) = value.as_str() {
                    return Some(("Download file".to_string(), url.to_string()));
                }
                let url = value
                    .get("url")
                    .or_else(|| value.get("download_url"))
                    .and_then(|field| field.as_str())?;
                let label = value
                    .get("title")
                    .or_else(|| value.get("name"))
                    .and_then(|field| field.as_str())
                    .unwrap_or("Download file");
                Some((label.to_string(), url.to_string()))
            })
            .collect()
    }

    fn parse_local_file_references(metadata: &serde_json::Value) -> Vec<(String, String, String)> {
        metadata
            .get("file_references")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| {
                let local_path = value.get("local_path").and_then(|field| field.as_str())?;
                let filename = value
                    .get("name")
                    .or_else(|| value.get("title"))
                    .and_then(|field| field.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        Path::new(local_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("attachment")
                            .to_string()
                    });
                let title = value
                    .get("title")
                    .and_then(|field| field.as_str())
                    .unwrap_or(&filename)
                    .to_string();
                Some((local_path.to_string(), filename, title))
            })
            .collect()
    }

    async fn upload_local_files(
        &self,
        channel_id: &str,
        thread_ts: Option<&str>,
        text_comment: Option<&str>,
        local_files: &[(String, String, String)],
    ) -> Result<()> {
        if local_files.is_empty() {
            return Ok(());
        }

        let mut completed_files = Vec::with_capacity(local_files.len());

        for (local_path, filename, title) in local_files {
            let bytes =
                tokio::fs::read(local_path)
                    .await
                    .map_err(|e| ChannelError::SendFailed {
                        platform: "slack".to_string(),
                        message: format!("Failed to read Slack upload '{}': {}", local_path, e),
                    })?;

            let length = bytes.len();
            let upload_url = format!("{}/files.getUploadURLExternal", self.api_base_url());
            let upload_request = serde_json::json!({
                "filename": filename,
                "length": length,
            });

            let upload_response = self
                .client
                .post(upload_url)
                .bearer_auth(&self.config.token)
                .json(&upload_request)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: format!("Slack upload URL request failed: {}", e),
                })?;

            let status = upload_response.status();
            let body: serde_json::Value = upload_response
                .json()
                .await
                .unwrap_or_else(|_| serde_json::json!({}));

            if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                return Err(ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Slack file upload URL request failed")
                        .to_string(),
                }
                .into());
            }

            let upload_url = body
                .get("upload_url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "slack".to_string(),
                    message: "Slack upload response missing upload_url".to_string(),
                })?;
            let file_id = body
                .get("file_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "slack".to_string(),
                    message: "Slack upload response missing file_id".to_string(),
                })?;

            let raw_upload = self
                .client
                .post(upload_url)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: format!("Slack raw file upload failed: {}", e),
                })?;

            if !raw_upload.status().is_success() {
                let body = raw_upload.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: format!("Slack raw file upload failed: {}", body),
                }
                .into());
            }

            completed_files.push(serde_json::json!({
                "id": file_id,
                "title": title,
            }));
        }

        let mut complete_payload = serde_json::json!({
            "files": completed_files,
            "channel_id": channel_id,
        });
        if let Some(thread_ts) = thread_ts {
            complete_payload["thread_ts"] = serde_json::json!(thread_ts);
        }
        if let Some(comment) = text_comment
            && !comment.is_empty()
        {
            complete_payload["initial_comment"] = serde_json::json!(comment);
        }

        let complete_response = self
            .client
            .post(format!(
                "{}/files.completeUploadExternal",
                self.api_base_url()
            ))
            .bearer_auth(&self.config.token)
            .json(&complete_payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: format!("Slack complete upload failed: {}", e),
            })?;

        let status = complete_response.status();
        let body: serde_json::Value = complete_response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack complete upload failed")
                    .to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn parse_stream_chunks(msg: &OutgoingMessage) -> Vec<serde_json::Value> {
        if let Some(values) = msg
            .metadata
            .get("slack_stream_chunks")
            .and_then(|value| value.as_array())
        {
            let chunks: Vec<_> = values
                .iter()
                .filter_map(|value| {
                    if let Some(text) = value.as_str() {
                        Some(serde_json::json!({
                            "type": "markdown_text",
                            "text": Self::markdown_to_slack(text),
                        }))
                    } else if value.is_object() {
                        Some(value.clone())
                    } else {
                        None
                    }
                })
                .collect();
            if !chunks.is_empty() {
                return chunks;
            }
        }

        let chunk_size = msg
            .metadata
            .get("slack_stream_chunk_chars")
            .and_then(|value| value.as_u64())
            .map(|value| value.clamp(1, 12000) as usize)
            .unwrap_or(1200);
        let formatted = Self::markdown_to_slack(&msg.content);
        if formatted.is_empty() {
            return Vec::new();
        }
        let chars: Vec<char> = formatted.chars().collect();
        chars
            .chunks(chunk_size)
            .map(|chunk| {
                serde_json::json!({
                    "type": "markdown_text",
                    "text": chunk.iter().collect::<String>(),
                })
            })
            .collect()
    }

    async fn send_stream(&self, msg: &OutgoingMessage, channel_id: &str) -> Result<()> {
        let thread_ts = msg
            .metadata
            .get("slack_thread_ts")
            .and_then(|v| v.as_str())
            .or_else(|| msg.metadata.get("slack_event_ts").and_then(|v| v.as_str()))
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message:
                    "slack_thread_ts or slack_event_ts is required for Slack draft-stream replies"
                        .to_string(),
            })?;
        let recipient_user_id = msg
            .metadata
            .get("slack_recipient_user_id")
            .or_else(|| msg.metadata.get("slack_thread_owner_user_id"))
            .or_else(|| msg.metadata.get("slack_user_id"))
            .and_then(|v| v.as_str());
        let recipient_team_id = msg
            .metadata
            .get("slack_recipient_team_id")
            .or_else(|| msg.metadata.get("slack_team_id"))
            .and_then(|v| v.as_str());

        let chunks = Self::parse_stream_chunks(msg);
        if chunks.is_empty() {
            return Err(ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack draft-stream replies require content or slack_stream_chunks"
                    .to_string(),
            }
            .into());
        }

        let mut start_payload = serde_json::json!({
            "channel": channel_id,
            "thread_ts": thread_ts,
            "chunks": [chunks[0].clone()],
        });
        if !channel_id.starts_with('D') {
            let recipient_user_id = recipient_user_id.ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message:
                    "slack_recipient_user_id, slack_thread_owner_user_id, or slack_user_id is required for channel stream replies"
                        .to_string(),
            })?;
            let recipient_team_id = recipient_team_id.ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message:
                    "slack_recipient_team_id or slack_team_id is required for channel stream replies"
                        .to_string(),
            })?;
            start_payload["recipient_user_id"] = serde_json::json!(recipient_user_id);
            start_payload["recipient_team_id"] = serde_json::json!(recipient_team_id);
        }

        let metadata_payload = msg
            .metadata
            .get("slack_stream_mode")
            .and_then(|value| value.as_str())
            .map(|stream_mode| {
                serde_json::json!({
                    "event_type": "openrustclaw_stream",
                    "event_payload": {
                        "mode": stream_mode,
                    }
                })
            });

        let start_response = self
            .client
            .post(format!("{}/chat.startStream", self.api_base_url()))
            .bearer_auth(&self.config.token)
            .json(&start_payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: e.to_string(),
            })?;
        let start_status = start_response.status();
        let start_body: serde_json::Value = start_response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        if !start_status.is_success()
            || start_body.get("ok").and_then(|v| v.as_bool()) != Some(true)
        {
            return Err(ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: start_body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack chat.startStream failed")
                    .to_string(),
            }
            .into());
        }
        let stream_ts = start_body
            .get("ts")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: "Slack chat.startStream response missing ts".to_string(),
            })?
            .to_string();

        for chunk in chunks.iter().skip(1) {
            let append_response = self
                .client
                .post(format!("{}/chat.appendStream", self.api_base_url()))
                .bearer_auth(&self.config.token)
                .json(&serde_json::json!({
                    "channel": channel_id,
                    "ts": stream_ts,
                    "chunks": [chunk],
                }))
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: e.to_string(),
                })?;
            let append_status = append_response.status();
            let append_body: serde_json::Value = append_response
                .json()
                .await
                .unwrap_or_else(|_| serde_json::json!({}));
            if !append_status.is_success()
                || append_body.get("ok").and_then(|v| v.as_bool()) != Some(true)
            {
                return Err(ChannelError::SendFailed {
                    platform: "slack".to_string(),
                    message: append_body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Slack chat.appendStream failed")
                        .to_string(),
                }
                .into());
            }
        }

        let mut stop_payload = serde_json::json!({
            "channel": channel_id,
            "ts": stream_ts,
        });
        if let Some(blocks) = Self::parse_blocks(&msg.metadata) {
            stop_payload["blocks"] = serde_json::json!(blocks);
        }
        if let Some(metadata) = metadata_payload {
            stop_payload["metadata"] = metadata;
        }

        let stop_response = self
            .client
            .post(format!("{}/chat.stopStream", self.api_base_url()))
            .bearer_auth(&self.config.token)
            .json(&stop_payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: e.to_string(),
            })?;
        let stop_status = stop_response.status();
        let stop_body: serde_json::Value = stop_response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        if !stop_status.is_success() || stop_body.get("ok").and_then(|v| v.as_bool()) != Some(true)
        {
            return Err(ChannelError::SendFailed {
                platform: "slack".to_string(),
                message: stop_body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack chat.stopStream failed")
                    .to_string(),
            }
            .into());
        }

        info!(channel = %channel_id, thread_ts = %thread_ts, "Slack draft-stream reply sent");
        Ok(())
    }

    fn api_base_url(&self) -> String {
        Self::api_base_url_for(&self.config)
    }

    fn api_base_url_for(config: &SlackConfig) -> String {
        config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://slack.com/api".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    async fn open_socket_mode_url_for(config: &SlackConfig, client: &Client) -> Result<String> {
        let app_token = config
            .app_token
            .as_ref()
            .ok_or_else(|| ChannelError::Config {
                platform: "slack".to_string(),
                message: "Slack app_token is required for Socket Mode".to_string(),
            })?;
        let url = format!("{}/apps.connections.open", Self::api_base_url_for(config));
        let response = client
            .post(url)
            .bearer_auth(app_token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "slack".to_string(),
                message: format!("Failed to open Slack Socket Mode connection: {}", e),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "slack".to_string(),
                message: body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Slack Socket Mode open failed")
                    .to_string(),
            }
            .into());
        }

        body.get("url")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| {
                ChannelError::InvalidFormat {
                    platform: "slack".to_string(),
                    message: "Slack Socket Mode open response missing url".to_string(),
                }
                .into()
            })
    }

    async fn start_socket_mode_loop(&self) -> Result<()> {
        let config = self.config.clone();
        let client = self.client.clone();
        let handler = self.event_handler();
        let socket_task = tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);

            loop {
                let ws_url = match SlackChannel::open_socket_mode_url_for(&config, &client).await {
                    Ok(url) => url,
                    Err(error) => {
                        warn!(
                            error = %error,
                            retry_in = ?backoff,
                            "Failed to open Slack Socket Mode URL"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                        continue;
                    }
                };

                let stream = match connect_async(&ws_url).await {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        warn!(
                            error = %error,
                            retry_in = ?backoff,
                            "Failed to connect Slack Socket Mode websocket"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                        continue;
                    }
                };

                match run_socket_mode_loop(stream, handler.clone()).await {
                    Ok(()) => {
                        info!("Slack Socket Mode loop exited cleanly");
                        break;
                    }
                    Err(error) => {
                        warn!(
                            error = %error,
                            retry_in = ?backoff,
                            "Slack Socket Mode loop exited; reconnecting"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                    }
                }
            }
        });

        *self.socket_task.lock().await = Some(socket_task);
        Ok(())
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

        let use_stream = msg
            .metadata
            .get("slack_draft_stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || msg.metadata.get("slack_stream_chunks").is_some();
        if use_stream {
            return self.send_stream(&msg, &channel_id).await;
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
        if let Some(thread_ts) = thread_ts.as_deref() {
            payload["thread_ts"] = serde_json::json!(thread_ts);
        }
        if let Some(reply_broadcast) = msg
            .metadata
            .get("slack_reply_broadcast")
            .and_then(|v| v.as_bool())
        {
            payload["reply_broadcast"] = serde_json::json!(reply_broadcast);
        }
        if let Some(unfurl_links) = msg
            .metadata
            .get("slack_unfurl_links")
            .and_then(|v| v.as_bool())
        {
            payload["unfurl_links"] = serde_json::json!(unfurl_links);
        }
        if let Some(unfurl_media) = msg
            .metadata
            .get("slack_unfurl_media")
            .and_then(|v| v.as_bool())
        {
            payload["unfurl_media"] = serde_json::json!(unfurl_media);
        }
        let download_actions = msg
            .metadata
            .get("slack_attachment_download_actions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let file_refs = Self::parse_file_references(&msg.metadata);
        let local_files = Self::parse_local_file_references(&msg.metadata);
        let has_custom_rendering = blocks.is_some()
            || msg
                .metadata
                .get("slack_attachments")
                .and_then(|v| v.as_array())
                .is_some()
            || download_actions
            || !file_refs.is_empty();

        if !local_files.is_empty() {
            let upload_comment = if has_custom_rendering {
                None
            } else {
                Some(formatted_content.as_str())
            };
            self.upload_local_files(
                &channel_id,
                thread_ts.as_deref(),
                upload_comment,
                &local_files,
            )
            .await?;
            if !has_custom_rendering {
                debug!("Slack message sent via native file upload");
                return Ok(());
            }
        }

        if let Some(blocks) = blocks {
            payload["blocks"] = serde_json::json!(blocks);
        } else if download_actions && !file_refs.is_empty() {
            let buttons: Vec<_> = file_refs
                .iter()
                .take(5)
                .map(|(label, url)| {
                    serde_json::json!({
                        "type": "button",
                        "text": {
                            "type": "plain_text",
                            "text": label,
                        },
                        "url": url,
                    })
                })
                .collect();
            payload["blocks"] = serde_json::json!([
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": formatted_content,
                    }
                },
                {
                    "type": "actions",
                    "elements": buttons,
                }
            ]);
        }
        if let Some(attachments) = msg
            .metadata
            .get("slack_attachments")
            .and_then(|v| v.as_array())
        {
            payload["attachments"] = serde_json::json!(attachments);
        } else if !file_refs.is_empty() && !download_actions {
            let attachment_text = file_refs
                .iter()
                .map(|(_, url)| url.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            if !attachment_text.is_empty() {
                payload["attachments"] = serde_json::json!([{
                    "fallback": attachment_text,
                    "text": attachment_text,
                }]);
            }
        }
        if let Some(stream_mode) = msg
            .metadata
            .get("slack_stream_mode")
            .and_then(|v| v.as_str())
        {
            payload["metadata"] = serde_json::json!({
                "event_type": "openrustclaw_stream",
                "event_payload": {
                    "mode": stream_mode,
                }
            });
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
                    return Err(ChannelError::Config {
                        platform: "slack".to_string(),
                        message: "Slack app_token is required for Socket Mode".to_string(),
                    }
                    .into());
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
        if self.config.mode == SlackMode::SocketMode {
            self.start_socket_mode_loop().await?;
        }
        info!(mode = ?self.config.mode, "Slack channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Slack...");

        *self.is_connected.write().await = false;
        if let Some(task) = self.socket_task.lock().await.take() {
            task.abort();
        }

        info!("Slack channel disconnected");
        Ok(())
    }
}

/// Handle Slack Events API HTTP requests and enqueue normalized incoming messages.
#[derive(Clone)]
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
            "slack_stream_mode": if event_type == "app_mention" { "mention" } else { "message" },
            "slack_user_id": user_id,
        });
        if let Some(team_id) = team_id {
            metadata["slack_team_id"] = serde_json::json!(team_id);
        }
        if let Some(thread_ts) = event.get("thread_ts").and_then(|value| value.as_str()) {
            metadata["slack_thread_ts"] = serde_json::json!(thread_ts);
            metadata["slack_thread_owner_user_id"] = serde_json::json!(user_id);
        }
        if let Some(event_ts) = event.get("event_ts").and_then(|value| value.as_str()) {
            metadata["slack_event_ts"] = serde_json::json!(event_ts);
        }
        if let Some(files) = event.get("files").and_then(|value| value.as_array()) {
            metadata["slack_files"] = serde_json::json!(files);
            metadata["slack_attachment_count"] = serde_json::json!(files.len());
            let file_refs: Vec<_> = files
                .iter()
                .filter_map(|file| {
                    file.get("url_private_download")
                        .or_else(|| file.get("url_private"))
                        .and_then(|value| value.as_str())
                })
                .collect();
            if !file_refs.is_empty() {
                metadata["file_references"] = serde_json::json!(file_refs);
            }
        }

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            content: Self::normalize_slack_text(&content),
            platform: Platform::Slack,
            metadata,
        };

        self.enqueue_incoming(incoming).await
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

    async fn handle_socket_envelope(
        &self,
        envelope_type: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        match envelope_type {
            "events_api" => self.handle_event_callback(payload).await,
            "slash_commands" => self.handle_socket_slash_command(payload).await,
            "interactive" => self.handle_socket_interaction(payload).await,
            "hello" => Ok(()),
            other => {
                debug!(envelope_type = %other, "Ignoring unsupported Slack Socket Mode envelope");
                Ok(())
            }
        }
    }

    async fn handle_socket_slash_command(&self, payload: &serde_json::Value) -> Result<()> {
        let user_id = payload
            .get("user_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack slash command payload missing user_id".to_string(),
            })?;
        let channel_id = payload
            .get("channel_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack slash command payload missing channel_id".to_string(),
            })?;
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

        let command = payload
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let text = payload
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let content = if text.is_empty() {
            if command.is_empty() {
                "[slack slash command]".to_string()
            } else {
                command.to_string()
            }
        } else {
            text
        };

        let mut metadata = serde_json::json!({
            "slack_channel": channel_id,
            "slack_is_group": !channel_id.starts_with('D'),
            "slack_bot_mentioned": true,
            "slack_stream_mode": "slash_command",
            "slack_user_id": user_id,
            "slack_command": command,
            "slack_socket_mode": true,
        });
        if let Some(team_id) = team_id {
            metadata["slack_team_id"] = serde_json::json!(team_id);
        }
        if let Some(trigger_id) = payload.get("trigger_id").and_then(|value| value.as_str()) {
            metadata["slack_trigger_id"] = serde_json::json!(trigger_id);
        }
        if let Some(response_url) = payload.get("response_url").and_then(|value| value.as_str()) {
            metadata["slack_response_url"] = serde_json::json!(response_url);
        }

        self.enqueue_incoming(IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            content: Self::normalize_slack_text(&content),
            platform: Platform::Slack,
            metadata,
        })
        .await
    }

    async fn handle_socket_interaction(&self, payload: &serde_json::Value) -> Result<()> {
        let user = payload
            .get("user")
            .and_then(|value| value.as_object())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack interaction payload missing user".to_string(),
            })?;
        let user_id = user
            .get("id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack interaction payload missing user.id".to_string(),
            })?;
        let channel = payload
            .get("channel")
            .and_then(|value| value.as_object())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack interaction payload missing channel".to_string(),
            })?;
        let channel_id = channel
            .get("id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Slack interaction payload missing channel.id".to_string(),
            })?;
        let team_id = payload
            .get("team")
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_str());
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

        let interaction_type = payload
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("interactive");
        let actions = payload
            .get("actions")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let action_ids: Vec<_> = actions
            .iter()
            .filter_map(|action| action.get("action_id").and_then(|value| value.as_str()))
            .collect();
        let content = actions
            .iter()
            .find_map(|action| {
                action
                    .get("value")
                    .and_then(|value| value.as_str())
                    .or_else(|| {
                        action
                            .get("text")
                            .and_then(|value| value.get("text"))
                            .and_then(|value| value.as_str())
                    })
            })
            .unwrap_or("[slack interaction]")
            .to_string();

        let mut metadata = serde_json::json!({
            "slack_channel": channel_id,
            "slack_is_group": !channel_id.starts_with('D'),
            "slack_bot_mentioned": true,
            "slack_stream_mode": "interactive",
            "slack_user_id": user_id,
            "slack_socket_mode": true,
            "slack_interaction_type": interaction_type,
            "slack_action_ids": action_ids,
            "slack_action_count": actions.len(),
        });
        if let Some(team_id) = team_id {
            metadata["slack_team_id"] = serde_json::json!(team_id);
        }
        if let Some(trigger_id) = payload.get("trigger_id").and_then(|value| value.as_str()) {
            metadata["slack_trigger_id"] = serde_json::json!(trigger_id);
        }
        if let Some(response_url) = payload.get("response_url").and_then(|value| value.as_str()) {
            metadata["slack_response_url"] = serde_json::json!(response_url);
        }
        if let Some(container) = payload.get("container").and_then(|value| value.as_object())
            && let Some(message_ts) = container.get("message_ts").and_then(|value| value.as_str())
        {
            metadata["slack_event_ts"] = serde_json::json!(message_ts);
            metadata["slack_thread_ts"] = serde_json::json!(message_ts);
        }

        self.enqueue_incoming(IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            content: Self::normalize_slack_text(&content),
            platform: Platform::Slack,
            metadata,
        })
        .await
    }

    async fn enqueue_incoming(&self, incoming: IncomingMessage) -> Result<()> {
        self.incoming_tx
            .send(incoming)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "slack".to_string(),
                message: format!("Failed to enqueue Slack incoming message: {}", e),
            })?;

        Ok(())
    }
}

async fn run_socket_mode_loop<S>(
    mut stream: tokio_tungstenite::WebSocketStream<S>,
    handler: SlackEventHandler,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(message) = stream.next().await {
        match message.map_err(|e| ChannelError::Connection {
            platform: "slack".to_string(),
            message: format!("Slack Socket Mode websocket read failed: {}", e),
        })? {
            WsMessage::Text(text) => {
                let envelope: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| ChannelError::InvalidFormat {
                        platform: "slack".to_string(),
                        message: format!("Failed to parse Slack Socket Mode payload: {}", e),
                    })?;
                let envelope_type = envelope
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");

                if let Some(envelope_id) = envelope.get("envelope_id").and_then(|v| v.as_str()) {
                    stream
                        .send(WsMessage::Text(
                            serde_json::json!({ "envelope_id": envelope_id })
                                .to_string()
                                .into(),
                        ))
                        .await
                        .map_err(|e| ChannelError::Connection {
                            platform: "slack".to_string(),
                            message: format!("Failed to ACK Slack Socket Mode envelope: {}", e),
                        })?;
                }

                if envelope_type == "disconnect" || envelope_type == "connection_error" {
                    return Err(ChannelError::Connection {
                        platform: "slack".to_string(),
                        message: envelope
                            .get("reason")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Slack Socket Mode disconnected")
                            .to_string(),
                    }
                    .into());
                }

                let payload = envelope.get("payload").cloned().unwrap_or(envelope.clone());
                handler
                    .handle_socket_envelope(envelope_type, &payload)
                    .await?;
            }
            WsMessage::Ping(payload) => {
                stream.send(WsMessage::Pong(payload)).await.map_err(|e| {
                    ChannelError::Connection {
                        platform: "slack".to_string(),
                        message: format!("Failed to respond to Slack websocket ping: {}", e),
                    }
                })?;
            }
            WsMessage::Close(_) => {
                return Err(ChannelError::Connection {
                    platform: "slack".to_string(),
                    message: "Slack Socket Mode websocket closed".to_string(),
                }
                .into());
            }
            _ => {}
        }
    }

    Err(ChannelError::Connection {
        platform: "slack".to_string(),
        message: "Slack Socket Mode websocket ended".to_string(),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    async fn spawn_mock_slack_socket(
        envelopes: Vec<serde_json::Value>,
    ) -> (
        String,
        tokio::sync::oneshot::Receiver<Vec<serde_json::Value>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(tcp_stream).await.unwrap();
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "type": "hello",
                        "num_connections": 1
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let mut acks = Vec::new();
            for envelope in envelopes {
                ws_stream
                    .send(WsMessage::Text(envelope.to_string().into()))
                    .await
                    .unwrap();
                let ack = ws_stream
                    .next()
                    .await
                    .unwrap()
                    .unwrap()
                    .into_text()
                    .unwrap();
                acks.push(serde_json::from_str(&ack).unwrap());
            }
            let _ = ack_tx.send(acks);
            ws_stream.close(None).await.unwrap();
        });

        (format!("ws://{}/socket", addr), ack_rx)
    }

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
                        "event_ts":"171234.000200",
                        "files":[
                            {
                                "id":"F1",
                                "url_private":"https://files.example.com/file-1",
                                "url_private_download":"https://files.example.com/file-1/download"
                            }
                        ]
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
        assert_eq!(incoming.metadata["slack_attachment_count"], 1);
        assert_eq!(
            incoming.metadata["file_references"][0],
            "https://files.example.com/file-1/download"
        );
    }

    #[tokio::test]
    async fn test_socket_mode_events_api_enqueues_message() {
        use wiremock::matchers::{bearer_token, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let (ws_url, ack_rx) = spawn_mock_slack_socket(vec![serde_json::json!({
            "envelope_id": "1",
            "type": "events_api",
            "payload": {
                "team_id": "T123",
                "event": {
                    "type": "app_mention",
                    "user": "U123",
                    "text": "hello from socket",
                    "channel": "C456",
                    "event_ts": "171234.000200"
                }
            }
        })])
        .await;

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
            .and(path("/apps.connections.open"))
            .and(bearer_token("xapp-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "url": ws_url
            })))
            .mount(&server)
            .await;

        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: Some(server.uri()),
            app_token: Some("xapp-test".to_string()),
            signing_secret: None,
            mode: SlackMode::SocketMode,
            socket_mode: true,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        };
        let mut channel = SlackChannel::new(config);
        channel.connect().await.unwrap();

        let incoming = tokio::time::timeout(std::time::Duration::from_secs(2), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(incoming.content, "hello from socket");
        assert_eq!(incoming.metadata["slack_stream_mode"], "mention");
        assert_eq!(
            incoming.metadata["slack_socket_mode"],
            serde_json::Value::Null
        );

        let acks = ack_rx.await.unwrap();
        assert_eq!(acks[0]["envelope_id"], "1");

        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_socket_mode_slash_command_enqueues_message() {
        use wiremock::matchers::{bearer_token, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let (ws_url, ack_rx) = spawn_mock_slack_socket(vec![serde_json::json!({
            "envelope_id": "2",
            "type": "slash_commands",
            "payload": {
                "team_id": "T123",
                "channel_id": "C456",
                "user_id": "U123",
                "command": "/assign",
                "text": "take this",
                "trigger_id": "trigger-1",
                "response_url": "https://example.com/respond"
            }
        })])
        .await;

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
            .and(path("/apps.connections.open"))
            .and(bearer_token("xapp-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "url": ws_url
            })))
            .mount(&server)
            .await;

        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: Some(server.uri()),
            app_token: Some("xapp-test".to_string()),
            signing_secret: None,
            mode: SlackMode::SocketMode,
            socket_mode: true,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        };
        let mut channel = SlackChannel::new(config);
        channel.connect().await.unwrap();

        let incoming = tokio::time::timeout(std::time::Duration::from_secs(2), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(incoming.content, "take this");
        assert_eq!(incoming.metadata["slack_command"], "/assign");
        assert_eq!(incoming.metadata["slack_stream_mode"], "slash_command");
        assert_eq!(incoming.metadata["slack_socket_mode"], true);

        let acks = ack_rx.await.unwrap();
        assert_eq!(acks[0]["envelope_id"], "2");

        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_socket_mode_interactive_enqueues_message() {
        use wiremock::matchers::{bearer_token, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let (ws_url, ack_rx) = spawn_mock_slack_socket(vec![serde_json::json!({
            "envelope_id": "3",
            "type": "interactive",
            "payload": {
                "type": "block_actions",
                "team": {"id": "T123"},
                "user": {"id": "U123"},
                "channel": {"id": "C456"},
                "container": {"message_ts": "171234.000300"},
                "trigger_id": "trigger-2",
                "actions": [{
                    "action_id": "approve",
                    "value": "yes"
                }]
            }
        })])
        .await;

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
            .and(path("/apps.connections.open"))
            .and(bearer_token("xapp-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "url": ws_url
            })))
            .mount(&server)
            .await;

        let config = SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: Some(server.uri()),
            app_token: Some("xapp-test".to_string()),
            signing_secret: None,
            mode: SlackMode::SocketMode,
            socket_mode: true,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        };
        let mut channel = SlackChannel::new(config);
        channel.connect().await.unwrap();

        let incoming = tokio::time::timeout(std::time::Duration::from_secs(2), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(incoming.content, "yes");
        assert_eq!(incoming.metadata["slack_interaction_type"], "block_actions");
        assert_eq!(incoming.metadata["slack_action_ids"][0], "approve");
        assert_eq!(incoming.metadata["slack_socket_mode"], true);

        let acks = ack_rx.await.unwrap();
        assert_eq!(acks[0]["envelope_id"], "3");

        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_send_slack_file_reference_download_actions() {
        use wiremock::matchers::{bearer_token, body_string_contains, method, path};
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
            .and(body_string_contains("\"type\":\"actions\""))
            .and(body_string_contains("https://files.example.com/report.pdf"))
            .and(body_string_contains(
                "\"event_type\":\"openrustclaw_stream\"",
            ))
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
                content: "Download the report".to_string(),
                metadata: serde_json::json!({
                    "slack_channel": "C123",
                    "slack_team_id": "T123",
                    "slack_attachment_download_actions": true,
                    "slack_stream_mode": "draft",
                    "file_references": [{
                        "title": "Report",
                        "url": "https://files.example.com/report.pdf"
                    }]
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_slack_local_file_upload() {
        use wiremock::matchers::{bearer_token, body_string_contains, method, path};
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
            .and(path("/files.getUploadURLExternal"))
            .and(bearer_token("xoxb-test"))
            .and(body_string_contains("\"filename\":\"report.txt\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "upload_url": format!("{}/upload/report.txt", server.uri()),
                "file_id": "F123"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/upload/report.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/files.completeUploadExternal"))
            .and(bearer_token("xoxb-test"))
            .and(body_string_contains("\"channel_id\":\"C123\""))
            .and(body_string_contains("\"title\":\"Report\""))
            .and(body_string_contains(
                "\"initial_comment\":\"Attached report\"",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "files": [{"id":"F123"}]
            })))
            .mount(&server)
            .await;

        let attachment_path =
            std::env::temp_dir().join(format!("openrustclaw-slack-upload-{}.txt", Uuid::new_v4()));
        tokio::fs::write(&attachment_path, "report body")
            .await
            .unwrap();

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
                content: "Attached report".to_string(),
                metadata: serde_json::json!({
                    "slack_channel": "C123",
                    "slack_team_id": "T123",
                    "file_references": [{
                        "local_path": attachment_path,
                        "name": "report.txt",
                        "title": "Report"
                    }]
                }),
            })
            .await
            .unwrap();
        let _ = tokio::fs::remove_file(&attachment_path).await;
    }

    #[tokio::test]
    async fn test_send_slack_draft_stream_reply() {
        use wiremock::matchers::{bearer_token, body_string_contains, method, path};
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
            .and(path("/chat.startStream"))
            .and(bearer_token("xoxb-test"))
            .and(body_string_contains("\"channel\":\"C123\""))
            .and(body_string_contains("\"thread_ts\":\"171234.000100\""))
            .and(body_string_contains("\"recipient_user_id\":\"U123\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channel": "C123",
                "ts": "171234.000200"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat.appendStream"))
            .and(bearer_token("xoxb-test"))
            .and(body_string_contains("\"ts\":\"171234.000200\""))
            .and(body_string_contains("Second chunk"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat.stopStream"))
            .and(bearer_token("xoxb-test"))
            .and(body_string_contains("\"ts\":\"171234.000200\""))
            .and(body_string_contains(
                "\"event_type\":\"openrustclaw_stream\"",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channel": "C123",
                "ts": "171234.000200",
                "message": {"text": "done"}
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
                content: String::new(),
                metadata: serde_json::json!({
                    "slack_channel": "C123",
                    "slack_team_id": "T123",
                    "slack_thread_ts": "171234.000100",
                    "slack_thread_owner_user_id": "U123",
                    "slack_draft_stream": true,
                    "slack_stream_mode": "draft",
                    "slack_stream_chunks": [
                        "First chunk",
                        "Second chunk"
                    ]
                }),
            })
            .await
            .unwrap();
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
