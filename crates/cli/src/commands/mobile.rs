use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use openrustclaw_mobile::node::MobileMessage;
use openrustclaw_mobile::notifications::{
    Notification, NotificationConfig, NotificationPriority, NotificationType,
};
use openrustclaw_mobile::sync::{
    ConflictResolution, SyncConfig, SyncManager, SyncMode, SyncPriority,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_MOBILE_ROOT: &str = ".claw/mobile";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeManifest {
    pub version: u32,
    pub node: MobileNodeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeSpec {
    pub id: String,
    pub gateway_url: String,
    pub auth_token_env: String,
    pub device_name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mobile_platform")]
    pub platform: String,
    #[serde(default = "default_mobile_capabilities")]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub sync: MobileSyncSpec,
    #[serde(default)]
    pub notifications: MobileNotificationSpec,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncSpec {
    #[serde(default = "default_sync_mode")]
    pub mode: String,
    #[serde(default = "default_sync_priority")]
    pub priority: String,
    #[serde(default = "default_sync_conflict_resolution")]
    pub conflict_resolution: String,
    #[serde(default = "default_sync_interval_secs")]
    pub max_sync_interval_secs: u64,
    #[serde(default = "default_sync_min_battery_percent")]
    pub min_battery_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotificationSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub apns_enabled: bool,
    #[serde(default)]
    pub fcm_enabled: bool,
    #[serde(default = "default_true")]
    pub show_badge: bool,
    #[serde(default = "default_true")]
    pub play_sound: bool,
    #[serde(default)]
    pub sound_name: Option<String>,
    #[serde(default = "default_true")]
    pub vibration: bool,
    #[serde(default = "default_batch_interval_secs")]
    pub batch_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobilePairRequest {
    pub id: String,
    pub gateway_url: String,
    pub auth_token_env: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sync: Option<MobileSyncSpec>,
    #[serde(default)]
    pub notifications: Option<MobileNotificationSpec>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeStatus {
    pub id: String,
    pub enabled: bool,
    pub platform: String,
    pub gateway_url: String,
    pub device_name: String,
    pub capabilities: Vec<String>,
    pub auth_token_env: String,
    pub auth_token_present: bool,
    pub readiness: String,
    pub sync: MobileSyncSpec,
    pub notifications: MobileNotificationSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotificationPreviewRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileMessagePreviewRequest {
    pub source_node_id: String,
    pub target: String,
    pub content: String,
    #[serde(default)]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncPreviewRequest {
    pub node_id: String,
    #[serde(default = "default_sync_battery_percent")]
    pub battery_percent: u8,
    #[serde(default)]
    pub pending_change_count: usize,
}

fn default_true() -> bool {
    true
}

fn default_mobile_platform() -> String {
    "generic_mobile".to_string()
}

fn default_mobile_capabilities() -> Vec<String> {
    vec!["mobile".to_string()]
}

fn default_sync_mode() -> String {
    "automatic".to_string()
}

fn default_sync_priority() -> String {
    "normal".to_string()
}

fn default_sync_conflict_resolution() -> String {
    "last_write_wins".to_string()
}

fn default_sync_interval_secs() -> u64 {
    300
}

fn default_sync_min_battery_percent() -> u8 {
    20
}

fn default_batch_interval_secs() -> u64 {
    5
}

fn default_sync_battery_percent() -> u8 {
    50
}

impl Default for MobileSyncSpec {
    fn default() -> Self {
        Self {
            mode: default_sync_mode(),
            priority: default_sync_priority(),
            conflict_resolution: default_sync_conflict_resolution(),
            max_sync_interval_secs: default_sync_interval_secs(),
            min_battery_percent: default_sync_min_battery_percent(),
        }
    }
}

impl Default for MobileNotificationSpec {
    fn default() -> Self {
        let config = NotificationConfig::default();
        Self {
            enabled: config.enabled,
            apns_enabled: config.apns_enabled,
            fcm_enabled: config.fcm_enabled,
            show_badge: config.show_badge,
            play_sound: config.play_sound,
            sound_name: config.sound_name,
            vibration: config.vibration,
            batch_interval_secs: config.batch_interval_secs,
        }
    }
}

pub fn list_nodes_data(workspace_root: &Path) -> Result<Vec<MobileNodeManifest>> {
    let mut entries = Vec::new();
    let dir = nodes_dir(workspace_root);
    if !dir.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let manifest: MobileNodeManifest = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        entries.push(manifest);
    }

    entries.sort_by(|left, right| left.node.id.cmp(&right.node.id));
    Ok(entries)
}

pub fn inspect_node_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodeManifest> {
    let path = manifest_path(workspace_root, node_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn pair_node_data(
    workspace_root: &Path,
    request: MobilePairRequest,
) -> Result<MobileNodeManifest> {
    if request.id.trim().is_empty() {
        return Err(anyhow!("node id is required"));
    }
    if request.gateway_url.trim().is_empty() {
        return Err(anyhow!("gateway_url is required"));
    }
    if request.auth_token_env.trim().is_empty() {
        return Err(anyhow!("auth_token_env is required"));
    }

    fs::create_dir_all(nodes_dir(workspace_root))
        .with_context(|| format!("failed to create {}", nodes_dir(workspace_root).display()))?;
    let manifest = MobileNodeManifest {
        version: 1,
        node: MobileNodeSpec {
            id: request.id.clone(),
            gateway_url: request.gateway_url,
            auth_token_env: request.auth_token_env,
            device_name: request
                .device_name
                .unwrap_or_else(|| format!("{} mobile node", request.id)),
            enabled: request.enabled,
            platform: request.platform.unwrap_or_else(default_mobile_platform),
            capabilities: if request.capabilities.is_empty() {
                default_mobile_capabilities()
            } else {
                request.capabilities
            },
            sync: request.sync.unwrap_or_default(),
            notifications: request.notifications.unwrap_or_default(),
            metadata: request.metadata,
        },
    };

    let path = manifest_path(workspace_root, &request.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&manifest).context("failed to serialize mobile node manifest")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(manifest)
}

pub fn node_status_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodeStatus> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    let auth_token_present = std::env::var(&manifest.node.auth_token_env).is_ok();
    let readiness = if !manifest.node.enabled {
        "disabled"
    } else if auth_token_present {
        "ready_for_runtime"
    } else {
        "missing_auth_env"
    };

    Ok(MobileNodeStatus {
        id: manifest.node.id,
        enabled: manifest.node.enabled,
        platform: manifest.node.platform,
        gateway_url: manifest.node.gateway_url,
        device_name: manifest.node.device_name,
        capabilities: manifest.node.capabilities,
        auth_token_env: manifest.node.auth_token_env,
        auth_token_present,
        readiness: readiness.to_string(),
        sync: manifest.node.sync,
        notifications: manifest.node.notifications,
    })
}

pub fn preview_notification_data(
    request: MobileNotificationPreviewRequest,
) -> Result<Notification> {
    if request.title.trim().is_empty() || request.body.trim().is_empty() {
        return Err(anyhow!("title and body are required"));
    }
    let mut notification = Notification::new(request.title, request.body)
        .with_priority(parse_notification_priority(request.priority.as_deref())?)
        .with_type(parse_notification_type(
            request.notification_type.as_deref(),
        )?);
    for (key, value) in request.data {
        notification = notification.with_data(key, value);
    }
    Ok(notification)
}

pub fn preview_message_data(request: MobileMessagePreviewRequest) -> Result<Value> {
    if request.source_node_id.trim().is_empty() {
        return Err(anyhow!("source_node_id is required"));
    }
    if request.target.trim().is_empty() {
        return Err(anyhow!("target is required"));
    }
    let content_type = request
        .content_type
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "text/plain".to_string());
    let message = MobileMessage::new(
        &request.source_node_id,
        &request.target,
        request.content.clone().into_bytes(),
        &content_type,
    );
    Ok(json!({
        "id": message.id,
        "source": message.source,
        "target": message.target,
        "content_type": message.content_type,
        "content_preview": String::from_utf8_lossy(&message.content),
        "bytes": message.content.len(),
    }))
}

pub fn preview_sync_data(
    workspace_root: &Path,
    request: MobileSyncPreviewRequest,
) -> Result<Value> {
    let manifest = inspect_node_data(workspace_root, &request.node_id)?;
    let mut manager = SyncManager::new(sync_config_from_spec(&manifest.node.sync)?);
    for index in 0..request.pending_change_count {
        manager.queue_change(openrustclaw_mobile::sync::SyncChange::new(
            format!("pending-{index}"),
            "mobile_preview",
            openrustclaw_mobile::sync::SyncOperation::Update,
            json!({ "index": index }),
        ));
    }

    Ok(json!({
        "node_id": request.node_id,
        "battery_percent": request.battery_percent,
        "pending_change_count": manager.pending_count(),
        "should_sync": manager.should_sync(request.battery_percent),
        "sync_due": manager.is_sync_due(),
        "sync_mode": manifest.node.sync.mode,
        "priority": manifest.node.sync.priority,
        "conflict_resolution": manifest.node.sync.conflict_resolution,
        "max_sync_interval_secs": manifest.node.sync.max_sync_interval_secs,
        "min_battery_percent": manifest.node.sync.min_battery_percent,
    }))
}

pub async fn list_nodes(workspace_root: &Path) -> Result<()> {
    let entries = list_nodes_data(workspace_root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "nodes": entries }))?
    );
    Ok(())
}

pub async fn pair_node(workspace_root: &Path, request: MobilePairRequest) -> Result<()> {
    let manifest = pair_node_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

pub async fn inspect_node(workspace_root: &Path, node_id: &str) -> Result<()> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

pub async fn node_status(workspace_root: &Path, node_id: &str) -> Result<()> {
    let status = node_status_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

pub async fn preview_notification(request: MobileNotificationPreviewRequest) -> Result<()> {
    let preview = preview_notification_data(request)?;
    println!("{}", serde_json::to_string_pretty(&preview)?);
    Ok(())
}

pub async fn preview_message(request: MobileMessagePreviewRequest) -> Result<()> {
    let preview = preview_message_data(request)?;
    println!("{}", serde_json::to_string_pretty(&preview)?);
    Ok(())
}

pub async fn preview_sync(workspace_root: &Path, request: MobileSyncPreviewRequest) -> Result<()> {
    let preview = preview_sync_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&preview)?);
    Ok(())
}

fn mobile_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(DEFAULT_MOBILE_ROOT)
}

fn nodes_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("nodes")
}

fn manifest_path(workspace_root: &Path, node_id: &str) -> PathBuf {
    nodes_dir(workspace_root).join(format!("{node_id}.json"))
}

fn parse_notification_priority(raw: Option<&str>) -> Result<NotificationPriority> {
    Ok(
        match raw.unwrap_or("normal").trim().to_ascii_lowercase().as_str() {
            "low" => NotificationPriority::Low,
            "normal" => NotificationPriority::Normal,
            "high" => NotificationPriority::High,
            "critical" => NotificationPriority::Critical,
            other => return Err(anyhow!("unsupported notification priority '{other}'")),
        },
    )
}

fn parse_notification_type(raw: Option<&str>) -> Result<NotificationType> {
    Ok(
        match raw
            .unwrap_or("message")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "message" => NotificationType::Message,
            "alert" => NotificationType::Alert,
            "update" => NotificationType::Update,
            "system" => NotificationType::System,
            "custom" => NotificationType::Custom,
            other => return Err(anyhow!("unsupported notification type '{other}'")),
        },
    )
}

fn sync_config_from_spec(spec: &MobileSyncSpec) -> Result<SyncConfig> {
    Ok(SyncConfig {
        sync_mode: match spec.mode.trim().to_ascii_lowercase().as_str() {
            "realtime" => SyncMode::Realtime,
            "automatic" => SyncMode::Automatic,
            "manual" => SyncMode::Manual,
            "scheduled" => SyncMode::Scheduled,
            other => return Err(anyhow!("unsupported sync mode '{other}'")),
        },
        priority: match spec.priority.trim().to_ascii_lowercase().as_str() {
            "low" => SyncPriority::Low,
            "normal" => SyncPriority::Normal,
            "high" => SyncPriority::High,
            "critical" => SyncPriority::Critical,
            other => return Err(anyhow!("unsupported sync priority '{other}'")),
        },
        conflict_resolution: match spec
            .conflict_resolution
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "last_write_wins" => ConflictResolution::LastWriteWins,
            "first_write_wins" => ConflictResolution::FirstWriteWins,
            "server_wins" => ConflictResolution::ServerWins,
            "client_wins" => ConflictResolution::ClientWins,
            "manual" => ConflictResolution::Manual,
            other => return Err(anyhow!("unsupported conflict resolution '{other}'")),
        },
        max_sync_interval_secs: spec.max_sync_interval_secs,
        min_battery_percent: spec.min_battery_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pair_and_list_mobile_nodes_round_trip() {
        let temp = tempdir().expect("tempdir");
        let manifest = pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-1".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_TOKEN".to_string(),
                device_name: Some("Studio iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "camera".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: json!({"team":"ops"}),
            },
        )
        .expect("pair node");

        assert_eq!(manifest.node.id, "iphone-1");
        let listed = list_nodes_data(temp.path()).expect("list nodes");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].node.platform, "ios");
    }

    #[test]
    fn mobile_status_reports_missing_auth_env() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "android-1".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MISSING_MOBILE_TOKEN".to_string(),
                device_name: None,
                platform: Some("android".to_string()),
                capabilities: vec![],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let status = node_status_data(temp.path(), "android-1").expect("status");
        assert!(!status.auth_token_present);
        assert_eq!(status.readiness, "missing_auth_env");
    }

    #[test]
    fn mobile_message_preview_returns_envelope_summary() {
        let preview = preview_message_data(MobileMessagePreviewRequest {
            source_node_id: "iphone-1".to_string(),
            target: "main-gateway".to_string(),
            content: "hello".to_string(),
            content_type: None,
        })
        .expect("preview message");

        assert_eq!(preview["source"], "iphone-1");
        assert_eq!(preview["target"], "main-gateway");
        assert_eq!(preview["content_type"], "text/plain");
        assert_eq!(preview["content_preview"], "hello");
    }
}
