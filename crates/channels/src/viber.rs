//! Viber Channel Integration
//!
//! Uses Viber Bot API (https://developers.viber.com/)

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::ViberConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Viber channel implementation.
pub struct ViberChannel {
    config: ViberConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    http: reqwest::Client,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: RwLock<bool>,
    bot_info: RwLock<Option<ViberBotInfo>>,
}

/// Viber webhook payload (callback).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberCallback {
    pub event: String,
    pub timestamp: i64,
    #[serde(rename = "message_token")]
    pub message_token: Option<i64>,
    pub context: Option<String>,
    pub user_id: Option<String>,
    pub sender: Option<ViberSender>,
    pub message: Option<ViberMessage>,
    pub user: Option<ViberUser>,
    pub subscribed: Option<bool>,
}

/// Viber message sender.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberSender {
    pub id: String,
    pub name: String,
    #[serde(rename = "avatar")]
    pub avatar: Option<String>,
    pub country: Option<String>,
    pub language: Option<String>,
    #[serde(rename = "api_version")]
    pub api_version: Option<i32>,
}

/// Viber message.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: Option<String>,
    pub tracking_data: Option<String>,
    pub file_name: Option<String>,
    #[serde(rename = "file_size")]
    pub file_size: Option<i64>,
    pub duration: Option<i32>,
    pub sticker_id: Option<i64>,
}

/// Viber user (for conversation_started events).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberUser {
    pub id: String,
    pub name: String,
    #[serde(rename = "avatar")]
    pub avatar: Option<String>,
    pub country: Option<String>,
    pub language: Option<String>,
    #[serde(rename = "api_version")]
    pub api_version: Option<i32>,
}

/// Viber bot info response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberBotInfo {
    pub status: i32,
    pub status_message: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "avatar")]
    pub avatar: Option<String>,
    #[serde(rename = "category")]
    pub category: Option<String>,
    #[serde(rename = "subcategory")]
    pub subcategory: Option<String>,
    #[serde(rename = "event_types")]
    pub event_types: Option<Vec<String>>,
    #[serde(rename = "members_count")]
    pub members_count: Option<i64>,
    #[serde(rename = "welcome_screen")]
    pub welcome_screen: Option<ViberWelcomeScreen>,
}

/// Viber welcome screen.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberWelcomeScreen {
    #[serde(rename = "BackgroundColor")]
    pub background_color: Option<String>,
    #[serde(rename = "Text")]
    pub text: Option<String>,
    #[serde(rename = "TextColor")]
    pub text_color: Option<String>,
    #[serde(rename = "Opacity")]
    pub opacity: Option<i32>,
}

/// Viber API response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ViberApiResponse {
    pub status: i32,
    #[serde(rename = "status_message")]
    pub status_message: String,
}

/// Viber keyboard button.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ViberButton {
    #[serde(rename = "ActionType")]
    pub action_type: String,
    #[serde(rename = "ActionBody")]
    pub action_body: String,
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "TextSize")]
    pub text_size: Option<String>,
    #[serde(rename = "BgColor")]
    pub bg_color: Option<String>,
}

/// Viber keyboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ViberKeyboard {
    #[serde(rename = "Type")]
    pub keyboard_type: String,
    #[serde(rename = "DefaultHeight")]
    pub default_height: bool,
    #[serde(rename = "Buttons")]
    pub buttons: Vec<ViberButton>,
}

impl ViberChannel {
    /// Create a new Viber channel with the given configuration.
    pub fn new(config: ViberConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Viber: 300 messages/minute per bot)
        let quota = Quota::per_minute(
            NonZeroU32::new(config.rate_limit_per_minute.max(1))
                .unwrap_or(NonZeroU32::new(300).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            http,
            rate_limiter,
            is_connected: RwLock::new(false),
            bot_info: RwLock::new(None),
        }
    }

    /// Check if a user is allowed to interact with the bot.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&user_id.to_string())
    }

    /// Get bot information from Viber API.
    pub async fn get_account_info(&self) -> Result<ViberBotInfo> {
        let url = "https://chatapi.viber.com/pa/get_account_info";

        let payload = serde_json::json!({
            "auth_token": self.config.auth_token
        });

        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "viber".to_string(),
                message: e.to_string(),
            })?;

        let info: ViberBotInfo =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;

        if info.status != 0 {
            return Err(ChannelError::AuthFailed {
                platform: "viber".to_string(),
                message: info.status_message,
            }
            .into());
        }

        Ok(info)
    }

    /// Set webhook URL for receiving callbacks.
    pub async fn set_webhook(&self, url: &str, event_types: Option<Vec<&str>>) -> Result<()> {
        let endpoint = "https://chatapi.viber.com/pa/set_webhook";

        let events = event_types.unwrap_or_else(|| {
            vec![
                "delivered",
                "seen",
                "failed",
                "subscribed",
                "unsubscribed",
                "conversation_started",
                "message",
            ]
        });

        let payload = serde_json::json!({
            "auth_token": self.config.auth_token,
            "url": url,
            "event_types": events,
            "send_name": true,
            "send_photo": true
        });

        let response = self
            .http
            .post(endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "viber".to_string(),
                message: e.to_string(),
            })?;

        let result: ViberApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;

        if result.status != 0 {
            return Err(ChannelError::Config {
                platform: "viber".to_string(),
                message: result.status_message,
            }
            .into());
        }

        info!(webhook_url = %url, "Viber webhook set successfully");
        Ok(())
    }

    /// Handle incoming Viber callback.
    pub async fn handle_callback(&self, callback: ViberCallback) -> Result<()> {
        match callback.event.as_str() {
            "message" => {
                if let (Some(sender), Some(message)) = (callback.sender, callback.message) {
                    // Check allowlist
                    if !self.is_user_allowed(&sender.id) {
                        warn!(user_id = %sender.id, "User not in allowlist, ignoring message");
                        return Ok(());
                    }

                    let content = match message.message_type.as_str() {
                        "text" => message.text.unwrap_or_default(),
                        "picture" => "[Picture]".to_string(),
                        "video" => "[Video]".to_string(),
                        "file" => format!("[File: {}]", message.file_name.unwrap_or_default()),
                        "contact" => "[Contact]".to_string(),
                        "location" => "[Location]".to_string(),
                        "url" => "[URL]".to_string(),
                        "sticker" => "[Sticker]".to_string(),
                        _ => "[Unknown message type]".to_string(),
                    };

                    let session_id = Uuid::new_v4();
                    let metadata = serde_json::json!({
                        "viber_user_name": sender.name,
                        "viber_user_avatar": sender.avatar,
                        "viber_user_country": sender.country,
                        "viber_user_language": sender.language,
                        "viber_message_type": message.message_type,
                        "viber_tracking_data": message.tracking_data,
                    });

                    let msg = IncomingMessage {
                        session_id,
                        user_id: sender.id,
                        content,
                        platform: Platform::Viber,
                        metadata,
                    };

                    if let Err(e) = self.incoming_tx.send(msg).await {
                        error!("Failed to send incoming message: {}", e);
                    }
                }
            }
            "conversation_started" => {
                // User opened chat for the first time
                if let Some(user) = callback.user {
                    info!(user_id = %user.id, name = %user.name, "Viber conversation started");

                    // Send welcome message if configured
                    if let Some(ref welcome) = self.config.welcome_message {
                        let _ = self.send_to_user(&user.id, welcome, None).await;
                    }
                }
            }
            "subscribed" => {
                if let Some(user_id) = callback.user_id {
                    info!(user_id = %user_id, "User subscribed to Viber bot");
                }
            }
            "unsubscribed" => {
                if let Some(user_id) = callback.user_id {
                    info!(user_id = %user_id, "User unsubscribed from Viber bot");
                }
            }
            "delivered" => {
                debug!(message_token = ?callback.message_token, "Message delivered");
            }
            "seen" => {
                debug!(user_id = ?callback.user_id, "Message seen");
            }
            "failed" => {
                warn!(message_token = ?callback.message_token, context = ?callback.context, "Message delivery failed");
            }
            _ => {
                debug!(event = %callback.event, "Unhandled Viber event");
            }
        }

        Ok(())
    }

    /// Send a message to a user.
    async fn send_to_user(
        &self,
        user_id: &str,
        text: &str,
        keyboard: Option<ViberKeyboard>,
    ) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = "https://chatapi.viber.com/pa/send_message";

        let mut payload = serde_json::json!({
            "auth_token": self.config.auth_token,
            "receiver": user_id,
            "type": "text",
            "text": text,
            "min_api_version": 1
        });

        if let Some(kb) = keyboard {
            payload["keyboard"] =
                serde_json::to_value(kb).map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;
        }

        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: e.to_string(),
            })?;

        let result: ViberApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;

        if result.status != 0 {
            return Err(ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: result.status_message,
            }
            .into());
        }

        Ok(())
    }

    /// Send a picture message.
    pub async fn send_picture(
        &self,
        user_id: &str,
        image_url: &str,
        caption: Option<&str>,
    ) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = "https://chatapi.viber.com/pa/send_message";

        let payload = serde_json::json!({
            "auth_token": self.config.auth_token,
            "receiver": user_id,
            "type": "picture",
            "text": caption.unwrap_or(""),
            "media": image_url,
            "min_api_version": 1
        });

        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: e.to_string(),
            })?;

        let result: ViberApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;

        if result.status != 0 {
            return Err(ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: result.status_message,
            }
            .into());
        }

        Ok(())
    }

    /// Broadcast a message to all subscribers (admin only).
    pub async fn broadcast_message(&self, text: &str) -> Result<()> {
        if !self.config.allow_broadcast {
            return Err(ChannelError::PermissionDenied {
                platform: "viber".to_string(),
                message: "Broadcast not enabled for this bot".to_string(),
            }
            .into());
        }

        self.rate_limiter.until_ready().await;

        let url = "https://chatapi.viber.com/pa/broadcast_message";

        let payload = serde_json::json!({
            "auth_token": self.config.auth_token,
            "type": "text",
            "text": text,
            "min_api_version": 1
        });

        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: e.to_string(),
            })?;

        let result: ViberApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "viber".to_string(),
                    message: e.to_string(),
                })?;

        if result.status != 0 {
            return Err(ChannelError::SendFailed {
                platform: "viber".to_string(),
                message: result.status_message,
            }
            .into());
        }

        Ok(())
    }

    /// Build keyboard from metadata.
    fn build_keyboard(&self, metadata: &serde_json::Value) -> Option<ViberKeyboard> {
        if !self.config.enable_keyboards {
            return None;
        }

        metadata.get("keyboard").and_then(|kb| {
            let buttons: Vec<ViberButton> = kb
                .get("buttons")?
                .as_array()?
                .iter()
                .filter_map(|btn| {
                    Some(ViberButton {
                        action_type: btn.get("action_type")?.as_str()?.to_string(),
                        action_body: btn.get("action_body")?.as_str()?.to_string(),
                        text: btn.get("text")?.as_str()?.to_string(),
                        text_size: btn
                            .get("text_size")
                            .and_then(|v| v.as_str().map(String::from)),
                        bg_color: btn
                            .get("bg_color")
                            .and_then(|v| v.as_str().map(String::from)),
                    })
                })
                .collect();

            if buttons.is_empty() {
                return None;
            }

            Some(ViberKeyboard {
                keyboard_type: kb.get("type")?.as_str()?.to_string(),
                default_height: kb
                    .get("default_height")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                buttons,
            })
        })
    }
}

#[async_trait]
impl Channel for ViberChannel {
    fn platform(&self) -> Platform {
        Platform::Viber
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract recipient ID from metadata
        let recipient_id = msg
            .metadata
            .get("viber_user_id")
            .and_then(|v| v.as_str())
            .or_else(|| msg.metadata.get("recipient_id").and_then(|v| v.as_str()))
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "viber".to_string(),
                message: "Missing recipient_id in metadata".to_string(),
            })?;

        // Check for keyboard in metadata
        let keyboard = self.build_keyboard(&msg.metadata);

        // Send message
        self.send_to_user(recipient_id, &msg.content, keyboard)
            .await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "viber".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Viber Bot API...");

        // Validate token
        if self.config.auth_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "viber".to_string(),
                message: "Viber auth token is required".to_string(),
            }
            .into());
        }

        // Get account info to verify token
        let info = self.get_account_info().await?;
        info!(bot_name = %info.name, "Connected to Viber Bot API");

        // Store bot info
        *self.bot_info.write().await = Some(info);

        // Set webhook if URL is configured
        if let Some(ref webhook_url) = self.config.webhook_url {
            self.set_webhook(webhook_url, None).await?;
        }

        *self.is_connected.write().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Viber...");

        // Remove webhook
        if self.config.webhook_url.is_some() {
            let _ = self.set_webhook("", None).await;
        }

        *self.is_connected.write().await = false;
        *self.bot_info.write().await = None;

        info!("Viber channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = ViberConfig {
            enabled: true,
            auth_token: "test_token".to_string(),
            webhook_url: None,
            webhook_path: "/webhook/viber".to_string(),
            allowlist: vec![],
            rate_limit_per_minute: 300,
            allow_broadcast: false,
            welcome_message: None,
            enable_keyboards: true,
        };
        let channel = ViberChannel::new(config);
        assert!(channel.is_user_allowed("abc123xyz"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = ViberConfig {
            enabled: true,
            auth_token: "test_token".to_string(),
            webhook_url: None,
            webhook_path: "/webhook/viber".to_string(),
            allowlist: vec!["user1".to_string(), "user2".to_string()],
            rate_limit_per_minute: 300,
            allow_broadcast: false,
            welcome_message: None,
            enable_keyboards: true,
        };
        let channel = ViberChannel::new(config);
        assert!(channel.is_user_allowed("user1"));
        assert!(channel.is_user_allowed("user2"));
        assert!(!channel.is_user_allowed("user3"));
    }
}
