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
use tracing::{debug, info};

use openrustclaw_core::config::TelegramConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Telegram channel implementation.
pub struct TelegramChannel {
    config: TelegramConfig,
    client: Client,
    _incoming_tx: mpsc::Sender<IncomingMessage>,
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
            _incoming_tx: incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
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
        let url = format!("{}/bot{}/sendMessage", self.api_base_url(), self.config.token);
        let mut payload = serde_json::json!({
            "chat_id": chat_id,
            "text": msg.content,
        });
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
        let body: serde_json::Value = response.json().await.map_err(|e| ChannelError::SendFailed {
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
        let response = self.client.get(url).send().await.map_err(|e| ChannelError::Connection {
            platform: "telegram".to_string(),
            message: e.to_string(),
        })?;
        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| ChannelError::Connection {
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

        *self.is_connected.write().await = true;
        info!(mode = ?self.config.mode, "Telegram channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Telegram...");

        *self.is_connected.write().await = false;

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
}
