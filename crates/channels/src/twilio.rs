//! Twilio SMS/MMS integration for OpenRustClaw.
//!
//! Features:
//! - Text messages (SMS)
//! - Media messages (MMS)
//! - Webhook-based incoming message handling
//! - Allowlist for phone numbers
//! - Rate limiting
//! - Message status tracking
//!
//! # Authentication
//!
//! Uses Twilio Account SID and Auth Token for API authentication.
//! The phone number must be purchased from Twilio and in E.164 format.
//!
//! # Webhook Setup
//!
//! Configure your Twilio phone number's messaging webhook to point to:
//! `https://your-domain.com/webhooks/twilio`
//!
//! The webhook handler expects form-encoded data from Twilio.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use openrustclaw_core::config::TwilioConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Twilio API base URL.
const TWILIO_API_BASE: &str = "https://api.twilio.com/2010-04-01";

/// Twilio SMS channel implementation.
pub struct TwilioChannel {
    config: TwilioConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
    http_client: reqwest::Client,
}

/// Twilio API message response.
#[derive(Debug, Clone, Deserialize)]
pub struct TwilioMessageResponse {
    pub sid: String,
    pub status: String,
    #[serde(rename = "error_message")]
    pub error_message: Option<String>,
    #[serde(rename = "error_code")]
    pub error_code: Option<String>,
}

/// Incoming Twilio webhook payload.
#[derive(Debug, Clone, Deserialize)]
pub struct TwilioWebhook {
    #[serde(rename = "MessageSid")]
    pub message_sid: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "Body")]
    pub body: String,
    #[serde(rename = "NumMedia")]
    #[serde(deserialize_with = "deserialize_num_media")]
    pub num_media: u32,
    #[serde(rename = "MediaUrl0")]
    pub media_url_0: Option<String>,
    #[serde(rename = "MediaContentType0")]
    pub media_content_type_0: Option<String>,
    #[serde(rename = "SmsStatus")]
    pub sms_status: String,
    #[serde(rename = "AccountSid")]
    pub account_sid: Option<String>,
}

/// Deserialize NumMedia which comes as a string from Twilio.
fn deserialize_num_media<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

/// Twilio media attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioMedia {
    pub url: String,
    pub content_type: String,
    pub filename: String,
}

impl TwilioChannel {
    /// Create a new Twilio channel with the given configuration.
    pub fn new(config: TwilioConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Twilio allows ~10+ requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap())
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            http_client,
        }
    }

    /// Check if a phone number is in the allowlist.
    fn is_number_allowed(&self, phone_number: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&phone_number.to_string())
    }

    /// Normalize a phone number to E.164 format.
    pub fn normalize_phone_number(phone: &str) -> String {
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
        
        // If it starts with country code (assume 1 for US if 10 digits)
        if digits.len() == 10 {
            format!("+1{}", digits)
        } else if digits.len() > 10 && !phone.starts_with('+') {
            format!("+{}", digits)
        } else {
            phone.to_string()
        }
    }

    /// Send an SMS message.
    pub async fn send_sms(&self, to: &str, body: &str) -> Result<TwilioMessageResponse> {
        self.rate_limiter.until_ready().await;

        let url = format!(
            "{}/Accounts/{}/Messages.json",
            TWILIO_API_BASE, self.config.account_sid
        );

        let normalized_to = Self::normalize_phone_number(to);
        
        // Handle long messages
        let content = if body.len() > self.config.max_message_length {
            format!("{}...", &body[..self.config.max_message_length.saturating_sub(3)])
        } else {
            body.to_string()
        };

        let params = [
            ("To", normalized_to.as_str()),
            ("From", self.config.phone_number.as_str()),
            ("Body", content.as_str()),
        ];

        let response = self.http_client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| ChannelError::SendFailed {
            platform: "twilio".to_string(),
            message: format!("Failed to read response: {}", e),
        })?;

        if !status.is_success() {
            return Err(ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("Twilio API error ({}): {}", status, response_text),
            }.into());
        }

        let result: TwilioMessageResponse = serde_json::from_str(&response_text)
            .map_err(|e| ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("Failed to parse response: {} - {}", e, response_text),
            })?;

        if result.status == "failed" {
            return Err(ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: result.error_message.unwrap_or_else(|| "Unknown error".to_string()),
            }.into());
        }

        Ok(result)
    }

    /// Send an MMS message with media.
    pub async fn send_mms(
        &self,
        to: &str,
        body: &str,
        media_url: &str,
    ) -> Result<TwilioMessageResponse> {
        if !self.config.enable_mms {
            return Err(ChannelError::Config {
                platform: "twilio".to_string(),
                message: "MMS is not enabled in configuration".to_string(),
            }.into());
        }

        self.rate_limiter.until_ready().await;

        let url = format!(
            "{}/Accounts/{}/Messages.json",
            TWILIO_API_BASE, self.config.account_sid
        );

        let normalized_to = Self::normalize_phone_number(to);

        let params = [
            ("To", normalized_to.as_str()),
            ("From", self.config.phone_number.as_str()),
            ("Body", body),
            ("MediaUrl", media_url),
        ];

        let response = self.http_client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| ChannelError::SendFailed {
            platform: "twilio".to_string(),
            message: format!("Failed to read response: {}", e),
        })?;

        if !status.is_success() {
            return Err(ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("Twilio API error ({}): {}", status, response_text),
            }.into());
        }

        let result: TwilioMessageResponse = serde_json::from_str(&response_text)
            .map_err(|e| ChannelError::SendFailed {
                platform: "twilio".to_string(),
                message: format!("Failed to parse response: {} - {}", e, response_text),
            })?;

        Ok(result)
    }

    /// Handle an incoming webhook event.
    /// 
    /// This should be called by your HTTP server when a webhook is received
    /// from Twilio. Returns the incoming message if it was accepted.
    pub async fn handle_webhook(
        &self,
        payload: &TwilioWebhook,
    ) -> Result<Option<IncomingMessage>> {
        // Verify account SID if provided (security check)
        if let Some(ref account_sid) = payload.account_sid {
            if account_sid != &self.config.account_sid {
                warn!("Webhook from different account: {}", account_sid);
                return Err(ChannelError::AuthFailed {
                    platform: "twilio".to_string(),
                    message: "Invalid account SID".to_string(),
                }.into());
            }
        }

        // Check allowlist
        if !self.is_number_allowed(&payload.from) {
            info!(from = %payload.from, "SMS from number rejected (not in allowlist)");
            return Ok(None);
        }

        // Build metadata
        let mut metadata = serde_json::json!({
            "twilio_message_sid": payload.message_sid,
            "twilio_status": payload.sms_status,
            "twilio_to": payload.to,
        });

        // Parse media attachments
        if payload.num_media > 0 {
            let mut media_list = Vec::new();

            if let Some(ref url) = payload.media_url_0 {
                let content_type = payload.media_content_type_0
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                
                let extension = content_type
                    .split('/')
                    .nth(1)
                    .unwrap_or("bin");

                media_list.push(TwilioMedia {
                    url: url.clone(),
                    content_type: content_type.clone(),
                    filename: format!("media_{}.{}", payload.message_sid, extension),
                });

                metadata["twilio_media"] = serde_json::json!(media_list);
            }

            metadata["twilio_num_media"] = serde_json::json!(payload.num_media);
        }

        // Create incoming message
        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: payload.from.clone(),
            content: payload.body.clone(),
            platform: Platform::Twilio,
            metadata,
        };

        // Send to channel
        let _ = self.incoming_tx.send(incoming.clone()).await;

        info!(
            from = %payload.from,
            message_sid = %payload.message_sid,
            has_media = payload.num_media > 0,
            "Received Twilio SMS"
        );

        Ok(Some(incoming))
    }

    /// Verify Twilio credentials by fetching account info.
    async fn verify_credentials(&self) -> Result<()> {
        let url = format!(
            "{}/Accounts/{}.json",
            TWILIO_API_BASE, self.config.account_sid
        );

        let response = self.http_client
            .get(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "twilio".to_string(),
                message: format!("Failed to verify credentials: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "twilio".to_string(),
                message: format!("Invalid credentials: {}", error_text),
            }.into());
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for TwilioChannel {
    fn platform(&self) -> Platform {
        Platform::Twilio
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Get recipient from metadata
        let to = msg.metadata.get("twilio_to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "twilio".to_string(),
                message: "Missing 'twilio_to' in metadata".to_string(),
            })?;

        // Check if we should send MMS
        if let Some(media_url) = msg.metadata.get("twilio_media_url").and_then(|v| v.as_str()) {
            self.send_mms(to, &msg.content, media_url).await?;
        } else {
            self.send_sms(to, &msg.content).await?;
        }

        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "twilio".to_string(),
                message: "Incoming message channel closed".to_string(),
            }.into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Twilio SMS API...");

        // Validate configuration
        if self.config.account_sid.is_empty() {
            return Err(ChannelError::Config {
                platform: "twilio".to_string(),
                message: "Twilio Account SID is required".to_string(),
            }.into());
        }

        if self.config.auth_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "twilio".to_string(),
                message: "Twilio Auth Token is required".to_string(),
            }.into());
        }

        if self.config.phone_number.is_empty() {
            return Err(ChannelError::Config {
                platform: "twilio".to_string(),
                message: "Twilio phone number is required".to_string(),
            }.into());
        }

        // Verify credentials
        self.verify_credentials().await?;

        *self.is_connected.write().await = true;

        info!(phone = %self.config.phone_number, "Twilio SMS channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Twilio SMS...");

        *self.is_connected.write().await = false;

        info!("Twilio SMS channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_phone_number() {
        assert_eq!(TwilioChannel::normalize_phone_number("5551234567"), "+15551234567");
        assert_eq!(TwilioChannel::normalize_phone_number("+15551234567"), "+15551234567");
        assert_eq!(TwilioChannel::normalize_phone_number("(555) 123-4567"), "+15551234567");
    }

    #[test]
    fn test_is_number_allowed_empty_allowlist() {
        let config = TwilioConfig {
            enabled: true,
            account_sid: "test".to_string(),
            auth_token: "test".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: None,
            allowlist: vec![],
            max_message_length: 1600,
            enable_mms: true,
            rate_limit_per_second: 10,
        };

        let channel = TwilioChannel::new(config);
        assert!(channel.is_number_allowed("+15551234567"));
    }

    #[test]
    fn test_is_number_allowed_with_allowlist() {
        let config = TwilioConfig {
            enabled: true,
            account_sid: "test".to_string(),
            auth_token: "test".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: None,
            allowlist: vec!["+15551234567".to_string()],
            max_message_length: 1600,
            enable_mms: true,
            rate_limit_per_second: 10,
        };

        let channel = TwilioChannel::new(config);
        assert!(channel.is_number_allowed("+15551234567"));
        assert!(!channel.is_number_allowed("+15559876543"));
    }

    #[test]
    fn test_webhook_parsing() {
        let webhook_data = r#"{
            "MessageSid": "SM1234567890",
            "From": "+15551234567",
            "To": "+1234567890",
            "Body": "Hello, world!",
            "NumMedia": "0",
            "SmsStatus": "received"
        }"#;

        let webhook: TwilioWebhook = serde_json::from_str(webhook_data).unwrap();
        assert_eq!(webhook.message_sid, "SM1234567890");
        assert_eq!(webhook.from, "+15551234567");
        assert_eq!(webhook.body, "Hello, world!");
    }
}
