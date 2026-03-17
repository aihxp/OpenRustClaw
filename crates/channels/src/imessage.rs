//! iMessage Channel Integration (macOS only)
//!
//! Uses BlueBubbles server (https://bluebubbles.app/) as bridge
//! or direct macOS AppleScript/System Events for local macOS bots.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use openrustclaw_core::config::{IMessageBridgeMode, IMessageConfig};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// iMessage channel implementation.
pub struct IMessageChannel {
    config: IMessageConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    is_connected: RwLock<bool>,
}

impl IMessageChannel {
    /// Create a new iMessage channel with the given configuration.
    pub fn new(config: IMessageConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            is_connected: RwLock::new(false),
        }
    }

    /// Check if an address (phone number or Apple ID) is allowed.
    fn is_address_allowed(&self, address: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&address.to_string())
    }

    /// Handle BlueBubbles webhook payload.
    pub async fn handle_bluebubbles_webhook(&self, payload: BlueBubblesMessage) -> Result<()> {
        // Skip messages from self
        if payload.is_from_me {
            return Ok(());
        }

        // Check allowlist
        if !self.is_address_allowed(&payload.handle.address) {
            debug!(
                address = %payload.handle.address,
                "iMessage: ignoring message from non-allowed address"
            );
            return Ok(());
        }

        // Determine if this is a tapback/reaction
        let (content, is_reaction) = if let Some(tapback_type) = payload.associated_message_type {
            let reaction_text = match tapback_type {
                0 => "❤️ Loved",
                1 => "👍 Liked",
                2 => "👎 Disliked",
                3 => "😂 Laughed at",
                4 => "❗ Emphasized",
                5 => "❓ Question",
                _ => "Reacted to",
            };
            (format!("{} your message", reaction_text), true)
        } else {
            (payload.text.clone(), false)
        };

        let name = payload
            .handle
            .first_name
            .clone()
            .or(payload.handle.last_name.clone())
            .unwrap_or_else(|| payload.handle.address.clone());

        // Create metadata with iMessage-specific info
        let metadata = serde_json::json!({
            "imessage_chat_guid": payload.chat_guid,
            "imessage_message_guid": payload.guid,
            "imessage_handle_address": payload.handle.address,
            "imessage_handle_name": name,
            "imessage_is_reaction": is_reaction,
            "imessage_associated_message_guid": payload.associated_message_guid,
        });

        let msg = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: payload.handle.address.clone(),
            content,
            platform: Platform::IMessage,
            metadata,
        };

        let _ = self.incoming_tx.send(msg).await;
        Ok(())
    }

    /// Send a message via BlueBubbles API.
    async fn send_bluebubbles(
        &self,
        server_url: &str,
        password: &str,
        chat_guid: &str,
        content: &str,
    ) -> Result<()> {
        #[derive(Serialize)]
        #[allow(non_snake_case)]
        struct BlueBubblesSendPayload {
            chatGuid: String,
            message: String,
            tempGuid: String,
        }

        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/message", server_url);

        let payload = BlueBubblesSendPayload {
            chatGuid: chat_guid.to_string(),
            message: content.to_string(),
            tempGuid: format!("temp-{}", Uuid::new_v4()),
        };

        let response = client
            .post(&url)
            .header("Authorization", password)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "imessage".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ChannelError::SendFailed {
                platform: "imessage".to_string(),
                message: error_text,
            }
            .into());
        }

        Ok(())
    }

    /// Send a message via macOS AppleScript.
    async fn send_applescript(&self, recipient_id: &str, content: &str) -> Result<()> {
        // Check if running on macOS
        if !cfg!(target_os = "macos") {
            return Err(ChannelError::Config {
                platform: "imessage".to_string(),
                message: "AppleScript mode only available on macOS".to_string(),
            }
            .into());
        }

        // Escape quotes in message
        let escaped_content = content.replace('"', "\\\"");

        let script = format!(
            r#"tell application "Messages"
                set targetService to 1st service whose service type = iMessage
                set targetBuddy to buddy "{}" of targetService
                send "{}" to targetBuddy
            end tell"#,
            recipient_id, escaped_content
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "imessage".to_string(),
                message: format!("Failed to execute AppleScript: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::SendFailed {
                platform: "imessage".to_string(),
                message: stderr.to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Send tapback/reaction via BlueBubbles.
    pub async fn send_tapback(
        &self,
        chat_guid: &str,
        message_guid: &str,
        tapback: TapbackType,
    ) -> Result<()> {
        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => {
                let client = reqwest::Client::new();
                let url = format!("{}/api/v1/message/react", server_url);

                #[derive(Serialize)]
                #[allow(non_snake_case)]
                struct TapbackPayload {
                    chatGuid: String,
                    selectedMessageGuid: String,
                    reaction: i32,
                }

                let payload = TapbackPayload {
                    chatGuid: chat_guid.to_string(),
                    selectedMessageGuid: message_guid.to_string(),
                    reaction: tapback as i32,
                };

                client
                    .post(&url)
                    .header("Authorization", password)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| ChannelError::SendFailed {
                        platform: "imessage".to_string(),
                        message: format!("Failed to send tapback: {}", e),
                    })?;

                Ok(())
            }
            _ => Err(ChannelError::Config {
                platform: "imessage".to_string(),
                message: "Tapbacks only supported in BlueBubbles mode".to_string(),
            }
            .into()),
        }
    }
}

#[async_trait]
impl Channel for IMessageChannel {
    fn platform(&self) -> Platform {
        Platform::IMessage
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Extract chat GUID or recipient ID from metadata
        let chat_guid = msg
            .metadata
            .get("imessage_chat_guid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "imessage".to_string(),
                message: "Missing imessage_chat_guid in metadata".to_string(),
            })?;

        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => {
                self.send_bluebubbles(server_url, password, chat_guid, &msg.content)
                    .await
            }
            IMessageBridgeMode::MacOSDirect => self.send_applescript(chat_guid, &msg.content).await,
            IMessageBridgeMode::PrivateApi => Err(ChannelError::Config {
                platform: "imessage".to_string(),
                message: "Private API mode not yet implemented".to_string(),
            }
            .into()),
        }
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "imessage".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to iMessage...");

        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles { server_url, .. } => {
                info!("iMessage: Using BlueBubbles bridge at {}", server_url);
                // In a full implementation, this would verify the connection
                // by fetching server info from the BlueBubbles API
            }
            IMessageBridgeMode::MacOSDirect => {
                if !cfg!(target_os = "macos") {
                    return Err(ChannelError::Config {
                        platform: "imessage".to_string(),
                        message: "MacOS direct mode only available on macOS".to_string(),
                    }
                    .into());
                }
                info!("iMessage: Using macOS direct AppleScript mode");
                // Verify Messages.app is accessible
            }
            IMessageBridgeMode::PrivateApi => {
                return Err(ChannelError::Config {
                    platform: "imessage".to_string(),
                    message: "Private API mode not yet implemented".to_string(),
                }
                .into());
            }
        }

        *self.is_connected.write().await = true;
        info!("iMessage channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from iMessage...");

        *self.is_connected.write().await = false;

        info!("iMessage channel disconnected");
        Ok(())
    }
}

/// Tapback/reaction types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TapbackType {
    /// Heart/love reaction
    Love = 0,
    /// Thumbs up
    Like = 1,
    /// Thumbs down
    Dislike = 2,
    /// Ha ha/laugh
    Laugh = 3,
    /// Emphasis/exclamation
    Emphasize = 4,
    /// Question mark
    Question = 5,
}

/// BlueBubbles webhook payload for incoming messages.
#[derive(Debug, Clone, Deserialize)]
pub struct BlueBubblesMessage {
    /// Message GUID
    #[serde(rename = "guid")]
    pub guid: String,
    /// Message text content
    #[serde(rename = "text")]
    pub text: String,
    /// Sender handle information
    #[serde(rename = "handle")]
    pub handle: BlueBubblesHandle,
    /// Message creation date
    #[serde(rename = "dateCreated")]
    pub date_created: String,
    /// Whether the message was sent by the bot
    #[serde(rename = "isFromMe")]
    pub is_from_me: bool,
    /// Chat GUID
    #[serde(rename = "chatGUID")]
    pub chat_guid: String,
    /// Associated message GUID (for reactions/tapbacks)
    #[serde(rename = "associatedMessageGuid")]
    pub associated_message_guid: Option<String>,
    /// Associated message type (tapback type)
    #[serde(rename = "associatedMessageType")]
    pub associated_message_type: Option<i32>,
}

/// BlueBubbles sender handle information.
#[derive(Debug, Clone, Deserialize)]
pub struct BlueBubblesHandle {
    /// Phone number or Apple ID
    #[serde(rename = "address")]
    pub address: String,
    /// First name (if available)
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    /// Last name (if available)
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

/// Check if running on macOS.
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_macos() {
        // This test will pass on macOS and fail elsewhere, but that's expected
        // We just want to ensure the function compiles
        let _ = is_macos();
    }

    #[test]
    fn test_address_allowed_empty_list() {
        let config = IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::MacOSDirect,
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        };
        let channel = IMessageChannel::new(config);
        assert!(channel.is_address_allowed("+1234567890"));
        assert!(channel.is_address_allowed("user@icloud.com"));
    }

    #[test]
    fn test_address_allowed_with_list() {
        let config = IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::MacOSDirect,
            allowlist: vec!["+1234567890".to_string(), "user@icloud.com".to_string()],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        };
        let channel = IMessageChannel::new(config);
        assert!(channel.is_address_allowed("+1234567890"));
        assert!(channel.is_address_allowed("user@icloud.com"));
        assert!(!channel.is_address_allowed("+9876543210"));
        assert!(!channel.is_address_allowed("other@gmail.com"));
    }

    #[test]
    fn test_tapback_type_values() {
        assert_eq!(TapbackType::Love as i32, 0);
        assert_eq!(TapbackType::Like as i32, 1);
        assert_eq!(TapbackType::Dislike as i32, 2);
        assert_eq!(TapbackType::Laugh as i32, 3);
        assert_eq!(TapbackType::Emphasize as i32, 4);
        assert_eq!(TapbackType::Question as i32, 5);
    }

    #[test]
    fn test_bluebubbles_message_deserialization() {
        let json = r#"{
            "guid": "test-guid-123",
            "text": "Hello world",
            "handle": {
                "address": "+1234567890",
                "firstName": "John",
                "lastName": "Doe"
            },
            "dateCreated": "2024-01-15T10:30:00Z",
            "isFromMe": false,
            "chatGUID": "chat-guid-456"
        }"#;

        let msg: BlueBubblesMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.guid, "test-guid-123");
        assert_eq!(msg.text, "Hello world");
        assert_eq!(msg.handle.address, "+1234567890");
        assert_eq!(msg.handle.first_name, Some("John".to_string()));
        assert_eq!(msg.handle.last_name, Some("Doe".to_string()));
        assert!(!msg.is_from_me);
        assert_eq!(msg.chat_guid, "chat-guid-456");
    }

    #[test]
    fn test_bluebubbles_reaction_deserialization() {
        let json = r#"{
            "guid": "reaction-guid-789",
            "text": "",
            "handle": {
                "address": "+1234567890",
                "firstName": null,
                "lastName": null
            },
            "dateCreated": "2024-01-15T10:31:00Z",
            "isFromMe": false,
            "chatGUID": "chat-guid-456",
            "associatedMessageGuid": "original-msg-guid",
            "associatedMessageType": 0
        }"#;

        let msg: BlueBubblesMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.guid, "reaction-guid-789");
        assert_eq!(
            msg.associated_message_guid,
            Some("original-msg-guid".to_string())
        );
        assert_eq!(msg.associated_message_type, Some(0));
    }
}
