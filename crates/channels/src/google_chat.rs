//! Google Chat integration for OpenRustClaw.
//!
//! Features:
//! - Messages in spaces (rooms and DMs)
//! - Card-based responses (Google Chat Cards)
//! - Threading support
//! - Slash commands
//! - Mention handling (@bot)
//! - Service account or token authentication
//! - HTTP webhook support
//!
//! # API Reference
//!
//! - <https://developers.google.com/chat/api/reference/rest>

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use governor::{Quota, RateLimiter};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{GoogleChatConfig, GoogleChatResponseMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Google Chat API base URL.
#[allow(dead_code)]
const GOOGLE_CHAT_API_BASE: &str = "https://chat.googleapis.com/v1";
const GOOGLE_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CHAT_SCOPE: &str = "https://www.googleapis.com/auth/chat.bot";

/// Google Chat channel implementation.
pub struct GoogleChatChannel {
    config: GoogleChatConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: RwLock<bool>,
    /// Access token for API calls (cached and refreshed as needed)
    access_token: RwLock<Option<String>>,
    /// HTTP client for API calls
    http_client: reqwest::Client,
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

#[derive(Debug, Clone, serde::Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_google_token_uri")]
    token_uri: String,
}

fn default_google_token_uri() -> String {
    GOOGLE_OAUTH_TOKEN_URL.to_string()
}

#[derive(Debug, serde::Serialize)]
struct ServiceAccountClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PubSubPushEnvelope {
    message: PubSubPushMessage,
    #[serde(default)]
    subscription: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PubSubPushMessage {
    data: String,
    #[serde(rename = "messageId")]
    #[allow(dead_code)]
    message_id: Option<String>,
    #[serde(default)]
    attributes: Option<std::collections::HashMap<String, String>>,
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
    #[serde(default)]
    attachments: Vec<serde_json::Value>,
}

/// Slash command in a message.
#[derive(Debug, Clone, serde::Deserialize)]
struct SlashCommand {
    #[serde(rename = "commandId")]
    command_id: String,
    #[serde(rename = "commandName")]
    #[serde(default)]
    command_name: Option<String>,
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
            access_token: RwLock::new(None),
            http_client,
        }
    }

    /// Create an HTTP event handler for Google Chat push events.
    pub fn event_handler(&self) -> GoogleChatWebhookHandler {
        GoogleChatWebhookHandler::new(self.config.clone(), self.incoming_tx.clone())
    }

    /// Check if the user is in the allowlist.
    #[allow(dead_code)]
    fn is_user_allowed(&self, email: Option<&str>, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }

        if let Some(email) = email
            && self.config.allowlist.contains(&email.to_string())
        {
            return true;
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
                event
                    .message
                    .as_ref()
                    .and_then(|m| m.slash_command.as_ref())
                    .is_some()
            }
            GoogleChatResponseMode::Mention => {
                // Check if the bot was mentioned
                event
                    .message
                    .as_ref()
                    .and_then(|m| m.annotations.as_ref())
                    .map(|annots| {
                        annots.iter().any(|a| {
                            a.annotation_type == "USER_MENTION"
                                && a.user_mention
                                    .as_ref()
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
            thread: thread_name.map(|name| Thread {
                name: name.to_string(),
            }),
        }
    }

    /// Build a card message payload.
    #[allow(dead_code)]
    fn build_card_message(title: &str, content: &str, thread_name: Option<&str>) -> ChatMessage {
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
            thread: thread_name.map(|name| Thread {
                name: name.to_string(),
            }),
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
                            subtitle: card_data
                                .get("subtitle")
                                .and_then(|s| s.as_str().map(String::from)),
                            image_url: card_data
                                .get("image_url")
                                .and_then(|u| u.as_str().map(String::from)),
                        }),
                        sections: vec![CardSection {
                            header: card_data
                                .get("section_header")
                                .and_then(|h| h.as_str().map(String::from)),
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

    fn parse_file_reference_cards(metadata: &serde_json::Value) -> Option<Vec<CardV2>> {
        let file_refs = metadata.get("file_references")?.as_array()?;
        let buttons: Vec<_> = file_refs
            .iter()
            .filter_map(|value| {
                let url = value
                    .get("url")
                    .or_else(|| value.get("download_url"))
                    .and_then(|field| field.as_str())
                    .or_else(|| value.as_str())?;
                let label = value
                    .get("title")
                    .or_else(|| value.get("name"))
                    .and_then(|field| field.as_str())
                    .unwrap_or("Open file");
                Some(Button {
                    text: label.to_string(),
                    on_click: OnClick::OpenLink {
                        open_link: OpenLink {
                            url: url.to_string(),
                        },
                    },
                })
            })
            .collect();

        if buttons.is_empty() {
            return None;
        }

        Some(vec![CardV2 {
            card_id: Uuid::new_v4().to_string(),
            card: Card {
                header: Some(CardHeader {
                    title: "Attachments".to_string(),
                    subtitle: None,
                    image_url: None,
                }),
                sections: vec![CardSection {
                    header: None,
                    widgets: vec![Widget::ButtonList { buttons }],
                }],
            },
        }])
    }

    fn attachment_file_references(attachments: &[serde_json::Value]) -> Vec<serde_json::Value> {
        attachments
            .iter()
            .filter_map(|attachment| {
                let url = attachment
                    .get("downloadUri")
                    .or_else(|| attachment.get("attachmentDataRef"))
                    .and_then(|value| value.as_str())?;
                Some(serde_json::json!({
                    "url": url,
                    "name": attachment
                        .get("name")
                        .or_else(|| attachment.get("contentName"))
                        .and_then(|value| value.as_str()),
                    "mime": attachment
                        .get("contentType")
                        .or_else(|| attachment.get("mimeType"))
                        .and_then(|value| value.as_str()),
                    "attachment_data_ref": attachment
                        .get("attachmentDataRef")
                        .and_then(|value| value.as_str()),
                }))
            })
            .collect()
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
        user_name
            .split('/')
            .next_back()
            .unwrap_or(user_name)
            .to_string()
    }

    /// Extract space ID from the space resource name.
    fn extract_space_id(space_name: &str) -> String {
        // Format: "spaces/AAAAxxxxxx"
        space_name
            .split('/')
            .next_back()
            .unwrap_or(space_name)
            .to_string()
    }

    /// Load service account key and obtain access token.
    async fn authenticate(&self) -> Result<String> {
        if let Some(token) = self.config.service_account_key.strip_prefix("token:") {
            let token = token.trim();
            if !token.is_empty() {
                return Ok(token.to_string());
            }
        }

        if let Some(var_name) = self.config.service_account_key.strip_prefix("env:") {
            let value = std::env::var(var_name.trim()).map_err(|_| ChannelError::AuthFailed {
                platform: "google_chat".to_string(),
                message: format!(
                    "Google Chat access token env var '{}' is not set",
                    var_name.trim()
                ),
            })?;
            if !value.trim().is_empty() {
                return Ok(value);
            }
        }

        let key_path = std::path::Path::new(&self.config.service_account_key);
        if key_path.exists() {
            let raw = tokio::fs::read_to_string(key_path).await.map_err(|e| {
                ChannelError::AuthFailed {
                    platform: "google_chat".to_string(),
                    message: format!("Failed to read Google Chat key file: {}", e),
                }
            })?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
                && let Some(token) = json.get("access_token").and_then(|value| value.as_str())
                && !token.trim().is_empty()
            {
                return Ok(token.to_string());
            }
            if let Ok(service_account) = serde_json::from_str::<ServiceAccountKey>(&raw) {
                return self.exchange_service_account_token(&service_account).await;
            }
        }

        warn!(
            "Google Chat auth requires `token:<value>`, `env:VAR`, or a JSON file containing `access_token`"
        );
        Err(ChannelError::AuthFailed {
            platform: "google_chat".to_string(),
            message: "Google Chat authentication is not configured with a usable access token"
                .to_string(),
        }
        .into())
    }

    async fn exchange_service_account_token(
        &self,
        service_account: &ServiceAccountKey,
    ) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let claims = ServiceAccountClaims {
            iss: service_account.client_email.clone(),
            scope: GOOGLE_CHAT_SCOPE.to_string(),
            aud: service_account.token_uri.clone(),
            iat: now,
            exp: now + 3600,
        };

        let jwt = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(service_account.private_key.as_bytes()).map_err(|e| {
                ChannelError::AuthFailed {
                    platform: "google_chat".to_string(),
                    message: format!("Invalid Google service account private key: {}", e),
                }
            })?,
        )
        .map_err(|e| ChannelError::AuthFailed {
            platform: "google_chat".to_string(),
            message: format!("Failed to sign Google service account JWT: {}", e),
        })?;

        let response = self
            .http_client
            .post(&service_account.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "google_chat".to_string(),
                message: format!("Google OAuth token exchange failed: {}", e),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if !status.is_success() {
            return Err(ChannelError::AuthFailed {
                platform: "google_chat".to_string(),
                message: body
                    .get("error_description")
                    .and_then(|value| value.as_str())
                    .or_else(|| body.get("error").and_then(|value| value.as_str()))
                    .unwrap_or("Google OAuth token exchange failed")
                    .to_string(),
            }
            .into());
        }

        body.get("access_token")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .ok_or_else(|| {
                ChannelError::AuthFailed {
                    platform: "google_chat".to_string(),
                    message: "Google OAuth response did not contain access_token".to_string(),
                }
                .into()
            })
    }

    fn decode_pubsub_event_body(
        raw_body: &[u8],
    ) -> Result<(serde_json::Value, Option<String>)> {
        let outer_value: serde_json::Value =
            serde_json::from_slice(raw_body).map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to parse raw event: {}", e),
            })?;

        let maybe_message = outer_value.get("message");
        let is_pubsub_envelope = maybe_message
            .and_then(|value| value.as_object())
            .map(|message| message.contains_key("data"))
            .unwrap_or(false);

        if !is_pubsub_envelope {
            return Ok((outer_value, None));
        }

        let envelope: PubSubPushEnvelope =
            serde_json::from_value(outer_value).map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to parse Google Chat Pub/Sub envelope: {}", e),
            })?;

        let decoded = BASE64
            .decode(envelope.message.data.as_bytes())
            .map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to decode Google Chat Pub/Sub payload: {}", e),
            })?;

        let inner_value: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to parse decoded Google Chat Pub/Sub payload: {}", e),
            })?;

        let subscription = envelope
            .subscription
            .or_else(|| {
                envelope
                    .message
                    .attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("subscription").cloned())
            });

        Ok((inner_value, subscription))
    }
}

#[async_trait]
impl Channel for GoogleChatChannel {
    fn platform(&self) -> Platform {
        Platform::GoogleChat
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "google_chat".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get space from metadata
        let space_name = msg
            .metadata
            .get("google_chat_space")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "Missing space in metadata".to_string(),
            })?;

        // Get thread from metadata (optional)
        let thread_name = msg
            .metadata
            .get("google_chat_thread")
            .and_then(|v| v.as_str());

        // Format content
        let formatted_content = Self::markdown_to_chat(&msg.content);

        // Build message payload
        let message_payload = if self.config.cards_enabled {
            if let Some(cards) = Self::parse_card_from_metadata(&msg.metadata)
                .or_else(|| Self::parse_file_reference_cards(&msg.metadata))
            {
                ChatMessage {
                    text: None,
                    cards_v2: Some(cards),
                    thread: thread_name.map(|name| Thread {
                        name: name.to_string(),
                    }),
                }
            } else {
                Self::build_text_message(&formatted_content, thread_name)
            }
        } else {
            Self::build_text_message(&formatted_content, thread_name)
        };

        // Get access token
        let token = {
            let token_guard = self.access_token.read().await;
            token_guard
                .clone()
                .ok_or_else(|| ChannelError::AuthFailed {
                    platform: "google_chat".to_string(),
                    message: "Missing access token for Google Chat API".to_string(),
                })?
        };

        let response = self
            .http_client
            .post(format!(
                "{}/messages",
                GOOGLE_CHAT_API_BASE.to_string() + "/" + space_name
            ))
            .bearer_auth(&token)
            .json(&message_payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "google_chat".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ChannelError::RateLimited {
                platform: "google_chat".to_string(),
                retry_after_secs: None,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "google_chat".to_string(),
                message: body
                    .get("error")
                    .and_then(|value| value.get("message"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() {
            return Err(ChannelError::SendFailed {
                platform: "google_chat".to_string(),
                message: body
                    .get("error")
                    .and_then(|value| value.get("message"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("Google Chat send failed")
                    .to_string(),
            }
            .into());
        }

        debug!(space = %space_name, content = %msg.content, "Google Chat message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "google_chat".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
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
            }
            .into());
        }

        if self.config.project_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "google_chat".to_string(),
                message: "Google Cloud project ID is required".to_string(),
            }
            .into());
        }

        // Authenticate
        let token = self.authenticate().await?;
        *self.access_token.write().await = Some(token);

        // In a full implementation, this would:
        // 1. Set up event subscription (Pub/Sub or HTTP webhook)
        // 2. Start event listener based on configured mode
        // 3. Handle events: MESSAGE, CARD_CLICKED, SLASH_COMMAND

        if self.config.pubsub_subscription.is_some() {
            info!(
                "Google Chat Pub/Sub push mode enabled - webhook ingress can decode Pub/Sub envelopes"
            );
            if self.config.webhook_url.is_none() {
                warn!(
                    "Google Chat Pub/Sub subscription is configured without webhook_url; ensure Pub/Sub push targets /webhooks/google-chat/events"
                );
            }
        } else if self.config.webhook_url.is_some() {
            info!("Google Chat HTTP webhook mode - events will be received via webhooks");
        } else {
            warn!(
                "No Pub/Sub subscription or webhook URL configured - bot will not receive messages"
            );
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
        Self {
            config,
            incoming_tx,
        }
    }

    fn should_respond(&self, event: &ChatEvent) -> bool {
        match self.config.response_mode {
            GoogleChatResponseMode::SlashCommands => event
                .message
                .as_ref()
                .and_then(|m| m.slash_command.as_ref())
                .is_some(),
            GoogleChatResponseMode::Mention => event
                .message
                .as_ref()
                .and_then(|m| m.annotations.as_ref())
                .map(|annots| {
                    annots.iter().any(|a| {
                        a.annotation_type == "USER_MENTION"
                            && a.user_mention
                                .as_ref()
                                .map(|um| um.user.name.contains("/bots/"))
                                .unwrap_or(false)
                    })
                })
                .unwrap_or(false),
            GoogleChatResponseMode::Open => true,
        }
    }

    /// Handle an incoming webhook event.
    ///
    /// This should be called by your HTTP server when a webhook is received
    /// from Google Chat.
    pub async fn handle_event(&self, body: &[u8]) -> Result<Option<serde_json::Value>> {
        let (raw_event, pubsub_subscription) = GoogleChatChannel::decode_pubsub_event_body(body)?;
        let event: ChatEvent =
            serde_json::from_value(raw_event.clone()).map_err(|e| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: format!("Failed to parse event: {}", e),
            })?;

        if let Some(subscription) = pubsub_subscription.as_ref() {
            debug!(
                subscription = %subscription,
                event_type = %event.event_type,
                "Decoded Google Chat Pub/Sub push envelope"
            );
        }

        // Handle different event types
        match event.event_type.as_str() {
            "MESSAGE" => self.handle_message_event(event).await,
            "CARD_CLICKED" => self.handle_card_click_event(event, raw_event).await,
            "SLASH_COMMAND" => self.handle_slash_command_event(event).await,
            "ADDED_TO_SPACE" => self.handle_space_event(event, raw_event, true).await,
            "REMOVED_FROM_SPACE" => self.handle_space_event(event, raw_event, false).await,
            _ => {
                debug!("Unknown event type: {}", event.event_type);
                Ok(None)
            }
        }
    }

    async fn handle_message_event(&self, event: ChatEvent) -> Result<Option<serde_json::Value>> {
        let message = event
            .message
            .as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "MESSAGE event without message data".to_string(),
            })?;

        let user = event
            .user
            .as_ref()
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

        if !self.should_respond(&event) {
            debug!(
                response_mode = ?self.config.response_mode,
                space_id = %space_id,
                "Ignoring Google Chat message because response mode did not match"
            );
            return Ok(None);
        }

        // Build metadata
        let mut metadata = serde_json::json!({
            "google_chat_space": event.space.name,
            "google_chat_space_type": event.space.space_type,
            "google_chat_user_name": user.name,
            "google_chat_user_display_name": user.display_name,
            "google_chat_is_group": event.space.space_type != "DM",
            "google_chat_event_time": event.event_time,
        });
        if let Some(email) = user.email.as_ref() {
            metadata["google_chat_user_email"] = serde_json::json!(email);
        }
        if let Some(display_name) = event.space.display_name.as_ref() {
            metadata["google_chat_space_display_name"] = serde_json::json!(display_name);
        }
        if let Some(argument_text) = message.argument_text.as_ref() {
            metadata["google_chat_argument_text"] = serde_json::json!(argument_text);
        }
        if let Some(slash_command) = message.slash_command.as_ref() {
            metadata["google_chat_slash_command_id"] = serde_json::json!(slash_command.command_id);
            if let Some(command_name) = slash_command.command_name.as_ref() {
                metadata["google_chat_slash_command_name"] = serde_json::json!(command_name);
            }
        }
        let mentioned_users: Vec<_> = message
            .annotations
            .as_ref()
            .map(|annots| {
                annots
                    .iter()
                    .filter_map(|annotation| {
                        annotation.user_mention.as_ref().map(|mention| {
                            serde_json::json!({
                                "user_name": mention.user.name,
                                "display_name": mention.user.display_name,
                                "mention_type": mention.mention_type,
                            })
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !mentioned_users.is_empty() {
            metadata["google_chat_mentions"] = serde_json::json!(mentioned_users);
            metadata["google_chat_mention_count"] = serde_json::json!(mentioned_users.len());
        }
        let bot_mentioned = message
            .annotations
            .as_ref()
            .map(|annots| {
                annots.iter().any(|annotation| {
                    annotation.annotation_type == "USER_MENTION"
                        && annotation
                            .user_mention
                            .as_ref()
                            .map(|mention| mention.user.name.contains("/bots/"))
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        metadata["google_chat_bot_mentioned"] = serde_json::json!(bot_mentioned);

        // Add thread info if present
        if let Some(thread) = &message.thread {
            metadata["google_chat_thread"] = serde_json::json!(thread.name);
        }

        // Add message ID
        metadata["google_chat_message_id"] = serde_json::json!(message.name);
        if !message.attachments.is_empty() {
            metadata["google_chat_attachments"] = serde_json::json!(message.attachments);
            metadata["google_chat_attachment_count"] = serde_json::json!(message.attachments.len());
            let file_refs = GoogleChatChannel::attachment_file_references(&message.attachments);
            if !file_refs.is_empty() {
                metadata["file_references"] = serde_json::json!(file_refs);
            }
        }

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

    async fn handle_card_click_event(
        &self,
        event: ChatEvent,
        raw_event: serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        let user = event
            .user
            .as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "CARD_CLICKED event without user data".to_string(),
            })?;
        let user_id = GoogleChatChannel::extract_user_id(&user.name);
        let email = user.email.as_deref();
        if !self.is_user_allowed(email, &user_id) {
            warn!(user_id = %user_id, "User not in allowlist");
            return Ok(None);
        }

        let space_id = GoogleChatChannel::extract_space_id(&event.space.name);
        if !self.is_space_allowed(&space_id) {
            warn!(space_id = %space_id, "Space not in allowed spaces");
            return Ok(None);
        }

        let invoked_function = raw_event
            .pointer("/common/invokedFunction")
            .and_then(|value| value.as_str());
        let action_name = raw_event
            .pointer("/action/actionMethodName")
            .and_then(|value| value.as_str())
            .or(invoked_function);
        let action_parameters = raw_event
            .pointer("/action/parameters")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        let mut metadata = serde_json::json!({
            "google_chat_space": event.space.name,
            "google_chat_space_type": event.space.space_type,
            "google_chat_user_name": user.name,
            "google_chat_user_display_name": user.display_name,
            "google_chat_is_group": event.space.space_type != "DM",
            "google_chat_interaction_type": "CARD_CLICKED",
            "google_chat_card_click": raw_event,
            "google_chat_event_time": event.event_time,
        });
        if let Some(email) = user.email.as_ref() {
            metadata["google_chat_user_email"] = serde_json::json!(email);
        }
        if let Some(display_name) = event.space.display_name.as_ref() {
            metadata["google_chat_space_display_name"] = serde_json::json!(display_name);
        }
        if let Some(name) = action_name {
            metadata["google_chat_action_name"] = serde_json::json!(name);
        }
        if let Some(name) = invoked_function {
            metadata["google_chat_invoked_function"] = serde_json::json!(name);
        }
        if !action_parameters.is_empty() {
            metadata["google_chat_action_parameters"] = serde_json::json!(action_parameters);
            metadata["google_chat_action_parameter_count"] =
                serde_json::json!(metadata["google_chat_action_parameters"].as_array().map(|items| items.len()).unwrap_or(0));
        }

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id,
            content: action_name
                .unwrap_or("[google_chat card clicked]")
                .to_string(),
            platform: Platform::GoogleChat,
            metadata,
        };
        let _ = self.incoming_tx.send(incoming).await;
        Ok(None)
    }

    async fn handle_space_event(
        &self,
        event: ChatEvent,
        raw_event: serde_json::Value,
        added: bool,
    ) -> Result<Option<serde_json::Value>> {
        let event_type = if added {
            "ADDED_TO_SPACE"
        } else {
            "REMOVED_FROM_SPACE"
        };
        info!(
            space = ?event.space.display_name,
            event_type = event_type,
            "Google Chat space lifecycle event received"
        );

        let user_id = event
            .user
            .as_ref()
            .map(|user| GoogleChatChannel::extract_user_id(&user.name))
            .unwrap_or_else(|| "system".to_string());

        let mut metadata = serde_json::json!({
            "google_chat_space": event.space.name,
            "google_chat_space_type": event.space.space_type,
            "google_chat_is_group": event.space.space_type != "DM",
            "google_chat_interaction_type": event_type,
            "google_chat_space_event": raw_event,
            "google_chat_event_time": event.event_time,
            "google_chat_space_event_added": added,
        });
        if let Some(display_name) = event.space.display_name.as_ref() {
            metadata["google_chat_space_display_name"] = serde_json::json!(display_name);
        }
        if let Some(user) = event.user.as_ref() {
            metadata["google_chat_user_name"] = serde_json::json!(user.name);
            metadata["google_chat_user_display_name"] = serde_json::json!(user.display_name);
            if let Some(email) = user.email.as_ref() {
                metadata["google_chat_user_email"] = serde_json::json!(email);
            }
        }

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id,
            content: if added {
                "[google_chat added to space]".to_string()
            } else {
                "[google_chat removed from space]".to_string()
            },
            platform: Platform::GoogleChat,
            metadata,
        };
        let _ = self.incoming_tx.send(incoming).await;
        Ok(None)
    }

    async fn handle_slash_command_event(
        &self,
        event: ChatEvent,
    ) -> Result<Option<serde_json::Value>> {
        let message = event
            .message
            .as_ref()
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "google_chat".to_string(),
                message: "SLASH_COMMAND event without message data".to_string(),
            })?;

        let slash_command =
            message
                .slash_command
                .as_ref()
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

        if let Some(email) = email
            && self.config.allowlist.contains(&email.to_string())
        {
            return true;
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
    use tokio::sync::mpsc;

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
        let msg =
            GoogleChatChannel::build_text_message("Hello world", Some("spaces/AAA/threads/BBB"));
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
    fn test_parse_file_reference_cards() {
        let metadata = json!({
            "file_references": [{
                "title": "Download report",
                "url": "https://files.example.com/report.pdf"
            }]
        });
        let cards = GoogleChatChannel::parse_file_reference_cards(&metadata).unwrap();
        assert_eq!(cards[0].card.sections[0].widgets.len(), 1);
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
        let event_no_command: ChatEvent = serde_json::from_str(
            r#"{
            "type": "MESSAGE",
            "eventTime": "2024-01-01T00:00:00Z",
            "space": {"name": "spaces/AAA", "type": "ROOM"},
            "message": {"name": "messages/123", "text": "Hello"}
        }"#,
        )
        .unwrap();
        assert!(!channel.should_respond(&event_no_command));

        // Event with slash command
        let event_with_command: ChatEvent = serde_json::from_str(
            r#"{
            "type": "MESSAGE",
            "eventTime": "2024-01-01T00:00:00Z",
            "space": {"name": "spaces/AAA", "type": "ROOM"},
            "message": {"name": "messages/123", "text": "/help", "slashCommand": {"commandId": "1"}}
        }"#,
        )
        .unwrap();
        assert!(channel.should_respond(&event_with_command));
    }

    #[tokio::test]
    async fn test_handle_card_clicked_event_routes_incoming_message() {
        let (tx, mut rx) = mpsc::channel(4);
        let handler = GoogleChatWebhookHandler::new(
            GoogleChatConfig {
                enabled: true,
                service_account_key: String::new(),
                project_id: String::new(),
                webhook_url: None,
                pubsub_subscription: None,
                allowlist: vec![],
                allowed_spaces: vec![],
                rate_limit_requests_per_second: 10,
                cards_enabled: true,
                response_mode: GoogleChatResponseMode::Open,
            },
            tx,
        );

        handler
            .handle_event(
                br#"{
                    "type": "CARD_CLICKED",
                    "eventTime": "2024-01-01T00:00:00Z",
                    "space": {"name": "spaces/AAA", "type": "ROOM", "displayName": "Ops"},
                    "user": {"name": "users/123", "displayName": "Alice", "email": "alice@example.com"},
                    "common": {"invokedFunction": "open_report"},
                    "action": {
                        "actionMethodName": "open_report",
                        "parameters": [
                            {"key": "report_id", "value": "123"},
                            {"key": "format", "value": "pdf"}
                        ]
                    }
                }"#,
            )
            .await
            .expect("card event");

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.content, "open_report");
        assert_eq!(
            incoming.metadata["google_chat_interaction_type"],
            serde_json::json!("CARD_CLICKED")
        );
        assert_eq!(
            incoming.metadata["google_chat_action_name"],
            serde_json::json!("open_report")
        );
        assert_eq!(
            incoming.metadata["google_chat_invoked_function"],
            serde_json::json!("open_report")
        );
        assert_eq!(
            incoming.metadata["google_chat_action_parameter_count"],
            serde_json::json!(2)
        );
        assert_eq!(
            incoming.metadata["google_chat_action_parameters"][0]["key"],
            serde_json::json!("report_id")
        );
        assert_eq!(
            incoming.metadata["google_chat_event_time"],
            serde_json::json!("2024-01-01T00:00:00Z")
        );
        assert_eq!(
            incoming.metadata["google_chat_user_email"],
            serde_json::json!("alice@example.com")
        );
    }

    #[tokio::test]
    async fn test_handle_added_to_space_routes_lifecycle_message() {
        let (tx, mut rx) = mpsc::channel(4);
        let handler = GoogleChatWebhookHandler::new(
            GoogleChatConfig {
                enabled: true,
                service_account_key: String::new(),
                project_id: String::new(),
                webhook_url: None,
                pubsub_subscription: None,
                allowlist: vec![],
                allowed_spaces: vec![],
                rate_limit_requests_per_second: 10,
                cards_enabled: true,
                response_mode: GoogleChatResponseMode::Open,
            },
            tx,
        );

        handler
            .handle_event(
                br#"{
                    "type": "ADDED_TO_SPACE",
                    "eventTime": "2024-01-01T00:00:00Z",
                    "space": {"name": "spaces/AAA", "type": "ROOM", "displayName": "Ops"},
                    "user": {"name": "users/123", "displayName": "Alice", "email": "alice@example.com"}
                }"#,
            )
            .await
            .expect("space event");

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.content, "[google_chat added to space]");
        assert_eq!(
            incoming.metadata["google_chat_interaction_type"],
            serde_json::json!("ADDED_TO_SPACE")
        );
        assert_eq!(
            incoming.metadata["google_chat_space_event_added"],
            serde_json::json!(true)
        );
        assert_eq!(
            incoming.metadata["google_chat_event_time"],
            serde_json::json!("2024-01-01T00:00:00Z")
        );
        assert_eq!(
            incoming.metadata["google_chat_user_email"],
            serde_json::json!("alice@example.com")
        );
    }

    #[tokio::test]
    async fn test_handle_pubsub_wrapped_message_event() {
        let (tx, mut rx) = mpsc::channel(4);
        let handler = GoogleChatWebhookHandler::new(
            GoogleChatConfig {
                enabled: true,
                service_account_key: String::new(),
                project_id: String::new(),
                webhook_url: Some("https://example.com/webhooks/google-chat/events".to_string()),
                pubsub_subscription: Some("projects/demo/subscriptions/google-chat".to_string()),
                allowlist: vec![],
                allowed_spaces: vec![],
                rate_limit_requests_per_second: 10,
                cards_enabled: true,
                response_mode: GoogleChatResponseMode::Open,
            },
            tx,
        );

        let inner = serde_json::json!({
            "type": "MESSAGE",
            "eventTime": "2024-01-01T00:00:00Z",
            "space": {"name": "spaces/AAA", "type": "ROOM", "displayName": "Ops"},
            "user": {"name": "users/123", "displayName": "Alice", "email": "alice@example.com"},
            "message": {
                "name": "spaces/AAA/messages/1",
                "text": "hello from pubsub"
            }
        });
        let outer = serde_json::json!({
            "subscription": "projects/demo/subscriptions/google-chat",
            "message": {
                "data": BASE64.encode(inner.to_string()),
                "messageId": "msg-1",
                "attributes": {
                    "subscription": "projects/demo/subscriptions/google-chat"
                }
            }
        });

        handler
            .handle_event(outer.to_string().as_bytes())
            .await
            .expect("pubsub event");

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.content, "hello from pubsub");
        assert_eq!(incoming.platform, Platform::GoogleChat);
        assert_eq!(incoming.metadata["google_chat_space"], "spaces/AAA");
        assert_eq!(
            incoming.metadata["google_chat_event_time"],
            serde_json::json!("2024-01-01T00:00:00Z")
        );
    }

    #[tokio::test]
    async fn test_handle_message_event_normalizes_attachment_file_references() {
        let (tx, mut rx) = mpsc::channel(4);
        let handler = GoogleChatWebhookHandler::new(
            GoogleChatConfig {
                enabled: true,
                service_account_key: String::new(),
                project_id: String::new(),
                webhook_url: Some("https://example.com/webhooks/google-chat/events".to_string()),
                pubsub_subscription: None,
                allowlist: vec![],
                allowed_spaces: vec![],
                rate_limit_requests_per_second: 10,
                cards_enabled: true,
                response_mode: GoogleChatResponseMode::Open,
            },
            tx,
        );

        handler
            .handle_event(
                br#"{
                    "type": "MESSAGE",
                    "eventTime": "2024-01-01T00:00:00Z",
                    "space": {"name": "spaces/AAA", "type": "ROOM", "displayName": "Ops"},
                    "user": {"name": "users/123", "displayName": "Alice", "email": "alice@example.com"},
                    "message": {
                        "name": "spaces/AAA/messages/1",
                        "text": "attachment event",
                        "attachments": [{
                            "downloadUri": "https://chat.google.com/download/attachment-1",
                            "name": "incident-report.pdf",
                            "contentType": "application/pdf",
                            "attachmentDataRef": "spaces/AAA/attachments/1"
                        }]
                    }
                }"#,
            )
            .await
            .expect("message event");

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.content, "attachment event");
        assert_eq!(incoming.metadata["google_chat_attachment_count"], 1);
        assert_eq!(
            incoming.metadata["file_references"][0]["url"],
            "https://chat.google.com/download/attachment-1"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["name"],
            "incident-report.pdf"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["mime"],
            "application/pdf"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["attachment_data_ref"],
            "spaces/AAA/attachments/1"
        );
    }

    #[tokio::test]
    async fn test_handle_message_event_preserves_mentions_and_slash_command_name() {
        let (tx, mut rx) = mpsc::channel(4);
        let handler = GoogleChatWebhookHandler::new(
            GoogleChatConfig {
                enabled: true,
                service_account_key: String::new(),
                project_id: String::new(),
                webhook_url: Some("https://example.com/webhooks/google-chat/events".to_string()),
                pubsub_subscription: None,
                allowlist: vec![],
                allowed_spaces: vec![],
                rate_limit_requests_per_second: 10,
                cards_enabled: true,
                response_mode: GoogleChatResponseMode::Open,
            },
            tx,
        );

        handler
            .handle_event(
                br#"{
                    "type": "MESSAGE",
                    "eventTime": "2024-01-01T00:00:00Z",
                    "space": {"name": "spaces/AAA", "type": "ROOM", "displayName": "Ops"},
                    "user": {"name": "users/123", "displayName": "Alice", "email": "alice@example.com"},
                    "message": {
                        "name": "spaces/AAA/messages/2",
                        "text": "/assign @OpenRustClaw ticket",
                        "argumentText": "ticket",
                        "slashCommand": {
                            "commandId": "1",
                            "commandName": "/assign"
                        },
                        "annotations": [{
                            "type": "USER_MENTION",
                            "userMention": {
                                "user": {
                                    "name": "users/bots/999",
                                    "displayName": "OpenRustClaw"
                                },
                                "type": "MENTION"
                            }
                        }]
                    }
                }"#,
            )
            .await
            .expect("message event");

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.metadata["google_chat_slash_command_id"], "1");
        assert_eq!(incoming.metadata["google_chat_slash_command_name"], "/assign");
        assert_eq!(incoming.metadata["google_chat_mention_count"], 1);
        assert_eq!(
            incoming.metadata["google_chat_mentions"][0]["display_name"],
            "OpenRustClaw"
        );
        assert_eq!(
            incoming.metadata["google_chat_mentions"][0]["mention_type"],
            "MENTION"
        );
    }
}
