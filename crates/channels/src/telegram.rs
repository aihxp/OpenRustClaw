//! Telegram Bot API integration for OpenRustClaw.
//!
//! Features:
//! - Text messages
//! - Commands (/start, /help, etc.)
//! - Inline queries
//! - File uploads/downloads
//! - Group chats
//! - Reply keyboards
//! - Webhook or polling mode

use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use reqwest::Client;
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{TelegramConfig, TelegramMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Telegram channel implementation.
pub struct TelegramChannel {
    config: TelegramConfig,
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
    polling_task: Mutex<Option<JoinHandle<()>>>,
    bot_username: RwLock<Option<String>>,
}

impl TelegramChannel {
    /// Create a new Telegram channel with the given configuration.
    pub fn new(config: TelegramConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(30).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            client: Client::new(),
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            polling_task: Mutex::new(None),
            bot_username: RwLock::new(None),
        }
    }

    /// Check if a user is allowed to interact with the bot.
    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: i64) -> bool {
        if self.config.allowed_users.is_empty() {
            return true;
        }
        let user_id_str = user_id.to_string();
        self.config.allowed_users.contains(&user_id_str)
    }

    /// Parse inline keyboard from metadata.
    fn parse_inline_keyboard(metadata: &serde_json::Value) -> Option<serde_json::Value> {
        metadata.get("inline_keyboard").cloned()
    }

    fn api_base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://api.telegram.org".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn chat_id_from_metadata(metadata: &serde_json::Value) -> Result<String> {
        if let Some(value) = metadata.get("telegram_chat_id") {
            if let Some(chat_id) = value.as_str() {
                return Ok(chat_id.to_string());
            }
            if let Some(chat_id) = value.as_i64() {
                return Ok(chat_id.to_string());
            }
            if let Some(chat_id) = value.as_u64() {
                return Ok(chat_id.to_string());
            }
        }

        Err(ChannelError::InvalidFormat {
            platform: "telegram".to_string(),
            message: "Missing chat_id in metadata".to_string(),
        }
        .into())
    }

    async fn spawn_polling_task(&self) -> Result<()> {
        let mut task_guard = self.polling_task.lock().await;
        if task_guard.is_some() {
            return Ok(());
        }

        let client = self.client.clone();
        let incoming_tx = self.incoming_tx.clone();
        let token = self.config.token.clone();
        let api_base_url = self.api_base_url();
        let allowed_users = self.config.allowed_users.clone();
        let bot_username = self.bot_username.read().await.clone();

        *task_guard = Some(tokio::spawn(async move {
            let mut next_offset: i64 = 0;

            loop {
                let response = client
                    .get(format!("{}/bot{}/getUpdates", api_base_url, token))
                    .query(&[
                        ("timeout", "30"),
                        ("offset", &next_offset.to_string()),
                        ("allowed_updates", "[\"message\"]"),
                    ])
                    .send()
                    .await;

                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        warn!(error = %error, "Telegram polling request failed");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let status = response.status();
                let body: serde_json::Value = match response.json().await {
                    Ok(body) => body,
                    Err(error) => {
                        warn!(error = %error, "Failed to parse Telegram polling response");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                    warn!(status = %status, body = %body, "Telegram polling returned failure");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                let updates = body
                    .get("result")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default();

                for update in updates {
                    if let Some(update_id) =
                        update.get("update_id").and_then(|value| value.as_i64())
                    {
                        next_offset = update_id + 1;
                    }

                    let Some(message) = update.get("message") else {
                        continue;
                    };

                    let Some(from) = message.get("from") else {
                        continue;
                    };

                    let user_id = from
                        .get("id")
                        .and_then(|value| value.as_i64())
                        .map(|value| value.to_string())
                        .unwrap_or_default();

                    if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
                        debug!(user_id = %user_id, "Skipping Telegram user outside allowlist");
                        continue;
                    }

                    let content = message
                        .get("text")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string)
                        .or_else(|| {
                            message
                                .get("caption")
                                .and_then(|value| value.as_str())
                                .map(ToString::to_string)
                        })
                        .or_else(|| {
                            message.get("poll").map(|poll| {
                                let question = poll
                                    .get("question")
                                    .and_then(|value| value.as_str())
                                    .unwrap_or("Poll");
                                let options = poll
                                    .get("options")
                                    .and_then(|value| value.as_array())
                                    .map(|values| {
                                        values
                                            .iter()
                                            .filter_map(|option| {
                                                option
                                                    .get("text")
                                                    .and_then(|value| value.as_str())
                                            })
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                    .unwrap_or_default();
                                if options.is_empty() {
                                    format!("[Poll] {question}")
                                } else {
                                    format!("[Poll] {question} Options: {options}")
                                }
                            })
                        })
                        .or_else(|| {
                            if message.get("photo").is_some() {
                                Some("[Image]".to_string())
                            } else if message.get("document").is_some() {
                                Some("[Document]".to_string())
                            } else if message.get("audio").is_some() {
                                Some("[Audio]".to_string())
                            } else if message.get("voice").is_some() {
                                Some("[Voice message]".to_string())
                            } else if message.get("video").is_some() {
                                Some("[Video]".to_string())
                            } else {
                                None
                            }
                        });

                    let Some(content) = content else {
                        continue;
                    };

                    let chat_id = message
                        .get("chat")
                        .and_then(|chat| chat.get("id"))
                        .and_then(|value| value.as_i64())
                        .map(|value| value.to_string())
                        .unwrap_or_default();

                    let mut metadata = serde_json::json!({
                        "telegram_chat_id": chat_id,
                        "telegram_chat_type": message
                            .get("chat")
                            .and_then(|chat| chat.get("type"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("private"),
                        "telegram_is_group": message
                            .get("chat")
                            .and_then(|chat| chat.get("type"))
                            .and_then(|value| value.as_str())
                            .map(|value| matches!(value, "group" | "supergroup"))
                            .unwrap_or(false),
                    });
                    if let Some(message_id) =
                        message.get("message_id").and_then(|value| value.as_i64())
                    {
                        metadata["telegram_message_id"] = serde_json::json!(message_id);
                    }
                    if let Some(username) = from.get("username").and_then(|value| value.as_str()) {
                        metadata["telegram_username"] = serde_json::json!(username);
                    }
                    if let Some(reply_to) = message
                        .get("reply_to_message")
                        .and_then(|value| value.get("message_id"))
                        .and_then(|value| value.as_i64())
                    {
                        metadata["telegram_reply_to_message_id"] = serde_json::json!(reply_to);
                    }
                    if let Some(thread_id) = message
                        .get("message_thread_id")
                        .and_then(|value| value.as_i64())
                    {
                        metadata["telegram_message_thread_id"] = serde_json::json!(thread_id);
                    }
                    if let Some(poll) = message.get("poll") {
                        metadata["telegram_poll"] = poll.clone();
                    }
                    if let Some(photo) = message.get("photo").and_then(|value| value.as_array()) {
                        metadata["telegram_media_type"] = serde_json::json!("image");
                        metadata["telegram_file_references"] = serde_json::json!(photo
                            .iter()
                            .filter_map(|entry| entry.get("file_id").and_then(|value| value.as_str()))
                            .collect::<Vec<_>>());
                    } else if let Some(document) = message.get("document") {
                        metadata["telegram_media_type"] = serde_json::json!("document");
                        metadata["telegram_file_references"] = serde_json::json!([document
                            .get("file_id")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()]);
                    } else if let Some(audio) = message.get("audio") {
                        metadata["telegram_media_type"] = serde_json::json!("audio");
                        metadata["telegram_file_references"] = serde_json::json!([audio
                            .get("file_id")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()]);
                    } else if let Some(voice) = message.get("voice") {
                        metadata["telegram_media_type"] = serde_json::json!("voice");
                        metadata["telegram_file_references"] = serde_json::json!([voice
                            .get("file_id")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()]);
                    } else if let Some(video) = message.get("video") {
                        metadata["telegram_media_type"] = serde_json::json!("video");
                        metadata["telegram_file_references"] = serde_json::json!([video
                            .get("file_id")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()]);
                    }
                    if let Some(bot_username) = bot_username.as_deref() {
                        metadata["telegram_bot_mentioned"] =
                            serde_json::json!(content.contains(&format!("@{bot_username}")));
                    }

                    let incoming = IncomingMessage {
                        session_id: Uuid::new_v4(),
                        user_id,
                        content,
                        platform: Platform::Telegram,
                        metadata,
                    };

                    if let Err(error) = incoming_tx.send(incoming).await {
                        error!(error = %error, "Failed to enqueue Telegram incoming message");
                        return;
                    }
                }
            }
        }));

        Ok(())
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn platform(&self) -> Platform {
        Platform::Telegram
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "telegram".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract chat ID from session metadata
        let chat_id = Self::chat_id_from_metadata(&msg.metadata)?;

        // Parse inline keyboard from metadata
        let reply_markup = Self::parse_inline_keyboard(&msg.metadata);
        let media_type = msg
            .metadata
            .get("telegram_media_type")
            .and_then(|value| value.as_str());
        let media_ref = msg
            .metadata
            .get("telegram_media_url")
            .or_else(|| msg.metadata.get("telegram_media_path"))
            .or_else(|| {
                msg.metadata
                    .get("file_references")
                    .and_then(|value| value.as_array())
                    .and_then(|values| values.first())
            })
            .and_then(|value| value.as_str());
        let is_poll = msg.metadata.get("telegram_poll_options").and_then(|value| value.as_array());
        let endpoint = if is_poll.is_some() {
            "sendPoll"
        } else {
            match media_type {
                Some("image") => "sendPhoto",
                Some("document") => "sendDocument",
                Some("audio") | Some("voice") => "sendAudio",
                Some("video") => "sendVideo",
                _ => "sendMessage",
            }
        };
        let url = format!("{}/bot{}/{}", self.api_base_url(), self.config.token, endpoint);
        let mut payload = serde_json::json!({
            "chat_id": chat_id,
        });
        if let Some(options) = is_poll {
            payload["question"] = serde_json::json!(msg.content);
            payload["options"] = serde_json::json!(options);
        } else if let (Some(kind), Some(reference)) = (media_type, media_ref) {
            let field = match kind {
                "image" => "photo",
                "document" => "document",
                "audio" | "voice" => "audio",
                "video" => "video",
                _ => "text",
            };
            payload[field] = serde_json::json!(reference);
            if !msg.content.is_empty() {
                payload["caption"] = serde_json::json!(msg.content);
            }
        } else {
            payload["text"] = serde_json::json!(msg.content);
        }
        if let Some(reply_to) = msg
            .metadata
            .get("telegram_reply_to_message_id")
            .and_then(|value| value.as_i64())
        {
            payload["reply_to_message_id"] = serde_json::json!(reply_to);
        }
        if let Some(thread_id) = msg
            .metadata
            .get("telegram_message_thread_id")
            .and_then(|value| value.as_i64())
        {
            payload["message_thread_id"] = serde_json::json!(thread_id);
        }
        if let Some(reply_markup) = reply_markup {
            payload["reply_markup"] = reply_markup;
        }

        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "telegram".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "telegram".to_string(),
                    message: format!("failed to parse Telegram response: {}", e),
                })?;

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = body
                .get("parameters")
                .and_then(|v| v.get("retry_after"))
                .and_then(|v| v.as_u64());
            return Err(ChannelError::RateLimited {
                platform: "telegram".to_string(),
                retry_after_secs: retry_after,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::SendFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram send failed")
                    .to_string(),
            }
            .into());
        }

        debug!("Telegram message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "telegram".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Telegram Bot API...");

        // Validate token
        if self.config.token.is_empty() {
            return Err(ChannelError::Config {
                platform: "telegram".to_string(),
                message: "Telegram bot token is required".to_string(),
            }
            .into());
        }

        let url = format!("{}/bot{}/getMe", self.api_base_url(), self.config.token);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "telegram".to_string(),
                message: e.to_string(),
            })?;
        let status = response.status();
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "telegram".to_string(),
                    message: format!("failed to parse Telegram auth response: {}", e),
                })?;

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram authentication probe failed")
                    .to_string(),
            }
            .into());
        }

        if let Some(username) = body
            .get("result")
            .and_then(|value| value.get("username"))
            .and_then(|value| value.as_str())
        {
            *self.bot_username.write().await = Some(username.to_string());
        }

        if self.config.mode == TelegramMode::Polling {
            self.spawn_polling_task().await?;
        }

        *self.is_connected.write().await = true;
        info!(mode = ?self.config.mode, "Telegram channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Telegram...");

        *self.is_connected.write().await = false;
        if let Some(task) = self.polling_task.lock().await.take() {
            task.abort();
        }

        info!("Telegram channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "test".to_string(),
            api_base_url: None,
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let channel = TelegramChannel::new(config);
        assert!(channel.is_user_allowed(12345));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "test".to_string(),
            api_base_url: None,
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec!["12345".to_string(), "67890".to_string()],
            rate_limit_per_second: 30,
        };
        let channel = TelegramChannel::new(config);
        assert!(channel.is_user_allowed(12345));
        assert!(channel.is_user_allowed(67890));
        assert!(!channel.is_user_allowed(11111));
    }

    #[tokio::test]
    async fn test_connect_and_send_via_bot_api() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/sendMessage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"message_id": 42}
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello".to_string(),
                metadata: serde_json::json!({"telegram_chat_id": "123"}),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_polling_receive_enqueues_message() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getUpdates"))
            .and(query_param("timeout", "30"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": [{
                    "update_id": 100,
                    "message": {
                        "message_id": 7,
                        "text": "hello from telegram",
                        "chat": {"id": 12345},
                        "from": {"id": 999, "username": "alice"}
                    }
                }]
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        let incoming = tokio::time::timeout(tokio::time::Duration::from_secs(2), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(incoming.content, "hello from telegram");
        assert_eq!(incoming.user_id, "999");
        assert_eq!(incoming.metadata["telegram_chat_id"], "12345");
        channel.disconnect().await.unwrap();
    }
}
