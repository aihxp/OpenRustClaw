//! iMessage Channel Integration (macOS only)
//!
//! Uses BlueBubbles server (https://bluebubbles.app/) as bridge
//! or direct macOS AppleScript/System Events for local macOS bots.

use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    #[allow(dead_code)]
    fn is_address_allowed(&self, address: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&address.to_string())
    }

    /// Handle BlueBubbles webhook payload.
    pub async fn handle_bluebubbles_webhook(&self, payload: BlueBubblesMessage) -> Result<()> {
        let Some(msg) = Self::incoming_from_bluebubbles(&self.config, payload) else {
            return Ok(());
        };
        let _ = self.incoming_tx.send(msg).await;
        Ok(())
    }

    /// Create an HTTP webhook handler for BlueBubbles callbacks.
    pub fn webhook_handler(&self) -> IMessageWebhookHandler {
        IMessageWebhookHandler {
            config: self.config.clone(),
            incoming_tx: self.incoming_tx.clone(),
        }
    }

    fn incoming_from_bluebubbles(
        config: &IMessageConfig,
        payload: BlueBubblesMessage,
    ) -> Option<IncomingMessage> {
        if payload.is_from_me {
            return None;
        }

        if !address_allowed(config, &payload.handle.address) {
            debug!(
                address = %payload.handle.address,
                "iMessage: ignoring message from non-allowed address"
            );
            return None;
        }

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

        let primary_chat = payload.chats.as_ref().and_then(|chats| chats.first());
        let participant_addresses: Vec<String> = primary_chat
            .and_then(|chat| chat.participants.as_deref())
            .map(Self::participant_addresses)
            .or_else(|| {
                payload
                    .participants
                    .as_deref()
                    .map(Self::participant_addresses)
            })
            .unwrap_or_default();
        let chat_guid = if payload.chat_guid.trim().is_empty() {
            primary_chat
                .and_then(|chat| chat.guid.as_deref())
                .unwrap_or_default()
                .to_string()
        } else {
            payload.chat_guid.clone()
        };
        let chat_identifier = primary_chat
            .and_then(|chat| chat.chat_identifier.clone())
            .filter(|value| !value.trim().is_empty());
        let chat_display_name = primary_chat
            .and_then(|chat| chat.display_name.clone())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                payload
                    .chat_display_name
                    .clone()
                    .filter(|value| !value.trim().is_empty())
            });
        let is_group = payload.is_group.unwrap_or_else(|| {
            !participant_addresses.is_empty() && participant_addresses.len() > 1
        });
        let file_references: Vec<Value> = payload
            .attachments
            .as_deref()
            .map(Self::attachment_file_references)
            .unwrap_or_default();

        let mut metadata = serde_json::json!({
            "imessage_chat_guid": chat_guid,
            "imessage_message_guid": payload.guid,
            "imessage_handle_address": payload.handle.address,
            "imessage_handle_name": name,
            "imessage_is_reaction": is_reaction,
            "imessage_is_group": is_group,
            "imessage_associated_message_guid": payload.associated_message_guid,
        });
        if let Some(chat_identifier) = chat_identifier {
            metadata["imessage_chat_identifier"] = serde_json::json!(chat_identifier);
        }
        if let Some(display_name) = chat_display_name {
            metadata["imessage_chat_display_name"] = serde_json::json!(display_name);
        }
        if !participant_addresses.is_empty() {
            metadata["imessage_participants"] = serde_json::json!(participant_addresses);
            metadata["imessage_participant_count"] = serde_json::json!(
                metadata["imessage_participants"]
                    .as_array()
                    .map(|items| items.len())
                    .unwrap_or(0)
            );
        }
        if !file_references.is_empty() {
            metadata["imessage_attachment_count"] = serde_json::json!(file_references.len());
            metadata["file_references"] = serde_json::json!(file_references);
        }

        Some(IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: payload.handle.address.clone(),
            content,
            platform: Platform::IMessage,
            metadata,
        })
    }

    fn participant_addresses(participants: &[Value]) -> Vec<String> {
        participants
            .iter()
            .filter_map(|participant| {
                participant
                    .as_str()
                    .map(|value| value.to_string())
                    .or_else(|| {
                        participant
                            .get("address")
                            .or_else(|| participant.get("id"))
                            .or_else(|| participant.get("identifier"))
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_string())
                    })
            })
            .collect()
    }

    fn attachment_file_references(attachments: &[Value]) -> Vec<Value> {
        attachments
            .iter()
            .filter_map(|attachment| {
                let local_path = attachment
                    .get("path")
                    .or_else(|| attachment.get("transferPath"))
                    .or_else(|| attachment.get("tempPath"))
                    .or_else(|| attachment.get("filename"))
                    .and_then(|value| value.as_str());
                let url = attachment
                    .get("url")
                    .or_else(|| attachment.get("downloadUrl"))
                    .and_then(|value| value.as_str());

                if local_path.is_none() && url.is_none() {
                    return None;
                }

                Some(serde_json::json!({
                    "local_path": local_path,
                    "url": url,
                    "name": attachment
                        .get("name")
                        .or_else(|| attachment.get("filename"))
                        .and_then(|value| value.as_str()),
                    "mime": attachment
                        .get("mimeType")
                        .or_else(|| attachment.get("mime"))
                        .and_then(|value| value.as_str()),
                    "size": attachment.get("size").and_then(|value| value.as_u64()),
                }))
            })
            .collect()
    }

    fn parse_tapback(value: &Value) -> Option<TapbackType> {
        if let Some(number) = value.as_i64() {
            return match number {
                0 => Some(TapbackType::Love),
                1 => Some(TapbackType::Like),
                2 => Some(TapbackType::Dislike),
                3 => Some(TapbackType::Laugh),
                4 => Some(TapbackType::Emphasize),
                5 => Some(TapbackType::Question),
                _ => None,
            };
        }

        value
            .as_str()
            .and_then(|value| match value.to_lowercase().as_str() {
                "love" | "heart" => Some(TapbackType::Love),
                "like" | "thumbs_up" | "thumbsup" => Some(TapbackType::Like),
                "dislike" | "thumbs_down" | "thumbsdown" => Some(TapbackType::Dislike),
                "laugh" | "ha" | "haha" => Some(TapbackType::Laugh),
                "emphasize" | "emphasis" | "exclaim" => Some(TapbackType::Emphasize),
                "question" | "question_mark" => Some(TapbackType::Question),
                _ => None,
            })
    }

    fn bluebubbles_request(
        client: &reqwest::Client,
        method: reqwest::Method,
        server_url: &str,
        password: &str,
        path: &str,
    ) -> reqwest::RequestBuilder {
        client
            .request(
                method,
                format!("{}/{}", server_url.trim_end_matches('/'), path),
            )
            .query(&[("password", password)])
            .header("Authorization", password)
    }

    fn direct_target_from_metadata(metadata: &Value) -> Option<String> {
        metadata
            .get("imessage_chat_guid")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| {
                metadata
                    .get("imessage_chat_identifier")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            })
            .or_else(|| {
                metadata
                    .get("imessage_recipient")
                    .and_then(|value| value.as_str())
                    .map(|value| format!("any;-;{}", value))
            })
            .or_else(|| {
                metadata
                    .get("imessage_handle_address")
                    .and_then(|value| value.as_str())
                    .map(|value| format!("any;-;{}", value))
            })
    }

    fn applescript_target_from_metadata(metadata: &Value) -> Option<String> {
        metadata
            .get("imessage_recipient")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| {
                metadata
                    .get("imessage_handle_address")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            })
            .or_else(|| {
                metadata
                    .get("imessage_chat_guid")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            })
    }

    async fn outgoing_attachments(metadata: &Value) -> Result<Vec<OutgoingAttachment>> {
        let mut attachments = Vec::new();
        let Some(entries) = metadata.get("file_references").and_then(|value| value.as_array()) else {
            return Ok(attachments);
        };

        for entry in entries {
            let Some(local_path) = entry.get("local_path").and_then(|value| value.as_str()) else {
                continue;
            };

            let bytes = tokio::fs::read(local_path)
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "imessage".to_string(),
                    message: format!("Failed to read iMessage attachment '{}': {}", local_path, e),
                })?;

            let filename = entry
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| {
                    std::path::Path::new(local_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.to_string())
                })
                .unwrap_or_else(|| "attachment".to_string());

            let mime = entry
                .get("mime")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            attachments.push(OutgoingAttachment {
                bytes,
                filename,
                mime,
            });
        }

        Ok(attachments)
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

        let payload = BlueBubblesSendPayload {
            chatGuid: chat_guid.to_string(),
            message: content.to_string(),
            tempGuid: format!("temp-{}", Uuid::new_v4()),
        };

        let response = Self::bluebubbles_request(
            &client,
            reqwest::Method::POST,
            server_url,
            password,
            "api/v1/message/text",
        )
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

    async fn send_bluebubbles_attachments(
        &self,
        server_url: &str,
        password: &str,
        chat_guid: &str,
        content: &str,
        attachments: &[OutgoingAttachment],
    ) -> Result<()> {
        let client = reqwest::Client::new();

        for (index, attachment) in attachments.iter().enumerate() {
            let part = if let Some(mime) = &attachment.mime {
                Part::bytes(attachment.bytes.clone())
                    .file_name(attachment.filename.clone())
                    .mime_str(mime)
                    .unwrap_or_else(|_| {
                        Part::bytes(attachment.bytes.clone()).file_name(attachment.filename.clone())
                    })
            } else {
                Part::bytes(attachment.bytes.clone()).file_name(attachment.filename.clone())
            };

            let message = if index == 0 { content } else { "" };
            let form = Form::new()
                .text("chatGuid", chat_guid.to_string())
                .text("tempGuid", format!("temp-{}", Uuid::new_v4()))
                .text("message", message.to_string())
                .part("attachment", part);

            let response = Self::bluebubbles_request(
                &client,
                reqwest::Method::POST,
                server_url,
                password,
                "api/v1/message/attachment",
            )
            .multipart(form)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "imessage".to_string(),
                message: format!("Failed to send iMessage attachment: {}", e),
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

                Self::bluebubbles_request(
                    &client,
                    reqwest::Method::POST,
                    server_url,
                    password,
                    "api/v1/message/react",
                )
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

    pub async fn ping(&self) -> Result<Value> {
        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => {
                let client = reqwest::Client::new();
                let response = Self::bluebubbles_request(
                    &client,
                    reqwest::Method::GET,
                    server_url,
                    password,
                    "api/v1/ping",
                )
                .send()
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "imessage".to_string(),
                    message: format!("Failed to ping BlueBubbles: {}", e),
                })?;

                let status = response.status();
                let value: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));
                if !status.is_success() {
                    return Err(ChannelError::Connection {
                        platform: "imessage".to_string(),
                        message: format!("BlueBubbles ping failed: {}", value),
                    }
                    .into());
                }
                Ok(value)
            }
            IMessageBridgeMode::MacOSDirect => Ok(serde_json::json!({
                "status": 200,
                "message": "macOS direct mode available",
                "data": { "mode": "macos_direct", "supported": cfg!(target_os = "macos") }
            })),
            IMessageBridgeMode::PrivateApi => Err(ChannelError::Config {
                platform: "imessage".to_string(),
                message: "Private API mode not yet implemented".to_string(),
            }
            .into()),
        }
    }

    pub async fn server_info(&self) -> Result<Value> {
        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => {
                let client = reqwest::Client::new();
                let response = Self::bluebubbles_request(
                    &client,
                    reqwest::Method::GET,
                    server_url,
                    password,
                    "api/v1/server",
                )
                .send()
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "imessage".to_string(),
                    message: format!("Failed to fetch BlueBubbles server info: {}", e),
                })?;

                let status = response.status();
                let value: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));
                if !status.is_success() {
                    return Err(ChannelError::Connection {
                        platform: "imessage".to_string(),
                        message: format!("BlueBubbles server info failed: {}", value),
                    }
                    .into());
                }
                Ok(value)
            }
            IMessageBridgeMode::MacOSDirect => Ok(serde_json::json!({
                "mode": "macos_direct",
                "supported": cfg!(target_os = "macos"),
            })),
            IMessageBridgeMode::PrivateApi => Err(ChannelError::Config {
                platform: "imessage".to_string(),
                message: "Private API mode not yet implemented".to_string(),
            }
            .into()),
        }
    }

    pub async fn list_chats(&self, limit: usize, offset: usize) -> Result<Value> {
        let (server_url, password) = match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => (server_url, password),
            _ => {
                return Err(ChannelError::Config {
                    platform: "imessage".to_string(),
                    message: "Chat listing is only supported in BlueBubbles mode".to_string(),
                }
                .into())
            }
        };

        let client = reqwest::Client::new();
        let response = Self::bluebubbles_request(
            &client,
            reqwest::Method::POST,
            server_url,
            password,
            "api/v1/chat/query",
        )
        .json(&serde_json::json!({
            "limit": limit,
            "offset": offset
        }))
        .send()
        .await
        .map_err(|e| ChannelError::Connection {
            platform: "imessage".to_string(),
            message: format!("Failed to list iMessage chats: {}", e),
        })?;

        let status = response.status();
        let value: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));
        if !status.is_success() {
            return Err(ChannelError::Connection {
                platform: "imessage".to_string(),
                message: format!("BlueBubbles chat query failed: {}", value),
            }
            .into());
        }

        Ok(value)
    }

    pub async fn list_contacts(&self) -> Result<Value> {
        let (server_url, password) = match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => (server_url, password),
            _ => {
                return Err(ChannelError::Config {
                    platform: "imessage".to_string(),
                    message: "Contact listing is only supported in BlueBubbles mode".to_string(),
                }
                .into())
            }
        };

        let client = reqwest::Client::new();
        let response = Self::bluebubbles_request(
            &client,
            reqwest::Method::POST,
            server_url,
            password,
            "api/v1/contact",
        )
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| ChannelError::Connection {
            platform: "imessage".to_string(),
            message: format!("Failed to list iMessage contacts: {}", e),
        })?;

        let status = response.status();
        let value: Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));
        if !status.is_success() {
            return Err(ChannelError::Connection {
                platform: "imessage".to_string(),
                message: format!("BlueBubbles contact query failed: {}", value),
            }
            .into());
        }

        Ok(value)
    }
}

#[derive(Clone)]
pub struct IMessageWebhookHandler {
    config: IMessageConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl IMessageWebhookHandler {
    pub async fn handle_event(&self, payload: BlueBubblesMessage) -> Result<()> {
        let Some(msg) = IMessageChannel::incoming_from_bluebubbles(&self.config, payload) else {
            return Ok(());
        };
        let _ = self.incoming_tx.send(msg).await;
        Ok(())
    }

    pub fn verify_password(&self, candidate: Option<&str>) -> bool {
        match (&self.config.bridge_mode, candidate) {
            (IMessageBridgeMode::BlueBubbles { password, .. }, Some(candidate)) => {
                candidate == password
            }
            (IMessageBridgeMode::BlueBubbles { .. }, None) => false,
            _ => true,
        }
    }
}

fn address_allowed(config: &IMessageConfig, address: &str) -> bool {
    if config.allowlist.is_empty() {
        return true;
    }
    config.allowlist.contains(&address.to_string())
}

#[async_trait]
impl Channel for IMessageChannel {
    fn platform(&self) -> Platform {
        Platform::IMessage
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if let Some(tapback) = msg
            .metadata
            .get("imessage_tapback")
            .or_else(|| msg.metadata.get("imessage_reaction"))
            .and_then(Self::parse_tapback)
        {
            if !self.config.enable_tapbacks {
                return Err(ChannelError::Config {
                    platform: "imessage".to_string(),
                    message: "Tapbacks are disabled in iMessage config".to_string(),
                }
                .into());
            }

            let chat_guid = msg
                .metadata
                .get("imessage_chat_guid")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "imessage".to_string(),
                    message: "Missing imessage_chat_guid in metadata".to_string(),
                })?;
            let message_guid = msg
                .metadata
                .get("imessage_associated_message_guid")
                .or_else(|| msg.metadata.get("imessage_message_guid"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "imessage".to_string(),
                    message: "Missing target message guid for iMessage tapback".to_string(),
                })?;

            return self.send_tapback(chat_guid, message_guid, tapback).await;
        }

        let attachments = Self::outgoing_attachments(&msg.metadata).await?;

        match &self.config.bridge_mode {
            IMessageBridgeMode::BlueBubbles {
                server_url,
                password,
            } => {
                let chat_guid = Self::direct_target_from_metadata(&msg.metadata).ok_or_else(|| {
                    ChannelError::InvalidFormat {
                        platform: "imessage".to_string(),
                        message: "Missing imessage_chat_guid or imessage_recipient in metadata"
                            .to_string(),
                    }
                })?;

                if attachments.is_empty() {
                    self.send_bluebubbles(server_url, password, &chat_guid, &msg.content)
                        .await
                } else {
                    self.send_bluebubbles_attachments(
                        server_url,
                        password,
                        &chat_guid,
                        &msg.content,
                        &attachments,
                    )
                    .await
                }
            }
            IMessageBridgeMode::MacOSDirect => {
                if !attachments.is_empty() {
                    return Err(ChannelError::Config {
                        platform: "imessage".to_string(),
                        message: "macOS direct mode does not support file attachments".to_string(),
                    }
                    .into());
                }
                let recipient = Self::applescript_target_from_metadata(&msg.metadata).ok_or_else(
                    || ChannelError::InvalidFormat {
                        platform: "imessage".to_string(),
                        message: "Missing imessage_recipient or imessage_chat_guid in metadata"
                            .to_string(),
                    },
                )?;
                self.send_applescript(&recipient, &msg.content).await
            }
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
                self.ping().await?;
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
    /// Whether the chat is a group conversation
    #[serde(rename = "isGroup")]
    pub is_group: Option<bool>,
    /// Group or conversation display name
    #[serde(rename = "chatDisplayName")]
    pub chat_display_name: Option<String>,
    /// Participants in the conversation, when provided by BlueBubbles
    #[serde(rename = "participants")]
    pub participants: Option<Vec<Value>>,
    /// Attachments carried by the message, when provided by BlueBubbles
    #[serde(rename = "attachments")]
    pub attachments: Option<Vec<Value>>,
    /// Chat metadata, when BlueBubbles includes the expanded chat payload.
    #[serde(rename = "chats")]
    pub chats: Option<Vec<BlueBubblesChat>>,
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

/// BlueBubbles chat metadata included in some webhook events.
#[derive(Debug, Clone, Deserialize)]
pub struct BlueBubblesChat {
    #[serde(rename = "guid")]
    pub guid: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "chatIdentifier")]
    pub chat_identifier: Option<String>,
    #[serde(rename = "participants")]
    pub participants: Option<Vec<Value>>,
}

struct OutgoingAttachment {
    bytes: Vec<u8>,
    filename: String,
    mime: Option<String>,
}

/// Check if running on macOS.
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::traits::Channel;
    use openrustclaw_core::types::OutgoingMessage;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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

    #[test]
    fn test_incoming_from_bluebubbles_group_mapping_and_attachments() {
        let config = IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::MacOSDirect,
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        };

        let payload: BlueBubblesMessage = serde_json::from_value(serde_json::json!({
            "guid": "group-guid-1",
            "text": "hello group",
            "handle": {
                "address": "+1234567890",
                "firstName": "Alice",
                "lastName": null
            },
            "dateCreated": "2024-01-15T10:30:00Z",
            "isFromMe": false,
            "chatGUID": "chat-guid-group",
            "isGroup": true,
            "chatDisplayName": "Weekend Plans",
            "participants": [
                {"address": "+1234567890"},
                {"address": "+1098765432"}
            ],
            "attachments": [{
                "path": "/tmp/photo.jpg",
                "name": "photo.jpg",
                "mimeType": "image/jpeg",
                "size": 2048
            }]
        }))
        .unwrap();

        let incoming = IMessageChannel::incoming_from_bluebubbles(&config, payload).unwrap();
        assert_eq!(incoming.metadata["imessage_is_group"], true);
        assert_eq!(
            incoming.metadata["imessage_chat_display_name"],
            "Weekend Plans"
        );
        assert_eq!(incoming.metadata["imessage_participant_count"], 2);
        assert_eq!(
            incoming.metadata["file_references"][0]["local_path"],
            "/tmp/photo.jpg"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["mime"],
            "image/jpeg"
        );
    }

    #[test]
    fn test_parse_tapback_aliases() {
        assert_eq!(
            IMessageChannel::parse_tapback(&serde_json::json!("heart")),
            Some(TapbackType::Love)
        );
        assert_eq!(
            IMessageChannel::parse_tapback(&serde_json::json!("thumbs_down")),
            Some(TapbackType::Dislike)
        );
        assert_eq!(
            IMessageChannel::parse_tapback(&serde_json::json!(3)),
            Some(TapbackType::Laugh)
        );
    }

    #[test]
    fn test_webhook_handler_verifies_password() {
        let channel = IMessageChannel::new(IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::BlueBubbles {
                server_url: "http://127.0.0.1:1234".to_string(),
                password: "secret".to_string(),
            },
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        });

        let handler = channel.webhook_handler();
        assert!(handler.verify_password(Some("secret")));
        assert!(!handler.verify_password(Some("wrong")));
        assert!(!handler.verify_password(None));
    }

    #[tokio::test]
    async fn test_send_uses_tapback_metadata_path() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let response =
                b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\ncontent-type: application/json\r\n\r\n{}";
            socket.write_all(response).await.unwrap();
            request
        });

        let channel = IMessageChannel::new(IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::BlueBubbles {
                server_url: format!("http://{}", addr),
                password: "test-password".to_string(),
            },
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        });

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "ignored".to_string(),
                metadata: serde_json::json!({
                    "imessage_chat_guid": "chat-guid-1",
                    "imessage_message_guid": "message-guid-1",
                    "imessage_tapback": "heart"
                }),
            })
            .await
            .unwrap();

        let request = server.await.unwrap();
        assert!(request.contains("POST /api/v1/message/react"));
        assert!(request.contains("chat-guid-1"));
        assert!(request.contains("message-guid-1"));
        assert!(request.contains("\"reaction\":0"));
    }

    #[tokio::test]
    async fn test_connect_pings_bluebubbles() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let response = b"HTTP/1.1 200 OK\r\ncontent-length: 55\r\ncontent-type: application/json\r\n\r\n{\"status\":200,\"message\":\"Ping received!\",\"data\":\"pong\"}";
            socket.write_all(response).await.unwrap();
            request
        });

        let mut channel = IMessageChannel::new(IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::BlueBubbles {
                server_url: format!("http://{}", addr),
                password: "test-password".to_string(),
            },
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        });

        channel.connect().await.unwrap();
        channel.disconnect().await.unwrap();

        let request = server.await.unwrap();
        assert!(request.contains("GET /api/v1/ping?password=test-password"));
    }

    #[tokio::test]
    async fn test_send_accepts_imessage_recipient_alias() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let response =
                b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\ncontent-type: application/json\r\n\r\n{}";
            socket.write_all(response).await.unwrap();
            request
        });

        let channel = IMessageChannel::new(IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::BlueBubbles {
                server_url: format!("http://{}", addr),
                password: "test-password".to_string(),
            },
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        });

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "hello there".to_string(),
                metadata: serde_json::json!({
                    "imessage_recipient": "+15555550123"
                }),
            })
            .await
            .unwrap();

        let request = server.await.unwrap();
        assert!(request.contains("POST /api/v1/message/text?password=test-password"));
        assert!(request.contains("\"chatGuid\":\"any;-;+15555550123\""));
        assert!(request.contains("\"message\":\"hello there\""));
    }

    #[tokio::test]
    async fn test_send_supports_local_attachment_uploads() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let response =
                b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\ncontent-type: application/json\r\n\r\n{}";
            socket.write_all(response).await.unwrap();
            request
        });

        let attachment_path = std::env::temp_dir().join(format!(
            "orc-imessage-attachment-{}.txt",
            Uuid::new_v4()
        ));
        tokio::fs::write(&attachment_path, b"hello from attachment")
            .await
            .unwrap();

        let channel = IMessageChannel::new(IMessageConfig {
            enabled: true,
            bridge_mode: IMessageBridgeMode::BlueBubbles {
                server_url: format!("http://{}", addr),
                password: "test-password".to_string(),
            },
            allowlist: vec![],
            enable_tapbacks: true,
            enable_typing_indicator: false,
        });

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "see attached".to_string(),
                metadata: serde_json::json!({
                    "imessage_recipient": "+15555550123",
                    "file_references": [{
                        "local_path": attachment_path,
                        "name": "note.txt",
                        "mime": "text/plain"
                    }]
                }),
            })
            .await
            .unwrap();

        let request = server.await.unwrap();
        assert!(request.contains("POST /api/v1/message/attachment?password=test-password"));
        assert!(request.contains("name=\"chatGuid\""));
        assert!(request.contains("any;-;+15555550123"));
        assert!(request.contains("filename=\"note.txt\""));
        assert!(request.contains("see attached"));
    }
}
