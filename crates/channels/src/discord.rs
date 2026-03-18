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
use std::time::Duration;
use tokio::time::Instant;

use async_trait::async_trait;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures::{SinkExt, StreamExt};
use governor::{Quota, RateLimiter};
use reqwest::Client;
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::DiscordConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

#[derive(Debug, Default)]
struct GatewaySessionState {
    session_id: Option<String>,
    last_sequence: Option<i64>,
}

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
    gateway_task: Mutex<Option<JoinHandle<()>>>,
    _message_cache: Arc<RwLock<HashMap<Uuid, String>>>, // Maps session_id to message_id
    gateway_session: Arc<RwLock<GatewaySessionState>>,
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
            gateway_task: Mutex::new(None),
            _message_cache: Arc::new(RwLock::new(HashMap::new())),
            gateway_session: Arc::new(RwLock::new(GatewaySessionState::default())),
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

    async fn fetch_gateway_url(&self) -> Result<String> {
        let url = format!("{}/gateway/bot", self.api_base_url());
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
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: body
                    .get("message")
                    .and_then(|value| value.as_str())
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
                    .and_then(|value| value.as_str())
                    .unwrap_or("Discord gateway discovery failed")
                    .to_string(),
            }
            .into());
        }

        let gateway_url = body
            .get("url")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "discord".to_string(),
                message: "Discord gateway response missing url".to_string(),
            })?;
        let separator = if gateway_url.contains('?') { "&" } else { "?" };
        Ok(format!("{gateway_url}{separator}v=10&encoding=json"))
    }

    async fn start_gateway_loop(&self, gateway_url: String) -> Result<()> {
        let config = self.config.clone();
        let incoming_tx = self.incoming_tx.clone();
        let gateway_session = self.gateway_session.clone();
        let initial_stream = connect_async(&gateway_url)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "discord".to_string(),
                message: format!("Failed to connect to Discord gateway websocket: {}", e),
            })?
            .0;
        let gateway_task = tokio::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            let max_backoff = Duration::from_secs(30);
            let mut next_stream = Some(initial_stream);

            loop {
                let stream = match next_stream.take() {
                    Some(stream) => stream,
                    None => match connect_async(&gateway_url).await {
                        Ok((stream, _)) => stream,
                        Err(error) => {
                            warn!(
                                error = %error,
                                retry_in = ?backoff,
                                "Failed to reconnect Discord gateway websocket"
                            );
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(max_backoff);
                            continue;
                        }
                    },
                };

                match run_gateway_loop(
                    stream,
                    config.clone(),
                    incoming_tx.clone(),
                    gateway_session.clone(),
                )
                .await
                {
                    Ok(()) => {
                        info!("Discord gateway loop exited cleanly");
                        break;
                    }
                    Err(error) => {
                        warn!(
                            error = %error,
                            retry_in = ?backoff,
                            "Discord gateway loop exited; reconnecting"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                    }
                }
            }
        });
        *self.gateway_task.lock().await = Some(gateway_task);
        Ok(())
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
            .get("discord_thread_id")
            .or_else(|| msg.metadata.get("discord_channel_id"))
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

        if let Some(guild_id) = msg
            .metadata
            .get("discord_guild_id")
            .and_then(|v| v.as_str())
        {
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

        let edit_message_id = msg
            .metadata
            .get("discord_edit_message_id")
            .and_then(|v| v.as_str());
        let reaction_emoji = msg
            .metadata
            .get("discord_reaction_emoji")
            .and_then(|v| v.as_str());
        let reaction_target = msg
            .metadata
            .get("discord_reaction_target_message_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                msg.metadata
                    .get("discord_reply_to_message_id")
                    .and_then(|v| v.as_str())
            });

        let (request_builder, expects_auth_body) =
            if let (Some(emoji), Some(target)) = (reaction_emoji, reaction_target) {
                let encoded = urlencoding::encode(emoji);
                let url = format!(
                    "{}/channels/{}/messages/{}/reactions/{}/@me",
                    self.api_base_url(),
                    channel_id,
                    target,
                    encoded
                );
                (
                    self.client
                        .put(url)
                        .header("Authorization", self.header_value()),
                    true,
                )
            } else if let (Some(interaction_token), Some(application_id)) =
                (interaction_token, application_id)
            {
                let followup_url = format!(
                    "{}/webhooks/{}/{}",
                    self.api_base_url(),
                    application_id,
                    interaction_token
                );
                (self.client.post(followup_url), false)
            } else if let Some(message_id) = edit_message_id {
                let url = format!(
                    "{}/channels/{}/messages/{}",
                    self.api_base_url(),
                    channel_id,
                    message_id
                );
                (
                    self.client
                        .patch(url)
                        .header("Authorization", self.header_value()),
                    true,
                )
            } else {
                let url = format!("{}/channels/{}/messages", self.api_base_url(), channel_id);
                (
                    self.client
                        .post(url)
                        .header("Authorization", self.header_value()),
                    true,
                )
            };

        if expects_auth_body
            && let Some(reference_id) = msg
                .metadata
                .get("discord_referenced_message_id")
                .or_else(|| msg.metadata.get("discord_reply_to_message_id"))
                .and_then(|v| v.as_str())
        {
            payload["message_reference"] = serde_json::json!({
                "message_id": reference_id,
            });
        }

        let response = if reaction_emoji.is_some() && reaction_target.is_some() {
            request_builder.send().await
        } else {
            request_builder.json(&payload).send().await
        }
        .map_err(|e| ChannelError::SendFailed {
            platform: "discord".to_string(),
            message: e.to_string(),
        })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = body
                .get("retry_after")
                .and_then(|v| v.as_f64())
                .map(|v| v.ceil() as u64);
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
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

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

        let gateway_url = self.fetch_gateway_url().await?;
        self.start_gateway_loop(gateway_url).await?;

        *self.is_connected.write().await = true;
        info!("Discord channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Discord...");

        *self.is_connected.write().await = false;
        if let Some(task) = self.gateway_task.lock().await.take() {
            task.abort();
        }
        *self.gateway_session.write().await = GatewaySessionState::default();

        info!("Discord channel disconnected");
        Ok(())
    }
}

async fn run_gateway_loop<S>(
    mut stream: tokio_tungstenite::WebSocketStream<S>,
    config: DiscordConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    gateway_session: Arc<RwLock<GatewaySessionState>>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut heartbeat = tokio::time::interval(Duration::from_secs(60));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut heartbeats_enabled = false;
    let mut last_sequence: Option<i64> = None;
    let mut heartbeat_ack_timeout = Duration::from_secs(120);
    let mut last_heartbeat_sent_at: Option<Instant> = None;
    let mut awaiting_heartbeat_ack = false;

    loop {
        tokio::select! {
            _ = heartbeat.tick(), if heartbeats_enabled => {
                if awaiting_heartbeat_ack
                    {
                    if last_heartbeat_sent_at
                        .map(|sent_at| sent_at.elapsed() >= heartbeat_ack_timeout)
                        .unwrap_or(false)
                    {
                        return Err(ChannelError::Connection {
                            platform: "discord".to_string(),
                            message: "Discord gateway heartbeat ACK timed out".to_string(),
                        }.into());
                    }
                    continue;
                }
                let payload = serde_json::json!({
                    "op": 1,
                    "d": last_sequence,
                });
                stream
                    .send(WsMessage::Text(payload.to_string().into()))
                    .await
                    .map_err(|e| ChannelError::Connection {
                        platform: "discord".to_string(),
                        message: format!("Failed to send Discord heartbeat: {}", e),
                    })?;
                awaiting_heartbeat_ack = true;
                last_heartbeat_sent_at = Some(Instant::now());
            }
            message = stream.next() => {
                let Some(message) = message else {
                    return Err(ChannelError::Connection {
                        platform: "discord".to_string(),
                        message: "Discord gateway closed the connection".to_string(),
                    }.into());
                };

                let message = message.map_err(|e| ChannelError::Connection {
                    platform: "discord".to_string(),
                    message: format!("Discord gateway read failed: {}", e),
                })?;

                match message {
                    WsMessage::Text(text) => {
                        let envelope: GatewayEnvelope = serde_json::from_str(&text).map_err(|e| ChannelError::InvalidFormat {
                            platform: "discord".to_string(),
                            message: format!("Failed to parse Discord gateway payload: {}", e),
                        })?;
                        if let Some(sequence) = envelope.s {
                            last_sequence = Some(sequence);
                        }

                        match envelope.op {
                            10 => {
                                let hello: GatewayHello = serde_json::from_value(envelope.d).map_err(|e| ChannelError::InvalidFormat {
                                    platform: "discord".to_string(),
                                    message: format!("Failed to parse Discord hello payload: {}", e),
                                })?;
                                heartbeat = tokio::time::interval(Duration::from_millis(hello.heartbeat_interval.max(1)));
                                heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                                heartbeat_ack_timeout = Duration::from_millis(hello.heartbeat_interval.max(1) * 2);
                                awaiting_heartbeat_ack = false;
                                last_heartbeat_sent_at = None;
                                heartbeats_enabled = true;
                                let resume_payload = {
                                    let session = gateway_session.read().await;
                                    match (&session.session_id, session.last_sequence) {
                                        (Some(session_id), Some(sequence)) => Some(serde_json::json!({
                                            "op": 6,
                                            "d": {
                                                "token": config.token,
                                                "session_id": session_id,
                                                "seq": sequence,
                                            }
                                        })),
                                        _ => None,
                                    }
                                };
                                let identify = resume_payload.unwrap_or_else(|| {
                                    serde_json::json!({
                                        "op": 2,
                                        "d": {
                                            "token": config.token,
                                            "intents": 37377,
                                            "properties": {
                                                "os": std::env::consts::OS,
                                                "browser": "openrustclaw",
                                                "device": "openrustclaw",
                                            },
                                        },
                                    })
                                });
                                stream
                                    .send(WsMessage::Text(identify.to_string().into()))
                                    .await
                                    .map_err(|e| ChannelError::Connection {
                                        platform: "discord".to_string(),
                                        message: format!("Failed to send Discord gateway handshake payload: {}", e),
                                    })?;
                            }
                            0 => {
                                if envelope.t.as_deref() == Some("READY") {
                                    let ready: GatewayReady = serde_json::from_value(envelope.d.clone()).map_err(|e| ChannelError::InvalidFormat {
                                        platform: "discord".to_string(),
                                        message: format!("Failed to parse Discord ready payload: {}", e),
                                    })?;
                                    gateway_session.write().await.session_id = Some(ready.session_id);
                                }
                                if envelope.t.as_deref() == Some("MESSAGE_CREATE") {
                                    let event: GatewayMessageCreate = serde_json::from_value(envelope.d).map_err(|e| ChannelError::InvalidFormat {
                                        platform: "discord".to_string(),
                                        message: format!("Failed to parse Discord message event: {}", e),
                                    })?;
                                    if let Some(message) = normalize_gateway_message(&config, event)? {
                                        incoming_tx
                                            .send(message)
                                            .await
                                            .map_err(|e| ChannelError::Connection {
                                                platform: "discord".to_string(),
                                                message: format!("Failed to enqueue Discord gateway message: {}", e),
                                            })?;
                                    }
                                }
                                if let Some(sequence) = last_sequence {
                                    gateway_session.write().await.last_sequence = Some(sequence);
                                }
                            }
                            7 => {
                                return Err(ChannelError::Connection {
                                    platform: "discord".to_string(),
                                    message: "Discord gateway requested reconnect".to_string(),
                                }
                                .into());
                            }
                            11 => {
                                awaiting_heartbeat_ack = false;
                                debug!("Received Discord heartbeat ACK");
                            }
                            9 => {
                                warn!("Discord gateway rejected session; clearing resume state");
                                *gateway_session.write().await = GatewaySessionState::default();
                                return Err(ChannelError::Connection {
                                    platform: "discord".to_string(),
                                    message: "Discord gateway invalidated the session".to_string(),
                                }
                                .into());
                            }
                            _ => {}
                        }
                    }
                    WsMessage::Ping(payload) => {
                        stream
                            .send(WsMessage::Pong(payload))
                            .await
                            .map_err(|e| ChannelError::Connection {
                                platform: "discord".to_string(),
                                message: format!("Failed to respond to Discord gateway ping: {}", e),
                            })?;
                    }
                    WsMessage::Close(_) => {
                        return Err(ChannelError::Connection {
                            platform: "discord".to_string(),
                            message: "Discord gateway closed the connection".to_string(),
                        }
                        .into());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn normalize_gateway_message(
    config: &DiscordConfig,
    event: GatewayMessageCreate,
) -> Result<Option<IncomingMessage>> {
    if event.author.bot.unwrap_or(false) {
        return Ok(None);
    }
    if event.author.system.unwrap_or(false) {
        return Ok(None);
    }
    if event.webhook_id.is_some() {
        return Ok(None);
    }
    if event.author.id == config.application_id {
        return Ok(None);
    }
    if !config.allowed_channels.is_empty() && !config.allowed_channels.contains(&event.channel_id) {
        return Ok(None);
    }
    if let Some(guild_id) = event.guild_id.as_ref()
        && !config.allowed_guilds.is_empty()
        && !config.allowed_guilds.contains(guild_id)
    {
        return Ok(None);
    }
    if event.guild_id.is_none() && !config.dm_enabled {
        return Ok(None);
    }
    if event.content.trim().is_empty() && event.attachments.is_empty() && event.embeds.is_empty() {
        return Ok(None);
    }

    let mut metadata = serde_json::json!({
        "discord_channel_id": event.channel_id,
        "discord_message_id": event.id,
        "discord_author_username": event.author.username,
        "discord_gateway": true,
        "discord_application_id": config.application_id,
        "discord_is_dm": event.guild_id.is_none(),
        "discord_bot_mentioned": event.content.contains(&format!("<@{}>", config.application_id))
            || event.content.contains(&format!("<@!{}>", config.application_id)),
        "discord_attachment_count": event.attachments.len(),
        "discord_embed_count": event.embeds.len(),
    });
    if let Some(guild_id) = event.guild_id {
        metadata["discord_guild_id"] = serde_json::json!(guild_id);
    }
    if let Some(thread_id) = event.thread_id {
        metadata["discord_thread_id"] = serde_json::json!(thread_id);
    }
    if let Some(reference) = event.message_reference
        && let Some(reference_id) = reference.message_id
    {
        metadata["discord_referenced_message_id"] = serde_json::json!(reference_id);
    }
    if let Some(timestamp) = event.timestamp {
        metadata["discord_timestamp"] = serde_json::json!(timestamp);
    }

    Ok(Some(IncomingMessage {
        session_id: Uuid::new_v4(),
        user_id: event.author.id,
        content: event.content,
        platform: Platform::Discord,
        metadata,
    }))
}

/// Handler for verified Discord Interactions HTTP requests.
pub struct DiscordInteractionsHandler {
    config: DiscordConfig,
    public_key: VerifyingKey,
    incoming_tx: mpsc::Sender<IncomingMessage>,
}

impl DiscordInteractionsHandler {
    pub fn new(config: DiscordConfig, incoming_tx: mpsc::Sender<IncomingMessage>) -> Result<Self> {
        let public_key_hex = config
            .interaction_public_key
            .as_deref()
            .unwrap_or("")
            .trim();
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
        let key_bytes: [u8; 32] =
            public_key_bytes
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
                debug!(
                    interaction_type = other,
                    "Ignoring unsupported Discord interaction"
                );
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
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|e| ChannelError::AuthFailed {
                platform: "discord".to_string(),
                message: format!("Invalid Discord signature: {}", e),
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
            "discord_is_dm": interaction.guild_id.is_none(),
        });
        if let Some(guild_id) = interaction.guild_id {
            metadata["discord_guild_id"] = serde_json::json!(guild_id);
        }
        if let Some(command_name) = interaction.data.as_ref().map(|data| data.name.as_str()) {
            metadata["discord_command_name"] = serde_json::json!(command_name);
        }
        metadata["discord_author_id"] = serde_json::json!(user.id.clone());

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

#[derive(Debug, serde::Deserialize)]
struct GatewayEnvelope {
    op: u8,
    #[serde(default)]
    d: serde_json::Value,
    #[serde(default)]
    s: Option<i64>,
    #[serde(default)]
    t: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GatewayHello {
    heartbeat_interval: u64,
}

#[derive(Debug, serde::Deserialize)]
struct GatewayReady {
    session_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct GatewayMessageCreate {
    id: String,
    channel_id: String,
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    guild_id: Option<String>,
    content: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    webhook_id: Option<String>,
    #[serde(default)]
    message_reference: Option<GatewayMessageReference>,
    #[serde(default)]
    attachments: Vec<serde_json::Value>,
    #[serde(default)]
    embeds: Vec<serde_json::Value>,
    author: GatewayAuthor,
}

#[derive(Debug, serde::Deserialize)]
struct GatewayAuthor {
    id: String,
    username: String,
    #[serde(default)]
    bot: Option<bool>,
    #[serde(default)]
    system: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct GatewayMessageReference {
    #[serde(default)]
    message_id: Option<String>,
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
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    async fn spawn_mock_gateway(dispatch_event: Option<serde_json::Value>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(tcp_stream).await.unwrap();
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

            let identify = ws_stream.next().await.unwrap().unwrap();
            let identify_text = identify.into_text().unwrap();
            let identify_json: serde_json::Value = serde_json::from_str(&identify_text).unwrap();
            assert_eq!(identify_json["op"], 2);

            if let Some(event) = dispatch_event {
                ws_stream
                    .send(WsMessage::Text(event.to_string().into()))
                    .await
                    .unwrap();
            }

            let _ =
                tokio::time::timeout(std::time::Duration::from_millis(500), ws_stream.next()).await;
        });

        format!("ws://{}/gateway", addr)
    }

    async fn spawn_mock_reconnecting_gateway(
        first_event: serde_json::Value,
        second_event: serde_json::Value,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for event in [first_event, second_event] {
                let (tcp_stream, _) = listener.accept().await.unwrap();
                let mut ws_stream = accept_async(tcp_stream).await.unwrap();
                ws_stream
                    .send(WsMessage::Text(
                        serde_json::json!({
                            "op": 10,
                            "d": {"heartbeat_interval": 250}
                        })
                        .to_string()
                        .into(),
                    ))
                    .await
                    .unwrap();
                let identify = ws_stream.next().await.unwrap().unwrap();
                let identify_json: serde_json::Value =
                    serde_json::from_str(&identify.into_text().unwrap()).unwrap();
                assert_eq!(identify_json["op"], 2);
                ws_stream
                    .send(WsMessage::Text(event.to_string().into()))
                    .await
                    .unwrap();
                ws_stream.close(None).await.unwrap();
            }
        });

        format!("ws://{}/gateway", addr)
    }

    async fn spawn_mock_resumable_gateway(
        ready_session_id: &str,
        first_event: serde_json::Value,
        second_event: serde_json::Value,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let ready_session_id = ready_session_id.to_string();
        tokio::spawn(async move {
            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(tcp_stream).await.unwrap();
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let identify = ws_stream.next().await.unwrap().unwrap();
            let identify_json: serde_json::Value =
                serde_json::from_str(&identify.into_text().unwrap()).unwrap();
            assert_eq!(identify_json["op"], 2);
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 0,
                        "t": "READY",
                        "s": 1,
                        "d": {"session_id": ready_session_id}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            ws_stream
                .send(WsMessage::Text(first_event.to_string().into()))
                .await
                .unwrap();
            ws_stream.close(None).await.unwrap();

            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(tcp_stream).await.unwrap();
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let resume = ws_stream.next().await.unwrap().unwrap();
            let resume_json: serde_json::Value =
                serde_json::from_str(&resume.into_text().unwrap()).unwrap();
            assert_eq!(resume_json["op"], 6);
            assert_eq!(resume_json["d"]["session_id"], ready_session_id);
            assert_eq!(resume_json["d"]["seq"], 2);
            ws_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 0,
                        "t": "RESUMED",
                        "s": 3,
                        "d": {}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            ws_stream
                .send(WsMessage::Text(second_event.to_string().into()))
                .await
                .unwrap();
            ws_stream.close(None).await.unwrap();
        });

        format!("ws://{}/gateway", addr)
    }

    async fn spawn_mock_stale_heartbeat_gateway(second_event: serde_json::Value) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut first_stream = accept_async(tcp_stream).await.unwrap();
            first_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 50}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let identify = first_stream.next().await.unwrap().unwrap();
            let identify_json: serde_json::Value =
                serde_json::from_str(&identify.into_text().unwrap()).unwrap();
            assert_eq!(identify_json["op"], 2);
            let heartbeat = first_stream.next().await.unwrap().unwrap();
            let heartbeat_json: serde_json::Value =
                serde_json::from_str(&heartbeat.into_text().unwrap()).unwrap();
            assert_eq!(heartbeat_json["op"], 1);

            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut second_stream = accept_async(tcp_stream).await.unwrap();
            second_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let identify = second_stream.next().await.unwrap().unwrap();
            let identify_json: serde_json::Value =
                serde_json::from_str(&identify.into_text().unwrap()).unwrap();
            assert_eq!(identify_json["op"], 2);
            second_stream
                .send(WsMessage::Text(second_event.to_string().into()))
                .await
                .unwrap();
            second_stream.close(None).await.unwrap();
            first_stream.close(None).await.unwrap();
        });

        format!("ws://{}/gateway", addr)
    }

    async fn spawn_mock_invalid_session_gateway(second_event: serde_json::Value) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut first_stream = accept_async(tcp_stream).await.unwrap();
            first_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let identify = first_stream.next().await.unwrap().unwrap();
            let identify_json: serde_json::Value =
                serde_json::from_str(&identify.into_text().unwrap()).unwrap();
            assert_eq!(identify_json["op"], 2);
            first_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 9,
                        "d": false
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            first_stream.close(None).await.unwrap();

            let (tcp_stream, _) = listener.accept().await.unwrap();
            let mut second_stream = accept_async(tcp_stream).await.unwrap();
            second_stream
                .send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": {"heartbeat_interval": 250}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let identify = second_stream.next().await.unwrap().unwrap();
            let identify_json: serde_json::Value =
                serde_json::from_str(&identify.into_text().unwrap()).unwrap();
            assert_eq!(identify_json["op"], 2);
            second_stream
                .send(WsMessage::Text(second_event.to_string().into()))
                .await
                .unwrap();
            second_stream.close(None).await.unwrap();
        });

        format!("ws://{}/gateway", addr)
    }

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

        let gateway_url = spawn_mock_gateway(None).await;
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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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

        let gateway_url = spawn_mock_gateway(None).await;
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "123",
                "username": "test-bot"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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
    async fn test_send_prefers_thread_id_and_reply_alias() {
        use wiremock::matchers::{body_partial_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/channels/thread-1/messages"))
            .and(header("authorization", "Bot token"))
            .and(body_partial_json(serde_json::json!({
                "message_reference": {"message_id": "parent-1"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "message-1"
            })))
            .mount(&server)
            .await;

        let channel = DiscordChannel::new(DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: None,
            api_base_url: Some(server.uri()),
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        });
        *channel.is_connected.write().await = true;

        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "thread reply".to_string(),
                metadata: serde_json::json!({
                    "discord_channel_id": "channel-1",
                    "discord_thread_id": "thread-1",
                    "discord_reply_to_message_id": "parent-1"
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_gateway_message_create_is_enqueued() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let gateway_url = spawn_mock_gateway(Some(serde_json::json!({
            "op": 0,
            "t": "MESSAGE_CREATE",
            "s": 1,
            "d": {
                "id": "message-1",
                "channel_id": "channel-1",
                "thread_id": "thread-1",
                "guild_id": "guild-1",
                "content": "hello from gateway",
                "timestamp": "2026-01-01T00:00:00Z",
                "message_reference": {
                    "message_id": "parent-1"
                },
                "attachments": [{"id":"attachment-1"}],
                "embeds": [{"title":"embed"}],
                "author": {
                    "id": "user-1",
                    "username": "alice",
                    "bot": false
                }
            }
        })))
        .await;

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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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
        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.content, "hello from gateway");
        assert_eq!(incoming.user_id, "user-1");
        assert_eq!(incoming.metadata["discord_channel_id"], "channel-1");
        assert_eq!(incoming.metadata["discord_thread_id"], "thread-1");
        assert_eq!(
            incoming.metadata["discord_referenced_message_id"],
            "parent-1"
        );
        assert_eq!(incoming.metadata["discord_is_dm"], false);
        assert_eq!(incoming.metadata["discord_attachment_count"], 1);
        assert_eq!(incoming.metadata["discord_embed_count"], 1);
        assert_eq!(
            incoming.metadata["discord_timestamp"],
            "2026-01-01T00:00:00Z"
        );
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_gateway_ignores_webhook_and_self_authored_messages() {
        let ignored_webhook = normalize_gateway_message(
            &DiscordConfig {
                enabled: true,
                token: "token".to_string(),
                application_id: "app-1".to_string(),
                interaction_public_key: None,
                api_base_url: None,
                rate_limit_requests_per_second: 5,
                allowed_guilds: vec![],
                allowed_channels: vec![],
                dm_enabled: true,
            },
            GatewayMessageCreate {
                id: "message-1".to_string(),
                channel_id: "channel-1".to_string(),
                thread_id: None,
                guild_id: None,
                content: "from webhook".to_string(),
                timestamp: None,
                webhook_id: Some("webhook-1".to_string()),
                message_reference: None,
                attachments: vec![],
                embeds: vec![],
                author: GatewayAuthor {
                    id: "user-1".to_string(),
                    username: "alice".to_string(),
                    bot: Some(false),
                    system: Some(false),
                },
            },
        )
        .unwrap();
        assert!(ignored_webhook.is_none());

        let ignored_self = normalize_gateway_message(
            &DiscordConfig {
                enabled: true,
                token: "token".to_string(),
                application_id: "app-1".to_string(),
                interaction_public_key: None,
                api_base_url: None,
                rate_limit_requests_per_second: 5,
                allowed_guilds: vec![],
                allowed_channels: vec![],
                dm_enabled: true,
            },
            GatewayMessageCreate {
                id: "message-2".to_string(),
                channel_id: "channel-1".to_string(),
                thread_id: None,
                guild_id: None,
                content: "from self".to_string(),
                timestamp: None,
                webhook_id: None,
                message_reference: None,
                attachments: vec![],
                embeds: vec![],
                author: GatewayAuthor {
                    id: "app-1".to_string(),
                    username: "bot".to_string(),
                    bot: Some(false),
                    system: Some(false),
                },
            },
        )
        .unwrap();
        assert!(ignored_self.is_none());
    }

    #[tokio::test]
    async fn test_gateway_reconnects_after_close() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let gateway_url = spawn_mock_reconnecting_gateway(
            serde_json::json!({
                "op": 0,
                "t": "MESSAGE_CREATE",
                "s": 1,
                "d": {
                    "id": "message-1",
                    "channel_id": "channel-1",
                    "guild_id": "guild-1",
                    "content": "first message",
                    "author": {
                        "id": "user-1",
                        "username": "alice",
                        "bot": false
                    }
                }
            }),
            serde_json::json!({
                "op": 0,
                "t": "MESSAGE_CREATE",
                "s": 2,
                "d": {
                    "id": "message-2",
                    "channel_id": "channel-1",
                    "guild_id": "guild-1",
                    "content": "second message",
                    "author": {
                        "id": "user-2",
                        "username": "bob",
                        "bot": false
                    }
                }
            }),
        )
        .await;

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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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

        let first = channel.receive().await.unwrap();
        assert_eq!(first.content, "first message");

        let second = tokio::time::timeout(Duration::from_secs(3), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.content, "second message");
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_gateway_resumes_session_after_reconnect() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let gateway_url = spawn_mock_resumable_gateway(
            "session-123",
            serde_json::json!({
                "op": 0,
                "t": "MESSAGE_CREATE",
                "s": 2,
                "d": {
                    "id": "message-1",
                    "channel_id": "channel-1",
                    "guild_id": "guild-1",
                    "content": "first message",
                    "author": {
                        "id": "user-1",
                        "username": "alice",
                        "bot": false
                    }
                }
            }),
            serde_json::json!({
                "op": 0,
                "t": "MESSAGE_CREATE",
                "s": 4,
                "d": {
                    "id": "message-2",
                    "channel_id": "channel-1",
                    "guild_id": "guild-1",
                    "content": "second message",
                    "author": {
                        "id": "user-2",
                        "username": "bob",
                        "bot": false
                    }
                }
            }),
        )
        .await;

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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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

        let first = channel.receive().await.unwrap();
        assert_eq!(first.content, "first message");

        let second = tokio::time::timeout(Duration::from_secs(3), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.content, "second message");
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_gateway_reconnects_after_heartbeat_ack_timeout() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let gateway_url = spawn_mock_stale_heartbeat_gateway(serde_json::json!({
            "op": 0,
            "t": "MESSAGE_CREATE",
            "s": 1,
            "d": {
                "id": "message-2",
                "channel_id": "channel-1",
                "guild_id": "guild-1",
                "content": "recovered after stale heartbeat",
                "author": {
                    "id": "user-2",
                    "username": "bob",
                    "bot": false
                }
            }
        }))
        .await;

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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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

        let recovered = tokio::time::timeout(Duration::from_secs(3), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.content, "recovered after stale heartbeat");
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_gateway_reconnects_after_invalid_session() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let gateway_url = spawn_mock_invalid_session_gateway(serde_json::json!({
            "op": 0,
            "t": "MESSAGE_CREATE",
            "s": 1,
            "d": {
                "id": "message-3",
                "channel_id": "channel-1",
                "guild_id": "guild-1",
                "content": "recovered after invalid session",
                "author": {
                    "id": "user-3",
                    "username": "carol",
                    "bot": false
                }
            }
        }))
        .await;

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
        Mock::given(method("GET"))
            .and(path("/gateway/bot"))
            .and(header("authorization", "Bot token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": gateway_url,
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

        let recovered = tokio::time::timeout(Duration::from_secs(3), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.content, "recovered after invalid session");
        channel.disconnect().await.unwrap();
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
        assert_eq!(
            incoming.metadata["discord_interaction_token"],
            "interaction-token"
        );
        assert_eq!(incoming.metadata["discord_is_dm"], false);
        assert_eq!(incoming.metadata["discord_author_id"], "user-1");
    }
}
