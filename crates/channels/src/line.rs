//! LINE Channel Integration
//!
//! Uses LINE Messaging API (https://developers.line.biz/)

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::LineConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// LINE channel implementation.
pub struct LineChannel {
    config: LineConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    http: reqwest::Client,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
}

/// LINE webhook payload.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineWebhook {
    pub events: Vec<LineEvent>,
}

/// LINE event.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: i64,
    pub source: LineSource,
    #[serde(rename = "message")]
    pub message: Option<LineMessage>,
    pub postback: Option<LinePostback>,
    #[serde(rename = "replyToken")]
    pub reply_token: Option<String>,
    pub mode: String,
    #[serde(rename = "webhookEventId")]
    pub webhook_event_id: String,
}

/// LINE message source.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineSource {
    User { user_id: String },
    Group { group_id: String, user_id: Option<String> },
    Room { room_id: String, user_id: Option<String> },
}

/// LINE message.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: Option<String>,
}

/// LINE postback data.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LinePostback {
    pub data: String,
}

/// LINE user profile.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineProfile {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "pictureUrl")]
    pub picture_url: Option<String>,
    #[serde(rename = "statusMessage")]
    pub status_message: Option<String>,
}

/// LINE API error response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LineApiError {
    pub message: String,
    pub details: Option<Vec<serde_json::Value>>,
}

impl LineChannel {
    /// Create a new LINE channel with the given configuration.
    pub fn new(config: LineConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        
        // Create rate limiter (LINE default: 1000 messages/second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1)).unwrap_or(NonZeroU32::new(1000).unwrap())
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
        }
    }

    /// Check if a user is allowed to interact with the bot.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&user_id.to_string())
    }

    /// Verify webhook signature using HMAC-SHA256.
    pub fn verify_signature(&self, body: &[u8], signature: &str) -> Result<()> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.channel_secret.as_bytes())
            .map_err(|_| ChannelError::AuthFailed {
                platform: "line".to_string(),
                message: "Invalid channel secret".to_string(),
            })?;
        mac.update(body);

        let result = mac.finalize();
        let expected = hex::encode(result.into_bytes());

        if signature != expected {
            return Err(ChannelError::AuthFailed {
                platform: "line".to_string(),
                message: "Invalid webhook signature".to_string(),
            }.into());
        }

        Ok(())
    }

    /// Get user profile from LINE API.
    pub async fn get_profile(&self, user_id: &str) -> Result<LineProfile> {
        let url = format!("https://api.line.me/v2/bot/profile/{}", user_id);
        
        let response = self.http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "line".to_string(),
                message: error_text,
            }.into());
        }

        let profile: LineProfile = response.json().await.map_err(|e| ChannelError::InvalidFormat {
            platform: "line".to_string(),
            message: e.to_string(),
        })?;

        Ok(profile)
    }

    /// Reply to a message using reply token.
    pub async fn reply(&self, reply_token: &str, text: &str) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = "https://api.line.me/v2/bot/message/reply";
        
        let payload = serde_json::json!({
            "replyToken": reply_token,
            "messages": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        });

        let response = self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "line".to_string(),
                message: error,
            }.into());
        }

        Ok(())
    }

    /// Handle incoming LINE webhook events.
    pub async fn handle_webhook(&self, payload: LineWebhook) -> Result<()> {
        for event in payload.events {
            // Get user ID based on source
            let (user_id, group_id) = match &event.source {
                LineSource::User { user_id } => (user_id.clone(), None),
                LineSource::Group { group_id, user_id } => {
                    (user_id.clone().unwrap_or_else(|| group_id.clone()), Some(group_id.clone()))
                }
                LineSource::Room { room_id, user_id } => {
                    (user_id.clone().unwrap_or_else(|| room_id.clone()), Some(room_id.clone()))
                }
            };

            // Check allowlist
            if !self.is_user_allowed(&user_id) {
                warn!(user_id = %user_id, "User not in allowlist, ignoring message");
                continue;
            }

            // Handle message events
            if event.event_type == "message" {
                if let Some(message) = event.message {
                    let content = match message.message_type.as_str() {
                        "text" => message.text.unwrap_or_default(),
                        "image" => "[Image]".to_string(),
                        "video" => "[Video]".to_string(),
                        "audio" => "[Audio]".to_string(),
                        "file" => "[File]".to_string(),
                        "location" => "[Location]".to_string(),
                        "sticker" => "[Sticker]".to_string(),
                        _ => "[Unknown message type]".to_string(),
                    };

                    let session_id = Uuid::new_v4();
                    let mut metadata = serde_json::json!({
                        "line_message_id": message.id,
                        "line_webhook_event_id": event.webhook_event_id,
                        "line_event_type": event.event_type,
                    });

                    if let Some(token) = &event.reply_token {
                        metadata["line_reply_token"] = serde_json::json!(token);
                    }

                    if let Some(group) = &group_id {
                        metadata["line_group_id"] = serde_json::json!(group);
                    }

                    let msg = IncomingMessage {
                        session_id,
                        user_id: user_id.clone(),
                        content,
                        platform: Platform::Line,
                        metadata,
                    };

                    if let Err(e) = self.incoming_tx.send(msg).await {
                        error!("Failed to send incoming message: {}", e);
                    }
                }
            }

            // Handle postback (button click)
            if event.event_type == "postback" {
                if let Some(postback) = event.postback {
                    let session_id = Uuid::new_v4();
                    let mut metadata = serde_json::json!({
                        "line_webhook_event_id": event.webhook_event_id,
                        "line_event_type": "postback",
                        "line_postback_data": &postback.data,
                    });

                    if let Some(token) = &event.reply_token {
                        metadata["line_reply_token"] = serde_json::json!(token);
                    }

                    let msg = IncomingMessage {
                        session_id,
                        user_id: user_id.clone(),
                        content: format!("[Postback: {}]", postback.data),
                        platform: Platform::Line,
                        metadata,
                    };

                    if let Err(e) = self.incoming_tx.send(msg).await {
                        error!("Failed to send incoming postback: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Send a sticker message.
    pub async fn send_sticker(&self, to: &str, package_id: &str, sticker_id: &str) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = "https://api.line.me/v2/bot/message/push";
        
        let payload = serde_json::json!({
            "to": to,
            "messages": [
                {
                    "type": "sticker",
                    "packageId": package_id,
                    "stickerId": sticker_id
                }
            ]
        });

        let response = self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "line".to_string(),
                message: error,
            }.into());
        }

        Ok(())
    }

    /// Send a quick reply message.
    pub async fn send_quick_reply(&self, to: &str, text: &str, items: Vec<QuickReplyItem>) -> Result<()> {
        if !self.config.enable_quick_replies {
            return self.send_to_user(to, text).await;
        }

        self.rate_limiter.until_ready().await;

        let url = "https://api.line.me/v2/bot/message/push";
        
        let quick_reply_items: Vec<_> = items.iter().map(|item| {
            serde_json::json!({
                "type": "action",
                "action": {
                    "type": "message",
                    "label": item.label,
                    "text": item.text
                }
            })
        }).collect();

        let payload = serde_json::json!({
            "to": to,
            "messages": [
                {
                    "type": "text",
                    "text": text,
                    "quickReply": {
                        "items": quick_reply_items
                    }
                }
            ]
        });

        let response = self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "line".to_string(),
                message: error,
            }.into());
        }

        Ok(())
    }

    /// Send a message to a specific user.
    async fn send_to_user(&self, to: &str, text: &str) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = "https://api.line.me/v2/bot/message/push";
        
        let payload = serde_json::json!({
            "to": to,
            "messages": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        });

        let response = self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "line".to_string(),
                message: error,
            }.into());
        }

        Ok(())
    }
}

/// Quick reply item for LINE messages.
#[derive(Debug, Clone)]
pub struct QuickReplyItem {
    pub label: String,
    pub text: String,
}

#[async_trait]
impl Channel for LineChannel {
    fn platform(&self) -> Platform {
        Platform::Line
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract recipient ID from metadata
        let recipient_id = msg.metadata.get("line_user_id")
            .and_then(|v| v.as_str())
            .or_else(|| msg.metadata.get("recipient_id").and_then(|v| v.as_str()))
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "line".to_string(),
                message: "Missing recipient_id in metadata".to_string(),
            })?;

        // Check if we have quick replies
        if let Some(quick_replies) = msg.metadata.get("quick_replies") {
            if let Some(items) = quick_replies.as_array() {
                let qr_items: Vec<QuickReplyItem> = items.iter()
                    .filter_map(|item| {
                        Some(QuickReplyItem {
                            label: item.get("label")?.as_str()?.to_string(),
                            text: item.get("text")?.as_str()?.to_string(),
                        })
                    })
                    .collect();
                
                if !qr_items.is_empty() {
                    return self.send_quick_reply(recipient_id, &msg.content, qr_items).await;
                }
            }
        }

        // Send regular text message
        self.send_to_user(recipient_id, &msg.content).await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "line".to_string(),
                message: "Incoming message channel closed".to_string(),
            }.into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to LINE Messaging API...");

        // Validate credentials
        if self.config.channel_access_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "line".to_string(),
                message: "LINE channel access token is required".to_string(),
            }.into());
        }

        if self.config.channel_secret.is_empty() {
            return Err(ChannelError::Config {
                platform: "line".to_string(),
                message: "LINE channel secret is required".to_string(),
            }.into());
        }

        // Verify credentials by getting bot info
        let url = "https://api.line.me/v2/bot/info";
        
        let response = self.http
            .get(url)
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "line".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(ChannelError::AuthFailed {
                platform: "line".to_string(),
                message: "Invalid channel access token".to_string(),
            }.into());
        }

        let info: serde_json::Value = response.json().await.map_err(|e| ChannelError::InvalidFormat {
            platform: "line".to_string(),
            message: e.to_string(),
        })?;

        info!(bot_name = ?info.get("displayName"), "Connected to LINE Messaging API");

        *self.is_connected.write().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from LINE...");
        
        *self.is_connected.write().await = false;
        
        info!("LINE channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = LineConfig {
            enabled: true,
            channel_access_token: "test_token".to_string(),
            channel_secret: "test_secret".to_string(),
            webhook_path: "/webhook/line".to_string(),
            allowlist: vec![],
            rate_limit_per_second: 1000,
            enable_rich_menu: true,
            enable_quick_replies: true,
        };
        let channel = LineChannel::new(config);
        assert!(channel.is_user_allowed("U1234567890"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = LineConfig {
            enabled: true,
            channel_access_token: "test_token".to_string(),
            channel_secret: "test_secret".to_string(),
            webhook_path: "/webhook/line".to_string(),
            allowlist: vec!["U1234567890".to_string(), "U0987654321".to_string()],
            rate_limit_per_second: 1000,
            enable_rich_menu: true,
            enable_quick_replies: true,
        };
        let channel = LineChannel::new(config);
        assert!(channel.is_user_allowed("U1234567890"));
        assert!(channel.is_user_allowed("U0987654321"));
        assert!(!channel.is_user_allowed("U5555555555"));
    }
}
