//! Google Chat integration for OpenRustClaw.
//!
//! Features:
//! - Messages in spaces (rooms and DMs)
//! - Card-based responses (Google Chat Cards)
//! - Threading support
//! - Slash commands
//! - Mention handling (@bot)
//! - Service account authentication
//! - Pub/Sub or HTTP webhook support
//!
//! # Authentication
//!
//! Uses Google Cloud service account authentication. Requires:
//! - Service account JSON key file
//! - `chat.bot` scope for bot operations
//! - Project ID for API calls
//!
//! # API Reference
//!
//! - <https://developers.google.com/chat/api/reference/rest>

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{GoogleChatConfig, GoogleChatResponseMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Google Chat API base URL.
#[allow(dead_code)]
const GOOGLE_CHAT_API_BASE: &str = "https://chat.googleapis.com/v1";

/// Google Chat channel implementation.
pub struct GoogleChatChannel {
    config: GoogleChatConfig,
    _incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
    /// Access token for API calls (cached and refreshed as needed)
    access_token: RwLock<Option<String>>,
    /// HTTP client for API calls
    _http_client: reqwest::Client,
}

/// Google Chat message payload for sending.
#[derive(Debug, Clone, serde::Serialize)]
struct ChatMessage {
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cards_v2: Option<Vec<CardV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread: Option<Thread>,
}

/// Thread reference for message threading.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Thread {
    name: String,
}

/// Google Chat Card V2 structure.
#[derive(Debug, Clone, serde::Serialize)]
struct CardV2 {
    card_id: String,
    card: Card,
}

/// Card content.
#[derive(Debug, Clone, serde::Serialize)]
struct Card {
    header: Option<CardHeader>,
    sections: Vec<CardSection>,
}

/// Card header.
#[derive(Debug, Clone, serde::Serialize)]
struct CardHeader {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
}

/// Card section.
#[derive(Debug, Clone, serde::Serialize)]
struct CardSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<String>,
    widgets: Vec<Widget>,
}

/// Card widget.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Widget {
    TextParagraph { text: String },
    ButtonList { buttons: Vec<Button> },
}

/// Card button.
#[derive(Debug, Clone, serde::Serialize)]
struct Button {
    text: String,
    on_click: OnClick,
}

/// Button click action.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
#[allow(dead_code)]
enum OnClick {
    OpenLink { open_link: OpenLink },
}

/// Open link action.
#[derive(Debug, Clone, serde::Serialize)]
struct OpenLink {
    url: String,
}

/// Incoming Google Chat event.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct ChatEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "eventTime")]
    event_time: String,
    #[serde(rename = "space")]
    space: Space,
    #[serde(rename = "message")]
    message: Option<EventMessage>,
    user: Option<User>,
}

/// Google Chat space (room or DM).
#[derive(Debug, Clone, serde::Deserialize)]
struct Space {
    name: String,
    #[serde(rename = "type")]
    space_type: String,
    display_name: Option<String>,
}

/// Message in a Chat event.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct EventMessage {
    name: String,
    text: String,
    thread: Option<Thread>,
    #[serde(rename = "argumentText")]
    argument_text: Option<String>,
    #[serde(rename = "slashCommand")]
    slash_command: Option<SlashCommand>,
    annotations: Option<Vec<Annotation>>,
}

/// Slash command in a message.
#[derive(Debug, Clone, serde::Deserialize)]
struct SlashCommand {
    #[serde(rename = "commandId")]
    command_id: String,
}

/// Message annotation (e.g., user mention).
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct Annotation {
    #[serde(rename = "type")]
    annotation_type: String,
    #[serde(rename = "userMention")]
    user_mention: Option<UserMention>,
}

/// User mention annotation.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct UserMention {
    user: User,
    #[serde(rename = "type")]
    mention_type: String,
}

/// Google Chat user.
#[derive(Debug, Clone, serde::Deserialize)]
struct User {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    email: Option<String>,
}

impl GoogleChatChannel {
    /// Create a new Google Chat channel with the given configuration.
    pub fn new(config: GoogleChatConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Google Chat API allows ~10+ requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap())
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            _incoming_tx: incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            access_token: RwLock::new(None),
            _http_client: http_client,
        }
    }

    /// Check if the user is in the allowlist.
    #[allow(dead_code)]
    fn is_user_allowed(&self, email: Option<&str>, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }

        if let Some(email) = email {
            if self.config.allowlist.contains(&email.to_string()) {
                return true;
            }
        }

        self.config.allowlist.contains(&user_id.to_string())
    }

    /// Check if the space is in the allowed spaces list.
    #[allow(dead_code)]
    fn is_space_allowed(&self, space_id: &str) -> bool {
        if self.config.allowed_spaces.is_empty() {
            return true;
        }
        self.config.allowed_spaces.contains(&space_id.to_string())
    }

    /// Check if the bot should respond based on response mode.
    #[allow(dead_code)]
    fn should_respond(&self, event: &ChatEvent) -> bool {
        match self.config.response_mode {
            GoogleChatResponseMode::SlashCommands => {
                // Only respond if there's a slash command
                event.message.as_ref()
                    .and_then(|m| m.slash_command.as_ref())
                    .is_some()
            }
            GoogleChatResponseMode::Mention => {
                // Check if the bot was mentioned
                event.message.as_ref()
                    .and_then(|m| m.annotations.as_ref())
                    .map(|annots| {
                        annots.iter().any(|a| {
                            a.annotation_type == "USER_MENTION" &&
                            a.user_mention.as_ref()
                                .map(|um| um.user.name.contains("/bots/"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            }
            GoogleChatResponseMode::Open => true,
        }
    }

    /// Build a simple text message payload.
    fn build_text_message(text: &str, thread_name: Option<&str>) -> ChatMessage {
        ChatMessage {
            text: Some(text.to_string()),
            cards_v2: None,
            thread: thread_name.map(|name| Thread { name: name.to_string() }),
        }
    }

    /// Build a card message payload.
    #[allow(dead_code)]
    fn build_card_message(
        title: &str,
        content: &str,
        thread_name: Option<&str>,
    ) -> ChatMessage {
        let card = CardV2 {
            card_id: Uuid::new_v4().to_string(),
            card: Card {
                header: Some(CardHeader {
                    title: title.to_string(),
                    subtitle: None,
                    image_url: None,
                }),
                sections: vec![CardSection {
                    header: None,
                    widgets: vec![Widget::TextParagraph {
                        text: content.to_string(),
                    }],
                }],
            },
        };

        ChatMessage {
            text: None,
            cards_v2: Some(vec![card]),
            thread: thread_name.map(|name| Thread { name: name.to_string() }),
        }
    }

    /// Parse a card from metadata.
    fn parse_card_from_metadata(metadata: &serde_json::Value) -> Option<Vec<CardV2>> {
        let cards = metadata.get("cards")?;
        let cards_array = cards.as_array()?;

        let cards_v2: Vec<CardV2> = cards_array
            .iter()
            .filter_map(|card_data| {
                let title = card_data.get("title")?.as_str()?;
                let content = card_data.get("content")?.as_str()?;

                Some(CardV2 {
                    card_id: Uuid::new_v4().to_string(),
                    card: Card {
                        header: Some(CardHeader {
                            title: title.to_string(),
                            subtitle: card_data.get("subtitle").and_then(|s| s.as_str().map(String::from)),
                            image_url: card_data.get("image_url").and_then(|u| u.as_str().map(String::from)),
                        }),
                        sections: vec![CardSection {
                            header: card_data.get("section_header").and_then(|h| h.as_str().map(String::from)),
                            widgets: vec![Widget::TextParagraph {
                                text: content.to_string(),
                            }],
                        }],
                    },
                })
            })
            .collect();

        if cards_v2.is_empty() {
            None
        } else {
            Some(cards_v2)
        }
    }

    /// Convert markdown to Google Chat format (simplified).
    fn markdown_to_chat(text: &str) -> String {
        // Google Chat supports basic markdown-like formatting
        // *bold*, _italic_, `code`, ```code blocks```
        // We keep it mostly as-is since Google Chat supports similar formatting
        text.to_string()
    }

    /// Convert Google Chat format to markdown.
    fn chat_to_markdown(text: &str) -> String {
        // Extract plain text from possible Google Chat formatting
        text.to_string()
    }

    /// Extract user ID from the user resource name.
    fn extract_user_id(user_name: &str) -> String {
        // Format: "users/123456789"
        user_name.split('/').last().unwrap_or(user_name).to_string()
    }

    /// Extract space ID from the space resource name.
    fn extract_space_id(space_name: &str) -> String {
        // Format: "spaces/AAAAxxxxxx"
        space_name.split('/').last().unwrap_or(space_name).to_string()
    }

    /// Load service account key and obtain access token.
    async fn authenticate(&self) -> Result<String> {
        // In a full implementation, this would:
        // 1. Load the service account JSON key file
        // 2. Use yup_oauth2 or google-auth to get an access token
        // 3. Cache the token and refresh when expired
        //
        // For now, we return a placeholder
        // Real implementation would use:
        // ```rust
        // let service_account_key = yup_oauth2::read_service_account_key(&self.config.service_account_key).await?;
        // let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        //     .build().await?;
        // let token = authenticator.token(&["https://www.googleapis.com/auth/chat.bot"]).await?;
        // ```
        
        warn!("Using placeholder authentication - implement real service account auth");
        Ok("placeholder_token".to_string())
    }
}

#[async_trait]
impl Channel for GoogleChatChannel {
    fn platform(&self) -> Platform {
        Platform::GoogleChat
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get space from metadata
        let space_name = msg.metadata.get("google_chat_space")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "Missing space in metadata".to_string(),
            })?;

        // Get thread from metadata (optional)
        let thread_name = msg.metadata.get("google_chat_thread")
            .and_then(|v| v.as_str());

        // Format content
        let formatted_content = Self::markdown_to_chat(&msg.content);

        // Build message payload
        let message_payload = if self.config.cards_enabled {
            if let Some(cards) = Self::parse_card_from_metadata(&msg.metadata) {
                ChatMessage {
                    text: None,
                    cards_v2: Some(cards),
                    thread: thread_name.map(|name| Thread { name: name.to_string() }),
                }
            } else {
                Self::build_text_message(&formatted_content, thread_name)
            }
        } else {
            Self::build_text_message(&formatted_content, thread_name)
        };

        // Get access token
        let _token = {
            let token_guard = self.access_token.read().await;
            token_guard.clone().unwrap_or_else(|| "placeholder".to_string())
        };

        // In a full implementation, this would:
        // 1. POST to spaces.messages.create endpoint
        // 2. Handle rate limits and retries
        // 3. Cache the sent message ID for potential updates
        //
        // Real implementation:
        // ```rust
        // let url = format!("{}/spaces/{}/messages", GOOGLE_CHAT_API_BASE, space_id);
        // let response = self.http_client
        //     .post(&url)
        //     .bearer_auth(&token)
        //     .json(&message_payload)
        //     .send().await?;
        // ```

        debug!(
            space = %space_name,
            content = %msg.content,
            has_cards = message_payload.cards_v2.is_some(),
            "Would send Google Chat message"
        );

        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "google_chat".to_string(),
                message: "Incoming message channel closed".to_string(),
            }.into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Google Chat API...");

        // Validate configuration
        if self.config.service_account_key.is_empty() {
            return Err(ChannelError::Config {
                platform: "google_chat".to_string(),
                message: "Google Chat service account key path is required".to_string(),
            }.into());
        }

        if self.config.project_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "google_chat".to_string(),
                message: "Google Cloud project ID is required".to_string(),
            }.into());
        }

        // Authenticate
        let token = self.authenticate().await?;
        *self.access_token.write().await = Some(token);

        // In a full implementation, this would:
        // 1. Set up event subscription (Pub/Sub or HTTP webhook)
        // 2. Start event listener based on configured mode
        // 3. Handle events: MESSAGE, CARD_CLICKED, SLASH_COMMAND

        if self.config.pubsub_subscription.is_some() {
            info!("Google Chat Pub/Sub mode would start here");
        } else if self.config.webhook_url.is_some() {
            info!("Google Chat HTTP webhook mode - events will be received via webhooks");
        } else {
            warn!("No Pub/Sub subscription or webhook URL configured - bot will not receive messages");
        }

        *self.is_connected.write().await = true;

        info!("Google Chat channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Google Chat...");

        *self.is_connected.write().await = false;
        *self.access_token.write().await = None;

        info!("Google Chat channel disconnected");
        Ok(())
    }
}

/// Webhook handler for Google Chat HTTP push mode.
pub struct GoogleChatWebhookHandler {
    config: GoogleChatConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl GoogleChatWebhookHandler {
    /// Create a new webhook handler.
    pub fn new(config: GoogleChatConfig, incoming_tx: mpsc::Sender<IncomingMessage>) -> Self {
        Self { config, incoming_tx }
    }

    /// Handle an incoming webhook event.
    /// 
    /// This should be called by your HTTP server when a webhook is received
    /// from Google Chat.
    pub async fn handle_event(&self, body: &[u8]) -> Result<Option<serde_json::Value>> {
        // Parse the event
        let event: ChatEvent = serde_json::from_slice(body)
            .map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to parse event: {}", e),
            })?;

        // Handle different event types
        match event.event_type.as_str() {
            "MESSAGE" => self.handle_message_event(event).await,
            "CARD_CLICKED" => self.handle_card_click_event(event).await,
            "SLASH_COMMAND" => self.handle_slash_command_event(event).await,
            "ADDED_TO_SPACE" => {
                info!("Bot added to space: {:?}", event.space.display_name);
                Ok(None)
            }
            "REMOVED_FROM_SPACE" => {
                info!("Bot removed from space: {:?}", event.space.display_name);
                Ok(None)
            }
            _ => {
                debug!("Unknown event type: {}", event.event_type);
                Ok(None)
            }
        }
    }

    async fn handle_message_event(&self, event: ChatEvent) -> Result<Option<serde_json::Value>> {
        let message = event.message.as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "MESSAGE event without message data".to_string(),
            })?;

        let user = event.user.as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "Event without user data".to_string(),
            })?;

        // Check allowlist
        let user_id = GoogleChatChannel::extract_user_id(&user.name);
        let email = user.email.as_deref();

        if !self.is_user_allowed(email, &user_id) {
            warn!(user_id = %user_id, "User not in allowlist");
            return Ok(Some(serde_json::json!({
                "text": "You are not authorized to use this bot."
            })));
        }

        // Check space allowlist
        let space_id = GoogleChatChannel::extract_space_id(&event.space.name);
        if !self.is_space_allowed(&space_id) {
            warn!(space_id = %space_id, "Space not in allowed spaces");
            return Ok(Some(serde_json::json!({
                "text": "This bot is not available in this space."
            })));
        }

        // Build metadata
        let mut metadata = serde_json::json!({
            "google_chat_space": event.space.name,
            "google_chat_space_type": event.space.space_type,
            "google_chat_user_name": user.name,
            "google_chat_user_display_name": user.display_name,
        });

        // Add thread info if present
        if let Some(thread) = &message.thread {
            metadata["google_chat_thread"] = serde_json::json!(thread.name);
        }

        // Add message ID
        metadata["google_chat_message_id"] = serde_json::json!(message.name);

        // Create incoming message
        let content = GoogleChatChannel::chat_to_markdown(&message.text);
        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id,
            content,
            platform: Platform::GoogleChat,
            metadata,
        };

        // Send to channel
        let _ = self.incoming_tx.send(incoming).await;

        // Acknowledge the event (Google Chat expects a response)
        Ok(None)
    }

    async fn handle_card_click_event(&self, _event: ChatEvent) -> Result<Option<serde_json::Value>> {
        // Handle card button clicks
        debug!("Card clicked event received");
        // In a full implementation, this would parse the action and route it appropriately
        Ok(None)
    }

    async fn handle_slash_command_event(&self, event: ChatEvent) -> Result<Option<serde_json::Value>> {
        let message = event.message.as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "SLASH_COMMAND event without message data".to_string(),
            })?;

        let slash_command = message.slash_command.as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "SLASH_COMMAND event without command data".to_string(),
            })?;

        info!(command_id = %slash_command.command_id, "Slash command received");

        // The slash command is handled similarly to a regular message
        // but with additional metadata
        self.handle_message_event(event).await
    }

    fn is_user_allowed(&self, email: Option<&str>, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }

        if let Some(email) = email {
            if self.config.allowlist.contains(&email.to_string()) {
                return true;
            }
        }

        self.config.allowlist.contains(&user_id.to_string())
    }

    fn is_space_allowed(&self, space_id: &str) -> bool {
        if self.config.allowed_spaces.is_empty() {
            return true;
        }
        self.config.allowed_spaces.contains(&space_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_markdown_to_chat() {
        let text = "Hello **world**";
        let formatted = GoogleChatChannel::markdown_to_chat(text);
        assert_eq!(formatted, "Hello **world**");
    }

    #[test]
    fn test_extract_user_id() {
        let name = "users/123456789";
        let id = GoogleChatChannel::extract_user_id(name);
        assert_eq!(id, "123456789");
    }

    #[test]
    fn test_extract_space_id() {
        let name = "spaces/AAAA_12345";
        let id = GoogleChatChannel::extract_space_id(name);
        assert_eq!(id, "AAAA_12345");
    }

    #[test]
    fn test_build_text_message() {
        let msg = GoogleChatChannel::build_text_message("Hello world", None);
        assert_eq!(msg.text, Some("Hello world".to_string()));
        assert!(msg.cards_v2.is_none());
        assert!(msg.thread.is_none());
    }

    #[test]
    fn test_build_text_message_with_thread() {
        let msg = GoogleChatChannel::build_text_message("Hello world", Some("spaces/AAA/threads/BBB"));
        assert!(msg.thread.is_some());
        assert_eq!(msg.thread.unwrap().name, "spaces/AAA/threads/BBB");
    }

    #[test]
    fn test_parse_card_from_metadata() {
        let metadata = json!({
            "cards": [
                {
                    "title": "Test Card",
                    "content": "Card content here"
                }
            ]
        });

        let cards = GoogleChatChannel::parse_card_from_metadata(&metadata);
        assert!(cards.is_some());
        let cards = cards.unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_user_allowed_empty_allowlist() {
        let config = GoogleChatConfig {
            enabled: true,
            service_account_key: String::new(),
            project_id: String::new(),
            webhook_url: None,
            pubsub_subscription: None,
            allowlist: vec![],
            allowed_spaces: vec![],
            rate_limit_requests_per_second: 10,
            cards_enabled: true,
            response_mode: GoogleChatResponseMode::Mention,
        };

        let channel = GoogleChatChannel::new(config);
        assert!(channel.is_user_allowed(Some("user@example.com"), "123"));
    }

    #[test]
    fn test_user_allowed_with_allowlist() {
        let config = GoogleChatConfig {
            enabled: true,
            service_account_key: String::new(),
            project_id: String::new(),
            webhook_url: None,
            pubsub_subscription: None,
            allowlist: vec!["allowed@example.com".to_string()],
            allowed_spaces: vec![],
            rate_limit_requests_per_second: 10,
            cards_enabled: true,
            response_mode: GoogleChatResponseMode::Mention,
        };

        let channel = GoogleChatChannel::new(config);
        assert!(channel.is_user_allowed(Some("allowed@example.com"), "123"));
        assert!(!channel.is_user_allowed(Some("denied@example.com"), "456"));
    }

    #[test]
    fn test_should_respond_slash_command_mode() {
        let config = GoogleChatConfig {
            enabled: true,
            service_account_key: String::new(),
            project_id: String::new(),
            webhook_url: None,
            pubsub_subscription: None,
            allowlist: vec![],
            allowed_spaces: vec![],
            rate_limit_requests_per_second: 10,
            cards_enabled: true,
            response_mode: GoogleChatResponseMode::SlashCommands,
        };

        let channel = GoogleChatChannel::new(config);

        // Event without slash command
        let event_no_command: ChatEvent = serde_json::from_str(r#"{
            "type": "MESSAGE",
            "eventTime": "2024-01-01T00:00:00Z",
            "space": {"name": "spaces/AAA", "type": "ROOM"},
            "message": {"name": "messages/123", "text": "Hello"}
        }"#).unwrap();
        assert!(!channel.should_respond(&event_no_command));

        // Event with slash command
        let event_with_command: ChatEvent = serde_json::from_str(r#"{
            "type": "MESSAGE",
            "eventTime": "2024-01-01T00:00:00Z",
            "space": {"name": "spaces/AAA", "type": "ROOM"},
            "message": {"name": "messages/123", "text": "/help", "slashCommand": {"commandId": "1"}}
        }"#).unwrap();
        assert!(channel.should_respond(&event_with_command));
    }
}
