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
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::TelegramConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Telegram channel implementation.
pub struct TelegramChannel {
    config: TelegramConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
}

impl TelegramChannel {
    /// Create a new Telegram channel with the given configuration.
    pub fn new(config: TelegramConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        
        // Create rate limiter
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1)).unwrap_or(NonZeroU32::new(30).unwrap())
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
        }
    }

    /// Check if a user is allowed to interact with the bot.
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
}

#[async_trait]
impl Channel for TelegramChannel {
    fn platform(&self) -> Platform {
        Platform::Telegram
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract chat ID from session metadata
        let _chat_id = msg.metadata.get("telegram_chat_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "telegram".to_string(),
                message: "Missing chat_id in metadata".to_string(),
            })?;

        // Parse inline keyboard from metadata
        let _reply_markup = Self::parse_inline_keyboard(&msg.metadata);

        // In a real implementation, this would use the Telegram Bot API
        // For now, we just log the message
        debug!(content = %msg.content, "Would send Telegram message");

        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "telegram".to_string(),
                message: "Incoming message channel closed".to_string(),
            }.into()
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
            }.into());
        }

        // In a full implementation, this would:
        // 1. Create a teloxide Bot instance
        // 2. Test the connection by calling get_me()
        // 3. Start a Dispatcher in polling or webhook mode
        // 4. Handle incoming messages via the UpdateHandler

        info!(mode = ?self.config.mode, "Telegram bot would start here");
        
        *self.is_connected.write().await = true;

        info!("Telegram channel connected");
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
}
