use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use openrustclaw_mobile::node::MobileMessage;
use openrustclaw_mobile::notifications::{
    Notification, NotificationConfig, NotificationPriority, NotificationType,
};
pub use openrustclaw_mobile::protocol::{
    DeviceCommandKind, MobileCommandDecisionRequest, MobileCommandDispatchRequest,
    MobileCommandRecord,
};
use openrustclaw_mobile::sync::{
    ConflictResolution, SyncConfig, SyncManager, SyncMode, SyncPriority,
};
use openrustclaw_mobile::{MobileNodeHandle, NodeConfig};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeRuntimeState {
    pub node_id: String,
    pub runtime_status: String,
    #[serde(default)]
    pub app_state: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub reachable: bool,
    #[serde(default)]
    pub push_token_present: bool,
    #[serde(default)]
    pub battery_percent: Option<u8>,
    #[serde(default)]
    pub last_heartbeat_at: Option<String>,
    #[serde(default)]
    pub wake_state: String,
    #[serde(default)]
    pub wake_requested_at: Option<String>,
    #[serde(default)]
    pub wake_requested_by: Option<String>,
    #[serde(default)]
    pub wake_reason: Option<String>,
    #[serde(default)]
    pub last_wake_command_id: Option<String>,
    #[serde(default)]
    pub rehydrate_state: String,
    #[serde(default)]
    pub rehydrate_requested_at: Option<String>,
    #[serde(default)]
    pub rehydrate_requested_by: Option<String>,
    #[serde(default)]
    pub rehydrate_reason: Option<String>,
    #[serde(default)]
    pub rehydrate_pending_change_count: Option<usize>,
    #[serde(default)]
    pub last_rehydrate_command_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileHeartbeatRequest {
    #[serde(default)]
    pub app_state: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub reachable: Option<bool>,
    #[serde(default)]
    pub push_token_present: Option<bool>,
    #[serde(default)]
    pub battery_percent: Option<u8>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileWakeRequest {
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileRehydrateRequest {
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub pending_change_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeRuntimeActionResult {
    pub node_id: String,
    pub runtime: MobileNodeRuntimeState,
    #[serde(default)]
    pub command: Option<MobileCommandRecord>,
    #[serde(default)]
    pub preview: Value,
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

fn default_runtime_status() -> String {
    "registered".to_string()
}

fn default_app_state() -> String {
    "unknown".to_string()
}

fn default_network_state() -> String {
    "unknown".to_string()
}

fn default_wake_state() -> String {
    "idle".to_string()
}

fn default_rehydrate_state() -> String {
    "idle".to_string()
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

impl MobileNodeRuntimeState {
    fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            runtime_status: default_runtime_status(),
            app_state: default_app_state(),
            network: default_network_state(),
            reachable: false,
            push_token_present: false,
            battery_percent: None,
            last_heartbeat_at: None,
            wake_state: default_wake_state(),
            wake_requested_at: None,
            wake_requested_by: None,
            wake_reason: None,
            last_wake_command_id: None,
            rehydrate_state: default_rehydrate_state(),
            rehydrate_requested_at: None,
            rehydrate_requested_by: None,
            rehydrate_reason: None,
            rehydrate_pending_change_count: None,
            last_rehydrate_command_id: None,
            metadata: Value::Null,
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

pub fn node_runtime_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodeRuntimeState> {
    inspect_node_data(workspace_root, node_id)?;
    load_runtime_state(workspace_root, node_id)
}

pub fn heartbeat_node_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobileHeartbeatRequest,
) -> Result<MobileNodeRuntimeState> {
    inspect_node_data(workspace_root, node_id)?;
    let mut runtime = load_runtime_state(workspace_root, node_id)?;
    let now = Utc::now().to_rfc3339();
    if let Some(app_state) = request.app_state.filter(|value| !value.trim().is_empty()) {
        runtime.app_state = app_state;
    }
    if let Some(network) = request.network.filter(|value| !value.trim().is_empty()) {
        runtime.network = network;
    }
    if let Some(reachable) = request.reachable {
        runtime.reachable = reachable;
    }
    if let Some(push_token_present) = request.push_token_present {
        runtime.push_token_present = push_token_present;
    }
    if let Some(battery_percent) = request.battery_percent {
        runtime.battery_percent = Some(battery_percent.min(100));
    }
    if !request.metadata.is_null() {
        runtime.metadata = request.metadata;
    }
    runtime.last_heartbeat_at = Some(now);
    if runtime.reachable {
        if matches!(runtime.wake_state.as_str(), "requested" | "dispatched") {
            runtime.wake_state = "acknowledged".to_string();
        }
        if matches!(runtime.rehydrate_state.as_str(), "requested") {
            runtime.rehydrate_state = "ready".to_string();
        }
    }
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    Ok(runtime)
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

pub fn list_command_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileCommandRecord>> {
    let mut entries = Vec::new();
    let dir = commands_dir(workspace_root);
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
        let record: MobileCommandRecord = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if let Some(node_id) = node_id
            && record.node_id != node_id
        {
            continue;
        }
        entries.push(record);
    }

    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    Ok(entries)
}

pub fn inspect_command_data(
    workspace_root: &Path,
    command_id: &str,
) -> Result<MobileCommandRecord> {
    let path = command_path(workspace_root, command_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn dispatch_command_data(
    workspace_root: &Path,
    request: MobileCommandDispatchRequest,
) -> Result<MobileCommandRecord> {
    let manifest = inspect_node_data(workspace_root, &request.node_id)?;
    let status = node_status_data(workspace_root, &request.node_id)?;
    let capability = request.command.required_capability();
    if !manifest
        .node
        .capabilities
        .iter()
        .any(|entry| entry == capability)
    {
        return Err(anyhow!(
            "node '{}' does not advertise required capability '{}'",
            manifest.node.id,
            capability
        ));
    }

    let approval_required = request
        .require_approval
        .unwrap_or_else(|| request.command.default_requires_approval());
    let mut record = MobileCommandRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: manifest.node.id.clone(),
        command: request.command.clone(),
        required_capability: capability.to_string(),
        approval_required,
        status: if approval_required && request.approved_by.is_none() {
            "pending_approval".to_string()
        } else {
            "approved".to_string()
        },
        payload: request.payload,
        result: Value::Null,
        created_at: Utc::now(),
        approved_by: request.approved_by.clone(),
        decided_reason: None,
        approved_at: request.approved_by.as_ref().map(|_| Utc::now()),
        executed_at: None,
    };

    if record.status == "approved" {
        record.result = execute_command_for_node(&manifest, &status, &record).await?;
        record.status = "executed".to_string();
        record.executed_at = Some(Utc::now());
    }

    write_command_record(workspace_root, &record)?;
    Ok(record)
}

pub async fn wake_node_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobileWakeRequest,
) -> Result<MobileNodeRuntimeActionResult> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    let status = node_status_data(workspace_root, node_id)?;
    let mut runtime = load_runtime_state(workspace_root, node_id)?;
    let now = Utc::now().to_rfc3339();
    runtime.wake_requested_at = Some(now);
    runtime.wake_requested_by = request.requested_by.clone();
    runtime.wake_reason = request.reason.clone();
    runtime.wake_state = "requested".to_string();

    let mut command = None;
    if status.readiness == "ready_for_runtime"
        && manifest
            .node
            .capabilities
            .iter()
            .any(|entry| entry == "notifications")
    {
        let record = dispatch_command_data(
            workspace_root,
            MobileCommandDispatchRequest {
                node_id: node_id.to_string(),
                command: DeviceCommandKind::PushNotification,
                payload: json!({
                    "title": request.title.unwrap_or_else(|| format!("Wake {}", manifest.node.device_name)),
                    "body": request.body.unwrap_or_else(|| "Operator requested a bounded wake ping".to_string()),
                    "type": "system",
                    "data": {
                        "node_id": node_id,
                        "reason": request.reason.clone().unwrap_or_else(|| "operator_request".to_string()),
                    }
                }),
                approved_by: request.requested_by.clone().or_else(|| Some("mobile_runtime".to_string())),
                require_approval: Some(false),
            },
        )
        .await?;
        runtime.wake_state = "dispatched".to_string();
        runtime.last_wake_command_id = Some(record.id.clone());
        command = Some(record);
    }

    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    Ok(MobileNodeRuntimeActionResult {
        node_id: node_id.to_string(),
        runtime,
        command,
        preview: json!({
            "requested": true,
            "ready_for_runtime": status.readiness == "ready_for_runtime",
            "notifications_capability": manifest.node.capabilities.iter().any(|entry| entry == "notifications"),
        }),
    })
}

pub async fn rehydrate_node_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobileRehydrateRequest,
) -> Result<MobileNodeRuntimeActionResult> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    let status = node_status_data(workspace_root, node_id)?;
    let mut runtime = load_runtime_state(workspace_root, node_id)?;
    let pending_change_count = request.pending_change_count.unwrap_or(1);
    let preview = preview_sync_data(
        workspace_root,
        MobileSyncPreviewRequest {
            node_id: node_id.to_string(),
            battery_percent: runtime
                .battery_percent
                .unwrap_or(default_sync_battery_percent()),
            pending_change_count,
        },
    )?;
    let now = Utc::now().to_rfc3339();
    runtime.rehydrate_requested_at = Some(now);
    runtime.rehydrate_requested_by = request.requested_by.clone();
    runtime.rehydrate_reason = request.reason.clone();
    runtime.rehydrate_pending_change_count = Some(pending_change_count);
    runtime.rehydrate_state = "requested".to_string();

    let mut command = None;
    if status.readiness == "ready_for_runtime"
        && manifest
            .node
            .capabilities
            .iter()
            .any(|entry| entry == "mobile")
    {
        let record = dispatch_command_data(
            workspace_root,
            MobileCommandDispatchRequest {
                node_id: node_id.to_string(),
                command: DeviceCommandKind::SyncNow,
                payload: json!({ "pending_change_count": pending_change_count }),
                approved_by: request
                    .requested_by
                    .clone()
                    .or_else(|| Some("mobile_runtime".to_string())),
                require_approval: Some(false),
            },
        )
        .await?;
        runtime.rehydrate_state = "synced".to_string();
        runtime.last_rehydrate_command_id = Some(record.id.clone());
        command = Some(record);
    }

    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    Ok(MobileNodeRuntimeActionResult {
        node_id: node_id.to_string(),
        runtime,
        command,
        preview,
    })
}

pub async fn approve_command_data(
    workspace_root: &Path,
    command_id: &str,
    request: MobileCommandDecisionRequest,
) -> Result<MobileCommandRecord> {
    let mut record = inspect_command_data(workspace_root, command_id)?;
    if record.status != "pending_approval" {
        return Err(anyhow!("command '{}' is not pending approval", record.id));
    }
    let manifest = inspect_node_data(workspace_root, &record.node_id)?;
    let status = node_status_data(workspace_root, &record.node_id)?;

    record.status = "approved".to_string();
    record.approved_by = Some(request.decided_by);
    record.decided_reason = request.reason;
    record.approved_at = Some(Utc::now());
    record.result = execute_command_for_node(&manifest, &status, &record).await?;
    record.status = "executed".to_string();
    record.executed_at = Some(Utc::now());

    write_command_record(workspace_root, &record)?;
    Ok(record)
}

pub fn reject_command_data(
    workspace_root: &Path,
    command_id: &str,
    request: MobileCommandDecisionRequest,
) -> Result<MobileCommandRecord> {
    let mut record = inspect_command_data(workspace_root, command_id)?;
    if record.status != "pending_approval" {
        return Err(anyhow!("command '{}' is not pending approval", record.id));
    }

    record.status = "rejected".to_string();
    record.approved_by = Some(request.decided_by);
    record.decided_reason = request.reason;
    record.approved_at = Some(Utc::now());
    write_command_record(workspace_root, &record)?;
    Ok(record)
}

fn write_command_record(workspace_root: &Path, record: &MobileCommandRecord) -> Result<()> {
    fs::create_dir_all(commands_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            commands_dir(workspace_root).display()
        )
    })?;
    let path = command_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record).context("failed to serialize mobile command record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

async fn execute_command_for_node(
    manifest: &MobileNodeManifest,
    status: &MobileNodeStatus,
    record: &MobileCommandRecord,
) -> Result<Value> {
    if status.readiness != "ready_for_runtime" {
        return Err(anyhow!(
            "node '{}' is not ready for runtime execution ({})",
            manifest.node.id,
            status.readiness
        ));
    }

    match &record.command {
        DeviceCommandKind::SendMessage => {
            execute_send_message_command(manifest, &record.payload).await
        }
        DeviceCommandKind::PushNotification => {
            execute_push_notification_command(manifest, &record.payload)
        }
        DeviceCommandKind::SyncNow => execute_sync_now_command(manifest, &record.payload).await,
    }
}

async fn execute_send_message_command(
    manifest: &MobileNodeManifest,
    payload: &Value,
) -> Result<Value> {
    let target = payload
        .get("target")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("send_message requires payload.target"))?;
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("send_message requires payload.content"))?;
    let auth_token = std::env::var(&manifest.node.auth_token_env).with_context(|| {
        format!(
            "{} environment variable not set",
            manifest.node.auth_token_env
        )
    })?;
    let config = NodeConfig::new(
        manifest.node.id.clone(),
        manifest.node.gateway_url.clone(),
        auth_token,
    )
    .with_device_name(manifest.node.device_name.clone())
    .with_capabilities(manifest.node.capabilities.clone())
    .with_sync_config(sync_config_from_spec(&manifest.node.sync)?);
    let target = target.to_string();
    let content = content.to_string();
    let target_for_send = target.clone();
    let content_for_send = content.clone();

    let message_id = tokio::task::spawn_blocking(move || -> Result<String> {
        let handle = MobileNodeHandle::new(config).context("failed to initialize mobile node")?;
        handle.start().context("failed to start mobile node")?;
        let result = handle
            .send_message(&target_for_send, &content_for_send)
            .context("failed to send mobile message")?;
        handle.stop();
        Ok(result)
    })
    .await
    .context("failed to join mobile message task")??;

    Ok(json!({
        "transport": "mobile_node_handle",
        "target": target,
        "content_length": content.chars().count(),
        "message_id": message_id,
    }))
}

fn execute_push_notification_command(
    manifest: &MobileNodeManifest,
    payload: &Value,
) -> Result<Value> {
    let preview = preview_notification_data(MobileNotificationPreviewRequest {
        title: payload
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        body: payload
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        priority: payload
            .get("priority")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        notification_type: payload
            .get("type")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        data: payload
            .get("data")
            .and_then(Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })?;
    Ok(json!({
        "transport": "notification_preview",
        "node_id": manifest.node.id,
        "notification": preview,
    }))
}

async fn execute_sync_now_command(manifest: &MobileNodeManifest, payload: &Value) -> Result<Value> {
    let mut manager = SyncManager::new(sync_config_from_spec(&manifest.node.sync)?);
    let pending_change_count = payload
        .get("pending_change_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    for index in 0..pending_change_count {
        manager.queue_change(openrustclaw_mobile::sync::SyncChange::new(
            format!("queued-{index}"),
            "mobile_command",
            openrustclaw_mobile::sync::SyncOperation::Update,
            json!({ "index": index }),
        ));
    }
    let result = manager
        .sync()
        .await
        .context("failed to execute sync manager")?;
    Ok(json!({
        "transport": "sync_manager",
        "node_id": manifest.node.id,
        "pending_change_count": pending_change_count,
        "sync_result": result,
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

pub async fn node_runtime(workspace_root: &Path, node_id: &str) -> Result<()> {
    let runtime = node_runtime_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&runtime)?);
    Ok(())
}

pub async fn heartbeat_node(
    workspace_root: &Path,
    node_id: &str,
    request: MobileHeartbeatRequest,
) -> Result<()> {
    let runtime = heartbeat_node_data(workspace_root, node_id, request)?;
    println!("{}", serde_json::to_string_pretty(&runtime)?);
    Ok(())
}

pub async fn wake_node(
    workspace_root: &Path,
    node_id: &str,
    request: MobileWakeRequest,
) -> Result<()> {
    let result = wake_node_data(workspace_root, node_id, request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn rehydrate_node(
    workspace_root: &Path,
    node_id: &str,
    request: MobileRehydrateRequest,
) -> Result<()> {
    let result = rehydrate_node_data(workspace_root, node_id, request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
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

pub async fn list_commands(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let commands = list_command_data(workspace_root, node_id, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "commands": commands }))?
    );
    Ok(())
}

pub async fn inspect_command(workspace_root: &Path, command_id: &str) -> Result<()> {
    let command = inspect_command_data(workspace_root, command_id)?;
    println!("{}", serde_json::to_string_pretty(&command)?);
    Ok(())
}

pub async fn dispatch_command(
    workspace_root: &Path,
    request: MobileCommandDispatchRequest,
) -> Result<()> {
    let command = dispatch_command_data(workspace_root, request).await?;
    println!("{}", serde_json::to_string_pretty(&command)?);
    Ok(())
}

pub async fn approve_command(
    workspace_root: &Path,
    command_id: &str,
    request: MobileCommandDecisionRequest,
) -> Result<()> {
    let command = approve_command_data(workspace_root, command_id, request).await?;
    println!("{}", serde_json::to_string_pretty(&command)?);
    Ok(())
}

pub async fn reject_command(
    workspace_root: &Path,
    command_id: &str,
    request: MobileCommandDecisionRequest,
) -> Result<()> {
    let command = reject_command_data(workspace_root, command_id, request)?;
    println!("{}", serde_json::to_string_pretty(&command)?);
    Ok(())
}

fn mobile_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(DEFAULT_MOBILE_ROOT)
}

fn nodes_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("nodes")
}

fn commands_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("commands")
}

fn runtime_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("runtime")
}

fn manifest_path(workspace_root: &Path, node_id: &str) -> PathBuf {
    nodes_dir(workspace_root).join(format!("{node_id}.json"))
}

fn command_path(workspace_root: &Path, command_id: &str) -> PathBuf {
    commands_dir(workspace_root).join(format!("{command_id}.json"))
}

fn runtime_path(workspace_root: &Path, node_id: &str) -> PathBuf {
    runtime_dir(workspace_root).join(format!("{node_id}.json"))
}

fn load_runtime_state(workspace_root: &Path, node_id: &str) -> Result<MobileNodeRuntimeState> {
    let path = runtime_path(workspace_root, node_id);
    if !path.exists() {
        return Ok(MobileNodeRuntimeState::new(node_id));
    }
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut runtime: MobileNodeRuntimeState = serde_json::from_str(&bytes)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    refresh_runtime_status(&mut runtime);
    Ok(runtime)
}

fn save_runtime_state(workspace_root: &Path, runtime: &MobileNodeRuntimeState) -> Result<()> {
    fs::create_dir_all(runtime_dir(workspace_root))
        .with_context(|| format!("failed to create {}", runtime_dir(workspace_root).display()))?;
    let path = runtime_path(workspace_root, &runtime.node_id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(runtime).context("failed to serialize mobile runtime state")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn refresh_runtime_status(runtime: &mut MobileNodeRuntimeState) {
    runtime.runtime_status = if !runtime.reachable && runtime.last_heartbeat_at.is_some() {
        "disconnected".to_string()
    } else if runtime.rehydrate_state == "requested" {
        "rehydrate_requested".to_string()
    } else if runtime.rehydrate_state == "synced" {
        "rehydrated".to_string()
    } else if matches!(runtime.wake_state.as_str(), "requested" | "dispatched") {
        "wake_requested".to_string()
    } else if runtime.reachable && runtime.app_state == "active" {
        "active".to_string()
    } else if runtime.reachable && runtime.app_state == "background" {
        "background".to_string()
    } else if runtime.reachable {
        "reachable".to_string()
    } else {
        "registered".to_string()
    };
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

    #[tokio::test]
    async fn dispatch_command_requires_approval_by_default() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_TOKEN_APPROVAL", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-2".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_TOKEN_APPROVAL".to_string(),
                device_name: Some("Studio iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-2".to_string(),
                command: DeviceCommandKind::SendMessage,
                payload: json!({"target":"ops-room","content":"hello"}),
                approved_by: None,
                require_approval: None,
            },
        )
        .await
        .expect("dispatch command");

        assert_eq!(record.status, "pending_approval");
        unsafe {
            std::env::remove_var("MOBILE_TOKEN_APPROVAL");
        }
    }

    #[tokio::test]
    async fn approve_command_executes_mobile_message_lane() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_TOKEN_EXEC", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-3".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_TOKEN_EXEC".to_string(),
                device_name: Some("Studio iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let pending = dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-3".to_string(),
                command: DeviceCommandKind::SendMessage,
                payload: json!({"target":"ops-room","content":"hello"}),
                approved_by: None,
                require_approval: Some(true),
            },
        )
        .await
        .expect("dispatch command");

        let approved = approve_command_data(
            temp.path(),
            &pending.id,
            MobileCommandDecisionRequest {
                decided_by: "operator".to_string(),
                reason: Some("ship it".to_string()),
            },
        )
        .await
        .expect("approve command");

        assert_eq!(approved.status, "executed");
        assert_eq!(approved.result["transport"], "mobile_node_handle");
        assert!(approved.result["message_id"].as_str().is_some());
        unsafe {
            std::env::remove_var("MOBILE_TOKEN_EXEC");
        }
    }

    #[tokio::test]
    async fn dispatch_command_rejects_missing_capability() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_TOKEN_CAPS", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-4".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_TOKEN_CAPS".to_string(),
                device_name: Some("Studio iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let error = dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-4".to_string(),
                command: DeviceCommandKind::PushNotification,
                payload: json!({"title":"hello","body":"world"}),
                approved_by: Some("operator".to_string()),
                require_approval: Some(false),
            },
        )
        .await
        .expect_err("missing capability should fail");

        assert!(error.to_string().contains("required capability"));
        unsafe {
            std::env::remove_var("MOBILE_TOKEN_CAPS");
        }
    }

    #[test]
    fn mobile_runtime_heartbeat_round_trip() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-runtime".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_RUNTIME_TOKEN".to_string(),
                device_name: Some("Runtime iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let runtime = heartbeat_node_data(
            temp.path(),
            "iphone-runtime",
            MobileHeartbeatRequest {
                app_state: Some("active".to_string()),
                network: Some("wifi".to_string()),
                reachable: Some(true),
                push_token_present: Some(true),
                battery_percent: Some(84),
                metadata: json!({"build":"debug"}),
            },
        )
        .expect("heartbeat");

        assert_eq!(runtime.runtime_status, "active");
        assert_eq!(runtime.battery_percent, Some(84));
        assert!(runtime.last_heartbeat_at.is_some());
    }

    #[tokio::test]
    async fn rehydrate_node_dispatches_sync_when_ready() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_REHYDRATE_TOKEN", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-rehydrate".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_REHYDRATE_TOKEN".to_string(),
                device_name: Some("Runtime iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let result = rehydrate_node_data(
            temp.path(),
            "iphone-rehydrate",
            MobileRehydrateRequest {
                requested_by: Some("test".to_string()),
                reason: Some("resume".to_string()),
                pending_change_count: Some(2),
            },
        )
        .await
        .expect("rehydrate");

        assert_eq!(result.runtime.rehydrate_state, "synced");
        assert!(result.command.is_some());
    }
}
