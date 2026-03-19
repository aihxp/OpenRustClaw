//! Messaging platform integrations for OpenRustClaw.
//!
//! Supports:
//! - WebChat (built-in WebSocket-based chat)
//! - Telegram (Bot API via teloxide)
//! - Discord (Bot Gateway via serenity)
//! - Slack (App API via slack-morphism)
//! - Microsoft Teams (Bot Framework)
//! - Google Chat (Google Workspace Chat API)
//! - Google Meet (operator/event integration for spaces, conference records, and artifacts)
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
pub mod google_meet;
pub mod imessage;
pub mod line;
pub mod mattermost;
pub mod matrix;
pub mod meta;
pub mod signal;
pub mod slack;
pub mod teams;
pub mod telegram;
pub mod viber;
pub mod webchat;
pub mod wechat;
pub mod whatsapp;

// Note: These modules are work-in-progress and not yet fully integrated
// pub mod twilio;
// pub mod x_twitter;

pub use discord::DiscordChannel;
pub use gmail_pubsub::GmailPubSub;
pub use google_chat::GoogleChatChannel;
pub use google_meet::{DecodedGoogleMeetEvent, GoogleMeetClient, GoogleMeetWebhookHandler};
pub use imessage::IMessageChannel;
pub use line::LineChannel;
pub use mattermost::{MattermostChannel, MattermostWebhookHandler};
pub use matrix::MatrixChannel;
pub use meta::MetaChannel;
pub use signal::SignalChannel;
pub use slack::SlackChannel;
pub use teams::TeamsChannel;
pub use telegram::TelegramChannel;
pub use viber::ViberChannel;
pub use webchat::WebChatChannel;
pub use wechat::WeChatChannel;
pub use whatsapp::{ConnectionState, MediaMetadata, MediaType, WhatsAppChannel};

use openrustclaw_core::config::ChannelsConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;

/// Factory for creating channel instances based on configuration.
pub struct ChannelFactory;

impl ChannelFactory {
    /// Create enabled channels based on configuration.
    ///
    /// This method instantiates all channels that are enabled in the configuration
    /// and returns them as a vector of boxed trait objects.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use openrustclaw_channels::ChannelFactory;
    /// use openrustclaw_core::config::AppConfig;
    ///
    /// let config = AppConfig::default();
    /// let channels = ChannelFactory::create_channels(&config.channels);
    /// ```
    pub fn create_channels(config: &ChannelsConfig) -> Vec<Box<dyn Channel>> {
        let mut channels: Vec<Box<dyn Channel>> = Vec::new();

        if config.telegram.enabled {
            tracing::info!("Creating Telegram channel");
            match Self::create_telegram(config.telegram.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Telegram channel: {}", e),
            }
        }

        if config.discord.enabled {
            tracing::info!("Creating Discord channel");
            match Self::create_discord(config.discord.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Discord channel: {}", e),
            }
        }

        if config.slack.enabled {
            tracing::info!("Creating Slack channel");
            match Self::create_slack(config.slack.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Slack channel: {}", e),
            }
        }

        if config.teams.enabled {
            tracing::info!("Creating Microsoft Teams channel");
            match Self::create_teams(config.teams.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Teams channel: {}", e),
            }
        }

        if config.mattermost.enabled {
            tracing::info!("Creating Mattermost channel");
            match Self::create_mattermost(config.mattermost.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Mattermost channel: {}", e),
            }
        }

        if config.google_chat.enabled {
            tracing::info!("Creating Google Chat channel");
            match Self::create_google_chat(config.google_chat.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Google Chat channel: {}", e),
            }
        }

        if config.whatsapp.enabled {
            tracing::info!("Creating WhatsApp channel");
            match Self::create_whatsapp(config.whatsapp.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create WhatsApp channel: {}", e),
            }
        }

        if config.gmail_pubsub.enabled {
            tracing::info!("Creating Gmail Pub/Sub channel");
            match Self::create_gmail_pubsub(config.gmail_pubsub.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Gmail Pub/Sub channel: {}", e),
            }
        }

        if config.signal.enabled {
            tracing::info!("Creating Signal channel");
            match Self::create_signal(config.signal.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Signal channel: {}", e),
            }
        }

        if config.matrix.enabled {
            tracing::info!("Creating Matrix channel");
            match Self::create_matrix(config.matrix.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Matrix channel: {}", e),
            }
        }

        if config.meta.enabled {
            tracing::info!("Creating Meta channel");
            match Self::create_meta(config.meta.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Meta channel: {}", e),
            }
        }

        if config.line.enabled {
            tracing::info!("Creating LINE channel");
            match Self::create_line(config.line.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create LINE channel: {}", e),
            }
        }

        if config.viber.enabled {
            tracing::info!("Creating Viber channel");
            match Self::create_viber(config.viber.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create Viber channel: {}", e),
            }
        }

        if config.wechat.enabled {
            tracing::info!("Creating WeChat channel");
            match Self::create_wechat(config.wechat.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create WeChat channel: {}", e),
            }
        }

        if config.imessage.enabled {
            tracing::info!("Creating iMessage channel");
            match Self::create_imessage(config.imessage.clone()) {
                Ok(channel) => channels.push(Box::new(channel)),
                Err(e) => tracing::error!("Failed to create iMessage channel: {}", e),
            }
        }

        channels
    }

    /// Create a Telegram channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_telegram(
        config: openrustclaw_core::config::TelegramConfig,
    ) -> Result<TelegramChannel> {
        if config.token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "telegram".to_string(),
                    message: "bot token is required".to_string(),
                },
            ));
        }
        Ok(TelegramChannel::new(config))
    }

    /// Create a Discord channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_discord(
        config: openrustclaw_core::config::DiscordConfig,
    ) -> Result<DiscordChannel> {
        if config.token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "discord".to_string(),
                    message: "bot token is required".to_string(),
                },
            ));
        }
        Ok(DiscordChannel::new(config))
    }

    /// Create a Slack channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_slack(config: openrustclaw_core::config::SlackConfig) -> Result<SlackChannel> {
        if config.token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "slack".to_string(),
                    message: "token is required".to_string(),
                },
            ));
        }
        Ok(SlackChannel::new(config))
    }

    /// Create a Microsoft Teams channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_teams(config: openrustclaw_core::config::TeamsConfig) -> Result<TeamsChannel> {
        if config.app_id.is_empty() || config.app_password.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "teams".to_string(),
                    message: "app_id and app_password are required".to_string(),
                },
            ));
        }
        Ok(TeamsChannel::new(config))
    }

    /// Create a Mattermost channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_mattermost(
        config: openrustclaw_core::config::MattermostConfig,
    ) -> Result<MattermostChannel> {
        if config.server_url.is_empty() || config.bot_token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "mattermost".to_string(),
                    message: "server_url and bot_token are required".to_string(),
                },
            ));
        }
        Ok(MattermostChannel::new(config))
    }

    /// Create a Google Chat channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_google_chat(
        config: openrustclaw_core::config::GoogleChatConfig,
    ) -> Result<GoogleChatChannel> {
        if config.service_account_key.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "google_chat".to_string(),
                    message: "service_account_key is required".to_string(),
                },
            ));
        }
        Ok(GoogleChatChannel::new(config))
    }

    /// Create a Google Meet operator client.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_google_meet(
        config: openrustclaw_core::config::GoogleMeetConfig,
    ) -> Result<GoogleMeetClient> {
        if config.service_account_key_path.is_empty() || config.delegated_user_email.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "google_meet".to_string(),
                    message: "service_account_key_path and delegated_user_email are required"
                        .to_string(),
                },
            ));
        }
        Ok(GoogleMeetClient::new(config))
    }

    /// Create a WhatsApp channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_whatsapp(
        config: openrustclaw_core::config::WhatsAppConfig,
    ) -> Result<WhatsAppChannel> {
        Ok(WhatsAppChannel::new(config))
    }

    /// Create a Gmail Pub/Sub channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_gmail_pubsub(
        config: openrustclaw_core::config::GmailPubSubConfig,
    ) -> Result<GmailPubSub> {
        if config.project_id.is_empty() || config.service_account_key_path.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "gmail_pubsub".to_string(),
                    message: "project_id and service_account_key_path are required".to_string(),
                },
            ));
        }
        Ok(GmailPubSub::new(config))
    }

    /// Create a Signal channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_signal(config: openrustclaw_core::config::SignalConfig) -> Result<SignalChannel> {
        if config.phone_number.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "signal".to_string(),
                    message: "phone_number is required".to_string(),
                },
            ));
        }
        Ok(SignalChannel::new(config))
    }

    /// Create a Matrix channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_matrix(config: openrustclaw_core::config::MatrixConfig) -> Result<MatrixChannel> {
        if config.homeserver.is_empty() || config.user_id.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "matrix".to_string(),
                    message: "homeserver and user_id are required".to_string(),
                },
            ));
        }
        Ok(MatrixChannel::new(config))
    }

    /// Create a Meta (Messenger/Instagram) channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_meta(config: openrustclaw_core::config::MetaConfig) -> Result<MetaChannel> {
        if config.app_secret.is_empty() || config.page_access_token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "meta".to_string(),
                    message: "app_secret and page_access_token are required".to_string(),
                },
            ));
        }
        Ok(MetaChannel::new(config))
    }

    /// Create a LINE channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_line(config: openrustclaw_core::config::LineConfig) -> Result<LineChannel> {
        if config.channel_access_token.is_empty() || config.channel_secret.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "line".to_string(),
                    message: "channel_access_token and channel_secret are required".to_string(),
                },
            ));
        }
        Ok(LineChannel::new(config))
    }

    /// Create a Viber channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_viber(config: openrustclaw_core::config::ViberConfig) -> Result<ViberChannel> {
        if config.auth_token.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "viber".to_string(),
                    message: "auth_token is required".to_string(),
                },
            ));
        }
        Ok(ViberChannel::new(config))
    }

    /// Create a WeChat channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_wechat(config: openrustclaw_core::config::WeChatConfig) -> Result<WeChatChannel> {
        if config.app_id.is_empty() || config.app_secret.is_empty() {
            return Err(openrustclaw_core::error::Error::Channel(
                ChannelError::Config {
                    platform: "wechat".to_string(),
                    message: "app_id and app_secret are required".to_string(),
                },
            ));
        }
        Ok(WeChatChannel::new(config))
    }

    /// Create an iMessage channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn create_imessage(
        config: openrustclaw_core::config::IMessageConfig,
    ) -> Result<IMessageChannel> {
        Ok(IMessageChannel::new(config))
    }
}

/// Channel types that can be started via CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    /// WebChat channel
    WebChat,
    /// Telegram channel
    Telegram,
    /// Discord channel
    Discord,
    /// Slack channel
    Slack,
    /// Microsoft Teams channel
    Teams,
    /// Mattermost channel
    Mattermost,
    /// Google Chat channel
    GoogleChat,
    /// WhatsApp channel
    WhatsApp,
    /// Gmail Pub/Sub channel
    Gmail,
    /// Signal channel
    Signal,
    /// Matrix channel
    Matrix,
    /// iMessage channel
    IMessage,
    /// LINE channel
    Line,
    /// Viber channel
    Viber,
    /// WeChat channel
    WeChat,
    /// Meta Messenger channel
    Messenger,
    /// Meta Instagram channel
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
            "mattermost" => Ok(ChannelType::Mattermost),
            "googlechat" | "google_chat" => Ok(ChannelType::GoogleChat),
            "whatsapp" | "whats_app" => Ok(ChannelType::WhatsApp),
            "gmail" | "gmail_pubsub" => Ok(ChannelType::Gmail),
            "signal" => Ok(ChannelType::Signal),
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
            ChannelType::Mattermost => write!(f, "mattermost"),
            ChannelType::GoogleChat => write!(f, "google_chat"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::Gmail => write!(f, "gmail"),
            ChannelType::Signal => write!(f, "signal"),
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
///
/// # Example
///
/// ```
/// use openrustclaw_channels::parse_channels_list;
/// use openrustclaw_channels::ChannelType;
///
/// let channels = parse_channels_list("telegram,discord").unwrap();
/// assert_eq!(channels.len(), 2);
/// ```
pub fn parse_channels_list(s: &str) -> std::result::Result<Vec<ChannelType>, String> {
    s.split(',').map(|s| s.trim().parse()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_channels_list() {
        let result = parse_channels_list("telegram,discord,slack,teams,mattermost,google_chat,whatsapp,gmail,signal,matrix,imessage,line,viber,wechat,messenger,instagram").expect("valid channel list");
        assert_eq!(result.len(), 16);
        assert_eq!(result[0], ChannelType::Telegram);
        assert_eq!(result[1], ChannelType::Discord);
        assert_eq!(result[2], ChannelType::Slack);
        assert_eq!(result[3], ChannelType::Teams);
        assert_eq!(result[4], ChannelType::Mattermost);
        assert_eq!(result[5], ChannelType::GoogleChat);
        assert_eq!(result[6], ChannelType::WhatsApp);
        assert_eq!(result[7], ChannelType::Gmail);
        assert_eq!(result[8], ChannelType::Signal);
        assert_eq!(result[9], ChannelType::Matrix);
        assert_eq!(result[10], ChannelType::IMessage);
        assert_eq!(result[11], ChannelType::Line);
        assert_eq!(result[12], ChannelType::Viber);
        assert_eq!(result[13], ChannelType::WeChat);
        assert_eq!(result[14], ChannelType::Messenger);
        assert_eq!(result[15], ChannelType::Instagram);
    }

    #[test]
    fn test_parse_channels_list_with_spaces() {
        let result = parse_channels_list("telegram, discord , slack").expect("valid channel list");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::Telegram.to_string(), "telegram");
        assert_eq!(ChannelType::Discord.to_string(), "discord");
        assert_eq!(ChannelType::Slack.to_string(), "slack");
        assert_eq!(ChannelType::Teams.to_string(), "teams");
        assert_eq!(ChannelType::Mattermost.to_string(), "mattermost");
        assert_eq!(ChannelType::GoogleChat.to_string(), "google_chat");
        assert_eq!(ChannelType::WhatsApp.to_string(), "whatsapp");
        assert_eq!(ChannelType::WebChat.to_string(), "webchat");
        assert_eq!(ChannelType::Gmail.to_string(), "gmail");
        assert_eq!(ChannelType::Signal.to_string(), "signal");
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
            "google_chat".parse::<ChannelType>().expect("valid channel"),
            ChannelType::GoogleChat
        );
        assert_eq!(
            "googlechat".parse::<ChannelType>().expect("valid channel"),
            ChannelType::GoogleChat
        );
    }

    #[test]
    fn test_parse_whatsapp_channel_type() {
        assert_eq!(
            "whatsapp".parse::<ChannelType>().expect("valid channel"),
            ChannelType::WhatsApp
        );
        assert_eq!(
            "whats_app".parse::<ChannelType>().expect("valid channel"),
            ChannelType::WhatsApp
        );
    }

    #[test]
    fn test_parse_matrix_channel_type() {
        assert_eq!(
            "matrix".parse::<ChannelType>().expect("valid channel"),
            ChannelType::Matrix
        );
    }

    #[test]
    fn test_parse_imessage_channel_type() {
        assert_eq!(
            "imessage".parse::<ChannelType>().expect("valid channel"),
            ChannelType::IMessage
        );
        assert_eq!(
            "i_message".parse::<ChannelType>().expect("valid channel"),
            ChannelType::IMessage
        );
    }

    #[test]
    fn test_parse_messenger_channel_type() {
        assert_eq!(
            "messenger".parse::<ChannelType>().expect("valid channel"),
            ChannelType::Messenger
        );
        assert_eq!(
            "meta_messenger"
                .parse::<ChannelType>()
                .expect("valid channel"),
            ChannelType::Messenger
        );
    }

    #[test]
    fn test_parse_instagram_channel_type() {
        assert_eq!(
            "instagram".parse::<ChannelType>().expect("valid channel"),
            ChannelType::Instagram
        );
        assert_eq!(
            "meta_instagram"
                .parse::<ChannelType>()
                .expect("valid channel"),
            ChannelType::Instagram
        );
    }
}
