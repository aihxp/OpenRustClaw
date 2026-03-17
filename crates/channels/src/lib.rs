//! Messaging platform integrations for OpenRustClaw.
//!
//! Supports:
//! - WebChat (built-in WebSocket-based chat)
//! - Telegram (Bot API via teloxide)
//! - Discord (Bot Gateway via serenity)
//! - Slack (App API via slack-morphism)

pub mod discord;
pub mod slack;
pub mod telegram;
pub mod webchat;

pub use discord::DiscordChannel;
pub use slack::SlackChannel;
pub use telegram::TelegramChannel;
pub use webchat::WebChatChannel;

use openrustclaw_core::config::{ChannelsConfig, TelegramConfig, DiscordConfig, SlackConfig};
use openrustclaw_core::traits::Channel;
use std::sync::Arc;

/// Factory for creating channel instances based on configuration.
pub struct ChannelFactory;

impl ChannelFactory {
    /// Create enabled channels based on configuration.
    pub fn create_channels(config: &ChannelsConfig) -> Vec<Box<dyn Channel>> {
        let mut _channels: Vec<Box<dyn Channel>> = Vec::new();

        // Note: These are placeholder implementations since the channel constructors
        // need mutable self for connect(). In production, you'd use Arc<Mutex<dyn Channel>>
        // or interior mutability patterns.
        
        if config.telegram.enabled {
            tracing::info!("Telegram channel enabled");
        }

        if config.discord.enabled {
            tracing::info!("Discord channel enabled");
        }

        if config.slack.enabled {
            tracing::info!("Slack channel enabled");
        }

        _channels
    }

    /// Create a Telegram channel.
    pub fn create_telegram(config: TelegramConfig) -> TelegramChannel {
        TelegramChannel::new(config)
    }

    /// Create a Discord channel.
    pub fn create_discord(config: DiscordConfig) -> DiscordChannel {
        DiscordChannel::new(config)
    }

    /// Create a Slack channel.
    pub fn create_slack(config: SlackConfig) -> SlackChannel {
        SlackChannel::new(config)
    }
}

/// Channel types that can be started via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    WebChat,
    Telegram,
    Discord,
    Slack,
}

impl std::str::FromStr for ChannelType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "webchat" | "web_chat" => Ok(ChannelType::WebChat),
            "telegram" => Ok(ChannelType::Telegram),
            "discord" => Ok(ChannelType::Discord),
            "slack" => Ok(ChannelType::Slack),
            _ => Err(format!("Unknown channel type: {}", s)),
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::WebChat => write!(f, "webchat"),
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::Slack => write!(f, "slack"),
        }
    }
}

/// Parse a comma-separated list of channel types.
pub fn parse_channels_list(s: &str) -> Result<Vec<ChannelType>, String> {
    s.split(',')
        .map(|s| s.trim().parse())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_channels_list() {
        let result = parse_channels_list("telegram,discord,slack").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ChannelType::Telegram);
        assert_eq!(result[1], ChannelType::Discord);
        assert_eq!(result[2], ChannelType::Slack);
    }

    #[test]
    fn test_parse_channels_list_with_spaces() {
        let result = parse_channels_list("telegram, discord , slack").unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::Telegram.to_string(), "telegram");
        assert_eq!(ChannelType::Discord.to_string(), "discord");
        assert_eq!(ChannelType::Slack.to_string(), "slack");
        assert_eq!(ChannelType::WebChat.to_string(), "webchat");
    }
}
