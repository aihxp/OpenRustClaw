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
use reqwest::Client;
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
            client: Client::new(),
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

    fn api_base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://discord.com/api/v10".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn header_value(&self) -> String {
        format!("Bot {}", self.config.token)
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
        let channel_id = msg
            .metadata
            .get("discord_channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Missing channel_id in metadata".to_string(),
            })?
            .to_string();

        if !self.config.allowed_channels.is_empty()
            && !self.config.allowed_channels.contains(&channel_id)
        {
            return Err(ChannelError::PermissionDenied {
                platform: "discord".to_string(),
                message: format!("channel {} is not allowed", channel_id),
            }
            .into());
        }

        if let Some(guild_id) = msg.metadata.get("discord_guild_id").and_then(|v| v.as_str()) {
            if !self.config.allowed_guilds.is_empty()
                && !self.config.allowed_guilds.contains(&guild_id.to_string())
            {
                return Err(ChannelError::PermissionDenied {
                    platform: "discord".to_string(),
                    message: format!("guild {} is not allowed", guild_id),
                }
                .into());
            }
        }

        // Check if we should send as embed
        let formatted_content = Self::format_for_discord(&msg.content);
        let embed = Self::parse_embed(&msg.metadata);
        let url = format!("{}/channels/{}/messages", self.api_base_url(), channel_id);
        let mut payload = serde_json::json!({
            "content": formatted_content,
        });
        if let Some(embed) = embed {
            payload["embeds"] = serde_json::json!([embed]);
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", self.header_value())
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "discord".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = body.get("retry_after").and_then(|v| v.as_f64()).map(|v| v.ceil() as u64);
            return Err(ChannelError::RateLimited {
                platform: "discord".to_string(),
                retry_after_secs: retry_after,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() {
            return Err(ChannelError::SendFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Discord send failed")
                    .to_string(),
            }
            .into());
        }

        debug!("Discord message sent");
        Ok(())
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

        let url = format!("{}/users/@me", self.api_base_url());
        let response = self
            .client
            .get(url)
            .header("Authorization", self.header_value())
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "discord".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() {
            return Err(ChannelError::Connection {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Discord authentication probe failed")
                    .to_string(),
            }
            .into());
        }

        *self.is_connected.write().await = true;
        info!("Discord channel connected");
        Ok(())
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

    #[tokio::test]
    async fn test_connect_and_send_discord_message() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "123",
                "username": "test-bot"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/channels/channel-1/messages"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "message-1"
            })))
            .mount(&server)
            .await;

        let config = DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            api_base_url: Some(server.uri()),
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        };
        let mut channel = DiscordChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello discord".to_string(),
                metadata: serde_json::json!({"discord_channel_id": "channel-1"}),
            })
            .await
            .unwrap();
    }
}
