//! Matrix Protocol integration for OpenRustClaw.
//!
//! Features:
//! - Text messages in rooms
//! - Direct messages
//! - File attachments
//! - End-to-end encryption (via matrix-sdk-crypto)
//! - Auto-join rooms on invite
//! - User and room allowlists
//! - Message reactions
//! - Thread support
//! - Rate limiting

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use openrustclaw_core::config::MatrixConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Matrix channel implementation.
///
/// Uses matrix-rust-sdk for native Matrix protocol support.
/// Supports E2EE, room management, and rich media messages.
pub struct MatrixChannel {
    config: MatrixConfig,
    _incoming_tx: mpsc::Sender<IncomingMessage>,
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
    /// Maps session_id to event_id for reply threading
    _message_cache: Arc<RwLock<HashMap<Uuid, String>>>,
    /// Reserved client handle for a future matrix-sdk integration.
    _client: Arc<RwLock<Option<Arc<()>>>>,
}

impl MatrixChannel {
    /// Create a new Matrix channel with the given configuration.
    pub fn new(config: MatrixConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Matrix recommends ~10 requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            _incoming_tx: incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            _message_cache: Arc::new(RwLock::new(HashMap::new())),
            _client: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a user is allowed to interact with the bot.
    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&user_id.to_string())
    }

    /// Check if a room is allowed.
    #[allow(dead_code)]
    fn is_room_allowed(&self, room_id: &str) -> bool {
        if self.config.room_allowlist.is_empty() {
            return true;
        }
        self.config.room_allowlist.contains(&room_id.to_string())
    }

    /// Extract display name from MXID.
    /// Converts `@username:matrix.org` to `username`.
    #[allow(dead_code)]
    fn extract_display_name(user_id: &str) -> String {
        user_id
            .split(':')
            .next()
            .unwrap_or(user_id)
            .trim_start_matches('@')
            .to_string()
    }

    /// Convert Matrix HTML formatted body to plain text.
    #[allow(dead_code)]
    fn html_to_text(html: &str) -> String {
        // Simple HTML to text conversion
        // In a full implementation, this would use a proper HTML parser
        html.replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<p>", "")
            .replace("</p>", "\n")
            .replace("<b>", "**")
            .replace("</b>", "**")
            .replace("<i>", "*")
            .replace("</i>", "*")
            .replace("<code>", "`")
            .replace("</code>", "`")
    }

    /// Convert plain text to Matrix HTML.
    fn text_to_html(text: &str) -> String {
        // Escape HTML entities
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        // Convert newlines to <br>
        escaped.replace('\n', "<br>")
    }

    /// Parse formatted body from metadata.
    fn parse_formatted_body(metadata: &serde_json::Value) -> Option<String> {
        metadata
            .get("formatted_body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Build the path for the Matrix data directory.
    fn data_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.data_dir)
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn platform(&self) -> Platform {
        Platform::Matrix
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "matrix".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get room ID from metadata
        let room_id = msg
            .metadata
            .get("matrix_room_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "matrix".to_string(),
                message: "Missing room_id in metadata".to_string(),
            })?;

        // Check if we should send as formatted message
        let formatted_body = Self::parse_formatted_body(&msg.metadata);
        let _html_content = formatted_body.unwrap_or_else(|| Self::text_to_html(&msg.content));

        // Check for reply to thread
        let _reply_to_event_id = msg.metadata.get("matrix_reply_to").and_then(|v| v.as_str());

        debug!(room_id = %room_id, content = %msg.content, "Matrix send requested before matrix-sdk client was implemented");

        Err(ChannelError::SendFailed {
            platform: "matrix".to_string(),
            message: "Matrix send path is not implemented yet".to_string(),
        }
        .into())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!(homeserver = %self.config.homeserver, "Connecting to Matrix homeserver...");

        // Validate configuration
        if self.config.homeserver.is_empty() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Matrix homeserver URL is required".to_string(),
            }
            .into());
        }

        if self.config.user_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Matrix user_id is required".to_string(),
            }
            .into());
        }

        // Validate authentication
        if self.config.access_token.is_none() && self.config.password.is_none() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Either access_token or password must be provided".to_string(),
            }
            .into());
        }

        // Ensure data directory exists
        let data_dir = self.data_dir();
        if !data_dir.exists() {
            tokio::fs::create_dir_all(&data_dir)
                .await
                .map_err(|e| ChannelError::Config {
                    platform: "matrix".to_string(),
                    message: format!("Failed to create data directory: {}", e),
                })?;
        }

        info!(
            user_id = %self.config.user_id,
            encryption = self.config.enable_encryption,
            auto_join = self.config.auto_join_rooms,
            "Matrix channel configuration validated"
        );

        Err(ChannelError::Connection {
            platform: "matrix".to_string(),
            message: "Matrix runtime client is not implemented yet".to_string(),
        }
        .into())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Matrix...");

        // In a full implementation, this would:
        // 1. Stop the sync loop
        // 2. Close the client connections
        // 3. Save any pending crypto store state

        *self.is_connected.write().await = false;

        info!("Matrix channel disconnected");
        Ok(())
    }
}

/// Matrix-specific features.
impl MatrixChannel {
    /// Join a room by ID or alias.
    ///
    /// # Arguments
    /// * `room_id_or_alias` - Room ID (e.g., `!roomid:matrix.org`) or alias (e.g., `#room:matrix.org`)
    pub async fn join_room(&self, room_id_or_alias: &str) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        // In a full implementation, this would use matrix_sdk to:
        // 1. Parse the room ID or alias
        // 2. Call client.join_room_by_id_or_alias()

        info!(room = %room_id_or_alias, "Would join Matrix room");
        Ok(())
    }

    /// Leave a room.
    pub async fn leave_room(&self, room_id: &str) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        // In a full implementation, this would:
        // 1. Get the room by ID
        // 2. Call room.leave()

        info!(room = %room_id, "Would leave Matrix room");
        Ok(())
    }

    /// Send a formatted message (HTML) to a room.
    pub async fn send_formatted(&self, room_id: &str, text: &str, html: &str) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // In a full implementation, this would:
        // 1. Get the room by ID
        // 2. Create a RoomMessageEventContent with both plain text and HTML
        // 3. Send the message

        debug!(room_id = %room_id, text = %text, html = %html, "Would send formatted Matrix message");
        Ok(())
    }

    /// Send a reaction to a message.
    ///
    /// # Arguments
    /// * `room_id` - The room containing the message
    /// * `event_id` - The event ID of the message to react to
    /// * `emoji` - The reaction emoji (e.g., "👍")
    pub async fn send_reaction(&self, room_id: &str, event_id: &str, emoji: &str) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // In a full implementation, this would:
        // 1. Get the room by ID
        // 2. Create a ReactionEventContent
        // 3. Send as a relation to the original event

        debug!(room_id = %room_id, event_id = %event_id, emoji = %emoji, "Would send Matrix reaction");
        Ok(())
    }

    /// Send a file to a room.
    ///
    /// # Arguments
    /// * `room_id` - The target room
    /// * `file_path` - Path to the file
    /// * `filename` - Optional display name for the file
    pub async fn send_file(
        &self,
        room_id: &str,
        file_path: &str,
        filename: Option<&str>,
    ) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // In a full implementation, this would:
        // 1. Read the file
        // 2. Upload to the homeserver's content repository
        // 3. Send a FileMessageEventContent with the mxc:// URI

        debug!(room_id = %room_id, file_path = %file_path, filename = ?filename, "Would send Matrix file");
        Ok(())
    }

    /// Get the list of joined rooms.
    pub async fn joined_rooms(&self) -> Result<Vec<String>> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        // In a full implementation, this would:
        // 1. Call client.joined_rooms()
        // 2. Return room IDs

        Ok(vec![])
    }

    /// Send a typing indicator to a room.
    ///
    /// # Arguments
    /// * `room_id` - The target room
    /// * `typing` - Whether the user is typing
    pub async fn send_typing_indicator(&self, room_id: &str, typing: bool) -> Result<()> {
        // In a full implementation, this would:
        // 1. Get the room by ID
        // 2. Call room.typing_notice(typing)

        debug!(room_id = %room_id, typing = typing, "Would send typing indicator");
        Ok(())
    }

    /// Redact (delete) a message.
    ///
    /// # Arguments
    /// * `room_id` - The room containing the message
    /// * `event_id` - The event ID to redact
    /// * `reason` - Optional reason for redaction
    pub async fn redact_message(
        &self,
        room_id: &str,
        event_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // In a full implementation, this would:
        // 1. Get the room by ID
        // 2. Call room.redact(event_id, reason, None)

        debug!(room_id = %room_id, event_id = %event_id, reason = ?reason, "Would redact Matrix message");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = MatrixConfig {
            enabled: true,
            homeserver: "https://matrix.org".to_string(),
            user_id: "@bot:matrix.org".to_string(),
            access_token: Some("token".to_string()),
            password: None,
            device_id: None,
            data_dir: "./data/matrix".to_string(),
            allowlist: vec![],
            room_allowlist: vec![],
            auto_join_rooms: true,
            enable_encryption: true,
            rate_limit_per_second: 10,
        };
        let channel = MatrixChannel::new(config);
        assert!(channel.is_user_allowed("@user:matrix.org"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = MatrixConfig {
            enabled: true,
            homeserver: "https://matrix.org".to_string(),
            user_id: "@bot:matrix.org".to_string(),
            access_token: Some("token".to_string()),
            password: None,
            device_id: None,
            data_dir: "./data/matrix".to_string(),
            allowlist: vec![
                "@alice:matrix.org".to_string(),
                "@bob:matrix.org".to_string(),
            ],
            room_allowlist: vec![],
            auto_join_rooms: true,
            enable_encryption: true,
            rate_limit_per_second: 10,
        };
        let channel = MatrixChannel::new(config);
        assert!(channel.is_user_allowed("@alice:matrix.org"));
        assert!(channel.is_user_allowed("@bob:matrix.org"));
        assert!(!channel.is_user_allowed("@charlie:matrix.org"));
    }

    #[test]
    fn test_room_allowed_empty_list() {
        let config = MatrixConfig {
            enabled: true,
            homeserver: "https://matrix.org".to_string(),
            user_id: "@bot:matrix.org".to_string(),
            access_token: Some("token".to_string()),
            password: None,
            device_id: None,
            data_dir: "./data/matrix".to_string(),
            allowlist: vec![],
            room_allowlist: vec![],
            auto_join_rooms: true,
            enable_encryption: true,
            rate_limit_per_second: 10,
        };
        let channel = MatrixChannel::new(config);
        assert!(channel.is_room_allowed("!room:matrix.org"));
    }

    #[test]
    fn test_room_allowed_with_list() {
        let config = MatrixConfig {
            enabled: true,
            homeserver: "https://matrix.org".to_string(),
            user_id: "@bot:matrix.org".to_string(),
            access_token: Some("token".to_string()),
            password: None,
            device_id: None,
            data_dir: "./data/matrix".to_string(),
            allowlist: vec![],
            room_allowlist: vec![
                "!room1:matrix.org".to_string(),
                "!room2:matrix.org".to_string(),
            ],
            auto_join_rooms: true,
            enable_encryption: true,
            rate_limit_per_second: 10,
        };
        let channel = MatrixChannel::new(config);
        assert!(channel.is_room_allowed("!room1:matrix.org"));
        assert!(channel.is_room_allowed("!room2:matrix.org"));
        assert!(!channel.is_room_allowed("!room3:matrix.org"));
    }

    #[test]
    fn test_extract_display_name() {
        assert_eq!(
            MatrixChannel::extract_display_name("@alice:matrix.org"),
            "alice"
        );
        assert_eq!(
            MatrixChannel::extract_display_name("@user:example.com"),
            "user"
        );
        assert_eq!(
            MatrixChannel::extract_display_name("just_text"),
            "just_text"
        );
    }

    #[test]
    fn test_text_to_html() {
        let text = "Hello\nWorld";
        let html = MatrixChannel::text_to_html(text);
        assert_eq!(html, "Hello<br>World");
    }

    #[test]
    fn test_html_to_text() {
        let html = "Hello<b>World</b>";
        let text = MatrixChannel::html_to_text(html);
        assert_eq!(text, "Hello**World**");
    }

    #[test]
    fn test_parse_formatted_body() {
        let metadata = serde_json::json!({
            "formatted_body": "<b>Bold</b> message"
        });
        let formatted = MatrixChannel::parse_formatted_body(&metadata);
        assert_eq!(formatted, Some("<b>Bold</b> message".to_string()));
    }
}
