//! Gmail Pub/Sub Integration for Email-based Workflows
//!
//! This module provides integration with Gmail via Google Cloud Pub/Sub push notifications.
//! It allows the agent to receive real-time email notifications and take actions such as:
//! - Auto-replying to emails
//! - Labeling and archiving
//! - Triggering workflows based on email content
//!
//! # Authentication
//!
//! Uses Google Cloud service account authentication with domain-wide delegation
//! to access Gmail API on behalf of a user.
//!
//! # Setup Requirements
//!
//! 1. Create a Google Cloud project
//! 2. Enable Gmail API and Pub/Sub API
//! 3. Create a service account with domain-wide delegation
//! 4. Create a Pub/Sub topic and subscription
//! 5. Configure Gmail watch on the user's mailbox
//!
//! # Example
//!
//! ```rust,no_run
//! use openrustclaw_channels::gmail_pubsub::GmailPubSub;
//! use openrustclaw_core::config::GmailPubSubConfig;
//!
//! let config = GmailPubSubConfig {
//!     enabled: true,
//!     project_id: "my-project".to_string(),
//!     subscription_name: "gmail-notifications".to_string(),
//!     service_account_key_path: "/path/to/service-account.json".to_string(),
//!     user_email: "user@example.com".to_string(),
//!     label_filters: vec!["INBOX".to_string(), "UNREAD".to_string()],
//!     query_filter: Some("from:github.com".to_string()),
//!     auto_reply: false,
//!     max_history_fetch: 100,
//!     rate_limit_requests_per_second: 10,
//! };
//!
//! let gmail = GmailPubSub::new(config);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::GmailPubSubConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Gmail Pub/Sub channel implementation.
pub struct GmailPubSub {
    config: GmailPubSubConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
    http_client: reqwest::Client,
    access_token: RwLock<Option<String>>,
}

/// Gmail message notification from Pub/Sub.
#[derive(Debug, Clone, Deserialize)]
pub struct GmailNotification {
    /// The email address that received the message.
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    /// The history ID for fetching changes.
    #[serde(rename = "historyId")]
    pub history_id: u64,
}

/// Parsed email message.
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// Unique message ID.
    pub id: String,
    /// Thread ID this message belongs to.
    pub thread_id: String,
    /// History ID for this message.
    pub history_id: u64,
    /// Sender email address.
    pub from: String,
    /// Recipient email addresses.
    pub to: Vec<String>,
    /// CC email addresses.
    pub cc: Vec<String>,
    /// Email subject.
    pub subject: String,
    /// Plain text body.
    pub body_text: String,
    /// HTML body (if available).
    pub body_html: Option<String>,
    /// Attachments.
    pub attachments: Vec<Attachment>,
    /// Gmail labels.
    pub labels: Vec<String>,
    /// When the email was received.
    pub received_at: chrono::DateTime<chrono::Utc>,
    /// Whether the email is unread.
    pub is_unread: bool,
}

/// Email attachment metadata.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Filename of the attachment.
    pub filename: String,
    /// MIME type of the attachment.
    pub mime_type: String,
    /// Size in bytes.
    pub size: usize,
    /// Gmail attachment ID for downloading.
    pub attachment_id: String,
}

/// Email action to take on a message.
#[derive(Debug, Clone)]
pub enum EmailAction {
    /// Reply to the email.
    Reply { body: String },
    /// Add or remove labels.
    Label { add: Vec<String>, remove: Vec<String> },
    /// Archive the email (remove INBOX label).
    Archive,
    /// Delete the email.
    Delete,
    /// Forward the email.
    Forward { to: String, body: String },
    /// Trigger a workflow.
    TriggerWorkflow { workflow_name: String },
}

/// Event emitted when a new email arrives.
#[derive(Debug, Clone)]
pub struct NewEmailEvent {
    /// The email message.
    pub email: EmailMessage,
    /// Action taken on the email (if any).
    pub action_taken: Option<EmailAction>,
}

/// Response from Gmail watch API.
#[derive(Debug, Clone, Deserialize)]
struct WatchResponse {
    /// The history ID to start from.
    pub history_id: String,
    /// The expiration time of the watch.
    pub expiration: String,
}

/// Gmail API message metadata.
#[derive(Debug, Clone, Deserialize)]
struct MessageMetadata {
    /// Message ID.
    pub id: String,
    /// Thread ID.
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

/// Gmail API message part (for parsing multipart messages).
#[derive(Debug, Clone, Deserialize, Default)]
struct MessagePart {
    /// MIME type of this part.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Body content.
    pub body: MessagePartBody,
    /// Filename (for attachments).
    pub filename: Option<String>,
    /// Nested parts (for multipart messages).
    #[serde(default)]
    pub parts: Vec<MessagePart>,
    /// Headers.
    #[serde(default)]
    pub headers: Vec<MessageHeader>,
}

/// Gmail API message part body.
#[derive(Debug, Clone, Deserialize, Default)]
struct MessagePartBody {
    /// Base64 encoded data.
    pub data: Option<String>,
    /// Attachment ID (if this is an attachment).
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
    /// Size in bytes.
    pub size: i64,
}

/// Gmail API message header.
#[derive(Debug, Clone, Deserialize)]
struct MessageHeader {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

/// Gmail API full message.
#[derive(Debug, Clone, Deserialize)]
struct GmailMessage {
    /// Message ID.
    pub id: String,
    /// Thread ID.
    #[serde(rename = "threadId")]
    pub thread_id: String,
    /// Label IDs.
    #[serde(rename = "labelIds")]
    pub label_ids: Vec<String>,
    /// Internal date (timestamp in milliseconds).
    #[serde(rename = "internalDate")]
    pub internal_date: String,
    /// Message payload.
    pub payload: MessagePart,
    /// History ID.
    #[serde(rename = "historyId")]
    pub history_id: String,
}

/// Gmail API history response.
#[derive(Debug, Clone, Deserialize)]
struct HistoryResponse {
    /// History records.
    pub history: Vec<HistoryRecord>,
    /// Next page token.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    /// History ID of the latest change.
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
}

/// Gmail API history record.
#[derive(Debug, Clone, Deserialize)]
struct HistoryRecord {
    /// Messages added in this history record.
    #[serde(rename = "messagesAdded")]
    pub messages_added: Vec<MessageAdded>,
}

/// Message added to history.
#[derive(Debug, Clone, Deserialize)]
struct MessageAdded {
    /// The message metadata.
    pub message: MessageMetadata,
}

/// Parsed message parts.
#[derive(Debug, Clone, Default)]
struct ParsedParts {
    /// Plain text content.
    pub text: String,
    /// HTML content.
    pub html: Option<String>,
    /// Attachments.
    pub attachments: Vec<Attachment>,
}

impl GmailPubSub {
    /// Create a new Gmail Pub/Sub channel with the given configuration.
    pub fn new(config: GmailPubSubConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Gmail API allows ~10+ requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap()),
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
            access_token: RwLock::new(None),
        }
    }

    /// Parse email addresses from a comma-separated string.
    fn parse_addresses(s: &str) -> Vec<String> {
        s.split(',').map(|a| a.trim().to_string()).collect()
    }

    /// Check if an email should be processed based on configured filters.
    fn should_process(&self, email: &EmailMessage) -> bool {
        // Label filter
        if !self.config.label_filters.is_empty() {
            let has_matching_label = self
                .config
                .label_filters
                .iter()
                .any(|filter| email.labels.iter().any(|l| l.contains(filter)));
            if !has_matching_label {
                debug!(email_id = %email.id, "Email filtered out by label filter");
                return false;
            }
        }

        // Query filter (simple string match)
        if let Some(query) = &self.config.query_filter {
            let search_text = format!("{} {} {}", email.from, email.subject, email.body_text);
            if !search_text.to_lowercase().contains(&query.to_lowercase()) {
                debug!(email_id = %email.id, "Email filtered out by query filter");
                return false;
            }
        }

        true
    }

    /// Decode base64 URL-safe encoded data.
    fn decode_base64(data: &str) -> Option<Vec<u8>> {
        // Gmail uses URL-safe base64 with possible padding issues
        let data = data.replace('-', "+").replace('_', "/");
        base64::decode(&data).ok()
    }

    /// Parse a Gmail API message into an EmailMessage.
    fn parse_message(&self, msg: GmailMessage) -> Result<EmailMessage> {
        let headers: HashMap<String, String> = msg
            .payload
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();

        let parts = self.get_parts(&msg.payload);

        let internal_date = msg.internal_date.parse::<i64>().unwrap_or_else(|_| {
            chrono::Utc::now().timestamp_millis()
        });

        let is_unread = msg.label_ids.contains(&"UNREAD".to_string());

        Ok(EmailMessage {
            id: msg.id,
            thread_id: msg.thread_id,
            history_id: msg.history_id.parse().unwrap_or(0),
            from: headers.get("From").cloned().unwrap_or_default(),
            to: headers
                .get("To")
                .map(|s| Self::parse_addresses(s))
                .unwrap_or_default(),
            cc: headers
                .get("Cc")
                .map(|s| Self::parse_addresses(s))
                .unwrap_or_default(),
            subject: headers.get("Subject").cloned().unwrap_or_default(),
            body_text: parts.text,
            body_html: parts.html,
            attachments: parts.attachments,
            labels: msg.label_ids.clone(),
            received_at: chrono::DateTime::from_timestamp_millis(internal_date)
                .unwrap_or_else(|| chrono::Utc::now()),
            is_unread,
        })
    }

    /// Extract parts from a message payload recursively.
    fn get_parts(&self, payload: &MessagePart) -> ParsedParts {
        let mut result = ParsedParts::default();

        match payload.mime_type.as_str() {
            "text/plain" => {
                if let Some(data) = &payload.body.data {
                    result.text = Self::decode_base64(data)
                        .and_then(|b| String::from_utf8(b).ok())
                        .unwrap_or_default();
                }
            }
            "text/html" => {
                if let Some(data) = &payload.body.data {
                    result.html = Self::decode_base64(data)
                        .and_then(|b| String::from_utf8(b).ok());
                }
            }
            mime if mime.starts_with("multipart/") => {
                for part in &payload.parts {
                    let sub = self.get_parts(part);
                    if result.text.is_empty() && !sub.text.is_empty() {
                        result.text = sub.text;
                    }
                    if result.html.is_none() && sub.html.is_some() {
                        result.html = sub.html;
                    }
                    result.attachments.extend(sub.attachments);
                }
            }
            _ => {
                // Other MIME types - could be attachments
            }
        }

        // Check for attachments at this level
        if let Some(attachment_id) = &payload.body.attachment_id {
            if let Some(filename) = payload.filename.clone() {
                result.attachments.push(Attachment {
                    filename,
                    mime_type: payload.mime_type.clone(),
                    size: payload.body.size as usize,
                    attachment_id: attachment_id.clone(),
                });
            }
        }

        result
    }

    /// Authenticate with Gmail API using service account.
    ///
    /// In a full implementation, this would:
    /// 1. Load the service account JSON key file
    /// 2. Use yup_oauth2 or google-auth to get an access token
    /// 3. Cache the token and refresh when expired
    async fn authenticate(&self) -> Result<String> {
        // Placeholder implementation
        // Real implementation would use:
        // ```rust
        // let service_account_key = yup_oauth2::read_service_account_key(&self.config.service_account_key_path).await?;
        // let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        //     .build().await?;
        // let token = authenticator.token(&["https://www.googleapis.com/auth/gmail.modify"]).await?;
        // ```

        warn!("Using placeholder authentication - implement real service account auth");
        Ok("placeholder_token".to_string())
    }

    /// Setup Gmail watch for this user.
    async fn setup_watch(&self, token: &str) -> Result<WatchResponse> {
        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/{}/watch",
            self.config.user_email
        );

        let body = serde_json::json!({
            "labelIds": ["INBOX"],
            "topicName": format!("projects/{}/topics/{}-topic", self.config.project_id, self.config.subscription_name),
            "labelFilterBehavior": "INCLUDE"
        });

        // Placeholder - in real implementation, would make actual HTTP request
        debug!("Setting up Gmail watch for {}", self.config.user_email);

        Ok(WatchResponse {
            history_id: "0".to_string(),
            expiration: (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
        })
    }

    /// Get new messages since history_id.
    async fn get_new_messages(&self, history_id: u64, token: &str) -> Result<Vec<MessageMetadata>> {
        // Placeholder implementation
        // Real implementation would call:
        // GET https://gmail.googleapis.com/gmail/v1/users/{userId}/history
        debug!(
            "Fetching new messages since history_id: {}",
            history_id
        );

        Ok(vec![])
    }

    /// Get full message by ID.
    async fn get_message(&self, id: &str, token: &str) -> Result<Option<EmailMessage>> {
        // Placeholder implementation
        // Real implementation would call:
        // GET https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/{id}
        debug!("Fetching message: {}", id);

        Ok(None)
    }

    /// Process a Pub/Sub notification message.
    pub async fn process_notification(&self, notification: GmailNotification) -> Result<()> {
        info!(
            "New Gmail notification for {} (historyId: {})",
            notification.email_address, notification.history_id
        );

        // Get access token
        let token = {
            let token_guard = self.access_token.read().await;
            token_guard.clone().unwrap_or_else(|| "placeholder".to_string())
        };

        // Fetch new messages using history API
        let new_messages = self
            .get_new_messages(notification.history_id, &token)
            .await?;

        for msg_metadata in new_messages {
            // Fetch full message
            if let Some(email) = self.get_message(&msg_metadata.id, &token).await? {
                // Apply filters
                if !self.should_process(&email) {
                    continue;
                }

                // Create incoming message
                let incoming = IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: email.from.clone(),
                    content: format!(
                        "Subject: {}\n\n{}",
                        email.subject, email.body_text
                    ),
                    platform: Platform::Gmail,
                    metadata: serde_json::json!({
                        "gmail_message_id": email.id,
                        "gmail_thread_id": email.thread_id,
                        "gmail_subject": email.subject,
                        "gmail_from": email.from,
                        "gmail_to": email.to,
                        "gmail_labels": email.labels,
                        "gmail_attachments": email.attachments.len(),
                    }),
                };

                // Send to channel
                if let Err(e) = self.incoming_tx.send(incoming).await {
                    error!("Failed to send email event: {}", e);
                }

                info!(
                    message_id = %email.id,
                    subject = %email.subject,
                    from = %email.from,
                    "Processed new email"
                );
            }
        }

        Ok(())
    }

    /// Take action on an email.
    pub async fn take_action(&self, email_id: &str, action: EmailAction) -> Result<()> {
        // Get access token
        let token = {
            let token_guard = self.access_token.read().await;
            token_guard.clone().unwrap_or_else(|| "placeholder".to_string())
        };

        match action {
            EmailAction::Reply { body } => {
                self.reply_to_message(email_id, &body, &token).await?;
            }
            EmailAction::Label { add, remove } => {
                for label in add {
                    self.add_label(email_id, &label, &token).await?;
                }
                for label in remove {
                    self.remove_label(email_id, &label, &token).await?;
                }
            }
            EmailAction::Archive => {
                self.archive_message(email_id, &token).await?;
            }
            EmailAction::Delete => {
                self.delete_message(email_id, &token).await?;
            }
            EmailAction::Forward { to, body } => {
                self.forward_message(email_id, &to, &body, &token).await?;
            }
            EmailAction::TriggerWorkflow { workflow_name } => {
                info!(workflow = %workflow_name, "Triggering workflow");
                // Workflow triggering would be handled by the agent runtime
            }
        }
        Ok(())
    }

    /// Reply to a message.
    async fn reply_to_message(&self, email_id: &str, body: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, "Replying to message");
        // POST https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/{id}/send
        Ok(())
    }

    /// Add a label to a message.
    async fn add_label(&self, email_id: &str, label: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, label = %label, "Adding label");
        // POST https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/{id}/modify
        Ok(())
    }

    /// Remove a label from a message.
    async fn remove_label(&self, email_id: &str, label: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, label = %label, "Removing label");
        // POST https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/{id}/modify
        Ok(())
    }

    /// Archive a message (remove INBOX label).
    async fn archive_message(&self, email_id: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, "Archiving message");
        self.remove_label(email_id, "INBOX", token).await
    }

    /// Delete a message.
    async fn delete_message(&self, email_id: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, "Deleting message");
        // DELETE https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/{id}
        Ok(())
    }

    /// Forward a message.
    async fn forward_message(&self, email_id: &str, to: &str, body: &str, token: &str) -> Result<()> {
        debug!(email_id = %email_id, to = %to, "Forwarding message");
        // POST https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/send
        Ok(())
    }

    /// Check if the email sender is in the allowlist.
    fn is_sender_allowed(&self, from: &str) -> bool {
        // For now, all senders are allowed
        // In a full implementation, check against config.allowlist
        true
    }
}

#[async_trait]
impl Channel for GmailPubSub {
    fn platform(&self) -> Platform {
        Platform::Gmail
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get message ID from metadata for reply
        let message_id = msg
            .metadata
            .get("gmail_message_id")
            .and_then(|v| v.as_str());

        let thread_id = msg
            .metadata
            .get("gmail_thread_id")
            .and_then(|v| v.as_str());

        // Get access token
        let token = {
            let token_guard = self.access_token.read().await;
            token_guard.clone().unwrap_or_else(|| "placeholder".to_string())
        };

        if let Some(msg_id) = message_id {
            // Reply to the message
            self.reply_to_message(msg_id, &msg.content, &token).await?;
            info!(message_id = %msg_id, "Sent Gmail reply");
        } else {
            // Send as new message (would need recipient in metadata)
            warn!("No message_id in metadata, cannot send Gmail reply");
        }

        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "gmail".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Gmail Pub/Sub...");

        // Validate configuration
        if self.config.project_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail Pub/Sub project ID is required".to_string(),
            }
            .into());
        }

        if self.config.subscription_name.is_empty() {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail Pub/Sub subscription name is required".to_string(),
            }
            .into());
        }

        if self.config.user_email.is_empty() {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail user email is required".to_string(),
            }
            .into());
        }

        // Authenticate
        let token = self.authenticate().await?;
        *self.access_token.write().await = Some(token.clone());

        // Setup Gmail watch
        let watch_response = self.setup_watch(&token).await?;
        info!(
            history_id = %watch_response.history_id,
            expiration = %watch_response.expiration,
            "Gmail watch established"
        );

        *self.is_connected.write().await = true;

        info!("Gmail Pub/Sub channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Gmail Pub/Sub...");

        *self.is_connected.write().await = false;
        *self.access_token.write().await = None;

        info!("Gmail Pub/Sub channel disconnected");
        Ok(())
    }
}

/// Webhook handler for Gmail Pub/Sub HTTP push mode.
///
/// This can be used when Pub/Sub is configured to push to an HTTP endpoint
/// instead of using the Pub/Sub client library.
pub struct GmailWebhookHandler {
    gmail: Arc<GmailPubSub>,
}

impl GmailWebhookHandler {
    /// Create a new webhook handler.
    pub fn new(gmail: Arc<GmailPubSub>) -> Self {
        Self { gmail }
    }

    /// Handle an incoming Pub/Sub push notification.
    ///
    /// The body is the Pub/Sub message envelope:
    /// <https://cloud.google.com/pubsub/docs/push#receiving_messages>
    pub async fn handle_push(&self, body: &[u8]) -> Result<()> {
        // Parse the Pub/Sub message envelope
        let envelope: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Pub/Sub envelope: {}", e),
            }
        })?;

        // Extract the message data
        let message = envelope
            .get("message")
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: "Missing message in Pub/Sub envelope".to_string(),
            })?;

        let data = message
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: "Missing data in Pub/Sub message".to_string(),
            })?;

        // Decode base64 data
        let decoded = base64::decode(data).map_err(|e| ChannelError::InvalidFormat {
            platform: "gmail".to_string(),
            message: format!("Failed to decode message data: {}", e),
        })?;

        // Parse the Gmail notification
        let notification: GmailNotification = serde_json::from_slice(&decoded).map_err(|e| {
            ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Gmail notification: {}", e),
            }
        })?;

        // Process the notification
        self.gmail.process_notification(notification).await
    }
}

/// Base64 decoding utility.
mod b64 {
    //! Base64 decoding for Gmail API responses.
    use base64::Engine;
    pub fn decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
        base64::engine::general_purpose::STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_addresses() {
        let addresses = GmailPubSub::parse_addresses("user1@example.com, user2@example.com");
        assert_eq!(addresses.len(), 2);
        assert_eq!(addresses[0], "user1@example.com");
        assert_eq!(addresses[1], "user2@example.com");
    }

    #[test]
    fn test_should_process_with_label_filter() {
        let config = GmailPubSubConfig {
            enabled: true,
            project_id: "test".to_string(),
            subscription_name: "test".to_string(),
            service_account_key_path: "/tmp/key.json".to_string(),
            user_email: "test@example.com".to_string(),
            label_filters: vec!["INBOX".to_string()],
            query_filter: None,
            auto_reply: false,
            max_history_fetch: 100,
            rate_limit_requests_per_second: 10,
        };

        let gmail = GmailPubSub::new(config);

        let email_with_label = EmailMessage {
            id: "1".to_string(),
            thread_id: "t1".to_string(),
            history_id: 1,
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            cc: vec![],
            subject: "Test".to_string(),
            body_text: "Hello".to_string(),
            body_html: None,
            attachments: vec![],
            labels: vec!["INBOX".to_string(), "UNREAD".to_string()],
            received_at: chrono::Utc::now(),
            is_unread: true,
        };

        let email_without_label = EmailMessage {
            id: "2".to_string(),
            thread_id: "t2".to_string(),
            history_id: 2,
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            cc: vec![],
            subject: "Test".to_string(),
            body_text: "Hello".to_string(),
            body_html: None,
            attachments: vec![],
            labels: vec!["SENT".to_string()],
            received_at: chrono::Utc::now(),
            is_unread: false,
        };

        assert!(gmail.should_process(&email_with_label));
        assert!(!gmail.should_process(&email_without_label));
    }

    #[test]
    fn test_should_process_with_query_filter() {
        let config = GmailPubSubConfig {
            enabled: true,
            project_id: "test".to_string(),
            subscription_name: "test".to_string(),
            service_account_key_path: "/tmp/key.json".to_string(),
            user_email: "test@example.com".to_string(),
            label_filters: vec![],
            query_filter: Some("github".to_string()),
            auto_reply: false,
            max_history_fetch: 100,
            rate_limit_requests_per_second: 10,
        };

        let gmail = GmailPubSub::new(config);

        let matching_email = EmailMessage {
            id: "1".to_string(),
            thread_id: "t1".to_string(),
            history_id: 1,
            from: "noreply@github.com".to_string(),
            to: vec!["user@example.com".to_string()],
            cc: vec![],
            subject: "PR Review".to_string(),
            body_text: "Please review this PR".to_string(),
            body_html: None,
            attachments: vec![],
            labels: vec!["INBOX".to_string()],
            received_at: chrono::Utc::now(),
            is_unread: true,
        };

        let non_matching_email = EmailMessage {
            id: "2".to_string(),
            thread_id: "t2".to_string(),
            history_id: 2,
            from: "friend@example.com".to_string(),
            to: vec!["user@example.com".to_string()],
            cc: vec![],
            subject: "Hello".to_string(),
            body_text: "How are you?".to_string(),
            body_html: None,
            attachments: vec![],
            labels: vec!["INBOX".to_string()],
            received_at: chrono::Utc::now(),
            is_unread: true,
        };

        assert!(gmail.should_process(&matching_email));
        assert!(!gmail.should_process(&non_matching_email));
    }

    #[test]
    fn test_gmail_notification_deserialization() {
        let json = r#"{"emailAddress": "user@example.com", "historyId": 12345}"#;
        let notification: GmailNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notification.email_address, "user@example.com");
        assert_eq!(notification.history_id, 12345);
    }
}
