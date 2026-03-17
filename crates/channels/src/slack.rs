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
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info, warn};

use openrustclaw_core::config::{SlackConfig, SlackMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Slack channel implementation.
pub struct SlackChannel {
    config: SlackConfig,
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
            _incoming_tx: incoming_tx,
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
        let _channel_str = msg
            .metadata
            .get("slack_channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "slack".to_string(),
                message: "Missing channel in metadata".to_string(),
            })?;

        // Format content for Slack
        let _formatted_content = Self::markdown_to_slack(&msg.content);

        // Check for thread_ts
        let _thread_ts = msg.metadata.get("slack_thread_ts").and_then(|v| v.as_str());

        // Check for blocks in metadata
        let _blocks = Self::parse_blocks(&msg.metadata);

        debug!(content = %msg.content, "Slack send requested before Web API client was implemented");

        Err(ChannelError::SendFailed {
            platform: "slack".to_string(),
            message: "Slack send path is not implemented yet".to_string(),
        }
        .into())
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

        info!(mode = ?self.config.mode, "Slack channel configuration validated");

        Err(ChannelError::Connection {
            platform: "slack".to_string(),
            message: "Slack runtime client is not implemented yet".to_string(),
        }
        .into())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Slack...");

        *self.is_connected.write().await = false;

        info!("Slack channel disconnected");
        Ok(())
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
}
