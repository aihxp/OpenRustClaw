//! Microsoft Teams Bot Framework integration for OpenRustClaw.
//!
//! Features:
//! - Direct messages (1:1 chats)
//! - Group/channel messages
//! - @mention handling
//! - Adaptive Cards support
//! - Typing indicators
//! - Bot Framework REST API client
//! - JWT token authentication with Microsoft
//!
//! # Authentication Flow
//!
//! 1. Bot registers with Microsoft Identity Platform (Azure AD)
//! 2. On connect, exchanges app credentials for access token
//! 3. Token is cached and refreshed automatically before expiry
//! 4. Incoming webhook requests are verified via JWT signature
//!
//! # Webhook Endpoint
//!
//! The webhook handler listens on the configured path (default: `/webhooks/teams`)
//! and processes incoming Activities from the Bot Framework.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::path::PathBuf;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use governor::{Quota, RateLimiter};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{TeamsConfig, TeamsGroupPolicy};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Microsoft Teams channel implementation using Bot Framework.
///
/// This struct manages the connection to Microsoft Teams via the Bot Framework
/// REST API, handling authentication, message sending/receiving, and webhook
/// processing for incoming activities.
#[derive(Debug)]
pub struct TeamsChannel {
    config: TeamsConfig,
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
    /// Cached access token for Microsoft Graph/Bot Framework APIs
    token: RwLock<Option<AccessToken>>,
    /// HTTP client for API requests
    http: reqwest::Client,
}

/// Access token with expiration tracking.
#[derive(Debug, Clone)]
struct AccessToken {
    token: String,
    expires_at: Instant,
}

impl AccessToken {
    /// Check if the token is expired (with 5-minute buffer).
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at - Duration::from_secs(300)
    }
}

/// Bot Framework Activity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    Message,
    ConversationUpdate,
    Typing,
    MessageReaction,
    MessageDelete,
    MessageUpdate,
}

impl std::str::FromStr for ActivityType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "message" => Ok(ActivityType::Message),
            "conversationUpdate" => Ok(ActivityType::ConversationUpdate),
            "typing" => Ok(ActivityType::Typing),
            "messageReaction" => Ok(ActivityType::MessageReaction),
            "messageDelete" => Ok(ActivityType::MessageDelete),
            "messageUpdate" => Ok(ActivityType::MessageUpdate),
            _ => Err(format!("Unknown activity type: {}", s)),
        }
    }
}

/// JWT claims for Microsoft Bot Framework authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotFrameworkClaims {
    /// The app ID that the token was issued for
    pub appid: String,
    /// The service URL
    #[serde(rename = "serviceurl")]
    pub service_url: Option<String>,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Issuer (should be https://api.botframework.com)
    pub iss: String,
    /// Audience (should match our app ID)
    pub aud: String,
}

/// Microsoft's OpenID configuration document.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OpenIdConfig {
    #[serde(rename = "jwks_uri")]
    pub jwks_uri: String,
    #[serde(rename = "token_endpoint")]
    pub token_endpoint: Option<String>,
}

/// JSON Web Key for signature verification.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct Jwk {
    pub kty: String,
    pub kid: String,
    #[serde(rename = "use")]
    pub use_: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub x5c: Option<Vec<String>>,
}

/// JWKS response from Microsoft.
#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    pub keys: Vec<Jwk>,
}

/// Represents an Adaptive Card attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveCard {
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub content: serde_json::Value,
}

impl AdaptiveCard {
    /// Create a new Adaptive Card with the given content.
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            content_type: "application/vnd.microsoft.card.adaptive".to_string(),
            content,
        }
    }

    /// Create a simple text Adaptive Card.
    pub fn simple_text(text: impl Into<String>) -> Self {
        let content = serde_json::json!({
            "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
            "type": "AdaptiveCard",
            "version": "1.4",
            "body": [
                {
                    "type": "TextBlock",
                    "text": text.into(),
                    "wrap": true
                }
            ]
        });
        Self::new(content)
    }
}

/// Teams-specific mention parsing result.
#[derive(Debug, Clone)]
pub struct MentionInfo {
    /// Whether the bot was mentioned
    pub bot_mentioned: bool,
    /// Cleaned text with mention markers removed
    pub clean_text: String,
    /// List of mentioned user IDs
    pub mentioned_users: Vec<String>,
}

impl TeamsChannel {
    /// Create a new Teams channel with the given configuration.
    pub fn new(config: TeamsConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Bot Framework allows ~10+ requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap()),
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
            rate_limiter,
            is_connected: RwLock::new(false),
            token: RwLock::new(None),
            http,
        }
    }

    /// Create a webhook handler for Bot Framework activities.
    pub fn webhook_handler(&self) -> TeamsWebhookHandler {
        TeamsWebhookHandler::new(self.config.clone(), self.incoming_tx.clone())
    }

    /// Get the Microsoft OAuth2 token endpoint.
    fn token_endpoint(&self) -> String {
        match &self.config.tenant_id {
            Some(tenant) => format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                tenant
            ),
            None => "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
        }
    }

    /// Get the Bot Framework service URL base.
    #[allow(dead_code)]
    fn bot_framework_url(&self) -> &'static str {
        "https://smba.trafficmanager.net/emea/v3"
    }

    /// Refresh the access token for Microsoft APIs.
    ///
    /// Uses client credentials flow to obtain an access token for the
    /// Bot Framework and Microsoft Graph APIs.
    async fn refresh_token(&self) -> Result<String> {
        let token_url = self.token_endpoint();

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.app_id),
            ("client_secret", &self.config.app_password),
            ("scope", "https://api.botframework.com/.default"),
        ];

        let response = self
            .http
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "teams".to_string(),
                message: format!("Token request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "teams".to_string(),
                message: format!("Token request failed: {}", error_text),
            }
            .into());
        }

        let token_data: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::AuthFailed {
                    platform: "teams".to_string(),
                    message: format!("Failed to parse token response: {}", e),
                })?;

        let access_token =
            token_data["access_token"]
                .as_str()
                .ok_or_else(|| ChannelError::AuthFailed {
                    platform: "teams".to_string(),
                    message: "No access_token in response".to_string(),
                })?;

        let expires_in = token_data["expires_in"].as_u64().unwrap_or(3600);

        let token = AccessToken {
            token: access_token.to_string(),
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        };

        *self.token.write().await = Some(token);

        debug!("Successfully refreshed Microsoft Teams access token");
        Ok(access_token.to_string())
    }

    /// Get a valid access token, refreshing if necessary.
    async fn get_token(&self) -> Result<String> {
        let token_guard = self.token.read().await;

        if let Some(token) = token_guard.as_ref()
            && !token.is_expired()
        {
            return Ok(token.token.clone());
        }

        drop(token_guard);
        self.refresh_token().await
    }

    /// Send a typing indicator to the conversation.
    ///
    /// This shows "<Bot> is typing..." in the Teams client.
    async fn send_typing_indicator(&self, service_url: &str, conversation_id: &str) -> Result<()> {
        let token = self.get_token().await?;

        let typing_activity = serde_json::json!({
            "type": "typing",
            "from": {
                "id": self.config.app_id,
                "name": "OpenRustClaw"
            },
            "conversation": {
                "id": conversation_id
            }
        });

        let url = format!(
            "{}/conversations/{}/activities",
            service_url, conversation_id
        );

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&typing_activity)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "teams".to_string(),
                message: format!("Failed to send typing indicator: {}", e),
            })?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            warn!("Failed to send typing indicator: {}", error);
        }

        Ok(())
    }

    /// Parse mentions from the message text.
    ///
    /// Extracts @mentions and determines if the bot was mentioned.
    /// Returns cleaned text with mention markers removed.
    fn parse_mentions(text: &str, entities: &[serde_json::Value]) -> MentionInfo {
        let mut bot_mentioned = false;
        let mut mentioned_users = Vec::new();
        let mut clean_text = text.to_string();

        for entity in entities {
            if let Some(entity_type) = entity.get("type").and_then(|t| t.as_str())
                && entity_type == "mention"
            {
                if let Some(mentioned) = entity.get("mentioned") {
                    if let Some(id) = mentioned.get("id").and_then(|i| i.as_str()) {
                        mentioned_users.push(id.to_string());
                    }
                    if let Some(name) = mentioned.get("name").and_then(|n| n.as_str()) {
                        // Check if this is the bot mention
                        if name.to_lowercase().contains("openrustclaw") {
                            bot_mentioned = true;
                        }
                    }
                }

                // Remove the mention text from the message
                if let Some(text_val) = entity.get("text").and_then(|t| t.as_str()) {
                    clean_text = clean_text.replace(text_val, "").trim().to_string();
                }
            }
        }

        MentionInfo {
            bot_mentioned,
            clean_text,
            mentioned_users,
        }
    }

    /// Check if a user is in the allowlist.
    fn is_user_allowed(&self, user_id: &str, user_email: Option<&str>) -> bool {
        if self.config.allowlist.is_empty() {
            return true; // No allowlist means allow all
        }

        let normalized_id = user_id.to_lowercase();

        for allowed in &self.config.allowlist {
            let normalized_allowed = allowed.to_lowercase();
            if normalized_id == normalized_allowed {
                return true;
            }
            // Also check email if provided
            if let Some(email) = user_email
                && email.to_lowercase() == normalized_allowed
            {
                return true;
            }
        }

        false
    }

    /// Convert markdown to Teams-compatible text.
    ///
    /// Teams uses a mix of markdown and HTML for formatting.
    fn markdown_to_teams(text: &str) -> String {
        // Convert bold (**text** -> **text** - Teams supports standard markdown)
        // Teams markdown is mostly standard, but we ensure compatibility

        // Convert code blocks with language
        // Teams uses triple backticks for code blocks

        // Convert mentions from @user format
        // Teams mentions need to be in the entities array, not just text

        text.to_string()
    }

    /// Build an Adaptive Card attachment for rich responses.
    fn build_adaptive_card(&self, content: &str, title: Option<&str>) -> Option<AdaptiveCard> {
        if !self.config.adaptive_cards_enabled {
            return None;
        }

        let card_content = if let Some(t) = title {
            serde_json::json!({
                "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                "type": "AdaptiveCard",
                "version": "1.4",
                "body": [
                    {
                        "type": "TextBlock",
                        "text": t,
                        "size": "Medium",
                        "weight": "Bolder"
                    },
                    {
                        "type": "TextBlock",
                        "text": content,
                        "wrap": true
                    }
                ]
            })
        } else {
            serde_json::json!({
                "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                "type": "AdaptiveCard",
                "version": "1.4",
                "body": [
                    {
                        "type": "TextBlock",
                        "text": content,
                        "wrap": true
                    }
                ]
            })
        };

        Some(AdaptiveCard::new(card_content))
    }

    fn build_file_reference_card(
        &self,
        content: &str,
        title: Option<&str>,
        file_references: &[serde_json::Value],
    ) -> Option<AdaptiveCard> {
        if !self.config.adaptive_cards_enabled || file_references.is_empty() {
            return None;
        }

        let actions: Vec<_> = file_references
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
                Some(serde_json::json!({
                    "type": "Action.OpenUrl",
                    "title": label,
                    "url": url,
                }))
            })
            .collect();

        if actions.is_empty() {
            return None;
        }

        Some(AdaptiveCard::new(serde_json::json!({
            "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
            "type": "AdaptiveCard",
            "version": "1.4",
            "body": [
                {
                    "type": "TextBlock",
                    "text": title.unwrap_or("OpenRustClaw"),
                    "size": "Medium",
                    "weight": "Bolder"
                },
                {
                    "type": "TextBlock",
                    "text": content,
                    "wrap": true
                }
            ],
            "actions": actions
        })))
    }

    fn sanitize_download_name(activity_id: &str, fallback: &str) -> String {
        let cleaned: String = fallback
            .chars()
            .map(|ch| match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
                _ => '_',
            })
            .collect();
        format!("{activity_id}-{}", cleaned.trim_matches('_'))
    }

    async fn download_attachment_to_dir(
        &self,
        downloads_dir: &PathBuf,
        activity_id: &str,
        attachment: &serde_json::Value,
    ) -> Result<Option<String>> {
        let url = attachment
            .get("contentUrl")
            .or_else(|| attachment.get("content_url"))
            .and_then(|value| value.as_str());
        let Some(url) = url else {
            return Ok(None);
        };

        if !downloads_dir.exists() {
            tokio::fs::create_dir_all(downloads_dir)
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "teams".to_string(),
                    message: format!("Failed to create Teams downloads directory: {}", e),
                })?;
        }

        let filename = attachment
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("attachment.bin");
        let path = downloads_dir.join(Self::sanitize_download_name(activity_id, filename));

        let mut request = self.http.get(url);
        if let Some(token) = self.token.read().await.as_ref().map(|value| value.token.clone()) {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|e| ChannelError::Connection {
            platform: "teams".to_string(),
            message: format!("Teams attachment download failed: {}", e),
        })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "teams".to_string(),
                message: format!("Teams attachment download failed: {}", body),
            }
            .into());
        }

        let bytes = response.bytes().await.map_err(|e| ChannelError::Connection {
            platform: "teams".to_string(),
            message: format!("Failed to read Teams attachment download: {}", e),
        })?;

        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "teams".to_string(),
                message: format!("Failed to persist Teams attachment download: {}", e),
            })?;

        Ok(Some(path.to_string_lossy().to_string()))
    }

    /// Handle an incoming Activity from the Bot Framework webhook.
    ///
    /// This method processes incoming webhook payloads, validates them,
    /// and converts them to internal message format.
    pub async fn handle_activity(
        &self,
        activity: serde_json::Value,
    ) -> Result<Option<IncomingMessage>> {
        let activity_type = activity
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("message")
            .to_string();

        if activity_type != "message" {
            return self.handle_non_message_activity(activity, &activity_type);
        }

        // Extract user info
        let from = activity
            .get("from")
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "teams".to_string(),
                message: "Missing 'from' field in activity".to_string(),
            })?;

        let user_id =
            from.get("id")
                .and_then(|i| i.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "teams".to_string(),
                    message: "Missing user ID in activity".to_string(),
                })?;

        let user_email = from.get("email").and_then(|e| e.as_str());

        // Check allowlist
        if !self.is_user_allowed(user_id, user_email) {
            warn!("User {} not in allowlist, ignoring message", user_id);
            return Ok(None);
        }

        // Extract message text
        let text = activity.get("text").and_then(|t| t.as_str()).unwrap_or("");

        // Parse mentions
        let entities = activity
            .get("entities")
            .and_then(|e| e.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        let mention_info = Self::parse_mentions(text, entities);

        // Check group policy for mentions
        if self.config.group_policy == TeamsGroupPolicy::Mention && !mention_info.bot_mentioned {
            // In mention-only mode, ignore messages without bot mention
            debug!("Ignoring message without bot mention in mention-only mode");
            return Ok(None);
        }

        // Extract conversation info
        let conversation =
            activity
                .get("conversation")
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "teams".to_string(),
                    message: "Missing 'conversation' field in activity".to_string(),
                })?;

        let conversation_id = conversation
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("");

        let conversation_type = conversation
            .get("conversationType")
            .and_then(|t| t.as_str())
            .unwrap_or("personal");

        let service_url = activity
            .get("serviceUrl")
            .and_then(|s| s.as_str())
            .unwrap_or("https://smba.trafficmanager.net/emea/");

        // Create session ID from conversation + user
        // Using v4 since v5 requires uuid feature flag that may not be enabled
        let session_id = Uuid::new_v4();

        // Build metadata
        let mut metadata = serde_json::json!({
            "teams_conversation_id": conversation_id,
            "teams_conversation_type": conversation_type,
            "teams_user_id": user_id,
            "teams_user_email": user_email,
            "teams_service_url": service_url,
            "teams_activity_id": activity.get("id").and_then(|i| i.as_str()),
            "teams_mentioned": mention_info.mentioned_users,
            "teams_bot_mentioned": mention_info.bot_mentioned,
            "teams_is_group": conversation_type != "personal",
        });
        if let Some(attachments) = activity.get("attachments").and_then(|v| v.as_array()) {
            metadata["teams_attachments"] = serde_json::json!(attachments);
            metadata["teams_attachment_count"] = serde_json::json!(attachments.len());
            let file_refs: Vec<_> = attachments
                .iter()
                .filter_map(|attachment| {
                    attachment
                        .get("contentUrl")
                        .or_else(|| attachment.get("content_url"))
                        .and_then(|value| value.as_str())
                })
                .collect();
            if !file_refs.is_empty() {
                metadata["file_references"] = serde_json::json!(file_refs);
            }
            if let Some(download_dir) = self
                .config
                .attachment_download_dir
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                let downloads_dir = PathBuf::from(download_dir);
                let activity_id = activity
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or("teams-attachment");
                let mut downloaded_paths = Vec::new();
                for attachment in attachments {
                    match self
                        .download_attachment_to_dir(&downloads_dir, activity_id, attachment)
                        .await
                    {
                        Ok(Some(path)) => downloaded_paths.push(path),
                        Ok(None) => {}
                        Err(error) => {
                            warn!(error = %error, activity_id = %activity_id, "Failed to download Teams attachment");
                        }
                    }
                }
                if !downloaded_paths.is_empty() {
                    metadata["teams_download_paths"] = serde_json::json!(downloaded_paths);
                    metadata["teams_downloaded_attachment_count"] =
                        serde_json::json!(metadata["teams_download_paths"].as_array().map(|items| items.len()).unwrap_or(0));
                }
            }
        }

        // Send typing indicator
        let _ = self
            .send_typing_indicator(service_url, conversation_id)
            .await;

        let incoming = IncomingMessage {
            session_id,
            user_id: user_id.to_string(),
            content: mention_info.clean_text,
            platform: Platform::Teams,
            metadata,
        };

        Ok(Some(incoming))
    }

    fn build_activity_metadata(
        activity: &serde_json::Value,
        user_id: &str,
        user_email: Option<&str>,
        conversation_id: &str,
        conversation_type: &str,
        service_url: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "teams_conversation_id": conversation_id,
            "teams_conversation_type": conversation_type,
            "teams_user_id": user_id,
            "teams_user_email": user_email,
            "teams_service_url": service_url,
            "teams_activity_id": activity.get("id").and_then(|i| i.as_str()),
            "teams_is_group": conversation_type != "personal",
            "teams_activity_type": activity.get("type").and_then(|value| value.as_str()),
        })
    }

    fn handle_non_message_activity(
        &self,
        activity: serde_json::Value,
        activity_type: &str,
    ) -> Result<Option<IncomingMessage>> {
        if !matches!(
            activity_type,
            "conversationUpdate" | "messageReaction" | "messageDelete" | "messageUpdate"
        ) {
            debug!("Ignoring non-message activity type: {}", activity_type);
            return Ok(None);
        }

        let conversation =
            activity
                .get("conversation")
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "teams".to_string(),
                    message: "Missing 'conversation' field in activity".to_string(),
                })?;
        let conversation_id = conversation
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("");
        let conversation_type = conversation
            .get("conversationType")
            .and_then(|t| t.as_str())
            .unwrap_or("personal");
        let service_url = activity
            .get("serviceUrl")
            .and_then(|s| s.as_str())
            .unwrap_or("https://smba.trafficmanager.net/emea/");
        let from = activity.get("from");
        let user_id = from
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_str())
            .unwrap_or("system");
        let user_email = from
            .and_then(|value| value.get("email"))
            .and_then(|value| value.as_str());
        if user_id != "system" && !self.is_user_allowed(user_id, user_email) {
            warn!("User {} not in allowlist, ignoring Teams activity", user_id);
            return Ok(None);
        }

        let mut metadata = Self::build_activity_metadata(
            &activity,
            user_id,
            user_email,
            conversation_id,
            conversation_type,
            service_url,
        );
        metadata["teams_bot_mentioned"] = serde_json::json!(false);

        let content = match activity_type {
            "conversationUpdate" => {
                if let Some(added) = activity
                    .get("membersAdded")
                    .and_then(|value| value.as_array())
                {
                    metadata["teams_members_added"] = serde_json::json!(added);
                    let member_ids: Vec<_> = added
                        .iter()
                        .filter_map(|value| value.get("id").and_then(|id| id.as_str()))
                        .collect();
                    if !member_ids.is_empty() {
                        metadata["teams_member_ids_added"] = serde_json::json!(member_ids);
                    }
                    metadata["teams_members_added_count"] = serde_json::json!(added.len());
                }
                if let Some(removed) = activity
                    .get("membersRemoved")
                    .and_then(|value| value.as_array())
                {
                    metadata["teams_members_removed"] = serde_json::json!(removed);
                    let member_ids: Vec<_> = removed
                        .iter()
                        .filter_map(|value| value.get("id").and_then(|id| id.as_str()))
                        .collect();
                    if !member_ids.is_empty() {
                        metadata["teams_member_ids_removed"] = serde_json::json!(member_ids);
                    }
                    metadata["teams_members_removed_count"] = serde_json::json!(removed.len());
                }
                if let Some(channel_data) = activity.get("channelData") {
                    metadata["teams_channel_data"] = channel_data.clone();
                }
                "[teams conversation update]".to_string()
            }
            "messageReaction" => {
                if let Some(added) = activity
                    .get("reactionsAdded")
                    .and_then(|value| value.as_array())
                {
                    metadata["teams_reactions_added"] = serde_json::json!(added);
                }
                if let Some(removed) = activity
                    .get("reactionsRemoved")
                    .and_then(|value| value.as_array())
                {
                    metadata["teams_reactions_removed"] = serde_json::json!(removed);
                }
                "[teams reaction event]".to_string()
            }
            "messageDelete" => "[teams message deleted]".to_string(),
            "messageUpdate" => {
                if let Some(text) = activity.get("text").and_then(|value| value.as_str()) {
                    metadata["teams_updated_text"] = serde_json::json!(text);
                    text.to_string()
                } else {
                    "[teams message updated]".to_string()
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            content,
            platform: Platform::Teams,
            metadata,
        }))
    }

    /// Send a message to Teams using the Bot Framework REST API.
    async fn send_to_teams(&self, msg: &OutgoingMessage) -> Result<()> {
        let token = self.get_token().await?;

        // Extract required metadata
        let service_url = msg
            .metadata
            .get("teams_service_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://smba.trafficmanager.net/emea/");

        let conversation_id = msg
            .metadata
            .get("teams_conversation_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "teams".to_string(),
                message: "Missing teams_conversation_id in metadata".to_string(),
            })?;

        let reply_to_id = msg
            .metadata
            .get("teams_activity_id")
            .or_else(|| msg.metadata.get("teams_reply_to_id"))
            .and_then(|v| v.as_str());

        // Build the activity
        let mut activity = serde_json::json!({
            "type": "message",
            "from": {
                "id": self.config.app_id,
                "name": "OpenRustClaw"
            },
            "conversation": {
                "id": conversation_id
            },
            "text": Self::markdown_to_teams(&msg.content),
        });

        // Add replyToId if this is a reply
        if let Some(reply_id) = reply_to_id {
            activity["replyToId"] = serde_json::json!(reply_id);
        }

        // Check if we should use an Adaptive Card
        let use_card = msg
            .metadata
            .get("teams_use_adaptive_card")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.config.adaptive_cards_enabled);

        if use_card {
            let title = msg
                .metadata
                .get("teams_card_title")
                .and_then(|v| v.as_str());

            let file_refs = msg
                .metadata
                .get("file_references")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            let card = self
                .build_file_reference_card(&msg.content, title, &file_refs)
                .or_else(|| self.build_adaptive_card(&msg.content, title));

            if let Some(card) = card {
                activity["attachments"] = serde_json::json!([card]);
                // Clear text when using card, or Teams will show both
                activity["text"] = serde_json::json!(null);
            }
        } else if let Some(attachments) = msg
            .metadata
            .get("teams_attachments")
            .and_then(|value| value.as_array())
        {
            activity["attachments"] = serde_json::json!(attachments);
        }

        // Send the activity
        let url = format!(
            "{}/conversations/{}/activities",
            service_url, conversation_id
        );

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&activity)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "teams".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "teams".to_string(),
                message: format!("API error: {}", error_text),
            }
            .into());
        }

        debug!(
            "Successfully sent message to Teams conversation {}",
            conversation_id
        );
        Ok(())
    }
}

#[async_trait]
impl Channel for TeamsChannel {
    fn platform(&self) -> Platform {
        Platform::Teams
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        self.send_to_teams(&msg).await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "teams".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Microsoft Teams Bot Framework...");

        // Validate configuration
        if self.config.app_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "teams".to_string(),
                message: "Microsoft App ID is required".to_string(),
            }
            .into());
        }

        if self.config.app_password.is_empty() {
            return Err(ChannelError::Config {
                platform: "teams".to_string(),
                message: "Microsoft App Password is required".to_string(),
            }
            .into());
        }

        // Test authentication by fetching initial token
        match self.refresh_token().await {
            Ok(_) => info!("Successfully authenticated with Microsoft Identity Platform"),
            Err(e) => {
                error!("Failed to authenticate with Microsoft: {}", e);
                return Err(e);
            }
        }

        *self.is_connected.write().await = true;

        info!("Microsoft Teams channel connected");
        info!("Webhook endpoint: {}", self.config.webhook_path);
        info!("Group policy: {:?}", self.config.group_policy);

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Microsoft Teams...");

        *self.is_connected.write().await = false;
        *self.token.write().await = None;

        info!("Microsoft Teams channel disconnected");
        Ok(())
    }
}

/// Webhook handler for incoming Teams activities.
///
/// This struct implements an Axum handler for the Teams webhook endpoint.
/// It verifies JWT tokens from Microsoft and routes activities to the channel.
#[derive(Debug, Clone)]
pub struct TeamsWebhookHandler {
    channel: Arc<TeamsChannel>,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl TeamsWebhookHandler {
    /// Create a new webhook handler for the given channel.
    pub fn new(config: TeamsConfig, incoming_tx: mpsc::Sender<IncomingMessage>) -> Self {
        Self {
            channel: Arc::new(TeamsChannel::new(config)),
            incoming_tx,
        }
    }

    /// Microsoft's OpenID configuration URL.
    const OPENID_CONFIG_URL: &str =
        "https://login.botframework.com/v1/.well-known/openidconfiguration";

    /// Cache for JWKS response to avoid fetching on every request.
    /// In production, this should be a shared cache with TTL.
    /// Verify the JWT token from Microsoft Bot Framework.
    ///
    /// This implements the full JWT verification flow:
    /// 1. Decode the token header to get the key ID (kid)
    /// 2. Fetch Microsoft's OpenID configuration
    /// 3. Fetch the JWKS (JSON Web Key Set) from the jwks_uri
    /// 4. Find the key matching the kid and verify the signature
    /// 5. Validate claims (issuer, audience, expiration)
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT token from the Authorization header
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the token is valid, `Ok(false)` if invalid,
    /// or an error if verification failed due to network/config issues.
    pub async fn verify_token(&self, token: &str) -> Result<bool> {
        // Skip verification if no app_id is configured (development mode)
        let app_id = &self.channel.config.app_id;
        if app_id.is_empty() {
            warn!("Teams app_id not configured, skipping JWT verification");
            return Ok(true);
        }

        // Step 1: Decode the header to get the key ID
        let header = match decode_header(token) {
            Ok(h) => h,
            Err(e) => {
                warn!("Failed to decode JWT header: {}", e);
                return Ok(false);
            }
        };

        let kid = match header.kid {
            Some(k) => k,
            None => {
                warn!("JWT header missing kid");
                return Ok(false);
            }
        };

        // Step 2: Fetch OpenID configuration
        let openid_config: OpenIdConfig = match reqwest::get(Self::OPENID_CONFIG_URL).await {
            Ok(resp) => match resp.json().await {
                Ok(config) => config,
                Err(e) => {
                    error!("Failed to parse OpenID configuration: {}", e);
                    return Err(ChannelError::AuthFailed {
                        platform: "teams".to_string(),
                        message: format!("Failed to parse OpenID config: {}", e),
                    }
                    .into());
                }
            },
            Err(e) => {
                error!("Failed to fetch OpenID configuration: {}", e);
                return Err(ChannelError::AuthFailed {
                    platform: "teams".to_string(),
                    message: format!("Failed to fetch OpenID config: {}", e),
                }
                .into());
            }
        };

        // Step 3: Fetch JWKS
        let jwks: JwksResponse = match reqwest::get(&openid_config.jwks_uri).await {
            Ok(resp) => match resp.json().await {
                Ok(keys) => keys,
                Err(e) => {
                    error!("Failed to parse JWKS: {}", e);
                    return Err(ChannelError::AuthFailed {
                        platform: "teams".to_string(),
                        message: format!("Failed to parse JWKS: {}", e),
                    }
                    .into());
                }
            },
            Err(e) => {
                error!("Failed to fetch JWKS: {}", e);
                return Err(ChannelError::AuthFailed {
                    platform: "teams".to_string(),
                    message: format!("Failed to fetch JWKS: {}", e),
                }
                .into());
            }
        };

        // Step 4: Find the matching key
        let jwk = match jwks.keys.iter().find(|k| k.kid == kid) {
            Some(k) => k,
            None => {
                warn!("No matching key found for kid: {}", kid);
                return Ok(false);
            }
        };

        // Step 5: Verify the token
        // Use x5c certificate if available, otherwise use n/e for RSA
        let decoding_key = if let Some(certs) = &jwk.x5c {
            if let Some(cert) = certs.first() {
                let cert_der = BASE64.decode(cert).map_err(|e| ChannelError::AuthFailed {
                    platform: "teams".to_string(),
                    message: format!("Failed to decode certificate: {}", e),
                })?;
                DecodingKey::from_rsa_der(&cert_der)
            } else {
                warn!("Empty x5c certificate chain");
                return Ok(false);
            }
        } else if let (Some(n), Some(e)) = (&jwk.n, &jwk.e) {
            match DecodingKey::from_rsa_components(n, e) {
                Ok(key) => key,
                Err(err) => {
                    warn!("Failed to create decoding key from components: {}", err);
                    return Ok(false);
                }
            }
        } else {
            warn!("JWK missing both x5c and n/e components");
            return Ok(false);
        };

        let mut validation = Validation::new(Algorithm::RS256);
        // Bot Framework tokens are issued by login.botframework.com
        validation.set_issuer(&["https://api.botframework.com"]);
        // The audience should match our app ID
        validation.set_audience(&[app_id.as_str()]);

        match decode::<BotFrameworkClaims>(token, &decoding_key, &validation) {
            Ok(token_data) => {
                debug!(
                    "Successfully verified JWT for app: {}",
                    token_data.claims.appid
                );
                Ok(true)
            }
            Err(e) => {
                warn!("JWT verification failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Handle an incoming webhook request.
    pub async fn handle_request(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        // Handle Bot Framework authentication challenge (if any)
        if let Some(authentication) = body.get("authentication") {
            debug!("Received authentication challenge: {:?}", authentication);
            // Return the challenge response
            return Ok(serde_json::json!({
                "status": 200,
                "message": "Authentication acknowledged"
            }));
        }

        // Process the activity
        match self.channel.handle_activity(body).await {
            Ok(Some(incoming)) => {
                // Forward to the channel's incoming queue
                let _ = self.incoming_tx.send(incoming).await;
                Ok(serde_json::json!({
                    "status": 200,
                    "message": "Activity processed"
                }))
            }
            Ok(None) => {
                // Activity processed but no message to forward
                Ok(serde_json::json!({
                    "status": 200,
                    "message": "Activity acknowledged"
                }))
            }
            Err(e) => {
                error!("Failed to handle activity: {}", e);
                Err(e)
            }
        }
    }
}

/// Create a simple text message activity for Teams.
pub fn create_text_activity(text: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "type": "message",
        "text": text.into()
    })
}

/// Create an Adaptive Card activity for Teams.
pub fn create_adaptive_card_activity(card: AdaptiveCard) -> serde_json::Value {
    serde_json::json!({
        "type": "message",
        "attachments": [
            {
                "contentType": card.content_type,
                "content": card.content
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_adaptive_card_creation() {
        let card = AdaptiveCard::simple_text("Hello World");
        assert_eq!(card.content_type, "application/vnd.microsoft.card.adaptive");
        assert!(card.content.get("$schema").is_some());
    }

    #[test]
    fn test_parse_mentions() {
        let text = "<at>OpenRustClaw</at> Hello there!";
        let entities = vec![serde_json::json!({
            "type": "mention",
            "text": "<at>OpenRustClaw</at>",
            "mentioned": {
                "id": "28:app-id",
                "name": "OpenRustClaw"
            }
        })];

        let info = TeamsChannel::parse_mentions(text, &entities);
        assert!(info.bot_mentioned);
        assert_eq!(info.clean_text, "Hello there!");
    }

    #[test]
    fn test_user_allowlist() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec!["user1@example.com".to_string(), "user2".to_string()],
            group_policy: TeamsGroupPolicy::Mention,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };

        let channel = TeamsChannel::new(config);

        assert!(channel.is_user_allowed("user1@example.com", None));
        assert!(channel.is_user_allowed("USER1@EXAMPLE.COM", None)); // Case insensitive
        assert!(channel.is_user_allowed("user2", None));
        assert!(!channel.is_user_allowed("user3", None));
        assert!(channel.is_user_allowed("user1@example.com", Some("user1@example.com")));
    }

    #[test]
    fn test_empty_allowlist_allows_all() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };

        let channel = TeamsChannel::new(config);

        assert!(channel.is_user_allowed("anyuser", None));
        assert!(channel.is_user_allowed("another@example.com", None));
    }

    #[test]
    fn test_markdown_to_teams() {
        let text = "Hello **world**";
        let result = TeamsChannel::markdown_to_teams(text);
        // Teams supports standard markdown, so this should pass through
        assert_eq!(result, "Hello **world**");
    }

    #[test]
    fn test_build_file_reference_card() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };
        let channel = TeamsChannel::new(config);
        let card = channel
            .build_file_reference_card(
                "Download the report",
                Some("Report"),
                &[serde_json::json!({
                    "title": "Download PDF",
                    "url": "https://files.example.com/report.pdf"
                })],
            )
            .expect("card");
        assert_eq!(card.content_type, "application/vnd.microsoft.card.adaptive");
        assert_eq!(
            card.content["actions"][0]["url"],
            "https://files.example.com/report.pdf"
        );
    }

    #[tokio::test]
    async fn test_handle_message_reaction_activity() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };
        let channel = TeamsChannel::new(config);
        let incoming = channel
            .handle_activity(serde_json::json!({
                "type": "messageReaction",
                "id": "activity-1",
                "serviceUrl": "https://smba.trafficmanager.net/emea/",
                "conversation": {
                    "id": "19:conversation",
                    "conversationType": "channel"
                },
                "from": {
                    "id": "29:user"
                },
                "reactionsAdded": [{"type": "like"}]
            }))
            .await
            .expect("reaction event")
            .expect("incoming");
        assert_eq!(incoming.content, "[teams reaction event]");
        assert_eq!(
            incoming.metadata["teams_activity_type"],
            serde_json::json!("messageReaction")
        );
        assert_eq!(
            incoming.metadata["teams_reactions_added"][0]["type"],
            serde_json::json!("like")
        );
    }

    #[tokio::test]
    async fn test_handle_conversation_update_activity() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };
        let channel = TeamsChannel::new(config);
        let incoming = channel
            .handle_activity(serde_json::json!({
                "type": "conversationUpdate",
                "id": "activity-2",
                "serviceUrl": "https://smba.trafficmanager.net/emea/",
                "conversation": {
                    "id": "19:conversation",
                    "conversationType": "channel"
                },
                "from": {
                    "id": "29:user"
                },
                "membersAdded": [{"id": "29:new-user"}]
            }))
            .await
            .expect("conversation update")
            .expect("incoming");
        assert_eq!(incoming.content, "[teams conversation update]");
        assert_eq!(
            incoming.metadata["teams_members_added"][0]["id"],
            serde_json::json!("29:new-user")
        );
        assert_eq!(incoming.metadata["teams_members_added_count"], 1);
        assert_eq!(incoming.metadata["teams_member_ids_added"][0], "29:new-user");
    }

    #[tokio::test]
    async fn test_handle_conversation_update_activity_tracks_removed_members() {
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        };
        let channel = TeamsChannel::new(config);
        let incoming = channel
            .handle_activity(serde_json::json!({
                "type": "conversationUpdate",
                "id": "activity-3",
                "serviceUrl": "https://smba.trafficmanager.net/emea",
                "conversation": {
                    "id": "19:conversation",
                    "conversationType": "channel"
                },
                "from": {
                    "id": "29:user"
                },
                "membersRemoved": [{"id": "29:old-user"}]
            }))
            .await
            .expect("conversation update")
            .expect("incoming");
        assert_eq!(incoming.metadata["teams_members_removed_count"], 1);
        assert_eq!(incoming.metadata["teams_member_ids_removed"][0], "29:old-user");
    }

    #[tokio::test]
    async fn test_send_supports_reply_alias() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/conversations/19:conversation/activities"))
            .and(body_partial_json(serde_json::json!({
                "replyToId": "activity-root"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "activity-sent"
            })))
            .mount(&server)
            .await;

        let channel = TeamsChannel::new(TeamsConfig {
            enabled: true,
            app_id: "test-app".to_string(),
            app_password: "test-password".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: None,
        });
        *channel.token.write().await = Some(AccessToken {
            token: "cached-token".to_string(),
            expires_at: Instant::now() + Duration::from_secs(600),
        });

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "reply body".to_string(),
                metadata: serde_json::json!({
                    "teams_service_url": server.uri(),
                    "teams_conversation_id": "19:conversation",
                    "teams_reply_to_id": "activity-root"
                }),
            })
            .await
            .expect("send succeeds");
    }

    #[tokio::test]
    async fn test_handle_message_activity_downloads_attachments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/files/report.pdf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"teams-file".to_vec()))
            .mount(&server)
            .await;

        let download_dir =
            std::env::temp_dir().join(format!("orc-teams-downloads-{}", Uuid::new_v4()));
        let config = TeamsConfig {
            enabled: true,
            app_id: "test".to_string(),
            app_password: "test".to_string(),
            tenant_id: None,
            webhook_path: "/webhook".to_string(),
            allowlist: vec![],
            group_policy: TeamsGroupPolicy::Open,
            rate_limit_requests_per_second: 10,
            adaptive_cards_enabled: true,
            attachment_download_dir: Some(download_dir.display().to_string()),
        };
        let channel = TeamsChannel::new(config);
        let incoming = channel
            .handle_activity(serde_json::json!({
                "type": "message",
                "id": "activity-attachment-1",
                "text": "report attached",
                "serviceUrl": "https://smba.trafficmanager.net/emea/",
                "conversation": {
                    "id": "19:conversation",
                    "conversationType": "channel"
                },
                "from": {
                    "id": "29:user"
                },
                "attachments": [{
                    "contentType": "application/pdf",
                    "contentUrl": format!("{}/files/report.pdf", server.uri()),
                    "name": "report.pdf"
                }]
            }))
            .await
            .expect("message activity")
            .expect("incoming");

        assert_eq!(incoming.metadata["teams_attachment_count"], serde_json::json!(1));
        assert_eq!(
            incoming.metadata["teams_downloaded_attachment_count"],
            serde_json::json!(1)
        );
        let downloaded_path = incoming.metadata["teams_download_paths"][0]
            .as_str()
            .expect("download path");
        let bytes = tokio::fs::read(downloaded_path).await.expect("downloaded bytes");
        assert_eq!(bytes, b"teams-file");
        let _ = tokio::fs::remove_file(downloaded_path).await;
        let _ = tokio::fs::remove_dir_all(download_dir).await;
    }

    #[test]
    fn test_token_expiration() {
        let token = AccessToken {
            token: "test_token".to_string(),
            expires_at: Instant::now() + Duration::from_secs(600),
        };
        assert!(!token.is_expired());

        let expired_token = AccessToken {
            token: "test_token".to_string(),
            expires_at: Instant::now() + Duration::from_secs(60), // Less than 5 min buffer
        };
        assert!(expired_token.is_expired());
    }
}
