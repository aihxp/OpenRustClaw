//! Discord Bot integration for OpenRustClaw.
//!
//! Features:
//! - Text messages in channels
//! - Direct messages
//! - Slash commands
//! - Embeds
//! - File attachments
//! - Reactions
//! - Threads
//! - Gateway connection management
//! - Rate limiting

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use governor::{Quota, RateLimiter};
use reqwest::Client;
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info};
use uuid::Uuid;

use openrustclaw_core::config::DiscordConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Discord channel implementation.
pub struct DiscordChannel {
    config: DiscordConfig,
    client: Client,
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
    _message_cache: Arc<RwLock<HashMap<Uuid, String>>>, // Maps session_id to message_id
}

impl DiscordChannel {
    /// Create a new Discord channel with the given configuration.
    pub fn new(config: DiscordConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Discord allows ~5 requests per second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(5).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            client: Client::new(),
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            _message_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Parse embeds from metadata.
    fn parse_embed(metadata: &serde_json::Value) -> Option<serde_json::Value> {
        metadata.get("embed").cloned()
    }

    /// Convert platform-specific formatting to Discord markdown.
    fn format_for_discord(text: &str) -> String {
        // Discord uses standard markdown
        text.to_string()
    }

    fn api_base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://discord.com/api/v10".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn header_value(&self) -> String {
        format!("Bot {}", self.config.token)
    }

    /// Create a verified Discord Interactions handler.
    pub fn interactions_handler(&self) -> Result<DiscordInteractionsHandler> {
        DiscordInteractionsHandler::new(self.config.clone(), self.incoming_tx.clone())
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn platform(&self) -> Platform {
        Platform::Discord
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "discord".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get channel ID from metadata
        let channel_id = msg
            .metadata
            .get("discord_channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Missing channel_id in metadata".to_string(),
            })?
            .to_string();

        if !self.config.allowed_channels.is_empty()
            && !self.config.allowed_channels.contains(&channel_id)
        {
            return Err(ChannelError::PermissionDenied {
                platform: "discord".to_string(),
                message: format!("channel {} is not allowed", channel_id),
            }
            .into());
        }

        if let Some(guild_id) = msg.metadata.get("discord_guild_id").and_then(|v| v.as_str()) {
            if !self.config.allowed_guilds.is_empty()
                && !self.config.allowed_guilds.contains(&guild_id.to_string())
            {
                return Err(ChannelError::PermissionDenied {
                    platform: "discord".to_string(),
                    message: format!("guild {} is not allowed", guild_id),
                }
                .into());
            }
        }

        // Check if we should send as embed
        let formatted_content = Self::format_for_discord(&msg.content);
        let embed = Self::parse_embed(&msg.metadata);
        let mut payload = serde_json::json!({
            "content": formatted_content,
        });
        if let Some(embed) = embed {
            payload["embeds"] = serde_json::json!([embed]);
        }

        let interaction_token = msg
            .metadata
            .get("discord_interaction_token")
            .and_then(|v| v.as_str());
        let application_id = msg
            .metadata
            .get("discord_application_id")
            .and_then(|v| v.as_str());

        let (request_builder, expects_auth_body) =
            if let (Some(interaction_token), Some(application_id)) = (interaction_token, application_id)
            {
                let followup_url = format!(
                    "{}/webhooks/{}/{}",
                    self.api_base_url(),
                    application_id,
                    interaction_token
                );
                (self.client.post(followup_url), false)
            } else {
                let url = format!("{}/channels/{}/messages", self.api_base_url(), channel_id);
                (
                    self.client.post(url).header("Authorization", self.header_value()),
                    true,
                )
            };

        let response = request_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "discord".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = body.get("retry_after").and_then(|v| v.as_f64()).map(|v| v.ceil() as u64);
            return Err(ChannelError::RateLimited {
                platform: "discord".to_string(),
                retry_after_secs: retry_after,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED && expects_auth_body {
            return Err(ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() {
            return Err(ChannelError::SendFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Discord send failed")
                    .to_string(),
            }
            .into());
        }

        debug!("Discord message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "discord".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Discord Gateway...");

        // Validate token
        if self.config.token.is_empty() {
            return Err(ChannelError::Config {
                platform: "discord".to_string(),
                message: "Discord bot token is required".to_string(),
            }
            .into());
        }

        let url = format!("{}/users/@me", self.api_base_url());
        let response = self
            .client
            .get(url)
            .header("Authorization", self.header_value())
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "discord".to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() {
            return Err(ChannelError::Connection {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Discord authentication probe failed")
                    .to_string(),
            }
            .into());
        }

        *self.is_connected.write().await = true;
        info!("Discord channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Discord...");

        *self.is_connected.write().await = false;

        info!("Discord channel disconnected");
        Ok(())
    }
}

/// Handler for verified Discord Interactions HTTP requests.
pub struct DiscordInteractionsHandler {
    config: DiscordConfig,
    public_key: VerifyingKey,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl DiscordInteractionsHandler {
    pub fn new(config: DiscordConfig, incoming_tx: mpsc::Sender<IncomingMessage>) -> Result<Self> {
        let public_key_hex = config.interaction_public_key.as_deref().unwrap_or("").trim();
        if public_key_hex.is_empty() {
            return Err(ChannelError::Config {
                platform: "discord".to_string(),
                message: "interaction_public_key is required for Discord Interactions ingress"
                    .to_string(),
            }
            .into());
        }

        let public_key_bytes = hex::decode(public_key_hex).map_err(|e| ChannelError::Config {
            platform: "discord".to_string(),
            message: format!("invalid Discord interaction public key: {}", e),
        })?;
        let key_bytes: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| ChannelError::Config {
                platform: "discord".to_string(),
                message: "Discord interaction public key must be 32 bytes".to_string(),
            })?;
        let public_key =
            VerifyingKey::from_bytes(&key_bytes).map_err(|e| ChannelError::Config {
                platform: "discord".to_string(),
                message: format!("invalid Discord interaction public key: {}", e),
            })?;

        Ok(Self {
            config,
            public_key,
            incoming_tx,
        })
    }

    pub async fn handle_event(
        &self,
        body: &[u8],
        signature: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<serde_json::Value> {
        self.verify_signature(body, signature, timestamp)?;

        let raw: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: format!("Failed to parse Discord interaction: {}", e),
            })?;

        if raw.get("type").and_then(|value| value.as_u64()) == Some(1) {
            return Ok(serde_json::json!({ "type": 1 }));
        }

        let interaction: DiscordInteraction =
            serde_json::from_value(raw).map_err(|e| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: format!("Failed to parse Discord interaction: {}", e),
            })?;

        match interaction.kind {
            2 => {
                self.handle_application_command(interaction).await?;
                Ok(serde_json::json!({ "type": 5 }))
            }
            other => {
                debug!(interaction_type = other, "Ignoring unsupported Discord interaction");
                Ok(serde_json::json!({ "type": 5 }))
            }
        }
    }

    fn verify_signature(
        &self,
        body: &[u8],
        signature: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<()> {
        let signature = signature.ok_or_else(|| ChannelError::AuthFailed {
            platform: "discord".to_string(),
            message: "Missing X-Signature-Ed25519 header".to_string(),
        })?;
        let timestamp = timestamp.ok_or_else(|| ChannelError::AuthFailed {
            platform: "discord".to_string(),
            message: "Missing X-Signature-Timestamp header".to_string(),
        })?;
        let signature_bytes = hex::decode(signature).map_err(|e| ChannelError::AuthFailed {
            platform: "discord".to_string(),
            message: format!("Invalid Discord signature: {}", e),
        })?;
        let signature = Signature::from_slice(&signature_bytes).map_err(|e| {
            ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: format!("Invalid Discord signature: {}", e),
            }
        })?;

        let mut signed_payload = Vec::with_capacity(timestamp.len() + body.len());
        signed_payload.extend_from_slice(timestamp.as_bytes());
        signed_payload.extend_from_slice(body);

        self.public_key
            .verify(&signed_payload, &signature)
            .map_err(|_| ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: "Discord interaction signature verification failed".to_string(),
            })?;

        Ok(())
    }

    async fn handle_application_command(&self, interaction: DiscordInteraction) -> Result<()> {
        let channel_id = interaction
            .channel_id
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Discord interaction missing channel_id".to_string(),
            })?;
        if !self.config.allowed_channels.is_empty()
            && !self.config.allowed_channels.contains(&channel_id)
        {
            return Err(ChannelError::PermissionDenied {
                platform: "discord".to_string(),
                message: format!("channel {} is not allowed", channel_id),
            }
            .into());
        }

        if let Some(guild_id) = interaction.guild_id.as_deref()
            && !self.config.allowed_guilds.is_empty()
            && !self.config.allowed_guilds.contains(&guild_id.to_string())
        {
            return Err(ChannelError::PermissionDenied {
                platform: "discord".to_string(),
                message: format!("guild {} is not allowed", guild_id),
            }
            .into());
        }

        if interaction.guild_id.is_none() && !self.config.dm_enabled {
            return Err(ChannelError::PermissionDenied {
                platform: "discord".to_string(),
                message: "direct messages are disabled".to_string(),
            }
            .into());
        }

        let user = interaction
            .member
            .as_ref()
            .and_then(|member| member.user.as_ref())
            .or(interaction.user.as_ref())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Discord interaction missing user".to_string(),
            })?;
        let content = interaction
            .data
            .as_ref()
            .map(extract_interaction_content)
            .unwrap_or_default();
        if content.trim().is_empty() {
            return Err(ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Discord interaction missing text input".to_string(),
            }
            .into());
        }

        let mut metadata = serde_json::json!({
            "discord_channel_id": channel_id,
            "discord_application_id": interaction.application_id,
            "discord_interaction_id": interaction.id,
            "discord_interaction_token": interaction.token,
        });
        if let Some(guild_id) = interaction.guild_id {
            metadata["discord_guild_id"] = serde_json::json!(guild_id);
        }
        if let Some(command_name) = interaction.data.as_ref().map(|data| data.name.as_str()) {
            metadata["discord_command_name"] = serde_json::json!(command_name);
        }

        let incoming = IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id: user.id.clone(),
            content,
            platform: Platform::Discord,
            metadata,
        };

        self.incoming_tx
            .send(incoming)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "discord".to_string(),
                message: format!("Failed to enqueue Discord interaction: {}", e),
            })?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiscordInteraction {
    id: String,
    application_id: String,
    #[serde(rename = "type")]
    kind: u8,
    token: String,
    guild_id: Option<String>,
    channel_id: Option<String>,
    data: Option<DiscordInteractionData>,
    member: Option<DiscordInteractionMember>,
    user: Option<DiscordInteractionUser>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiscordInteractionData {
    name: String,
    #[serde(default)]
    options: Vec<DiscordInteractionOption>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiscordInteractionOption {
    name: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
    #[serde(default)]
    options: Vec<DiscordInteractionOption>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiscordInteractionMember {
    user: Option<DiscordInteractionUser>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiscordInteractionUser {
    id: String,
}

fn extract_interaction_content(data: &DiscordInteractionData) -> String {
    find_option_value(&data.options)
        .unwrap_or_else(|| data.name.clone())
        .trim()
        .to_string()
}

fn find_option_value(options: &[DiscordInteractionOption]) -> Option<String> {
    const PREFERRED_NAMES: &[&str] = &["prompt", "message", "text", "query", "input"];

    for preferred in PREFERRED_NAMES {
        if let Some(option) = options.iter().find(|option| option.name == *preferred) {
            if let Some(value) = option
                .value
                .as_ref()
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
            {
                return Some(value);
            }
            if let Some(nested) = find_option_value(&option.options) {
                return Some(nested);
            }
        }
    }

    for option in options {
        if let Some(value) = option
            .value
            .as_ref()
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
        {
            return Some(value);
        }
        if let Some(nested) = find_option_value(&option.options) {
            return Some(nested);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_format_for_discord() {
        let text = "Hello **world**";
        let formatted = DiscordChannel::format_for_discord(text);
        assert_eq!(formatted, "Hello **world**");
    }

    #[tokio::test]
    async fn test_connect_and_send_discord_message() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "123",
                "username": "test-bot"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/channels/channel-1/messages"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "message-1"
            })))
            .mount(&server)
            .await;

        let config = DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: None,
            api_base_url: Some(server.uri()),
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        };
        let mut channel = DiscordChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello discord".to_string(),
                metadata: serde_json::json!({"discord_channel_id": "channel-1"}),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_followup_message_for_interaction() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "123",
                "username": "test-bot"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/webhooks/app-1/interaction-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "message-1"
            })))
            .mount(&server)
            .await;

        let config = DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: None,
            api_base_url: Some(server.uri()),
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        };
        let mut channel = DiscordChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello discord".to_string(),
                metadata: serde_json::json!({
                    "discord_channel_id": "channel-1",
                    "discord_application_id": "app-1",
                    "discord_interaction_token": "interaction-token"
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_interactions_handler_acknowledges_ping_and_enqueues_command() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let config = DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: Some(hex::encode(verifying_key.to_bytes())),
            api_base_url: None,
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        };
        let channel = DiscordChannel::new(config);
        let handler = channel.interactions_handler().unwrap();

        let ping_body = br#"{"type":1}"#;
        let ping_timestamp = "1712440000";
        let ping_signature = hex::encode(
            signing_key
                .sign(&[ping_timestamp.as_bytes(), ping_body].concat())
                .to_bytes(),
        );
        let ping_response = handler
            .handle_event(ping_body, Some(&ping_signature), Some(ping_timestamp))
            .await
            .unwrap();
        assert_eq!(ping_response["type"], 1);

        let command_body = br#"{
            "id":"interaction-1",
            "application_id":"app-1",
            "type":2,
            "token":"interaction-token",
            "guild_id":"guild-1",
            "channel_id":"channel-1",
            "data":{
                "name":"ask",
                "options":[{"name":"prompt","value":"hello from discord"}]
            },
            "member":{"user":{"id":"user-1"}}
        }"#;
        let command_timestamp = "1712440001";
        let command_signature = hex::encode(
            signing_key
                .sign(&[command_timestamp.as_bytes(), command_body].concat())
                .to_bytes(),
        );
        let command_response = handler
            .handle_event(
                command_body,
                Some(&command_signature),
                Some(command_timestamp),
            )
            .await
            .unwrap();
        assert_eq!(command_response["type"], 5);

        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.user_id, "user-1");
        assert_eq!(incoming.content, "hello from discord");
        assert_eq!(incoming.metadata["discord_channel_id"], "channel-1");
        assert_eq!(incoming.metadata["discord_guild_id"], "guild-1");
        assert_eq!(incoming.metadata["discord_interaction_token"], "interaction-token");
    }
}
