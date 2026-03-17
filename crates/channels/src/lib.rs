//! Messaging platform integrations for OpenRustClaw.
//!
//! Supports:
//! - WebChat (built-in WebSocket-based chat)
//! - Telegram (Bot API via teloxide)
//! - Discord (Bot Gateway via serenity)
//! - Slack (App API via slack-morphism)
//! - Microsoft Teams (Bot Framework)
//! - Google Chat (Google Workspace Chat API)
//! - WhatsApp (Web via Baileys bridge)
//! - Gmail Pub/Sub (Gmail push notifications)
//! - Matrix (Matrix protocol via matrix-rust-sdk)
//! - Meta (Messenger & Instagram Direct)
//! - iMessage (macOS BlueBubbles or AppleScript)
//! - LINE (LINE Messaging API)
//! - Viber (Viber Bot API)
//! - WeChat (WeChat Work & Official Accounts API)

pub mod commands;
pub mod discord;
pub mod gmail_pubsub;
pub mod google_chat;
pub mod imessage;
pub mod line;
pub mod matrix;
pub mod meta;
pub mod slack;
pub mod teams;
pub mod telegram;
pub mod viber;
pub mod webchat;
pub mod wechat;
pub mod whatsapp;

// Note: These modules are work-in-progress and not yet fully integrated
// pub mod signal;
// pub mod twilio;
// pub mod x_twitter;

pub use discord::DiscordChannel;
pub use gmail_pubsub::GmailPubSub;
pub use google_chat::GoogleChatChannel;
pub use imessage::IMessageChannel;
pub use line::LineChannel;
pub use matrix::MatrixChannel;
pub use meta::MetaChannel;
pub use slack::SlackChannel;
pub use teams::TeamsChannel;
pub use telegram::TelegramChannel;
pub use viber::ViberChannel;
pub use webchat::WebChatChannel;
pub use wechat::WeChatChannel;
pub use whatsapp::{ConnectionState, MediaMetadata, MediaType, WhatsAppChannel};

use openrustclaw_core::config::{ChannelsConfig, DiscordConfig, GmailPubSubConfig, GoogleChatConfig, IMessageConfig, LineConfig, MatrixConfig, MetaConfig, SlackConfig, TeamsConfig, TelegramConfig, ViberConfig, WeChatConfig, WhatsAppConfig};
use openrustclaw_core::traits::Channel;

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

        if config.teams.enabled {
            tracing::info!("Microsoft Teams channel enabled");
        }

        if config.google_chat.enabled {
            tracing::info!("Google Chat channel enabled");
        }

        if config.whatsapp.enabled {
            tracing::info!("WhatsApp channel enabled");
        }

        if config.gmail_pubsub.enabled {
            tracing::info!("Gmail Pub/Sub channel enabled");
        }

        if config.matrix.enabled {
            tracing::info!("Matrix channel enabled");
        }

        if config.meta.enabled {
            tracing::info!("Meta channel enabled");
        }

        if config.line.enabled {
            tracing::info!("LINE channel enabled");
        }

        if config.viber.enabled {
            tracing::info!("Viber channel enabled");
        }

        if config.wechat.enabled {
            tracing::info!("WeChat channel enabled");
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

    /// Create a Microsoft Teams channel.
    pub fn create_teams(config: TeamsConfig) -> TeamsChannel {
        TeamsChannel::new(config)
    }

    /// Create a Google Chat channel.
    pub fn create_google_chat(config: GoogleChatConfig) -> GoogleChatChannel {
        GoogleChatChannel::new(config)
    }

    /// Create a WhatsApp channel.
    pub fn create_whatsapp(config: WhatsAppConfig) -> WhatsAppChannel {
        WhatsAppChannel::new(config)
    }

    /// Create a Gmail Pub/Sub channel.
    pub fn create_gmail_pubsub(config: GmailPubSubConfig) -> GmailPubSub {
        GmailPubSub::new(config)
    }

    /// Create a Matrix channel.
    pub fn create_matrix(config: MatrixConfig) -> MatrixChannel {
        MatrixChannel::new(config)
    }

    /// Create an iMessage channel.
    pub fn create_imessage(config: IMessageConfig) -> IMessageChannel {
        IMessageChannel::new(config)
    }

    /// Create a LINE channel.
    pub fn create_line(config: LineConfig) -> LineChannel {
        LineChannel::new(config)
    }

    /// Create a Viber channel.
    pub fn create_viber(config: ViberConfig) -> ViberChannel {
        ViberChannel::new(config)
    }

    /// Create a WeChat channel.
    pub fn create_wechat(config: WeChatConfig) -> WeChatChannel {
        WeChatChannel::new(config)
    }
}

/// Channel types that can be started via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    WebChat,
    Telegram,
    Discord,
    Slack,
    Teams,
    GoogleChat,
    WhatsApp,
    Gmail,
    Matrix,
    IMessage,
    Line,
    Viber,
    WeChat,
    Messenger,
    Instagram,
}

impl std::str::FromStr for ChannelType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "webchat" | "web_chat" => Ok(ChannelType::WebChat),
            "telegram" => Ok(ChannelType::Telegram),
            "discord" => Ok(ChannelType::Discord),
            "slack" => Ok(ChannelType::Slack),
            "teams" | "microsoft_teams" => Ok(ChannelType::Teams),
            "googlechat" | "google_chat" => Ok(ChannelType::GoogleChat),
            "whatsapp" | "whats_app" => Ok(ChannelType::WhatsApp),
            "gmail" | "gmail_pubsub" => Ok(ChannelType::Gmail),
            "matrix" => Ok(ChannelType::Matrix),
            "imessage" | "i_message" => Ok(ChannelType::IMessage),
            "line" => Ok(ChannelType::Line),
            "viber" => Ok(ChannelType::Viber),
            "wechat" | "we_chat" => Ok(ChannelType::WeChat),
            "messenger" | "meta_messenger" => Ok(ChannelType::Messenger),
            "instagram" | "meta_instagram" => Ok(ChannelType::Instagram),
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
            ChannelType::Teams => write!(f, "teams"),
            ChannelType::GoogleChat => write!(f, "google_chat"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::Gmail => write!(f, "gmail"),
            ChannelType::Matrix => write!(f, "matrix"),
            ChannelType::IMessage => write!(f, "imessage"),
            ChannelType::Line => write!(f, "line"),
            ChannelType::Viber => write!(f, "viber"),
            ChannelType::WeChat => write!(f, "wechat"),
            ChannelType::Messenger => write!(f, "messenger"),
            ChannelType::Instagram => write!(f, "instagram"),
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
        let result = parse_channels_list("telegram,discord,slack,teams,google_chat,whatsapp,gmail,matrix,imessage,line,viber,wechat,messenger,instagram").unwrap();
        assert_eq!(result.len(), 14);
        assert_eq!(result[0], ChannelType::Telegram);
        assert_eq!(result[1], ChannelType::Discord);
        assert_eq!(result[2], ChannelType::Slack);
        assert_eq!(result[3], ChannelType::Teams);
        assert_eq!(result[4], ChannelType::GoogleChat);
        assert_eq!(result[5], ChannelType::WhatsApp);
        assert_eq!(result[6], ChannelType::Gmail);
        assert_eq!(result[7], ChannelType::Matrix);
        assert_eq!(result[8], ChannelType::IMessage);
        assert_eq!(result[9], ChannelType::Line);
        assert_eq!(result[10], ChannelType::Viber);
        assert_eq!(result[11], ChannelType::WeChat);
        assert_eq!(result[12], ChannelType::Messenger);
        assert_eq!(result[13], ChannelType::Instagram);
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
        assert_eq!(ChannelType::Teams.to_string(), "teams");
        assert_eq!(ChannelType::GoogleChat.to_string(), "google_chat");
        assert_eq!(ChannelType::WhatsApp.to_string(), "whatsapp");
        assert_eq!(ChannelType::WebChat.to_string(), "webchat");
        assert_eq!(ChannelType::Gmail.to_string(), "gmail");
        assert_eq!(ChannelType::Matrix.to_string(), "matrix");
        assert_eq!(ChannelType::IMessage.to_string(), "imessage");
        assert_eq!(ChannelType::Line.to_string(), "line");
        assert_eq!(ChannelType::Viber.to_string(), "viber");
        assert_eq!(ChannelType::WeChat.to_string(), "wechat");
        assert_eq!(ChannelType::Messenger.to_string(), "messenger");
        assert_eq!(ChannelType::Instagram.to_string(), "instagram");
    }

    #[test]
    fn test_parse_google_chat_channel_type() {
        assert_eq!(
            "google_chat".parse::<ChannelType>().unwrap(),
            ChannelType::GoogleChat
        );
        assert_eq!(
            "googlechat".parse::<ChannelType>().unwrap(),
            ChannelType::GoogleChat
        );
    }

    #[test]
    fn test_parse_whatsapp_channel_type() {
        assert_eq!(
            "whatsapp".parse::<ChannelType>().unwrap(),
            ChannelType::WhatsApp
        );
        assert_eq!(
            "whats_app".parse::<ChannelType>().unwrap(),
            ChannelType::WhatsApp
        );
    }

    #[test]
    fn test_parse_matrix_channel_type() {
        assert_eq!(
            "matrix".parse::<ChannelType>().unwrap(),
            ChannelType::Matrix
        );
    }

    #[test]
    fn test_parse_imessage_channel_type() {
        assert_eq!(
            "imessage".parse::<ChannelType>().unwrap(),
            ChannelType::IMessage
        );
        assert_eq!(
            "i_message".parse::<ChannelType>().unwrap(),
            ChannelType::IMessage
        );
    }

    #[test]
    fn test_parse_messenger_channel_type() {
        assert_eq!(
            "messenger".parse::<ChannelType>().unwrap(),
            ChannelType::Messenger
        );
        assert_eq!(
            "meta_messenger".parse::<ChannelType>().unwrap(),
            ChannelType::Messenger
        );
    }

    #[test]
    fn test_parse_instagram_channel_type() {
        assert_eq!(
            "instagram".parse::<ChannelType>().unwrap(),
            ChannelType::Instagram
        );
        assert_eq!(
            "meta_instagram".parse::<ChannelType>().unwrap(),
            ChannelType::Instagram
        );
    }
}
