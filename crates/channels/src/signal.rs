//! Signal Channel Integration for OpenRustClaw.
//!
//! Features:
//! - Text messages (direct and group chats)
//! - Attachments (images, audio, video, documents)
//! - Quotes/reply handling
//! - Mentions (@botname)
//! - Rate limiting
//! - Allowlist support for security
//!
//! Uses signal-cli (https://github.com/AsamK/signal-cli) via JSON-RPC
//! or direct libsignal-client bindings when available.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, error, info};
use uuid::Uuid;

use openrustclaw_core::config::SignalConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Signal channel implementation.
pub struct SignalChannel {
    config: SignalConfig,
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
    cli_process: Mutex<Option<Child>>,
    message_cache: Arc<RwLock<HashMap<Uuid, String>>>, // Maps session_id to Signal message timestamp
}

/// Signal envelope types received from signal-cli daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalEnvelope {
    #[serde(rename = "syncMessage")]
    SyncMessage {
        #[serde(rename = "syncMessage")]
        sync_message: SyncMessage,
    },
    #[serde(rename = "dataMessage")]
    DataMessage {
        #[serde(rename = "dataMessage")]
        data_message: DataMessage,
        timestamp: u64,
        source: String,
        #[serde(rename = "sourceNumber")]
        source_number: Option<String>,
        #[serde(rename = "sourceUuid")]
        source_uuid: Option<String>,
        #[serde(rename = "groupInfo")]
        group_info: Option<GroupInfo>,
    },
    Receipt {
        timestamp: u64,
        source: String,
    },
    #[serde(other)]
    Other,
}

/// Data message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMessage {
    pub message: Option<String>,
    pub attachments: Vec<SignalAttachment>,
    pub quote: Option<Quote>,
    pub mentions: Vec<Mention>,
    #[serde(rename = "groupInfo")]
    pub group_info: Option<GroupInfo>,
}

/// Sync message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    #[serde(rename = "sentMessage")]
    pub sent_message: Option<SentMessage>,
}

/// Sent message (from synced devices).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessage {
    pub message: Option<String>,
    pub timestamp: u64,
    pub destination: Option<String>,
}

/// Signal attachment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAttachment {
    pub content_type: String,
    pub filename: Option<String>,
    pub id: String,
    pub size: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub caption: Option<String>,
    #[serde(rename = "uploadTimestamp")]
    pub upload_timestamp: Option<u64>,
}

/// Quote/reply information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: u64,
    pub author: String,
    pub text: Option<String>,
}

/// Mention information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub name: String,
    pub number: Option<String>,
    pub uuid: Option<String>,
    pub start: usize,
    pub length: usize,
}

/// Group information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: Option<String>,
    pub members: Option<Vec<String>>,
}

impl SignalChannel {
    /// Create a new Signal channel with the given configuration.
    pub fn new(config: SignalConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (Signal has limits around 20 messages/minute)
        let quota = Quota::per_minute(
            NonZeroU32::new(config.rate_limit_per_minute.max(1))
                .unwrap_or(NonZeroU32::new(20).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            cli_process: Mutex::new(None),
            message_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn extract_attachment_paths(metadata: &serde_json::Value) -> Vec<String> {
        let mut paths = Vec::new();

        if let Some(entries) = metadata
            .get("signal_attachment_paths")
            .and_then(|value| value.as_array())
        {
            for entry in entries {
                if let Some(path) = entry.as_str().filter(|value| !value.trim().is_empty()) {
                    paths.push(path.to_string());
                }
            }
        }

        if let Some(entries) = metadata.get("file_references").and_then(|value| value.as_array()) {
            for entry in entries {
                let maybe_path = entry
                    .get("local_path")
                    .or_else(|| entry.get("path"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty());
                if let Some(path) = maybe_path {
                    paths.push(path.to_string());
                }
            }
        }

        let mut deduped = Vec::new();
        for path in paths {
            if !deduped.contains(&path) {
                deduped.push(path);
            }
        }
        deduped
    }

    fn validate_attachment_paths(paths: &[String]) -> Result<()> {
        for path in paths {
            if !Path::new(path).exists() {
                return Err(ChannelError::InvalidFormat {
                    platform: "signal".to_string(),
                    message: format!("Signal attachment path does not exist: {}", path),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Check if a phone number or UUID is in the allowlist.
    fn is_allowed(&self, identifier: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&identifier.to_string())
    }

    /// Check if a group is in the allowed groups list.
    fn is_group_allowed(&self, group_id: &str) -> bool {
        if self.config.allowed_groups.is_empty() {
            return true;
        }
        self.config.allowed_groups.contains(&group_id.to_string())
    }

    /// Start signal-cli daemon in JSON-RPC mode.
    async fn start_cli_daemon(&self) -> Result<()> {
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let mut cmd = Command::new(&cli_path);
        cmd.arg("--output")
            .arg("json")
            .arg("-a")
            .arg(&self.config.phone_number)
            .arg("daemon")
            .arg("--system")
            .env("SIGNAL_CLI_DATA_DIR", &self.config.data_dir);

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("Failed to start signal-cli: {}", e),
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ChannelError::Connection {
                platform: "signal".to_string(),
                message: "Failed to capture stdout".to_string(),
            })?;

        let mut reader = BufReader::new(stdout).lines();
        let tx = self.incoming_tx.clone();
        let allowlist = self.config.allowlist.clone();
        let allowed_groups = self.config.allowed_groups.clone();
        let require_allowlist = self.config.require_allowlist;
        let bot_phone_number = self.config.phone_number.clone();

        // Spawn task to process JSON output
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                debug!(line = %line, "Received Signal envelope");

                if let Ok(envelope) = serde_json::from_str::<SignalEnvelope>(&line) {
                    if let Err(e) = Self::process_envelope(
                        envelope,
                        &tx,
                        &allowlist,
                        &allowed_groups,
                        require_allowlist,
                        &bot_phone_number,
                    )
                    .await
                    {
                        error!(error = %e, "Failed to process Signal envelope");
                    }
                } else {
                    debug!(line = %line, "Non-envelope output from signal-cli");
                }
            }

            info!("Signal CLI stdout reader closed");
        });

        // Store the child process
        let mut process_guard = self.cli_process.lock().await;
        *process_guard = Some(child);

        Ok(())
    }

    /// Process a Signal envelope and convert to IncomingMessage.
    async fn process_envelope(
        envelope: SignalEnvelope,
        tx: &mpsc::Sender<IncomingMessage>,
        allowlist: &[String],
        allowed_groups: &[String],
        require_allowlist: bool,
        bot_phone_number: &str,
    ) -> Result<()> {
        match envelope {
            SignalEnvelope::DataMessage {
                data_message,
                timestamp,
                source,
                source_number,
                source_uuid,
                group_info,
            } => {
                // Check allowlist
                let user_id = source_number.clone().unwrap_or_else(|| source.clone());
                if require_allowlist && !allowlist.is_empty() {
                    let is_allowed = allowlist.contains(&user_id)
                        || source_uuid
                            .as_ref()
                            .map(|u| allowlist.contains(u))
                            .unwrap_or(false);
                    if !is_allowed {
                        debug!(user = %user_id, "User not in allowlist, ignoring message");
                        return Ok(());
                    }
                }

                // Check group allowlist
                if let Some(ref group) = group_info {
                    if require_allowlist && !allowed_groups.is_empty() {
                        if !allowed_groups.contains(&group.group_id) {
                            debug!(group_id = %group.group_id, "Group not in allowlist, ignoring message");
                            return Ok(());
                        }
                    }
                }

                // Process text message
                if let Some(text) = data_message.message {
                    let is_group = group_info.is_some();
                    let recipient = source_number.clone().unwrap_or_else(|| source.clone());
                    let mut metadata = serde_json::json!({
                        "signal_timestamp": timestamp,
                        "signal_source": source,
                        "signal_source_number": source_number,
                        "signal_source_uuid": source_uuid,
                        "signal_recipient": recipient,
                        "signal_is_group": is_group,
                    });

                    // Add group info to metadata
                    if let Some(ref group) = group_info {
                        metadata["signal_group_id"] = serde_json::json!(&group.group_id);
                        metadata["signal_group_name"] = serde_json::json!(&group.group_name);
                    }

                    // Add attachments info
                    if !data_message.attachments.is_empty() {
                        metadata["attachments"] = serde_json::to_value(&data_message.attachments)
                            .map_err(|e| ChannelError::InvalidFormat {
                            platform: "signal".to_string(),
                            message: e.to_string(),
                        })?;
                        metadata["signal_attachment_count"] =
                            serde_json::json!(data_message.attachments.len());
                        let file_references: Vec<serde_json::Value> = data_message
                            .attachments
                            .iter()
                            .filter_map(|attachment| {
                                if attachment.id.is_empty() {
                                    None
                                } else {
                                    Some(serde_json::json!({
                                        "url": format!("signal-attachment://{}", attachment.id),
                                        "name": attachment.filename,
                                        "mime": attachment.content_type,
                                        "size": attachment.size,
                                        "width": attachment.width,
                                        "height": attachment.height,
                                        "caption": attachment.caption,
                                        "upload_timestamp": attachment.upload_timestamp,
                                    }))
                                }
                            })
                            .collect();
                        if !file_references.is_empty() {
                            metadata["file_references"] = serde_json::json!(file_references);
                        }
                    }

                    // Add quote info
                    if let Some(ref quote) = data_message.quote {
                        metadata["quote"] = serde_json::json!({
                            "id": quote.id,
                            "author": quote.author,
                            "text": quote.text,
                        });
                        metadata["signal_quote"] = serde_json::json!({
                            "id": quote.id,
                            "author": quote.author,
                            "text": quote.text,
                        });
                    }

                    // Check if message is a mention of the bot
                    let is_mention = data_message.mentions.iter().any(|mention| {
                        mention
                            .number
                            .as_ref()
                            .map(|number| number == bot_phone_number)
                            .unwrap_or(false)
                            || mention
                                .uuid
                                .as_ref()
                                .map(|uuid| uuid == bot_phone_number)
                                .unwrap_or(false)
                    });
                    metadata["is_mention"] = serde_json::json!(is_mention);
                    metadata["signal_bot_mentioned"] = serde_json::json!(is_mention);

                    let msg = IncomingMessage {
                        session_id: Uuid::new_v4(),
                        user_id: user_id.clone(),
                        content: text,
                        platform: Platform::Signal,
                        metadata,
                    };

                    if let Err(e) = tx.send(msg).await {
                        error!(error = %e, "Failed to send message to channel");
                        return Err(ChannelError::Connection {
                            platform: "signal".to_string(),
                            message: "Message channel closed".to_string(),
                        }
                        .into());
                    }
                }

                // Process attachments even without text
                for attachment in &data_message.attachments {
                    debug!(
                        filename = ?attachment.filename,
                        content_type = %attachment.content_type,
                        size = ?attachment.size,
                        "Processing attachment"
                    );
                }
            }
            SignalEnvelope::SyncMessage { sync_message } => {
                // Handle sync messages from linked devices
                if let Some(sent) = sync_message.sent_message {
                    debug!(timestamp = sent.timestamp, "Received sync message");
                }
            }
            SignalEnvelope::Receipt { timestamp, source } => {
                debug!(timestamp = timestamp, source = %source, "Received receipt");
            }
            SignalEnvelope::Other => {
                // Ignore other message types
            }
        }

        Ok(())
    }

    async fn send_command(
        &self,
        recipient: Option<&str>,
        group_id: Option<&str>,
        content: &str,
        attachments: &[String],
    ) -> Result<()> {
        Self::validate_attachment_paths(attachments)?;
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let mut command = Command::new(&cli_path);
        command
            .arg("-a")
            .arg(&self.config.phone_number)
            .arg("send");

        if let Some(group_id) = group_id {
            command.arg("-g").arg(group_id);
        }

        command.arg("-m").arg(content);

        for attachment in attachments {
            command.arg("-a").arg(attachment);
        }

        if let Some(recipient) = recipient {
            command.arg(recipient);
        }

        let output = command.output().await.map_err(|e| ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("Failed to execute signal-cli: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::SendFailed {
                platform: "signal".to_string(),
                message: format!("signal-cli send failed: {}", stderr),
            }
            .into());
        }

        Ok(())
    }

    /// Send a message via signal-cli.
    async fn send_via_cli(&self, recipient: &str, content: &str, attachments: &[String]) -> Result<()> {
        self.send_command(Some(recipient), None, content, attachments)
            .await
    }

    /// Send a message to a group via signal-cli.
    async fn send_to_group_via_cli(
        &self,
        group_id: &str,
        content: &str,
        attachments: &[String],
    ) -> Result<()> {
        self.send_command(None, Some(group_id), content, attachments)
            .await
    }

    /// Register the phone number with Signal (one-time setup).
    pub async fn register(&self, voice_verification: bool) -> Result<()> {
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let mut cmd = Command::new(&cli_path);
        cmd.arg("-a").arg(&self.config.phone_number).arg("register");

        if voice_verification {
            cmd.arg("--voice");
        }

        let output = cmd.output().await.map_err(|e| ChannelError::Connection {
            platform: "signal".to_string(),
            message: format!("Failed to execute signal-cli: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::Config {
                platform: "signal".to_string(),
                message: format!("signal-cli register failed: {}", stderr),
            }
            .into());
        }

        info!("Registration initiated. Check your phone/SMS for verification code.");
        Ok(())
    }

    /// Verify registration with code.
    pub async fn verify(&self, code: &str) -> Result<()> {
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let output = Command::new(&cli_path)
            .arg("-a")
            .arg(&self.config.phone_number)
            .arg("verify")
            .arg(code)
            .output()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("Failed to execute signal-cli: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::Config {
                platform: "signal".to_string(),
                message: format!("signal-cli verify failed: {}", stderr),
            }
            .into());
        }

        info!("Phone number verified successfully");
        Ok(())
    }

    /// Link as secondary device (scan QR code from primary device).
    pub async fn link_device(&self, device_name: &str) -> Result<String> {
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let output = Command::new(&cli_path)
            .arg("link")
            .arg("--name")
            .arg(device_name)
            .output()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("Failed to execute signal-cli: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("signal-cli link failed: {}", stderr),
            }
            .into());
        }

        let uri = String::from_utf8_lossy(&output.stdout);
        Ok(uri.trim().to_string())
    }

    /// List all groups.
    pub async fn list_groups(&self) -> Result<Vec<GroupInfo>> {
        let cli_path = self
            .config
            .signal_cli_path
            .clone()
            .unwrap_or_else(|| "signal-cli".into());

        let output = Command::new(&cli_path)
            .arg("-a")
            .arg(&self.config.phone_number)
            .arg("listGroups")
            .arg("--json")
            .output()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("Failed to execute signal-cli: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::Connection {
                platform: "signal".to_string(),
                message: format!("signal-cli listGroups failed: {}", stderr),
            }
            .into());
        }

        let groups: Vec<GroupInfo> =
            serde_json::from_slice(&output.stdout).map_err(|e| ChannelError::InvalidFormat {
                platform: "signal".to_string(),
                message: format!("Failed to parse groups: {}", e),
            })?;

        Ok(groups)
    }
}

#[async_trait]
impl Channel for SignalChannel {
    fn platform(&self) -> Platform {
        Platform::Signal
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;
        let attachments = Self::extract_attachment_paths(&msg.metadata);

        // Extract recipient from metadata
        let recipient = msg
            .metadata
            .get("signal_recipient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "signal".to_string(),
                message: "Missing signal_recipient in metadata".to_string(),
            })?;

        // Check if sending to a group
        if let Some(group_id) = msg.metadata.get("signal_group_id").and_then(|v| v.as_str()) {
            self.send_to_group_via_cli(group_id, &msg.content, &attachments)
                .await?;
        } else {
            self.send_via_cli(recipient, &msg.content, &attachments)
                .await?;
        }

        debug!(recipient = %recipient, "Sent Signal message");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "signal".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Signal...");

        // Validate configuration
        if self.config.phone_number.is_empty() {
            return Err(ChannelError::Config {
                platform: "signal".to_string(),
                message: "Signal phone number is required".to_string(),
            }
            .into());
        }

        // Start signal-cli daemon
        self.start_cli_daemon().await?;

        info!(phone = %self.config.phone_number, "Signal CLI daemon started");

        *self.is_connected.write().await = true;
        info!("Signal channel connected");

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Signal...");

        // Kill the CLI process if running
        let mut process_guard = self.cli_process.lock().await;
        if let Some(mut child) = process_guard.take() {
            let _ = child.kill().await;
            info!("Signal CLI daemon stopped");
        }

        *self.is_connected.write().await = false;
        info!("Signal channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::config::SignalConfig;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn create_test_config() -> SignalConfig {
        SignalConfig {
            enabled: true,
            phone_number: "+1234567890".to_string(),
            data_dir: PathBuf::from("./data/signal"),
            allowlist: vec!["+9876543210".to_string()],
            allowed_groups: vec![],
            signal_cli_path: None,
            use_libsignal: false,
            rate_limit_per_minute: 20,
            require_allowlist: true,
        }
    }

    #[test]
    fn test_signal_channel_new() {
        let config = create_test_config();
        let channel = SignalChannel::new(config);
        assert_eq!(channel.platform(), Platform::Signal);
    }

    #[test]
    fn test_is_allowed_empty_allowlist() {
        let mut config = create_test_config();
        config.allowlist = vec![];
        let channel = SignalChannel::new(config);
        assert!(channel.is_allowed("+1234567890"));
    }

    #[test]
    fn test_is_allowed_with_allowlist() {
        let config = create_test_config();
        let channel = SignalChannel::new(config);
        assert!(channel.is_allowed("+9876543210"));
        assert!(!channel.is_allowed("+1111111111"));
    }

    #[test]
    fn test_is_group_allowed_empty() {
        let config = create_test_config();
        let channel = SignalChannel::new(config);
        assert!(channel.is_group_allowed("group1"));
    }

    #[test]
    fn test_is_group_allowed_with_list() {
        let mut config = create_test_config();
        config.allowed_groups = vec!["group1".to_string(), "group2".to_string()];
        let channel = SignalChannel::new(config);
        assert!(channel.is_group_allowed("group1"));
        assert!(!channel.is_group_allowed("group3"));
    }

    #[test]
    fn test_signal_envelope_deserialization() {
        let json = r#"{
            "type": "dataMessage",
            "dataMessage": {
                "message": "Hello World",
                "attachments": [],
                "mentions": []
            },
            "timestamp": 1234567890000,
            "source": "+1234567890",
            "sourceNumber": "+1234567890"
        }"#;

        let envelope: SignalEnvelope = serde_json::from_str(json).unwrap();
        match envelope {
            SignalEnvelope::DataMessage { data_message, .. } => {
                assert_eq!(data_message.message, Some("Hello World".to_string()));
            }
            _ => panic!("Expected DataMessage"),
        }
    }

    #[test]
    fn test_quote_deserialization() {
        let json = r#"{
            "id": 12345,
            "author": "+9876543210",
            "text": "Original message"
        }"#;

        let quote: Quote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.id, 12345);
        assert_eq!(quote.author, "+9876543210");
        assert_eq!(quote.text, Some("Original message".to_string()));
    }

    #[test]
    fn test_attachment_deserialization() {
        let json = r#"{
            "content_type": "image/jpeg",
            "filename": "image.jpg",
            "size": 1024,
            "id": "abc123",
            "width": 640,
            "height": 480,
            "caption": "preview",
            "uploadTimestamp": 1234567890
        }"#;

        let attachment: SignalAttachment = serde_json::from_str(json).unwrap();
        assert_eq!(attachment.content_type, "image/jpeg");
        assert_eq!(attachment.filename, Some("image.jpg".to_string()));
        assert_eq!(attachment.size, Some(1024));
        assert_eq!(attachment.width, Some(640));
        assert_eq!(attachment.height, Some(480));
        assert_eq!(attachment.caption, Some("preview".to_string()));
        assert_eq!(attachment.upload_timestamp, Some(1234567890));
    }

    #[test]
    fn test_extract_attachment_paths_from_metadata() {
        let metadata = serde_json::json!({
            "signal_attachment_paths": ["/tmp/a.png"],
            "file_references": [
                {"local_path": "/tmp/b.pdf"},
                {"path": "/tmp/c.txt"},
                {"url": "https://example.com/ignored"}
            ]
        });

        let paths = SignalChannel::extract_attachment_paths(&metadata);
        assert_eq!(paths, vec!["/tmp/a.png", "/tmp/b.pdf", "/tmp/c.txt"]);
    }

    #[tokio::test]
    async fn test_send_via_cli_includes_attachment_args() {
        let temp_root = std::env::temp_dir().join(format!("orc-signal-send-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let script_path = temp_root.join("signal-cli");
        let args_path = temp_root.join("args.txt");
        let attachment_path = temp_root.join("report.pdf");
        fs::write(&attachment_path, b"attachment").unwrap();
        fs::write(
            &script_path,
            format!(
                "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" > \"{}\"\n",
                args_path.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();

        let mut config = create_test_config();
        config.signal_cli_path = Some(script_path.clone());
        let channel = SignalChannel::new(config);
        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "hello".to_string(),
                metadata: serde_json::json!({
                    "signal_recipient": "+15551234567",
                    "signal_attachment_paths": [attachment_path],
                }),
            })
            .await
            .unwrap();

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(args.contains("send"));
        assert!(args.contains("-m\nhello\n"));
        assert!(args.contains(&format!("-a\n{}\n", attachment_path.display())));
        assert!(args.contains("+15551234567"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[tokio::test]
    async fn test_send_to_group_via_cli_includes_attachment_args() {
        let temp_root =
            std::env::temp_dir().join(format!("orc-signal-group-send-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let script_path = temp_root.join("signal-cli");
        let args_path = temp_root.join("args.txt");
        let attachment_path = temp_root.join("photo.jpg");
        fs::write(&attachment_path, b"attachment").unwrap();
        fs::write(
            &script_path,
            format!(
                "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" > \"{}\"\n",
                args_path.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();

        let mut config = create_test_config();
        config.signal_cli_path = Some(script_path.clone());
        let channel = SignalChannel::new(config);
        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "group hello".to_string(),
                metadata: serde_json::json!({
                    "signal_recipient": "+15551234567",
                    "signal_group_id": "group-123",
                    "file_references": [{"local_path": attachment_path}],
                }),
            })
            .await
            .unwrap();

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(args.contains("-g\ngroup-123\n"));
        assert!(args.contains(&format!("-a\n{}\n", attachment_path.display())));
        assert!(!args.contains("+15551234567\n"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[tokio::test]
    async fn test_process_envelope_builds_structured_signal_file_references() {
        let (tx, mut rx) = mpsc::channel(4);
        SignalChannel::process_envelope(
            SignalEnvelope::DataMessage {
                data_message: DataMessage {
                    message: Some("photo".to_string()),
                    attachments: vec![SignalAttachment {
                        content_type: "image/jpeg".to_string(),
                        filename: Some("photo.jpg".to_string()),
                        id: "att-1".to_string(),
                        size: Some(2048),
                        width: Some(800),
                        height: Some(600),
                        caption: Some("holiday".to_string()),
                        upload_timestamp: Some(1111),
                    }],
                    quote: None,
                    mentions: vec![],
                    group_info: None,
                },
                timestamp: 123,
                source: "+15551234567".to_string(),
                source_number: Some("+15551234567".to_string()),
                source_uuid: None,
                group_info: None,
            },
            &tx,
            &[],
            &[],
            false,
            "+19998887777",
        )
        .await
        .unwrap();

        let incoming = rx.recv().await.expect("incoming message");
        assert_eq!(incoming.metadata["signal_attachment_count"], serde_json::json!(1));
        assert_eq!(
            incoming.metadata["file_references"][0]["url"],
            "signal-attachment://att-1"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["name"],
            "photo.jpg"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["caption"],
            "holiday"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["width"],
            serde_json::json!(800)
        );
    }
}
