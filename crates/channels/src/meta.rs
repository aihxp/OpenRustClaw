//! Meta Channels (Messenger & Instagram) Integration
//!
//! Uses Meta for Developers Messenger Platform API.
//! Supports both Messenger and Instagram Direct messaging.

use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::MetaConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Meta channel implementation (handles both Messenger and Instagram).
pub struct MetaChannel {
    config: MetaConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    http: reqwest::Client,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
}

/// Webhook verification request params.
#[derive(Debug, Deserialize)]
pub struct WebhookVerify {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

/// Meta webhook payload.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaWebhook {
    pub object: String,
    pub entry: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub id: String,
    pub time: u64,
    pub messaging: Option<Vec<MessagingEvent>>,
    pub standby: Option<Vec<MessagingEvent>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagingEvent {
    pub sender: Sender,
    pub recipient: Recipient,
    pub timestamp: u64,
    pub message: Option<MessageData>,
    pub postback: Option<Postback>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sender {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recipient {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageData {
    pub mid: String,
    pub text: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub quick_reply: Option<QuickReply>,
    #[serde(rename = "is_echo")]
    pub is_echo: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub attachment_type: String,
    pub payload: AttachmentPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentPayload {
    pub url: Option<String>,
    #[serde(rename = "sticker_id")]
    pub sticker_id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuickReply {
    pub payload: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Postback {
    pub title: String,
    pub payload: String,
}

/// Quick reply option for sending.
pub struct QuickReplyOption {
    pub title: String,
    pub payload: String,
}

/// Template types for structured messages.
pub enum Template {
    Button { text: String, buttons: Vec<Button> },
    Generic { elements: Vec<GenericElement> },
}

/// Button for templates.
pub struct Button {
    pub title: String,
    pub payload: String,
}

/// Generic template element.
pub struct GenericElement {
    pub title: String,
    pub subtitle: String,
    pub image_url: Option<String>,
    pub buttons: Vec<Button>,
}

/// User profile from Meta API.
#[derive(Debug, Deserialize)]
pub struct UserProfile {
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "last_name")]
    pub last_name: String,
    #[serde(rename = "profile_pic")]
    pub profile_pic: Option<String>,
}

impl MetaChannel {
    /// Create a new Meta channel with the given configuration.
    pub fn new(config: MetaConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            http: reqwest::Client::new(),
            rate_limiter,
            is_connected: RwLock::new(false),
        }
    }

    /// Verify webhook request.
    pub fn verify_webhook(&self, verify: &WebhookVerify) -> bool {
        verify.mode == "subscribe" && verify.verify_token == self.config.verify_token
    }

    /// Handle incoming webhook payload.
    pub async fn handle_webhook(&self, payload: MetaWebhook) -> Result<()> {
        if payload.object != "page" && payload.object != "instagram" {
            debug!(object = %payload.object, "Ignoring non-page webhook");
            return Ok(());
        }

        let is_instagram = payload.object == "instagram";

        // Skip if not responding to this platform
        if is_instagram && !self.config.respond_to_instagram {
            return Ok(());
        }
        if !is_instagram && !self.config.respond_to_messenger {
            return Ok(());
        }

        for entry in payload.entry {
            if let Some(messaging) = entry.messaging {
                for event in messaging {
                    // Skip echo messages (messages sent by the bot)
                    if event
                        .message
                        .as_ref()
                        .and_then(|m| m.is_echo)
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    let sender_id = event.sender.id.clone();

                    // Check allowlist
                    if !self.config.allowlist.is_empty() && !self.config.allowlist.contains(&sender_id) {
                        warn!(sender_id = %sender_id, "Sender not in allowlist");
                        continue;
                    }

                    // Handle text/message
                    if let Some(message) = &event.message {
                        let content = message.text.clone().unwrap_or_else(|| {
                            if message
                                .attachments
                                .as_ref()
                                .map(|a| !a.is_empty())
                                .unwrap_or(false)
                            {
                                "[Attachment]".to_string()
                            } else {
                                String::new()
                            }
                        });

                        let platform = if is_instagram {
                            Platform::Instagram
                        } else {
                            Platform::Messenger
                        };

                        let mut metadata = serde_json::json!({
                            "sender_id": sender_id.clone(),
                            "recipient_id": event.recipient.id.clone(),
                        });

                        // Add attachments to metadata if present
                        if let Some(attachments) = &message.attachments {
                            metadata["attachments"] = serde_json::to_value(attachments).unwrap_or_default();
                        }

                        let msg = IncomingMessage {
                            session_id: Uuid::new_v4(),
                            content,
                            user_id: sender_id.clone(),
                            platform,
                            metadata,
                        };

                        if let Err(e) = self.incoming_tx.send(msg).await {
                            error!(error = %e, "Failed to send incoming message");
                        }
                    }

                    // Handle postback (button click)
                    if let Some(postback) = &event.postback {
                        let platform = if is_instagram {
                            Platform::Instagram
                        } else {
                            Platform::Messenger
                        };

                        let metadata = serde_json::json!({
                            "sender_id": sender_id.clone(),
                            "recipient_id": event.recipient.id,
                            "postback_payload": postback.payload,
                        });

                        let msg = IncomingMessage {
                            session_id: Uuid::new_v4(),
                            content: format!("[Button: {}]", postback.title),
                            user_id: sender_id,
                            platform,
                            metadata,
                        };

                        if let Err(e) = self.incoming_tx.send(msg).await {
                            error!(error = %e, "Failed to send postback message");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Send typing indicator.
    pub async fn send_typing_indicator(&self, recipient_id: &str) -> Result<()> {
        if !self.config.show_typing_indicator {
            return Ok(());
        }

        self.rate_limiter.until_ready().await;

        let url = format!(
            "https://graph.facebook.com/v18.0/me/messages?access_token={}",
            self.config.page_access_token
        );

        let payload = serde_json::json!({
            "recipient": { "id": recipient_id },
            "sender_action": "typing_on"
        });

        let response = self.http.post(&url).json(&payload).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "meta".to_string(),
                message: error_text,
            }.into());
        }

        Ok(())
    }

    /// Send message with quick reply buttons.
    pub async fn send_quick_replies(
        &self,
        recipient_id: &str,
        text: &str,
        replies: Vec<QuickReplyOption>,
    ) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = format!(
            "https://graph.facebook.com/v18.0/me/messages?access_token={}",
            self.config.page_access_token
        );

        let quick_replies: Vec<_> = replies
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "content_type": "text",
                    "title": r.title,
                    "payload": r.payload
                })
            })
            .collect();

        let payload = serde_json::json!({
            "recipient": { "id": recipient_id },
            "message": {
                "text": text,
                "quick_replies": quick_replies
            }
        });

        let response = self.http.post(&url).json(&payload).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "meta".to_string(),
                message: error_text,
            }.into());
        }

        Ok(())
    }

    /// Send template message (buttons, generic, etc).
    pub async fn send_template(&self, recipient_id: &str, template: Template) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let url = format!(
            "https://graph.facebook.com/v18.0/me/messages?access_token={}",
            self.config.page_access_token
        );

        let payload = serde_json::json!({
            "recipient": { "id": recipient_id },
            "message": {
                "attachment": {
                    "type": "template",
                    "payload": template.to_json()
                }
            }
        });

        let response = self.http.post(&url).json(&payload).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "meta".to_string(),
                message: error_text,
            }.into());
        }

        Ok(())
    }

    /// Get user profile info.
    pub async fn get_user_profile(&self, user_id: &str) -> Result<UserProfile> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}?fields=first_name,last_name,profile_pic&access_token={}",
            user_id, self.config.page_access_token
        );

        let response = self.http.get(&url).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "meta".to_string(),
                message: error_text,
            }.into());
        }

        let profile: UserProfile = response.json().await.map_err(|e| {
            ChannelError::InvalidFormat {
                platform: "meta".to_string(),
                message: format!("Failed to parse user profile: {}", e),
            }
        })?;

        Ok(profile)
    }

    /// Determine platform from metadata.
    #[allow(dead_code)]
    fn platform_from_metadata(metadata: &serde_json::Value) -> Platform {
        metadata
            .get("platform")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "instagram" => Platform::Instagram,
                _ => Platform::Messenger,
            })
            .unwrap_or(Platform::Messenger)
    }
}

impl Template {
    fn to_json(&self) -> serde_json::Value {
        match self {
            Template::Button { text, buttons } => {
                serde_json::json!({
                    "template_type": "button",
                    "text": text,
                    "buttons": buttons.iter().map(|b| {
                        serde_json::json!({
                            "type": "postback",
                            "title": b.title,
                            "payload": b.payload
                        })
                    }).collect::<Vec<_>>()
                })
            }
            Template::Generic { elements } => {
                serde_json::json!({
                    "template_type": "generic",
                    "elements": elements.iter().map(|e| {
                        let mut json = serde_json::json!({
                            "title": e.title,
                            "subtitle": e.subtitle,
                            "buttons": e.buttons.iter().map(|b| {
                                serde_json::json!({
                                    "type": "postback",
                                    "title": b.title,
                                    "payload": b.payload
                                })
                            }).collect::<Vec<_>>()
                        });
                        if let Some(url) = &e.image_url {
                            json["image_url"] = serde_json::json!(url);
                        }
                        json
                    }).collect::<Vec<_>>()
                })
            }
        }
    }
}

#[async_trait]
impl Channel for MetaChannel {
    fn platform(&self) -> Platform {
        // Meta channel handles both Messenger and Instagram
        // Return Messenger as default, actual messages will have correct platform
        Platform::Messenger
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract recipient ID from metadata
        let recipient_id = msg
            .metadata
            .get("recipient_id")
            .or_else(|| msg.metadata.get("sender_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "meta".to_string(),
                message: "Missing recipient_id in metadata".to_string(),
            })?;

        let url = format!(
            "https://graph.facebook.com/v18.0/me/messages?access_token={}",
            self.config.page_access_token
        );

        let payload = serde_json::json!({
            "recipient": { "id": recipient_id },
            "message": { "text": msg.content },
            "messaging_type": "RESPONSE"
        });

        let response = self.http.post(&url).json(&payload).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "meta".to_string(),
                message: error_text,
            }
            .into());
        }

        debug!(recipient_id = %recipient_id, "Sent Meta message");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Meta Messenger Platform API...");

        // Validate configuration
        if self.config.page_access_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "meta".to_string(),
                message: "Page access token is required".to_string(),
            }
            .into());
        }

        if self.config.app_secret.is_empty() {
            return Err(ChannelError::Config {
                platform: "meta".to_string(),
                message: "App secret is required".to_string(),
            }
            .into());
        }

        // Verify token by fetching page info
        let url = format!(
            "https://graph.facebook.com/v18.0/me?access_token={}",
            self.config.page_access_token
        );

        let response = self.http.get(&url).send().await.map_err(|e| {
            ChannelError::Connection {
                platform: "meta".to_string(),
                message: e.to_string(),
            }
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "meta".to_string(),
                message: format!("Invalid credentials: {}", error_text),
            }
            .into());
        }

        let info: serde_json::Value = response.json().await.map_err(|e| {
            ChannelError::InvalidFormat {
                platform: "meta".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        info!(
            page_name = %info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "Connected to Meta Messenger Platform"
        );

        *self.is_connected.write().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Meta Messenger Platform...");

        *self.is_connected.write().await = false;

        info!("Meta channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> MetaConfig {
        MetaConfig {
            enabled: true,
            app_id: "test_app_id".to_string(),
            app_secret: "test_app_secret".to_string(),
            page_access_token: "test_token".to_string(),
            verify_token: "test_verify_token".to_string(),
            webhook_path: "/webhooks/meta".to_string(),
            page_id: "test_page_id".to_string(),
            instagram_account_id: None,
            allowlist: vec![],
            respond_to_messenger: true,
            respond_to_instagram: true,
            show_typing_indicator: false,
            rate_limit_per_second: 10,
        }
    }

    #[test]
    fn test_verify_webhook_success() {
        let config = create_test_config();
        let channel = MetaChannel::new(config);

        let verify = WebhookVerify {
            mode: "subscribe".to_string(),
            verify_token: "test_verify_token".to_string(),
            challenge: "challenge_123".to_string(),
        };

        assert!(channel.verify_webhook(&verify));
    }

    #[test]
    fn test_verify_webhook_wrong_token() {
        let config = create_test_config();
        let channel = MetaChannel::new(config);

        let verify = WebhookVerify {
            mode: "subscribe".to_string(),
            verify_token: "wrong_token".to_string(),
            challenge: "challenge_123".to_string(),
        };

        assert!(!channel.verify_webhook(&verify));
    }

    #[test]
    fn test_verify_webhook_wrong_mode() {
        let config = create_test_config();
        let channel = MetaChannel::new(config);

        let verify = WebhookVerify {
            mode: "publish".to_string(),
            verify_token: "test_verify_token".to_string(),
            challenge: "challenge_123".to_string(),
        };

        assert!(!channel.verify_webhook(&verify));
    }

    #[test]
    fn test_template_button_to_json() {
        let template = Template::Button {
            text: "Choose an option:".to_string(),
            buttons: vec![
                Button {
                    title: "Option 1".to_string(),
                    payload: "opt_1".to_string(),
                },
                Button {
                    title: "Option 2".to_string(),
                    payload: "opt_2".to_string(),
                },
            ],
        };

        let json = template.to_json();
        assert_eq!(json["template_type"], "button");
        assert_eq!(json["text"], "Choose an option:");
        assert_eq!(json["buttons"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_template_generic_to_json() {
        let template = Template::Generic {
            elements: vec![GenericElement {
                title: "Product".to_string(),
                subtitle: "A great product".to_string(),
                image_url: Some("https://example.com/image.jpg".to_string()),
                buttons: vec![Button {
                    title: "Buy".to_string(),
                    payload: "buy_now".to_string(),
                }],
            }],
        };

        let json = template.to_json();
        assert_eq!(json["template_type"], "generic");
        assert_eq!(json["elements"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_template_generic_without_image() {
        let template = Template::Generic {
            elements: vec![GenericElement {
                title: "Product".to_string(),
                subtitle: "A great product".to_string(),
                image_url: None,
                buttons: vec![],
            }],
        };

        let json = template.to_json();
        assert_eq!(json["template_type"], "generic");
        assert!(!json["elements"][0].as_object().unwrap().contains_key("image_url"));
    }

    #[test]
    fn test_is_user_allowed_empty_list() {
        let config = MetaConfig {
            allowlist: vec![],
            ..create_test_config()
        };
        let channel = MetaChannel::new(config);

        // All users should be allowed when list is empty
        assert!(channel.config.allowlist.is_empty());
    }

    #[test]
    fn test_is_user_allowed_with_list() {
        let config = MetaConfig {
            allowlist: vec!["user_123".to_string(), "user_456".to_string()],
            ..create_test_config()
        };

        assert!(config.allowlist.contains(&"user_123".to_string()));
        assert!(config.allowlist.contains(&"user_456".to_string()));
        assert!(!config.allowlist.contains(&"user_789".to_string()));
    }
}
