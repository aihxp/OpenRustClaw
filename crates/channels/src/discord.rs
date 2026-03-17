//! Discord Bot integration for OpenRustClaw.
//!
//! Features:
//! - Text messages in channels
//! - Direct messages
//! - Slash commands
//! - Embeds
//! - File attachments
//! - Reactions
//! - Threads
//! - Gateway connection management
//! - Rate limiting

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use openrustclaw_core::config::DiscordConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Discord channel implementation.
pub struct DiscordChannel {
    config: DiscordConfig,
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
    _message_cache: Arc<RwLock<HashMap<Uuid, String>>>, // Maps session_id to message_id
}

impl DiscordChannel {
    /// Create a new Discord channel with the given configuration.
    pub fn new(config: DiscordConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Discord allows ~5 requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(5).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            _incoming_tx: incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            _message_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Parse embeds from metadata.
    fn parse_embed(metadata: &serde_json::Value) -> Option<serde_json::Value> {
        metadata.get("embed").cloned()
    }

    /// Convert platform-specific formatting to Discord markdown.
    fn format_for_discord(text: &str) -> String {
        // Discord uses standard markdown
        text.to_string()
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn platform(&self) -> Platform {
        Platform::Discord
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "discord".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get channel ID from metadata
        let _channel_id_str = msg
            .metadata
            .get("discord_channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Missing channel_id in metadata".to_string(),
            })?;

        // Check if we should send as embed
        let _formatted_content = Self::format_for_discord(&msg.content);
        let _embed = Self::parse_embed(&msg.metadata);

        debug!(content = %msg.content, "Discord send requested before Gateway/API client was implemented");

        Err(ChannelError::SendFailed {
            platform: "discord".to_string(),
            message: "Discord send path is not implemented yet".to_string(),
        }
        .into())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "discord".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Discord Gateway...");

        // Validate token
        if self.config.token.is_empty() {
            return Err(ChannelError::Config {
                platform: "discord".to_string(),
                message: "Discord bot token is required".to_string(),
            }
            .into());
        }

        info!("Discord channel configuration validated");

        Err(ChannelError::Connection {
            platform: "discord".to_string(),
            message: "Discord runtime client is not implemented yet".to_string(),
        }
        .into())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Discord...");

        *self.is_connected.write().await = false;

        info!("Discord channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_for_discord() {
        let text = "Hello **world**";
        let formatted = DiscordChannel::format_for_discord(text);
        assert_eq!(formatted, "Hello **world**");
    }
}
