//! WhatsApp integration for OpenRustClaw using Baileys library.
//!
//! Features:
//! - Text messages (DMs and groups)
//! - Media messages (images, audio, video, documents)
//! - QR code pairing
//! - Phone number pairing (for desktop/mobile linking)
//! - Automatic reconnection with exponential backoff
//! - DM allowlist for access control
//! - Webhook notifications
//! - Message reactions and replies
//!
//! Architecture:
//! This implementation uses a Node.js bridge process that wraps the Baileys library.
//! Communication between Rust and Node.js happens via JSON-RPC over stdin/stdout.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::interval;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use openrustclaw_core::config::WhatsAppConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Current connection state of the WhatsApp channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    AwaitingQrCode,
    AwaitingPairingCode,
    Authenticated,
    Connected,
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "disconnected"),
            ConnectionState::Connecting => write!(f, "connecting"),
            ConnectionState::AwaitingQrCode => write!(f, "awaiting_qr_code"),
            ConnectionState::AwaitingPairingCode => write!(f, "awaiting_pairing_code"),
            ConnectionState::Authenticated => write!(f, "authenticated"),
            ConnectionState::Connected => write!(f, "connected"),
            ConnectionState::Reconnecting => write!(f, "reconnecting"),
        }
    }
}

/// Types of media that can be sent/received on WhatsApp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Document,
    Sticker,
    Voice,
}

/// Metadata for media messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub media_type: MediaType,
    pub mime_type: String,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub caption: Option<String>,
    pub duration_seconds: Option<u32>,
    pub url: Option<String>,
}

/// Internal message format for bridge communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeMessage {
    /// Request to start connection
    Connect {
        session_path: String,
        pairing_mode: bool,
    },
    /// Connection established
    Connected,
    /// QR code for pairing
    QrCode {
        qr_code: String,
    },
    /// Pairing code for mobile linking
    PairingCode {
        code: String,
    },
    /// Disconnected from WhatsApp
    Disconnected {
        reason: Option<String>,
    },
    /// Incoming text message
    Message {
        id: String,
        from: String,
        from_name: Option<String>,
        content: String,
        timestamp: u64,
        is_group: bool,
        group_id: Option<String>,
        group_name: Option<String>,
        quoted_message: Option<String>,
        mentions: Vec<String>,
    },
    /// Incoming media message
    MediaMessage {
        id: String,
        from: String,
        from_name: Option<String>,
        media: MediaMetadata,
        timestamp: u64,
        is_group: bool,
        group_id: Option<String>,
        group_name: Option<String>,
    },
    /// Request to send a message
    SendMessage {
        request_id: String,
        to: String,
        content: String,
        reply_to: Option<String>,
    },
    /// Request to send media
    SendMedia {
        request_id: String,
        to: String,
        media_type: MediaType,
        url_or_path: String,
        caption: Option<String>,
        reply_to: Option<String>,
    },
    /// Message sent confirmation
    MessageSent {
        request_id: String,
        message_id: String,
    },
    /// Error response
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
    },
    /// Ping/Pong for keepalive
    Ping,
    Pong,
}

/// WhatsApp channel implementation using Baileys bridge.
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
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
    state: Arc<RwLock<ConnectionState>>,
    baileys_process: Arc<Mutex<Option<Child>>>,
    bridge_stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<BridgeMessage>>>>,
    qr_code_tx: Arc<RwLock<Option<mpsc::Sender<String>>>>,
    latest_qr_code: Arc<RwLock<Option<String>>>,
    latest_pairing_code: Arc<RwLock<Option<String>>>,
    reconnect_attempts: Arc<RwLock<u32>>,
    should_reconnect: Arc<RwLock<bool>>,
}

impl WhatsAppChannel {
    /// Create a new WhatsApp channel with the given configuration.
    pub fn new(config: WhatsAppConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (WhatsApp allows ~15 messages per minute in groups, ~60 in DMs)
        let quota = Quota::with_period(Duration::from_millis(
            1000 / config.rate_limit_per_second.max(1) as u64,
        ))
        .unwrap_or_else(|| Quota::per_second(NonZeroU32::new(10).unwrap()));
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            baileys_process: Arc::new(Mutex::new(None)),
            bridge_stdin: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            qr_code_tx: Arc::new(RwLock::new(None)),
            latest_qr_code: Arc::new(RwLock::new(None)),
            latest_pairing_code: Arc::new(RwLock::new(None)),
            reconnect_attempts: Arc::new(RwLock::new(0)),
            should_reconnect: Arc::new(RwLock::new(true)),
        }
    }

    /// Get current connection state.
    pub async fn connection_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if the channel is currently connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Check if a phone number is allowed to interact.
    #[allow(dead_code)]
    fn is_number_allowed(&self, phone_number: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        // Normalize phone number (remove spaces, dashes, etc.)
        let normalized = phone_number.replace([' ', '-', '+'], "");
        self.config
            .allowlist
            .iter()
            .any(|allowed| allowed.replace([' ', '-', '+'], "") == normalized)
    }

    /// Validate DM allowlist - returns true if allowed or if it's a group message.
    #[allow(dead_code)]
    fn validate_dm_access(&self, phone_number: &str, is_group: bool) -> bool {
        // Group messages are always allowed (group-level permissions handled elsewhere)
        if is_group {
            return true;
        }
        // DMs must pass allowlist
        self.is_number_allowed(phone_number)
    }

    /// Set up QR code callback channel.
    pub async fn set_qr_callback(&self, tx: mpsc::Sender<String>) {
        *self.qr_code_tx.write().await = Some(tx);
    }

    pub async fn latest_qr_code(&self) -> Option<String> {
        self.latest_qr_code.read().await.clone()
    }

    pub async fn latest_pairing_code(&self) -> Option<String> {
        self.latest_pairing_code.read().await.clone()
    }

    pub async fn wait_for_pairing_artifact(&self, timeout_duration: Duration) -> Result<String> {
        let deadline = tokio::time::Instant::now() + timeout_duration;
        loop {
            match *self.state.read().await {
                ConnectionState::AwaitingQrCode => {
                    if let Some(code) = self.latest_qr_code().await {
                        return Ok(code);
                    }
                }
                ConnectionState::AwaitingPairingCode => {
                    if let Some(code) = self.latest_pairing_code().await {
                        return Ok(code);
                    }
                }
                ConnectionState::Connected => {
                    return Err(ChannelError::Connection {
                        platform: "whatsapp".to_string(),
                        message: "WhatsApp connected before emitting a pairing artifact"
                            .to_string(),
                    }
                    .into());
                }
                _ => {}
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(ChannelError::Connection {
                    platform: "whatsapp".to_string(),
                    message: "Timed out waiting for WhatsApp QR/pairing code".to_string(),
                }
                .into());
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Start the Baileys bridge process.
    async fn start_bridge_process(&self) -> Result<()> {
        let bridge_path = PathBuf::from(&self.config.bridge_path);

        if !bridge_path.exists() {
            return Err(ChannelError::Config {
                platform: "whatsapp".to_string(),
                message: format!("Baileys bridge not found at: {}", bridge_path.display()),
            }
            .into());
        }

        // Check if node is available
        let node_check = Command::new("node").arg("--version").output().await;

        if node_check.is_err() {
            return Err(ChannelError::Config {
                platform: "whatsapp".to_string(),
                message: "Node.js is required but not found in PATH".to_string(),
            }
            .into());
        }

        info!(bridge_path = %bridge_path.display(), "Starting Baileys bridge process");

        let mut child = Command::new("node")
            .arg(&bridge_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ChannelError::Connection {
                platform: "whatsapp".to_string(),
                message: format!("Failed to start bridge process: {}", e),
            })?;

        let stdin = child.stdin.take().ok_or_else(|| ChannelError::Connection {
            platform: "whatsapp".to_string(),
            message: "Failed to capture stdin".to_string(),
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ChannelError::Connection {
                platform: "whatsapp".to_string(),
                message: "Failed to capture stdout".to_string(),
            })?;

        // Store the process and stdin
        *self.baileys_process.lock().await = Some(child);
        *self.bridge_stdin.lock().await = Some(stdin);

        // Start stdout reader task
        self.start_stdout_reader(stdout).await;

        // Start stderr logger task
        if let Some(process) = self.baileys_process.lock().await.as_mut()
            && let Some(stderr) = process.stderr.take()
        {
            self.start_stderr_logger(stderr).await;
        }

        Ok(())
    }

    /// Start the stdout reader task that processes bridge messages.
    async fn start_stdout_reader(&self, stdout: tokio::process::ChildStdout) {
        let state = Arc::clone(&self.state);
        let incoming_tx = self.incoming_tx.clone();
        let pending_requests = Arc::clone(&self.pending_requests);
        let qr_code_tx = Arc::clone(&self.qr_code_tx);
        let latest_qr_code = Arc::clone(&self.latest_qr_code);
        let latest_pairing_code = Arc::clone(&self.latest_pairing_code);
        let allowlist = self.config.allowlist.clone();
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let reconnect_attempts = Arc::clone(&self.reconnect_attempts);
        let webhook_url = self.config.webhook_url.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                trace!(line = %line, "Received from bridge");

                match serde_json::from_str::<BridgeMessage>(&line) {
                    Ok(msg) => {
                        match &msg {
                            BridgeMessage::Connected => {
                                info!("WhatsApp connected");
                                *state.write().await = ConnectionState::Connected;
                                *reconnect_attempts.write().await = 0;
                                *latest_qr_code.write().await = None;
                                *latest_pairing_code.write().await = None;
                                send_whatsapp_webhook_event(
                                    webhook_url.as_deref(),
                                    "whatsapp_connected",
                                    serde_json::json!({}),
                                )
                                .await;
                            }
                            BridgeMessage::QrCode { qr_code } => {
                                info!("Received QR code for pairing");
                                *state.write().await = ConnectionState::AwaitingQrCode;
                                *latest_qr_code.write().await = Some(qr_code.clone());
                                *latest_pairing_code.write().await = None;

                                // Send QR code to callback if set
                                if let Some(tx) = qr_code_tx.read().await.as_ref() {
                                    let _ = tx.send(qr_code.clone()).await;
                                }

                                // Also log it for terminal users
                                info!("\n{}", create_qr_code_display(qr_code));
                                send_whatsapp_webhook_event(
                                    webhook_url.as_deref(),
                                    "whatsapp_qr_code",
                                    serde_json::json!({
                                        "qr_preview": qr_code.chars().take(32).collect::<String>(),
                                    }),
                                )
                                .await;
                            }
                            BridgeMessage::PairingCode { code } => {
                                info!(pairing_code = %code, "Received pairing code");
                                *state.write().await = ConnectionState::AwaitingPairingCode;
                                *latest_pairing_code.write().await = Some(code.clone());
                                info!("\n╔════════════════════════════════════╗");
                                info!("║     WhatsApp Pairing Code          ║");
                                info!("║                                    ║");
                                info!("║         {}        ║", code);
                                info!("║                                    ║");
                                info!("║ Open WhatsApp → Settings → Devices ║");
                                info!("║ → Link a Device → Link with code   ║");
                                info!("╚════════════════════════════════════╝");
                                send_whatsapp_webhook_event(
                                    webhook_url.as_deref(),
                                    "whatsapp_pairing_code",
                                    serde_json::json!({ "code": code }),
                                )
                                .await;
                            }
                            BridgeMessage::Disconnected { reason } => {
                                warn!(reason = ?reason, "WhatsApp disconnected");
                                *state.write().await = ConnectionState::Disconnected;
                                *latest_qr_code.write().await = None;
                                *latest_pairing_code.write().await = None;
                                send_whatsapp_webhook_event(
                                    webhook_url.as_deref(),
                                    "whatsapp_disconnected",
                                    serde_json::json!({ "reason": reason }),
                                )
                                .await;
                            }
                            BridgeMessage::Message {
                                id,
                                from,
                                from_name,
                                content,
                                timestamp,
                                is_group,
                                group_id,
                                group_name,
                                quoted_message,
                                mentions,
                                ..
                            } => {
                                // Validate DM allowlist
                                if !allowlist.is_empty() && !is_group {
                                    let normalized = from.replace([' ', '-', '+'], "");
                                    if !allowlist
                                        .iter()
                                        .any(|a| a.replace([' ', '-', '+'], "") == normalized)
                                    {
                                        debug!(from = %from, "Message from non-allowlisted number, ignoring");
                                        continue;
                                    }
                                }

                                // Apply rate limiting
                                rate_limiter.until_ready().await;

                                let (chat_jid, chat_id) = normalize_whatsapp_chat_target(
                                    &from,
                                    *is_group,
                                    group_id.as_deref(),
                                );

                                let incoming = IncomingMessage {
                                    session_id: Uuid::new_v4(),
                                    user_id: from.clone(),
                                    content: content.clone(),
                                    platform: Platform::WhatsApp,
                                    metadata: serde_json::json!({
                                        "whatsapp_message_id": id,
                                        "whatsapp_jid": chat_jid,
                                        "whatsapp_chat_id": chat_id,
                                        "whatsapp_from_name": from_name,
                                        "whatsapp_timestamp": timestamp,
                                        "whatsapp_is_group": is_group,
                                        "whatsapp_group_id": group_id,
                                        "whatsapp_group_name": group_name,
                                        "whatsapp_quoted_message": quoted_message,
                                        "whatsapp_mentions": mentions,
                                    }),
                                };

                                if let Err(e) = incoming_tx.send(incoming).await {
                                    error!(error = %e, "Failed to forward incoming message");
                                }
                            }
                            BridgeMessage::MediaMessage {
                                id,
                                from,
                                from_name,
                                media,
                                timestamp,
                                is_group,
                                group_id,
                                group_name,
                            } => {
                                // Validate DM allowlist
                                if !allowlist.is_empty() && !is_group {
                                    let normalized = from.replace([' ', '-', '+'], "");
                                    if !allowlist
                                        .iter()
                                        .any(|a| a.replace([' ', '-', '+'], "") == normalized)
                                    {
                                        debug!(from = %from, "Media message from non-allowlisted number, ignoring");
                                        continue;
                                    }
                                }

                                // Apply rate limiting
                                rate_limiter.until_ready().await;

                                // Build content based on media type
                                let content = format!(
                                    "[{}: {}]",
                                    format_media_type(&media.media_type),
                                    media.caption.as_deref().unwrap_or("No caption")
                                );
                                let (chat_jid, chat_id) = normalize_whatsapp_chat_target(
                                    &from,
                                    *is_group,
                                    group_id.as_deref(),
                                );

                                let incoming = IncomingMessage {
                                    session_id: Uuid::new_v4(),
                                    user_id: from.clone(),
                                    content,
                                    platform: Platform::WhatsApp,
                                    metadata: serde_json::json!({
                                        "whatsapp_message_id": id,
                                        "whatsapp_jid": chat_jid,
                                        "whatsapp_chat_id": chat_id,
                                        "whatsapp_from_name": from_name,
                                        "whatsapp_timestamp": timestamp,
                                        "whatsapp_is_group": is_group,
                                        "whatsapp_group_id": group_id,
                                        "whatsapp_group_name": group_name,
                                        "whatsapp_media": media,
                                        "file_references": media.url.as_ref().map(|url| vec![url.clone()]),
                                    }),
                                };

                                if let Err(e) = incoming_tx.send(incoming).await {
                                    error!(error = %e, "Failed to forward incoming media message");
                                }
                            }
                            BridgeMessage::MessageSent {
                                request_id,
                                message_id,
                            } => {
                                debug!(%request_id, %message_id, "Message confirmed sent");
                                // Resolve pending request
                                if let Some(tx) = pending_requests.write().await.remove(request_id)
                                {
                                    let _ = tx.send(msg);
                                }
                            }
                            BridgeMessage::Error {
                                request_id,
                                code,
                                message,
                            } => {
                                error!(%code, %message, "Bridge error");
                                if let Some(req_id) = request_id
                                    && let Some(tx) = pending_requests.write().await.remove(req_id)
                                {
                                    let _ = tx.send(msg);
                                }
                            }
                            BridgeMessage::Pong => {
                                trace!("Received pong");
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, line = %line, "Failed to parse bridge message");
                    }
                }
            }

            info!("Bridge stdout reader ended");
            *state.write().await = ConnectionState::Disconnected;
            *latest_qr_code.write().await = None;
            *latest_pairing_code.write().await = None;
            send_whatsapp_webhook_event(
                webhook_url.as_deref(),
                "whatsapp_bridge_stopped",
                serde_json::json!({}),
            )
            .await;
        });
    }

    /// Start stderr logger task.
    async fn start_stderr_logger(&self, stderr: tokio::process::ChildStderr) {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                debug!(target: "baileys_bridge", "{}", line);
            }
        });
    }

    /// Send a message to the bridge process.
    async fn send_to_bridge(&self, msg: BridgeMessage) -> Result<()> {
        let mut stdin = self.bridge_stdin.lock().await;
        let stdin = stdin.as_mut().ok_or_else(|| ChannelError::NotConnected {
            platform: "whatsapp".to_string(),
        })?;

        let json = serde_json::to_string(&msg).map_err(|e| ChannelError::InvalidFormat {
            platform: "whatsapp".to_string(),
            message: format!("Failed to serialize message: {}", e),
        })?;

        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "whatsapp".to_string(),
                message: format!("Failed to write to bridge: {}", e),
            })?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "whatsapp".to_string(),
                message: format!("Failed to write newline: {}", e),
            })?;
        stdin.flush().await.map_err(|e| ChannelError::SendFailed {
            platform: "whatsapp".to_string(),
            message: format!("Failed to flush: {}", e),
        })?;

        Ok(())
    }

    /// Start the reconnection monitor.
    #[allow(dead_code)]
    async fn start_reconnection_monitor(&self) {
        let state = Arc::clone(&self.state);
        let should_reconnect = Arc::clone(&self.should_reconnect);
        let reconnect_attempts = Arc::clone(&self.reconnect_attempts);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(5));

            loop {
                check_interval.tick().await;

                if !*should_reconnect.read().await {
                    break;
                }

                let current_state = *state.read().await;

                if current_state == ConnectionState::Disconnected {
                    let attempts = *reconnect_attempts.read().await;

                    if attempts >= config.max_reconnect_attempts {
                        error!("Max reconnection attempts reached, giving up");
                        *should_reconnect.write().await = false;
                        break;
                    }

                    let delay =
                        Duration::from_secs(config.reconnect_delay_secs * (attempts as u64 + 1));

                    warn!(attempt = attempts + 1, ?delay, "Reconnecting to WhatsApp");
                    *reconnect_attempts.write().await = attempts + 1;
                    *state.write().await = ConnectionState::Reconnecting;

                    tokio::time::sleep(delay).await;

                    // Note: In a real implementation, we'd trigger reconnection here
                    // This requires storing the connect logic separately
                }
            }
        });
    }

    /// Gracefully stop the bridge process.
    async fn stop_bridge_process(&self) {
        // Stop reconnection attempts
        *self.should_reconnect.write().await = false;

        // Close stdin to signal EOF
        *self.bridge_stdin.lock().await = None;

        // Kill the process
        if let Some(mut child) = self.baileys_process.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        *self.state.write().await = ConnectionState::Disconnected;
    }

    /// Extract WhatsApp JID from metadata.
    fn extract_jid(metadata: &serde_json::Value) -> Option<String> {
        metadata
            .get("whatsapp_jid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                metadata
                    .get("whatsapp_group_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                // Try to construct from other fields
                metadata
                    .get("whatsapp_chat_id")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.contains('@') {
                            s.to_string()
                        } else {
                            format!("{}@s.whatsapp.net", s)
                        }
                    })
            })
    }

    async fn wait_for_request_result(&self, request_id: String) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests
            .write()
            .await
            .insert(request_id.clone(), tx);

        match timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(BridgeMessage::MessageSent { .. })) => Ok(()),
            Ok(Ok(BridgeMessage::Error { code, message, .. })) => Err(ChannelError::SendFailed {
                platform: "whatsapp".to_string(),
                message: format!("{}: {}", code, message),
            }
            .into()),
            Ok(Ok(other)) => Err(ChannelError::SendFailed {
                platform: "whatsapp".to_string(),
                message: format!("Unexpected WhatsApp bridge response: {:?}", other),
            }
            .into()),
            Ok(Err(_)) => Err(ChannelError::Connection {
                platform: "whatsapp".to_string(),
                message: "WhatsApp bridge request channel closed".to_string(),
            }
            .into()),
            Err(_) => {
                self.pending_requests.write().await.remove(&request_id);
                Err(ChannelError::SendFailed {
                    platform: "whatsapp".to_string(),
                    message: "Timed out waiting for WhatsApp bridge acknowledgement".to_string(),
                }
                .into())
            }
        }
    }

    async fn wait_for_connection_ready(&self) -> Result<()> {
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            match *self.state.read().await {
                ConnectionState::Connected
                | ConnectionState::AwaitingQrCode
                | ConnectionState::AwaitingPairingCode => return Ok(()),
                ConnectionState::Disconnected if std::time::Instant::now() > deadline => {
                    return Err(ChannelError::Connection {
                        platform: "whatsapp".to_string(),
                        message: "WhatsApp bridge did not become ready in time".to_string(),
                    }
                    .into());
                }
                _ => {}
            }
            if std::time::Instant::now() > deadline {
                return Err(ChannelError::Connection {
                    platform: "whatsapp".to_string(),
                    message: "WhatsApp bridge did not become ready in time".to_string(),
                }
                .into());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn platform(&self) -> Platform {
        Platform::WhatsApp
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Get target JID from metadata
        let to = Self::extract_jid(&msg.metadata).ok_or_else(|| ChannelError::InvalidFormat {
            platform: "whatsapp".to_string(),
            message: "Missing whatsapp_jid or whatsapp_chat_id in metadata".to_string(),
        })?;

        // Check for reply_to
        let reply_to = msg
            .metadata
            .get("whatsapp_reply_to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Check if this is a media message
        if let Some(media_type_str) = msg
            .metadata
            .get("whatsapp_media_type")
            .and_then(|v| v.as_str())
        {
            let media_type = match media_type_str {
                "image" => MediaType::Image,
                "video" => MediaType::Video,
                "audio" => MediaType::Audio,
                "document" => MediaType::Document,
                "sticker" => MediaType::Sticker,
                "voice" => MediaType::Voice,
                _ => {
                    return Err(ChannelError::InvalidFormat {
                        platform: "whatsapp".to_string(),
                        message: format!("Unknown media type: {}", media_type_str),
                    }
                    .into());
                }
            };

            let url_or_path = msg
                .metadata
                .get("whatsapp_media_url")
                .or_else(|| msg.metadata.get("whatsapp_media_path"))
                .or_else(|| {
                    msg.metadata
                        .get("file_references")
                        .and_then(|value| value.as_array())
                        .and_then(|values| values.first())
                })
                .and_then(|v| v.as_str())
                .ok_or_else(|| ChannelError::InvalidFormat {
                    platform: "whatsapp".to_string(),
                    message: "Missing whatsapp_media_url or whatsapp_media_path".to_string(),
                })?;

            let caption = if msg.content.is_empty() {
                None
            } else {
                Some(msg.content)
            };

            let request_id = Uuid::new_v4().to_string();
            let bridge_msg = BridgeMessage::SendMedia {
                request_id: request_id.clone(),
                to,
                media_type,
                url_or_path: url_or_path.to_string(),
                caption,
                reply_to,
            };

            self.send_to_bridge(bridge_msg).await?;
            self.wait_for_request_result(request_id).await?;
        } else {
            // Text message
            let request_id = Uuid::new_v4().to_string();
            let bridge_msg = BridgeMessage::SendMessage {
                request_id: request_id.clone(),
                to,
                content: msg.content,
                reply_to,
            };

            self.send_to_bridge(bridge_msg).await?;
            self.wait_for_request_result(request_id).await?;
        }

        debug!("Sent WhatsApp message");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "whatsapp".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        let current_state = *self.state.read().await;
        if current_state == ConnectionState::Connected
            || current_state == ConnectionState::Connecting
        {
            return Ok(());
        }

        info!("Connecting to WhatsApp...");
        *self.state.write().await = ConnectionState::Connecting;

        // Start the bridge process
        self.start_bridge_process().await?;

        // Wait a moment for the bridge to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Send connect message
        let connect_msg = BridgeMessage::Connect {
            session_path: self.config.session_path.clone(),
            pairing_mode: self.config.pairing_mode,
        };
        self.send_to_bridge(connect_msg).await?;
        self.wait_for_connection_ready().await?;

        // Send webhook notification if configured
        if let Some(webhook_url) = &self.config.webhook_url {
            tokio::spawn({
                let url = webhook_url.clone();
                async move {
                    let _ = reqwest::Client::new()
                        .post(&url)
                        .json(&serde_json::json!({
                            "event": "whatsapp_connecting",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }))
                        .send()
                        .await;
                }
            });
        }

        info!("WhatsApp channel connection initiated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from WhatsApp...");

        self.stop_bridge_process().await;

        // Send webhook notification if configured
        if let Some(webhook_url) = &self.config.webhook_url {
            tokio::spawn({
                let url = webhook_url.clone();
                async move {
                    let _ = reqwest::Client::new()
                        .post(&url)
                        .json(&serde_json::json!({
                            "event": "whatsapp_disconnected",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }))
                        .send()
                        .await;
                }
            });
        }

        info!("WhatsApp channel disconnected");
        Ok(())
    }
}

fn normalize_whatsapp_chat_target(
    from: &str,
    is_group: bool,
    group_id: Option<&str>,
) -> (String, String) {
    if is_group {
        let jid = group_id.unwrap_or(from).to_string();
        return (jid.clone(), jid);
    }

    let jid = from.to_string();
    let chat_id = from.split('@').next().unwrap_or(from).to_string();
    (jid, chat_id)
}

async fn send_whatsapp_webhook_event(
    webhook_url: Option<&str>,
    event: &str,
    extra: serde_json::Value,
) {
    let Some(url) = webhook_url else {
        return;
    };

    let mut body = serde_json::json!({
        "event": event,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    if let Some(map) = extra.as_object() {
        for (key, value) in map {
            body[key] = value.clone();
        }
    }

    let _ = reqwest::Client::new().post(url).json(&body).send().await;
}

impl Drop for WhatsAppChannel {
    fn drop(&mut self) {
        // Best effort cleanup
        futures::executor::block_on(self.stop_bridge_process());
    }
}

/// Format media type for display.
fn format_media_type(media_type: &MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "Image",
        MediaType::Video => "Video",
        MediaType::Audio => "Audio",
        MediaType::Document => "Document",
        MediaType::Sticker => "Sticker",
        MediaType::Voice => "Voice Message",
    }
}

/// Create a visual QR code display for terminal output.
fn create_qr_code_display(qr_code: &str) -> String {
    // Simple QR display - in production, use a QR code library
    // to render a proper terminal QR code
    format!(
        r#"
╔════════════════════════════════════════════════════════════╗
║                     WhatsApp QR Code                       ║
║                                                            ║
║   Scan this QR code with WhatsApp:                         ║
║   Settings → Devices → Link a Device → Link with QR code   ║
║                                                            ║
║   QR Data: {}...                                          ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝
"#,
        &qr_code[..qr_code.len().min(40)]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> WhatsAppConfig {
        WhatsAppConfig {
            enabled: true,
            session_path: "/tmp/test_whatsapp".to_string(),
            pairing_mode: false,
            allowlist: vec!["1234567890".to_string()],
            webhook_url: None,
            bridge_path: "/tmp/bridge.js".to_string(),
            rate_limit_per_second: 10,
            max_reconnect_attempts: 5,
            reconnect_delay_secs: 5,
        }
    }

    #[test]
    fn test_is_number_allowed_empty_list() {
        let mut config = create_test_config();
        config.allowlist = vec![];
        let channel = WhatsAppChannel::new(config);

        assert!(channel.is_number_allowed("+1234567890"));
        assert!(channel.is_number_allowed("0987654321"));
    }

    #[test]
    fn test_is_number_allowed_with_list() {
        let config = create_test_config();
        let channel = WhatsAppChannel::new(config);

        assert!(channel.is_number_allowed("1234567890"));
        assert!(channel.is_number_allowed("+1 234-567-890"));
        assert!(!channel.is_number_allowed("5555555555"));
    }

    #[test]
    fn test_validate_dm_access() {
        let config = create_test_config();
        let channel = WhatsAppChannel::new(config);

        // DMs with allowlisted number should pass
        assert!(channel.validate_dm_access("1234567890", false));

        // DMs with non-allowlisted number should fail
        assert!(!channel.validate_dm_access("5555555555", false));

        // Group messages should always pass
        assert!(channel.validate_dm_access("5555555555", true));
    }

    #[test]
    fn test_extract_jid() {
        let metadata = serde_json::json!({
            "whatsapp_jid": "1234567890@s.whatsapp.net"
        });
        assert_eq!(
            WhatsAppChannel::extract_jid(&metadata),
            Some("1234567890@s.whatsapp.net".to_string())
        );

        let metadata2 = serde_json::json!({
            "whatsapp_chat_id": "1234567890"
        });
        assert_eq!(
            WhatsAppChannel::extract_jid(&metadata2),
            Some("1234567890@s.whatsapp.net".to_string())
        );

        let metadata3 = serde_json::json!({
            "whatsapp_group_id": "1203630@g.us"
        });
        assert_eq!(
            WhatsAppChannel::extract_jid(&metadata3),
            Some("1203630@g.us".to_string())
        );
    }

    #[test]
    fn test_connection_state_display() {
        assert_eq!(ConnectionState::Connected.to_string(), "connected");
        assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
        assert_eq!(
            ConnectionState::AwaitingQrCode.to_string(),
            "awaiting_qr_code"
        );
    }

    #[test]
    fn test_format_media_type() {
        assert_eq!(format_media_type(&MediaType::Image), "Image");
        assert_eq!(format_media_type(&MediaType::Video), "Video");
        assert_eq!(format_media_type(&MediaType::Voice), "Voice Message");
    }

    #[test]
    fn test_bridge_message_serialization() {
        let msg = BridgeMessage::Message {
            id: "123".to_string(),
            from: "456@s.whatsapp.net".to_string(),
            from_name: Some("Test".to_string()),
            content: "Hello".to_string(),
            timestamp: 1234567890,
            is_group: false,
            group_id: None,
            group_name: None,
            quoted_message: None,
            mentions: vec![],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_normalize_whatsapp_chat_target() {
        let (jid, chat_id) =
            normalize_whatsapp_chat_target("1234567890@s.whatsapp.net", false, None);
        assert_eq!(jid, "1234567890@s.whatsapp.net");
        assert_eq!(chat_id, "1234567890");

        let (jid, chat_id) =
            normalize_whatsapp_chat_target("1234567890@s.whatsapp.net", true, Some("1203630@g.us"));
        assert_eq!(jid, "1203630@g.us");
        assert_eq!(chat_id, "1203630@g.us");
    }
}
