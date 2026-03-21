use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
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
    pub notifications_authorized: bool,
    #[serde(default)]
    pub push_provider: Option<String>,
    #[serde(default)]
    pub push_token_updated_at: Option<String>,
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
    pub sync_state: String,
    #[serde(default)]
    pub pending_change_count: Option<usize>,
    #[serde(default)]
    pub last_sync_requested_at: Option<String>,
    #[serde(default)]
    pub last_sync_at: Option<String>,
    #[serde(default)]
    pub last_sync_result: Option<String>,
    #[serde(default)]
    pub last_notification_at: Option<String>,
    #[serde(default)]
    pub pending_notification_count: usize,
    #[serde(default)]
    pub delivered_notification_count: usize,
    #[serde(default)]
    pub last_inbound_message_at: Option<String>,
    #[serde(default)]
    pub pending_inbound_message_count: usize,
    #[serde(default)]
    pub acknowledged_inbound_message_count: usize,
    #[serde(default)]
    pub last_outbound_message_at: Option<String>,
    #[serde(default)]
    pub pending_outbound_message_count: usize,
    #[serde(default)]
    pub acknowledged_outbound_message_count: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileAppSessionRecord {
    pub id: String,
    pub node_id: String,
    pub status: String,
    pub started_at: String,
    pub last_seen_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub duration_secs: Option<u64>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub battery_percent: Option<u8>,
    #[serde(default)]
    pub entry_reason: Option<String>,
    #[serde(default)]
    pub exit_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodePushState {
    pub node_id: String,
    pub push_token_present: bool,
    pub notifications_authorized: bool,
    #[serde(default)]
    pub push_provider: Option<String>,
    #[serde(default)]
    pub push_token_updated_at: Option<String>,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobilePushRegistrationRequest {
    #[serde(default)]
    pub push_provider: Option<String>,
    #[serde(default)]
    pub push_token_present: Option<bool>,
    #[serde(default)]
    pub notifications_authorized: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeSyncState {
    pub node_id: String,
    pub sync_state: String,
    #[serde(default)]
    pub pending_change_count: Option<usize>,
    #[serde(default)]
    pub pending_conflict_count: usize,
    #[serde(default)]
    pub resolved_conflict_count: usize,
    #[serde(default)]
    pub last_sync_requested_at: Option<String>,
    #[serde(default)]
    pub last_sync_at: Option<String>,
    #[serde(default)]
    pub last_sync_result: Option<String>,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncReportRequest {
    #[serde(default)]
    pub sync_state: Option<String>,
    #[serde(default)]
    pub pending_change_count: Option<usize>,
    #[serde(default)]
    pub last_sync_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncConflictRecord {
    pub id: String,
    pub node_id: String,
    pub item_key: String,
    pub conflict_type: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub resolution_hint: Option<String>,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub resolved_by: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncConflictReportRequest {
    pub node_id: String,
    pub item_key: String,
    pub conflict_type: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub resolution_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncConflictResolveRequest {
    pub resolved_by: String,
    #[serde(default)]
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeCapabilityInfo {
    pub capability: String,
    pub advertised: bool,
    pub preview_supported: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeCapabilitiesResult {
    pub node_id: String,
    pub platform: String,
    pub readiness: String,
    pub capabilities: Vec<MobileNodeCapabilityInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCapabilityPreviewRequest {
    pub node_id: String,
    pub capability: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCapabilityPreviewRecord {
    pub id: String,
    pub node_id: String,
    pub capability: String,
    pub created_at: String,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCapabilityExecuteRequest {
    pub node_id: String,
    pub capability: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCapabilityExecutionRecord {
    pub id: String,
    pub node_id: String,
    pub capability: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub requested_by: Option<String>,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileMediaArtifactRecord {
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub capability: String,
    pub media_kind: String,
    pub mime: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub requested_by: Option<String>,
    pub summary: String,
    pub artifact: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeActivityEntry {
    pub kind: String,
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeActivityResult {
    pub node_id: String,
    pub entry_count: usize,
    pub entries: Vec<MobileNodeActivityEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileCommandMetricsSummary {
    pub total_commands: usize,
    pub pending_approval_commands: usize,
    pub approved_commands: usize,
    pub executed_commands: usize,
    pub rejected_commands: usize,
    pub by_command_kind: HashMap<String, usize>,
    #[serde(default)]
    pub avg_execution_latency_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileCommandMetricsResult {
    pub status: String,
    pub metrics: MobileCommandMetricsSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileCommandEvent {
    pub index: usize,
    pub kind: String,
    pub observed_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileCommandEventsResult {
    pub command_id: String,
    pub node_id: String,
    pub command: DeviceCommandKind,
    pub status: String,
    pub event_count: usize,
    pub events: Vec<MobileCommandEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileNodeSummary {
    pub node_id: String,
    pub runtime_status: String,
    pub app_state: String,
    pub network: String,
    pub reachable: bool,
    pub push_token_present: bool,
    pub notifications_authorized: bool,
    pub wake_state: String,
    pub rehydrate_state: String,
    pub sync_state: String,
    pub battery_percent: Option<u8>,
    pub pairings: usize,
    pub app_sessions: usize,
    pub sync_conflicts: usize,
    pub notifications: usize,
    pub inbox_messages: usize,
    pub outbox_messages: usize,
    pub commands: usize,
    pub capability_executions: usize,
    pub media_artifacts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileMetricsSummary {
    pub total_nodes: usize,
    pub paired_records: usize,
    pub unpaired_records: usize,
    pub reachable_nodes: usize,
    pub push_token_present_nodes: usize,
    pub notifications_authorized_nodes: usize,
    pub waking_nodes: usize,
    pub rehydrate_pending_nodes: usize,
    pub active_app_sessions: usize,
    pub pending_notifications: usize,
    pub delivered_notifications: usize,
    pub pending_inbound_messages: usize,
    pub acknowledged_inbound_messages: usize,
    pub pending_outbound_messages: usize,
    pub acknowledged_outbound_messages: usize,
    pub commands: usize,
    pub sync_conflicts: usize,
    pub capability_executions: usize,
    pub media_artifacts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileMetricsResult {
    pub status: String,
    pub metrics: MobileMetricsSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileNodeSummaryResult {
    pub status: String,
    pub summary: MobileNodeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobilePairingRecord {
    pub id: String,
    pub node_id: String,
    pub kind: String,
    pub status: String,
    pub created_at: String,
    pub device_name: String,
    pub platform: String,
    pub gateway_url: String,
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileUnpairRequest {
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default = "default_true")]
    pub remove_runtime_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeUnpairResult {
    pub node_id: String,
    pub removed_manifest: bool,
    pub removed_runtime_state: bool,
    pub pairing: MobilePairingRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotificationRecord {
    pub id: String,
    pub node_id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, String>,
    pub created_at: String,
    #[serde(default)]
    pub dispatched_at: Option<String>,
    #[serde(default)]
    pub acknowledged_at: Option<String>,
    #[serde(default)]
    pub acknowledged_by: Option<String>,
    #[serde(default)]
    pub command_id: Option<String>,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotificationSendRequest {
    pub node_id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotificationAckRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileInboundMessageRecord {
    pub id: String,
    pub node_id: String,
    pub source: String,
    pub target: String,
    pub status: String,
    #[serde(default)]
    pub content_type: Option<String>,
    pub content_preview: String,
    pub bytes: usize,
    pub created_at: String,
    #[serde(default)]
    pub reported_at: Option<String>,
    #[serde(default)]
    pub acknowledged_at: Option<String>,
    #[serde(default)]
    pub acknowledged_by: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileInboundMessageReportRequest {
    pub node_id: String,
    pub source: String,
    pub target: String,
    pub content: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileInboundMessageAckRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileOutboundMessageRecord {
    pub id: String,
    pub node_id: String,
    pub target: String,
    pub status: String,
    #[serde(default)]
    pub content_type: Option<String>,
    pub content_preview: String,
    pub bytes: usize,
    pub created_at: String,
    #[serde(default)]
    pub dispatched_at: Option<String>,
    #[serde(default)]
    pub acknowledged_at: Option<String>,
    #[serde(default)]
    pub acknowledged_by: Option<String>,
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileOutboundMessageSendRequest {
    pub node_id: String,
    pub target: String,
    pub content: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileOutboundMessageAckRequest {
    pub acknowledged_by: String,
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

fn default_sync_state() -> String {
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
            notifications_authorized: false,
            push_provider: None,
            push_token_updated_at: None,
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
            sync_state: default_sync_state(),
            pending_change_count: None,
            last_sync_requested_at: None,
            last_sync_at: None,
            last_sync_result: None,
            last_notification_at: None,
            pending_notification_count: 0,
            delivered_notification_count: 0,
            last_inbound_message_at: None,
            pending_inbound_message_count: 0,
            acknowledged_inbound_message_count: 0,
            last_outbound_message_at: None,
            pending_outbound_message_count: 0,
            acknowledged_outbound_message_count: 0,
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
    let existed = manifest_path(workspace_root, &request.id).exists();
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
    write_pairing_record(
        workspace_root,
        &MobilePairingRecord {
            id: uuid::Uuid::new_v4().to_string(),
            node_id: manifest.node.id.clone(),
            kind: "paired".to_string(),
            status: if existed {
                "updated".to_string()
            } else {
                "active".to_string()
            },
            created_at: Utc::now().to_rfc3339(),
            device_name: manifest.node.device_name.clone(),
            platform: manifest.node.platform.clone(),
            gateway_url: manifest.node.gateway_url.clone(),
            requested_by: None,
            reason: None,
        },
    )?;

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

pub fn list_pairing_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobilePairingRecord>> {
    let mut entries = Vec::new();
    let dir = pairings_dir(workspace_root);
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
        let record: MobilePairingRecord = serde_json::from_str(&bytes)
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

pub fn list_app_session_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    status: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileAppSessionRecord>> {
    let mut entries = Vec::new();
    let dir = app_sessions_dir(workspace_root);
    if !dir.exists() {
        return Ok(entries);
    }

    let normalized_status = status
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record: MobileAppSessionRecord = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if let Some(node_id) = node_id
            && record.node_id != node_id
        {
            continue;
        }
        if let Some(status) = normalized_status.as_deref()
            && record.status != status
        {
            continue;
        }
        entries.push(record);
    }

    entries.sort_by(|left, right| right.last_seen_at.cmp(&left.last_seen_at));
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    Ok(entries)
}

pub fn inspect_app_session_data(
    workspace_root: &Path,
    session_id: &str,
) -> Result<MobileAppSessionRecord> {
    let path = app_session_path(workspace_root, session_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn unpair_node_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobileUnpairRequest,
) -> Result<MobileNodeUnpairResult> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    let manifest_file = manifest_path(workspace_root, node_id);
    let runtime_file = runtime_path(workspace_root, node_id);

    let removed_manifest = if manifest_file.exists() {
        fs::remove_file(&manifest_file)
            .with_context(|| format!("failed to remove {}", manifest_file.display()))?;
        true
    } else {
        false
    };
    let removed_runtime_state = if request.remove_runtime_state && runtime_file.exists() {
        fs::remove_file(&runtime_file)
            .with_context(|| format!("failed to remove {}", runtime_file.display()))?;
        true
    } else {
        false
    };

    let pairing = MobilePairingRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: node_id.to_string(),
        kind: "unpaired".to_string(),
        status: "removed".to_string(),
        created_at: Utc::now().to_rfc3339(),
        device_name: manifest.node.device_name,
        platform: manifest.node.platform,
        gateway_url: manifest.node.gateway_url,
        requested_by: request
            .requested_by
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        reason: request
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    };
    write_pairing_record(workspace_root, &pairing)?;

    Ok(MobileNodeUnpairResult {
        node_id: node_id.to_string(),
        removed_manifest,
        removed_runtime_state,
        pairing,
    })
}

pub fn node_runtime_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodeRuntimeState> {
    inspect_node_data(workspace_root, node_id)?;
    load_runtime_state(workspace_root, node_id)
}

pub fn node_capabilities_data(
    workspace_root: &Path,
    node_id: &str,
) -> Result<MobileNodeCapabilitiesResult> {
    let manifest = inspect_node_data(workspace_root, node_id)?;
    let status = node_status_data(workspace_root, node_id)?;
    let advertised = manifest.node.capabilities;
    let mut capabilities = Vec::new();
    for capability in [
        "camera",
        "screen_recording",
        "location",
        "contacts",
        "calendar",
        "photos",
        "canvas",
        "sms",
        "notifications",
    ] {
        let aliases = capability_aliases(capability)
            .iter()
            .map(|entry| entry.to_string())
            .collect::<Vec<_>>();
        capabilities.push(MobileNodeCapabilityInfo {
            capability: capability.to_string(),
            advertised: has_capability(&advertised, capability),
            preview_supported: is_preview_supported_capability(capability),
            aliases,
            notes: capability_notes(capability),
        });
    }
    Ok(MobileNodeCapabilitiesResult {
        node_id: node_id.to_string(),
        platform: manifest.node.platform,
        readiness: status.readiness,
        capabilities,
    })
}

pub fn list_capability_execution_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    capability: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileCapabilityExecutionRecord>> {
    let mut entries = Vec::new();
    let dir = capability_executions_dir(workspace_root);
    if !dir.exists() {
        return Ok(entries);
    }

    let normalized_capability = capability
        .map(normalize_capability_name)
        .filter(|value| !value.is_empty());

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record: MobileCapabilityExecutionRecord = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if let Some(node_id) = node_id
            && record.node_id != node_id
        {
            continue;
        }
        if let Some(capability) = normalized_capability.as_deref()
            && record.capability != capability
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

pub fn inspect_capability_execution_data(
    workspace_root: &Path,
    execution_id: &str,
) -> Result<MobileCapabilityExecutionRecord> {
    let path = capability_execution_path(workspace_root, execution_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn list_media_artifact_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    capability: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileMediaArtifactRecord>> {
    let mut entries = Vec::new();
    let dir = media_artifacts_dir(workspace_root);
    if !dir.exists() {
        return Ok(entries);
    }

    let normalized_capability = capability
        .map(normalize_capability_name)
        .filter(|value| !value.is_empty());

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record: MobileMediaArtifactRecord = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if let Some(node_id) = node_id
            && record.node_id != node_id
        {
            continue;
        }
        if let Some(capability) = normalized_capability.as_deref()
            && record.capability != capability
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

pub fn inspect_media_artifact_data(
    workspace_root: &Path,
    artifact_id: &str,
) -> Result<MobileMediaArtifactRecord> {
    let path = media_artifact_path(workspace_root, artifact_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn node_activity_data(
    workspace_root: &Path,
    node_id: &str,
    limit: Option<usize>,
) -> Result<MobileNodeActivityResult> {
    inspect_node_data(workspace_root, node_id)?;
    let runtime = load_runtime_state(workspace_root, node_id)?;
    let mut entries = runtime_activity_entries(&runtime);
    entries.extend(
        list_pairing_data(workspace_root, Some(node_id), None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: format!("node_{}", record.kind),
                id: record.id,
                status: record.status,
                created_at: record.created_at,
                summary: format!(
                    "{} | {} | {}",
                    record.device_name,
                    record.platform,
                    record
                        .reason
                        .unwrap_or_else(|| "node lifecycle recorded".to_string())
                ),
            }),
    );
    entries.extend(
        list_app_session_data(workspace_root, Some(node_id), None, None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "app_session".to_string(),
                id: record.id,
                status: record.status.clone(),
                created_at: record.started_at,
                summary: format!(
                    "{} | network={} | battery={}",
                    record.status,
                    record.network.as_deref().unwrap_or("-"),
                    record
                        .battery_percent
                        .map(|value| format!("{value}%"))
                        .unwrap_or_else(|| "-".to_string())
                ),
            }),
    );
    entries.extend(
        list_capability_execution_data(workspace_root, Some(node_id), None, None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "capability_execution".to_string(),
                id: record.id,
                status: record.status.clone(),
                created_at: record.created_at,
                summary: format!("{} | {}", record.capability, record.status),
            }),
    );
    entries.extend(
        list_media_artifact_data(workspace_root, Some(node_id), None, None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "media_artifact".to_string(),
                id: record.id,
                status: record.status.clone(),
                created_at: record.created_at,
                summary: format!(
                    "{} | {} | {}",
                    record.capability, record.media_kind, record.summary
                ),
            }),
    );
    entries.extend(
        list_sync_conflict_data(workspace_root, Some(node_id), None, None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "sync_conflict".to_string(),
                id: record.id,
                status: record.status.clone(),
                created_at: record.created_at,
                summary: format!(
                    "{} | {} | {}",
                    record.conflict_type,
                    record.item_key,
                    record
                        .summary
                        .clone()
                        .unwrap_or_else(|| "sync conflict recorded".to_string())
                ),
            }),
    );

    entries.extend(
        list_notification_data(workspace_root, Some(node_id), None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "notification".to_string(),
                id: record.id,
                status: record.status,
                created_at: record.created_at,
                summary: format!("{} | {}", record.title, summarize_body(&record.body, 72)),
            }),
    );
    entries.extend(
        list_inbound_message_data(workspace_root, Some(node_id), None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "inbound_message".to_string(),
                id: record.id,
                status: record.status,
                created_at: record.created_at,
                summary: format!(
                    "{} -> {} | {}",
                    record.source,
                    record.target,
                    summarize_body(&record.content_preview, 72)
                ),
            }),
    );
    entries.extend(
        list_outbound_message_data(workspace_root, Some(node_id), None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "outbound_message".to_string(),
                id: record.id,
                status: record.status,
                created_at: record.created_at,
                summary: format!(
                    "to {} | {}",
                    record.target,
                    summarize_body(&record.content_preview, 72)
                ),
            }),
    );
    entries.extend(
        list_command_data(workspace_root, Some(node_id), None)?
            .into_iter()
            .map(|record| MobileNodeActivityEntry {
                kind: "command".to_string(),
                id: record.id,
                status: record.status,
                created_at: record.created_at.to_rfc3339(),
                summary: format!(
                    "{:?} requires {}",
                    record.command, record.required_capability
                ),
            }),
    );

    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if let Some(limit) = limit {
        entries.truncate(limit);
    }

    Ok(MobileNodeActivityResult {
        node_id: node_id.to_string(),
        entry_count: entries.len(),
        entries,
    })
}

pub fn mobile_node_summary_data(
    workspace_root: &Path,
    node_id: &str,
) -> Result<MobileNodeSummaryResult> {
    inspect_node_data(workspace_root, node_id)?;
    let runtime = load_runtime_state(workspace_root, node_id)?;
    let pairings = list_pairing_data(workspace_root, Some(node_id), None)?.len();
    let app_sessions = list_app_session_data(workspace_root, Some(node_id), None, None)?.len();
    let sync_conflicts = list_sync_conflict_data(workspace_root, Some(node_id), None, None)?.len();
    let notifications = list_notification_data(workspace_root, Some(node_id), None)?.len();
    let inbox_messages = list_inbound_message_data(workspace_root, Some(node_id), None)?.len();
    let outbox_messages = list_outbound_message_data(workspace_root, Some(node_id), None)?.len();
    let commands = list_command_data(workspace_root, Some(node_id), None)?.len();
    let capability_executions =
        list_capability_execution_data(workspace_root, Some(node_id), None, None)?.len();
    let media_artifacts =
        list_media_artifact_data(workspace_root, Some(node_id), None, None)?.len();
    Ok(MobileNodeSummaryResult {
        status: "ok".to_string(),
        summary: MobileNodeSummary {
            node_id: node_id.to_string(),
            runtime_status: runtime.runtime_status,
            app_state: runtime.app_state,
            network: runtime.network,
            reachable: runtime.reachable,
            push_token_present: runtime.push_token_present,
            notifications_authorized: runtime.notifications_authorized,
            wake_state: runtime.wake_state,
            rehydrate_state: runtime.rehydrate_state,
            sync_state: runtime.sync_state,
            battery_percent: runtime.battery_percent,
            pairings,
            app_sessions,
            sync_conflicts,
            notifications,
            inbox_messages,
            outbox_messages,
            commands,
            capability_executions,
            media_artifacts,
        },
    })
}

pub fn mobile_metrics_data() -> Result<MobileMetricsResult> {
    let workspace_root = std::env::current_dir().context("failed to resolve workspace root")?;
    let nodes = list_nodes_data(&workspace_root)?;
    let mut summary = MobileMetricsSummary {
        total_nodes: nodes.len(),
        paired_records: 0,
        unpaired_records: 0,
        reachable_nodes: 0,
        push_token_present_nodes: 0,
        notifications_authorized_nodes: 0,
        waking_nodes: 0,
        rehydrate_pending_nodes: 0,
        active_app_sessions: 0,
        pending_notifications: 0,
        delivered_notifications: 0,
        pending_inbound_messages: 0,
        acknowledged_inbound_messages: 0,
        pending_outbound_messages: 0,
        acknowledged_outbound_messages: 0,
        commands: 0,
        sync_conflicts: 0,
        capability_executions: 0,
        media_artifacts: 0,
    };

    for manifest in &nodes {
        let runtime = load_runtime_state(&workspace_root, &manifest.node.id)?;
        if runtime.reachable {
            summary.reachable_nodes += 1;
        }
        if runtime.push_token_present {
            summary.push_token_present_nodes += 1;
        }
        if runtime.notifications_authorized {
            summary.notifications_authorized_nodes += 1;
        }
        if runtime.wake_state == "requested" {
            summary.waking_nodes += 1;
        }
        if runtime.rehydrate_state == "requested" {
            summary.rehydrate_pending_nodes += 1;
        }
        summary.pending_notifications += runtime.pending_notification_count;
        summary.delivered_notifications += runtime.delivered_notification_count;
        summary.pending_inbound_messages += runtime.pending_inbound_message_count;
        summary.acknowledged_inbound_messages += runtime.acknowledged_inbound_message_count;
        summary.pending_outbound_messages += runtime.pending_outbound_message_count;
        summary.acknowledged_outbound_messages += runtime.acknowledged_outbound_message_count;
    }

    for pairing in list_pairing_data(&workspace_root, None, None)? {
        match pairing.kind.as_str() {
            "paired" => summary.paired_records += 1,
            "unpaired" => summary.unpaired_records += 1,
            _ => {}
        }
    }

    summary.active_app_sessions =
        list_app_session_data(&workspace_root, None, Some("active"), None)?.len();
    summary.commands = list_command_data(&workspace_root, None, None)?.len();
    summary.sync_conflicts = list_sync_conflict_data(&workspace_root, None, None, None)?.len();
    summary.capability_executions =
        list_capability_execution_data(&workspace_root, None, None, None)?.len();
    summary.media_artifacts = list_media_artifact_data(&workspace_root, None, None, None)?.len();

    Ok(MobileMetricsResult {
        status: "ok".to_string(),
        metrics: summary,
    })
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
    record_app_session_heartbeat(workspace_root, &runtime, "heartbeat")?;
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

pub fn node_push_state_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodePushState> {
    inspect_node_data(workspace_root, node_id)?;
    let runtime = load_runtime_state(workspace_root, node_id)?;
    Ok(MobileNodePushState {
        node_id: node_id.to_string(),
        push_token_present: runtime.push_token_present,
        notifications_authorized: runtime.notifications_authorized,
        push_provider: runtime.push_provider,
        push_token_updated_at: runtime.push_token_updated_at,
        runtime_status: runtime.runtime_status,
    })
}

pub fn register_push_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobilePushRegistrationRequest,
) -> Result<MobileNodePushState> {
    inspect_node_data(workspace_root, node_id)?;
    let mut runtime = load_runtime_state(workspace_root, node_id)?;
    let now = Utc::now().to_rfc3339();
    if let Some(push_provider) = request
        .push_provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        runtime.push_provider = Some(push_provider.to_string());
    }
    if let Some(push_token_present) = request.push_token_present {
        runtime.push_token_present = push_token_present;
        runtime.push_token_updated_at = Some(now.clone());
    }
    if let Some(notifications_authorized) = request.notifications_authorized {
        runtime.notifications_authorized = notifications_authorized;
    }
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    node_push_state_data(workspace_root, node_id)
}

pub fn node_sync_state_data(workspace_root: &Path, node_id: &str) -> Result<MobileNodeSyncState> {
    inspect_node_data(workspace_root, node_id)?;
    let runtime = load_runtime_state(workspace_root, node_id)?;
    let conflicts = list_sync_conflict_data(workspace_root, Some(node_id), None, None)?;
    Ok(MobileNodeSyncState {
        node_id: node_id.to_string(),
        sync_state: runtime.sync_state,
        pending_change_count: runtime.pending_change_count,
        pending_conflict_count: conflicts
            .iter()
            .filter(|record| record.status == "pending")
            .count(),
        resolved_conflict_count: conflicts
            .iter()
            .filter(|record| record.status == "resolved")
            .count(),
        last_sync_requested_at: runtime.last_sync_requested_at,
        last_sync_at: runtime.last_sync_at,
        last_sync_result: runtime.last_sync_result,
        runtime_status: runtime.runtime_status,
    })
}

pub fn list_sync_conflict_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    status: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileSyncConflictRecord>> {
    let mut entries = Vec::new();
    let dir = sync_conflicts_dir(workspace_root);
    if !dir.exists() {
        return Ok(entries);
    }

    let normalized_status = status
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record: MobileSyncConflictRecord = serde_json::from_str(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if let Some(node_id) = node_id
            && record.node_id != node_id
        {
            continue;
        }
        if let Some(status) = normalized_status.as_deref()
            && record.status != status
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

pub fn inspect_sync_conflict_data(
    workspace_root: &Path,
    conflict_id: &str,
) -> Result<MobileSyncConflictRecord> {
    let path = sync_conflict_path(workspace_root, conflict_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn report_sync_conflict_data(
    workspace_root: &Path,
    request: MobileSyncConflictReportRequest,
) -> Result<MobileSyncConflictRecord> {
    if request.node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    inspect_node_data(workspace_root, &request.node_id)?;
    if request.item_key.trim().is_empty() {
        return Err(anyhow!("item_key is required"));
    }
    if request.conflict_type.trim().is_empty() {
        return Err(anyhow!("conflict_type is required"));
    }

    let record = MobileSyncConflictRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: request.node_id,
        item_key: request.item_key.trim().to_string(),
        conflict_type: request.conflict_type.trim().to_string(),
        status: "pending".to_string(),
        created_at: Utc::now().to_rfc3339(),
        summary: request
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        details: request
            .details
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        resolution_hint: request
            .resolution_hint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        resolved_at: None,
        resolved_by: None,
        resolution: None,
    };
    write_sync_conflict_record(workspace_root, &record)?;
    Ok(record)
}

pub fn resolve_sync_conflict_data(
    workspace_root: &Path,
    conflict_id: &str,
    request: MobileSyncConflictResolveRequest,
) -> Result<MobileSyncConflictRecord> {
    if request.resolved_by.trim().is_empty() {
        return Err(anyhow!("resolved_by is required"));
    }
    let mut record = inspect_sync_conflict_data(workspace_root, conflict_id)?;
    if record.status == "resolved" {
        return Ok(record);
    }
    record.status = "resolved".to_string();
    record.resolved_at = Some(Utc::now().to_rfc3339());
    record.resolved_by = Some(request.resolved_by);
    record.resolution = request
        .resolution
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    write_sync_conflict_record(workspace_root, &record)?;
    Ok(record)
}

pub fn report_sync_data(
    workspace_root: &Path,
    node_id: &str,
    request: MobileSyncReportRequest,
) -> Result<MobileNodeSyncState> {
    inspect_node_data(workspace_root, node_id)?;
    let mut runtime = load_runtime_state(workspace_root, node_id)?;
    let now = Utc::now().to_rfc3339();
    let sync_state = request
        .sync_state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("synced")
        .to_string();
    runtime.sync_state = sync_state.clone();
    runtime.pending_change_count = request.pending_change_count;
    if matches!(sync_state.as_str(), "requested" | "syncing" | "in_progress") {
        runtime.last_sync_requested_at = Some(now);
    } else {
        runtime.last_sync_at = Some(now);
    }
    if let Some(last_sync_result) = request
        .last_sync_result
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        runtime.last_sync_result = Some(last_sync_result.to_string());
    }
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    node_sync_state_data(workspace_root, node_id)
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

fn build_capability_preview_record(
    workspace_root: &Path,
    request: &MobileCapabilityPreviewRequest,
    record_id: &str,
) -> Result<MobileCapabilityPreviewRecord> {
    let manifest = inspect_node_data(workspace_root, &request.node_id)?;
    let capability = normalize_capability_name(&request.capability);
    if capability.is_empty() {
        return Err(anyhow!("capability is required"));
    }
    if !is_preview_supported_capability(&capability) {
        return Err(anyhow!("unsupported mobile capability '{capability}'"));
    }
    if !has_capability(&manifest.node.capabilities, &capability) {
        return Err(anyhow!(
            "node '{}' does not advertise capability '{}'",
            manifest.node.id,
            capability
        ));
    }

    let target = request
        .target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let query = request
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let preview = match capability.as_str() {
        "camera" => json!({
            "kind": "camera_capture_preview",
            "capture_mode": "photo",
            "target": target.clone().unwrap_or_else(|| "camera_roll".to_string()),
            "note": note.clone(),
        }),
        "screen_recording" => json!({
            "kind": "screen_recording_preview",
            "duration_secs": 15,
            "target": target.clone().unwrap_or_else(|| "recent_clip".to_string()),
            "note": note.clone(),
        }),
        "location" => json!({
            "kind": "location_preview",
            "granularity": "coarse",
            "query": query.clone(),
            "note": note.clone(),
        }),
        "contacts" => json!({
            "kind": "contacts_query_preview",
            "query": query.clone(),
            "target": target.clone(),
            "note": note.clone(),
        }),
        "calendar" => json!({
            "kind": "calendar_query_preview",
            "query": query.clone(),
            "target": target.clone(),
            "note": note.clone(),
        }),
        "photos" => json!({
            "kind": "photos_picker_preview",
            "query": query.clone(),
            "target": target.clone(),
            "note": note.clone(),
        }),
        "canvas" => json!({
            "kind": "canvas_preview",
            "target": target.clone().unwrap_or_else(|| "default_canvas".to_string()),
            "note": note.clone(),
        }),
        "sms" => json!({
            "kind": "sms_draft_preview",
            "target": target.clone(),
            "query": query.clone(),
            "note": note.clone(),
        }),
        "notifications" => json!({
            "kind": "notification_dispatch_preview",
            "target": target.clone(),
            "query": query.clone(),
            "note": note.clone(),
        }),
        other => return Err(anyhow!("unsupported mobile capability '{other}'")),
    };

    Ok(MobileCapabilityPreviewRecord {
        id: record_id.to_string(),
        node_id: manifest.node.id.clone(),
        capability,
        created_at: Utc::now().to_rfc3339(),
        preview: json!({
            "node_id": manifest.node.id,
            "device_name": manifest.node.device_name,
            "platform": manifest.node.platform,
            "request": {
                "target": target,
                "query": query,
                "note": note,
            },
            "preview": preview,
        }),
    })
}

pub fn preview_capability_data(
    workspace_root: &Path,
    request: MobileCapabilityPreviewRequest,
) -> Result<MobileCapabilityPreviewRecord> {
    let preview_id = format!(
        "{}-{}-{}",
        request.node_id,
        normalize_capability_name(&request.capability),
        Utc::now().timestamp_millis()
    );
    let record = build_capability_preview_record(workspace_root, &request, &preview_id)?;
    write_capability_preview_record(workspace_root, &record)?;
    Ok(record)
}

pub fn execute_capability_data(
    workspace_root: &Path,
    request: MobileCapabilityExecuteRequest,
) -> Result<MobileCapabilityExecutionRecord> {
    let execution_id = format!(
        "{}-{}-{}",
        request.node_id,
        normalize_capability_name(&request.capability),
        Utc::now().timestamp_millis()
    );
    let preview = build_capability_preview_record(
        workspace_root,
        &MobileCapabilityPreviewRequest {
            node_id: request.node_id.clone(),
            capability: request.capability.clone(),
            target: request.target.clone(),
            query: request.query.clone(),
            note: request.note.clone(),
        },
        &execution_id,
    )?;
    let mut result = preview.preview;
    if let Some(execution) = result.get_mut("preview").and_then(Value::as_object_mut) {
        if let Some(kind) = execution.get("kind").and_then(Value::as_str) {
            execution.insert(
                "kind".to_string(),
                Value::String(kind.replace("_preview", "_execution")),
            );
        }
        execution.insert(
            "execution_mode".to_string(),
            Value::String("bounded_runtime_receipt".to_string()),
        );
        execution.insert("status".to_string(), Value::String("executed".to_string()));
        execution.insert(
            "executed_at".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );
        if let Some(requested_by) = request
            .requested_by
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            execution.insert(
                "requested_by".to_string(),
                Value::String(requested_by.to_string()),
            );
        }
    }
    let record = MobileCapabilityExecutionRecord {
        id: execution_id,
        node_id: preview.node_id,
        capability: preview.capability,
        status: "executed".to_string(),
        created_at: Utc::now().to_rfc3339(),
        requested_by: request
            .requested_by
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        result,
    };
    write_capability_execution_record(workspace_root, &record)?;
    if let Some(artifact) = build_media_artifact_record(&record) {
        write_media_artifact_record(workspace_root, &artifact)?;
    }
    Ok(record)
}

pub fn list_notification_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileNotificationRecord>> {
    let mut entries = Vec::new();
    let dir = notifications_dir(workspace_root);
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
        let record: MobileNotificationRecord = serde_json::from_str(&bytes)
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

pub fn inspect_notification_data(
    workspace_root: &Path,
    notification_id: &str,
) -> Result<MobileNotificationRecord> {
    let path = notification_path(workspace_root, notification_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn send_notification_data(
    workspace_root: &Path,
    request: MobileNotificationSendRequest,
) -> Result<MobileNotificationRecord> {
    if request.node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    let manifest = inspect_node_data(workspace_root, &request.node_id)?;
    let status = node_status_data(workspace_root, &request.node_id)?;
    let mut runtime = load_runtime_state(workspace_root, &request.node_id)?;
    let preview = preview_notification_data(MobileNotificationPreviewRequest {
        title: request.title.clone(),
        body: request.body.clone(),
        priority: request.priority.clone(),
        notification_type: request.notification_type.clone(),
        data: request.data.clone(),
    })?;

    let now = Utc::now();
    let mut record = MobileNotificationRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: request.node_id.clone(),
        title: request.title,
        body: request.body,
        status: "queued".to_string(),
        priority: request.priority,
        notification_type: request.notification_type,
        data: request.data,
        created_at: now.to_rfc3339(),
        dispatched_at: None,
        acknowledged_at: None,
        acknowledged_by: None,
        command_id: None,
        preview: serde_json::to_value(&preview)
            .context("failed to serialize notification preview")?,
    };

    if status.readiness == "ready_for_runtime"
        && has_capability(&manifest.node.capabilities, "notifications")
    {
        let command = dispatch_command_data(
            workspace_root,
            MobileCommandDispatchRequest {
                node_id: request.node_id.clone(),
                command: DeviceCommandKind::PushNotification,
                payload: json!({
                    "title": record.title,
                    "body": record.body,
                    "priority": record.priority,
                    "type": record.notification_type,
                    "data": record.data,
                }),
                approved_by: request
                    .requested_by
                    .clone()
                    .or_else(|| Some("mobile_runtime".to_string())),
                require_approval: Some(false),
            },
        )
        .await?;
        record.status = "dispatched".to_string();
        record.dispatched_at = Some(Utc::now().to_rfc3339());
        record.command_id = Some(command.id);
    }

    runtime.last_notification_at = Some(Utc::now().to_rfc3339());
    runtime.pending_notification_count = runtime.pending_notification_count.saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_notification_record(workspace_root, &record)?;
    Ok(record)
}

pub fn acknowledge_notification_data(
    workspace_root: &Path,
    notification_id: &str,
    request: MobileNotificationAckRequest,
) -> Result<MobileNotificationRecord> {
    if request.acknowledged_by.trim().is_empty() {
        return Err(anyhow!("acknowledged_by is required"));
    }
    let mut record = inspect_notification_data(workspace_root, notification_id)?;
    if record.status == "acknowledged" {
        return Ok(record);
    }
    let mut runtime = load_runtime_state(workspace_root, &record.node_id)?;
    record.status = "acknowledged".to_string();
    record.acknowledged_by = Some(request.acknowledged_by);
    record.acknowledged_at = Some(Utc::now().to_rfc3339());
    runtime.pending_notification_count = runtime.pending_notification_count.saturating_sub(1);
    runtime.delivered_notification_count = runtime.delivered_notification_count.saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_notification_record(workspace_root, &record)?;
    Ok(record)
}

pub fn list_inbound_message_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileInboundMessageRecord>> {
    let mut entries = Vec::new();
    let dir = inbox_dir(workspace_root);
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
        let record: MobileInboundMessageRecord = serde_json::from_str(&bytes)
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

pub fn inspect_inbound_message_data(
    workspace_root: &Path,
    message_id: &str,
) -> Result<MobileInboundMessageRecord> {
    let path = inbox_message_path(workspace_root, message_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn report_inbound_message_data(
    workspace_root: &Path,
    request: MobileInboundMessageReportRequest,
) -> Result<MobileInboundMessageRecord> {
    if request.node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    inspect_node_data(workspace_root, &request.node_id)?;
    if request.source.trim().is_empty() {
        return Err(anyhow!("source is required"));
    }
    if request.target.trim().is_empty() {
        return Err(anyhow!("target is required"));
    }
    if request.content.trim().is_empty() {
        return Err(anyhow!("content is required"));
    }

    let content_type = request
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("text/plain")
        .to_string();
    let preview = preview_message_data(MobileMessagePreviewRequest {
        source_node_id: request.source.clone(),
        target: request.target.clone(),
        content: request.content.clone(),
        content_type: Some(content_type.clone()),
    })?;
    let now = Utc::now().to_rfc3339();
    let mut runtime = load_runtime_state(workspace_root, &request.node_id)?;
    let record = MobileInboundMessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: request.node_id,
        source: request.source,
        target: request.target,
        status: "reported".to_string(),
        content_type: Some(content_type),
        content_preview: request.content,
        bytes: preview
            .get("bytes")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize,
        created_at: now.clone(),
        reported_at: Some(now.clone()),
        acknowledged_at: None,
        acknowledged_by: None,
        metadata: request.metadata,
        preview,
    };

    runtime.last_inbound_message_at = Some(now);
    runtime.pending_inbound_message_count = runtime.pending_inbound_message_count.saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_inbound_message_record(workspace_root, &record)?;
    Ok(record)
}

pub fn acknowledge_inbound_message_data(
    workspace_root: &Path,
    message_id: &str,
    request: MobileInboundMessageAckRequest,
) -> Result<MobileInboundMessageRecord> {
    if request.acknowledged_by.trim().is_empty() {
        return Err(anyhow!("acknowledged_by is required"));
    }
    let mut record = inspect_inbound_message_data(workspace_root, message_id)?;
    if record.status == "acknowledged" {
        return Ok(record);
    }

    let mut runtime = load_runtime_state(workspace_root, &record.node_id)?;
    record.status = "acknowledged".to_string();
    record.acknowledged_by = Some(request.acknowledged_by);
    record.acknowledged_at = Some(Utc::now().to_rfc3339());
    runtime.pending_inbound_message_count = runtime.pending_inbound_message_count.saturating_sub(1);
    runtime.acknowledged_inbound_message_count =
        runtime.acknowledged_inbound_message_count.saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_inbound_message_record(workspace_root, &record)?;
    Ok(record)
}

pub fn list_outbound_message_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MobileOutboundMessageRecord>> {
    let mut entries = Vec::new();
    let dir = outbox_dir(workspace_root);
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
        let record: MobileOutboundMessageRecord = serde_json::from_str(&bytes)
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

pub fn inspect_outbound_message_data(
    workspace_root: &Path,
    message_id: &str,
) -> Result<MobileOutboundMessageRecord> {
    let path = outbox_message_path(workspace_root, message_id);
    let bytes =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn send_outbound_message_data(
    workspace_root: &Path,
    request: MobileOutboundMessageSendRequest,
) -> Result<MobileOutboundMessageRecord> {
    if request.node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    let manifest = inspect_node_data(workspace_root, &request.node_id)?;
    let status = node_status_data(workspace_root, &request.node_id)?;
    if request.target.trim().is_empty() {
        return Err(anyhow!("target is required"));
    }
    if request.content.trim().is_empty() {
        return Err(anyhow!("content is required"));
    }

    let content_type = request
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("text/plain")
        .to_string();
    let preview = preview_message_data(MobileMessagePreviewRequest {
        source_node_id: request.node_id.clone(),
        target: request.target.clone(),
        content: request.content.clone(),
        content_type: Some(content_type.clone()),
    })?;
    let now = Utc::now();
    let mut runtime = load_runtime_state(workspace_root, &request.node_id)?;
    let mut record = MobileOutboundMessageRecord {
        id: uuid::Uuid::new_v4().to_string(),
        node_id: request.node_id.clone(),
        target: request.target.clone(),
        status: "queued".to_string(),
        content_type: Some(content_type.clone()),
        content_preview: request.content.clone(),
        bytes: preview
            .get("bytes")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize,
        created_at: now.to_rfc3339(),
        dispatched_at: None,
        acknowledged_at: None,
        acknowledged_by: None,
        command_id: None,
        metadata: request.metadata,
        preview,
    };

    if status.readiness == "ready_for_runtime"
        && has_capability(&manifest.node.capabilities, "mobile")
    {
        let command = dispatch_command_data(
            workspace_root,
            MobileCommandDispatchRequest {
                node_id: request.node_id.clone(),
                command: DeviceCommandKind::SendMessage,
                payload: json!({
                    "target": request.target,
                    "content": request.content,
                    "content_type": content_type,
                }),
                approved_by: request
                    .requested_by
                    .clone()
                    .or_else(|| Some("mobile_runtime".to_string())),
                require_approval: Some(false),
            },
        )
        .await?;
        record.status = "dispatched".to_string();
        record.dispatched_at = Some(Utc::now().to_rfc3339());
        record.command_id = Some(command.id);
    }

    runtime.last_outbound_message_at = Some(Utc::now().to_rfc3339());
    runtime.pending_outbound_message_count =
        runtime.pending_outbound_message_count.saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_outbound_message_record(workspace_root, &record)?;
    Ok(record)
}

pub fn acknowledge_outbound_message_data(
    workspace_root: &Path,
    message_id: &str,
    request: MobileOutboundMessageAckRequest,
) -> Result<MobileOutboundMessageRecord> {
    if request.acknowledged_by.trim().is_empty() {
        return Err(anyhow!("acknowledged_by is required"));
    }
    let mut record = inspect_outbound_message_data(workspace_root, message_id)?;
    if record.status == "acknowledged" {
        return Ok(record);
    }

    let mut runtime = load_runtime_state(workspace_root, &record.node_id)?;
    record.status = "acknowledged".to_string();
    record.acknowledged_by = Some(request.acknowledged_by);
    record.acknowledged_at = Some(Utc::now().to_rfc3339());
    runtime.pending_outbound_message_count =
        runtime.pending_outbound_message_count.saturating_sub(1);
    runtime.acknowledged_outbound_message_count = runtime
        .acknowledged_outbound_message_count
        .saturating_add(1);
    refresh_runtime_status(&mut runtime);
    save_runtime_state(workspace_root, &runtime)?;
    write_outbound_message_record(workspace_root, &record)?;
    Ok(record)
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

fn command_timeline_events(command: &MobileCommandRecord) -> Vec<MobileCommandEvent> {
    let mut events = Vec::new();
    let mut index = 0usize;

    events.push(MobileCommandEvent {
        index,
        kind: "command_dispatched".to_string(),
        observed_at: command.created_at.to_rfc3339(),
        summary: format!(
            "{} dispatched for node {} (requires {})",
            command.command, command.node_id, command.required_capability
        ),
    });
    index += 1;

    if command.approval_required {
        if let Some(approved_at) = command.approved_at.as_ref() {
            let kind = if command.status == "rejected" {
                "command_rejected"
            } else {
                "command_approved"
            };
            events.push(MobileCommandEvent {
                index,
                kind: kind.to_string(),
                observed_at: approved_at.to_rfc3339(),
                summary: if command.status == "rejected" {
                    format!(
                        "{} rejected by {}",
                        command.command,
                        command.approved_by.as_deref().unwrap_or("operator")
                    )
                } else {
                    format!(
                        "{} approved by {}",
                        command.command,
                        command.approved_by.as_deref().unwrap_or("operator")
                    )
                },
            });
            index += 1;
        }
    } else if command.status == "rejected" {
        events.push(MobileCommandEvent {
            index,
            kind: "command_rejected".to_string(),
            observed_at: command.created_at.to_rfc3339(),
            summary: format!("{} rejected for node {}", command.command, command.node_id),
        });
        index += 1;
    }

    if let Some(executed_at) = command.executed_at.as_ref() {
        events.push(MobileCommandEvent {
            index,
            kind: "command_executed".to_string(),
            observed_at: executed_at.to_rfc3339(),
            summary: format!(
                "{} executed for node {} with status {}",
                command.command, command.node_id, command.status
            ),
        });
    }

    events
}

fn command_execution_latency_secs(command: &MobileCommandRecord) -> Option<u64> {
    let executed_at = command.executed_at.as_ref()?;
    let latency = executed_at
        .signed_duration_since(command.created_at)
        .num_seconds();
    Some(latency.max(0) as u64)
}

pub fn command_events_data(
    workspace_root: &Path,
    command_id: &str,
) -> Result<MobileCommandEventsResult> {
    let command = inspect_command_data(workspace_root, command_id)?;
    let events = command_timeline_events(&command);
    Ok(MobileCommandEventsResult {
        command_id: command.id,
        node_id: command.node_id,
        command: command.command,
        status: command.status,
        event_count: events.len(),
        events,
    })
}

pub fn command_metrics_data(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<MobileCommandMetricsResult> {
    let commands = list_command_data(workspace_root, node_id, limit)?;
    let mut by_command_kind: HashMap<String, usize> = HashMap::new();
    let mut pending_approval_commands = 0usize;
    let mut approved_commands = 0usize;
    let mut executed_commands = 0usize;
    let mut rejected_commands = 0usize;
    let mut execution_latencies = Vec::new();

    for command in &commands {
        *by_command_kind
            .entry(command.command.to_string())
            .or_insert(0) += 1;
        match command.status.as_str() {
            "pending_approval" => pending_approval_commands += 1,
            "approved" => approved_commands += 1,
            "executed" => executed_commands += 1,
            "rejected" => rejected_commands += 1,
            _ => {}
        }
        if let Some(latency) = command_execution_latency_secs(command) {
            execution_latencies.push(latency as f64);
        }
    }

    let avg_execution_latency_secs = if execution_latencies.is_empty() {
        None
    } else {
        Some(execution_latencies.iter().sum::<f64>() / execution_latencies.len() as f64)
    };

    Ok(MobileCommandMetricsResult {
        status: "ok".to_string(),
        metrics: MobileCommandMetricsSummary {
            total_commands: commands.len(),
            pending_approval_commands,
            approved_commands,
            executed_commands,
            rejected_commands,
            by_command_kind,
            avg_execution_latency_secs,
        },
    })
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

pub async fn list_pairings(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let pairings = list_pairing_data(workspace_root, node_id, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "pairings": pairings }))?
    );
    Ok(())
}

pub async fn list_app_sessions(
    workspace_root: &Path,
    node_id: Option<&str>,
    status: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let sessions = list_app_session_data(workspace_root, node_id, status, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "sessions": sessions }))?
    );
    Ok(())
}

pub async fn inspect_app_session(workspace_root: &Path, session_id: &str) -> Result<()> {
    let session = inspect_app_session_data(workspace_root, session_id)?;
    println!("{}", serde_json::to_string_pretty(&session)?);
    Ok(())
}

pub async fn unpair_node(
    workspace_root: &Path,
    node_id: &str,
    request: MobileUnpairRequest,
) -> Result<()> {
    let result = unpair_node_data(workspace_root, node_id, request)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
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

pub async fn node_push_state(workspace_root: &Path, node_id: &str) -> Result<()> {
    let push = node_push_state_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&push)?);
    Ok(())
}

pub async fn register_push(
    workspace_root: &Path,
    node_id: &str,
    request: MobilePushRegistrationRequest,
) -> Result<()> {
    let push = register_push_data(workspace_root, node_id, request)?;
    println!("{}", serde_json::to_string_pretty(&push)?);
    Ok(())
}

pub async fn node_sync_state(workspace_root: &Path, node_id: &str) -> Result<()> {
    let sync = node_sync_state_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&sync)?);
    Ok(())
}

pub async fn report_sync(
    workspace_root: &Path,
    node_id: &str,
    request: MobileSyncReportRequest,
) -> Result<()> {
    let sync = report_sync_data(workspace_root, node_id, request)?;
    println!("{}", serde_json::to_string_pretty(&sync)?);
    Ok(())
}

pub async fn list_sync_conflicts(
    workspace_root: &Path,
    node_id: Option<&str>,
    status: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let conflicts = list_sync_conflict_data(workspace_root, node_id, status, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "conflicts": conflicts }))?
    );
    Ok(())
}

pub async fn inspect_sync_conflict(workspace_root: &Path, conflict_id: &str) -> Result<()> {
    let conflict = inspect_sync_conflict_data(workspace_root, conflict_id)?;
    println!("{}", serde_json::to_string_pretty(&conflict)?);
    Ok(())
}

pub async fn report_sync_conflict(
    workspace_root: &Path,
    request: MobileSyncConflictReportRequest,
) -> Result<()> {
    let conflict = report_sync_conflict_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&conflict)?);
    Ok(())
}

pub async fn resolve_sync_conflict(
    workspace_root: &Path,
    conflict_id: &str,
    request: MobileSyncConflictResolveRequest,
) -> Result<()> {
    let conflict = resolve_sync_conflict_data(workspace_root, conflict_id, request)?;
    println!("{}", serde_json::to_string_pretty(&conflict)?);
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

pub async fn list_notifications(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let notifications = list_notification_data(workspace_root, node_id, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "notifications": notifications }))?
    );
    Ok(())
}

pub async fn inspect_notification(workspace_root: &Path, notification_id: &str) -> Result<()> {
    let notification = inspect_notification_data(workspace_root, notification_id)?;
    println!("{}", serde_json::to_string_pretty(&notification)?);
    Ok(())
}

pub async fn send_notification(
    workspace_root: &Path,
    request: MobileNotificationSendRequest,
) -> Result<()> {
    let record = send_notification_data(workspace_root, request).await?;
    println!("{}", serde_json::to_string_pretty(&record)?);
    Ok(())
}

pub async fn acknowledge_notification(
    workspace_root: &Path,
    notification_id: &str,
    request: MobileNotificationAckRequest,
) -> Result<()> {
    let record = acknowledge_notification_data(workspace_root, notification_id, request)?;
    println!("{}", serde_json::to_string_pretty(&record)?);
    Ok(())
}

pub async fn list_inbox(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let messages = list_inbound_message_data(workspace_root, node_id, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "messages": messages }))?
    );
    Ok(())
}

pub async fn inspect_inbox_message(workspace_root: &Path, message_id: &str) -> Result<()> {
    let message = inspect_inbound_message_data(workspace_root, message_id)?;
    println!("{}", serde_json::to_string_pretty(&message)?);
    Ok(())
}

pub async fn report_inbox_message(
    workspace_root: &Path,
    request: MobileInboundMessageReportRequest,
) -> Result<()> {
    let message = report_inbound_message_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&message)?);
    Ok(())
}

pub async fn acknowledge_inbox_message(
    workspace_root: &Path,
    message_id: &str,
    request: MobileInboundMessageAckRequest,
) -> Result<()> {
    let message = acknowledge_inbound_message_data(workspace_root, message_id, request)?;
    println!("{}", serde_json::to_string_pretty(&message)?);
    Ok(())
}

pub async fn list_outbox(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let messages = list_outbound_message_data(workspace_root, node_id, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "messages": messages }))?
    );
    Ok(())
}

pub async fn inspect_outbox_message(workspace_root: &Path, message_id: &str) -> Result<()> {
    let message = inspect_outbound_message_data(workspace_root, message_id)?;
    println!("{}", serde_json::to_string_pretty(&message)?);
    Ok(())
}

pub async fn send_outbox_message(
    workspace_root: &Path,
    request: MobileOutboundMessageSendRequest,
) -> Result<()> {
    let message = send_outbound_message_data(workspace_root, request).await?;
    println!("{}", serde_json::to_string_pretty(&message)?);
    Ok(())
}

pub async fn acknowledge_outbox_message(
    workspace_root: &Path,
    message_id: &str,
    request: MobileOutboundMessageAckRequest,
) -> Result<()> {
    let message = acknowledge_outbound_message_data(workspace_root, message_id, request)?;
    println!("{}", serde_json::to_string_pretty(&message)?);
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

pub async fn node_capabilities(workspace_root: &Path, node_id: &str) -> Result<()> {
    let capabilities = node_capabilities_data(workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&capabilities)?);
    Ok(())
}

pub async fn node_activity(
    workspace_root: &Path,
    node_id: &str,
    limit: Option<usize>,
) -> Result<()> {
    let activity = node_activity_data(workspace_root, node_id, limit)?;
    println!("{}", serde_json::to_string_pretty(&activity)?);
    Ok(())
}

pub async fn mobile_metrics() -> Result<()> {
    let metrics = mobile_metrics_data()?;
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

pub async fn mobile_node_summary(node_id: &str) -> Result<()> {
    let workspace_root = std::env::current_dir().context("failed to resolve workspace root")?;
    let summary = mobile_node_summary_data(&workspace_root, node_id)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub async fn preview_capability(
    workspace_root: &Path,
    request: MobileCapabilityPreviewRequest,
) -> Result<()> {
    let preview = preview_capability_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&preview)?);
    Ok(())
}

pub async fn execute_capability(
    workspace_root: &Path,
    request: MobileCapabilityExecuteRequest,
) -> Result<()> {
    let execution = execute_capability_data(workspace_root, request)?;
    println!("{}", serde_json::to_string_pretty(&execution)?);
    Ok(())
}

pub async fn list_capability_executions(
    workspace_root: &Path,
    node_id: Option<&str>,
    capability: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let executions = list_capability_execution_data(workspace_root, node_id, capability, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "executions": executions }))?
    );
    Ok(())
}

pub async fn inspect_capability_execution(workspace_root: &Path, execution_id: &str) -> Result<()> {
    let execution = inspect_capability_execution_data(workspace_root, execution_id)?;
    println!("{}", serde_json::to_string_pretty(&execution)?);
    Ok(())
}

pub async fn list_media_artifacts(
    workspace_root: &Path,
    node_id: Option<&str>,
    capability: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let artifacts = list_media_artifact_data(workspace_root, node_id, capability, limit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "artifacts": artifacts }))?
    );
    Ok(())
}

pub async fn inspect_media_artifact(workspace_root: &Path, artifact_id: &str) -> Result<()> {
    let artifact = inspect_media_artifact_data(workspace_root, artifact_id)?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
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

pub async fn command_events(workspace_root: &Path, command_id: &str) -> Result<()> {
    let events = command_events_data(workspace_root, command_id)?;
    println!("{}", serde_json::to_string_pretty(&events)?);
    Ok(())
}

pub async fn command_metrics(
    workspace_root: &Path,
    node_id: Option<&str>,
    limit: Option<usize>,
) -> Result<()> {
    let metrics = command_metrics_data(workspace_root, node_id, limit)?;
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

fn mobile_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(DEFAULT_MOBILE_ROOT)
}

fn nodes_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("nodes")
}

fn pairings_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("pairings")
}

fn app_sessions_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("app-sessions")
}

fn commands_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("commands")
}

fn notifications_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("notifications")
}

fn inbox_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("inbox")
}

fn outbox_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("outbox")
}

fn runtime_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("runtime")
}

fn sync_conflicts_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("sync-conflicts")
}

fn capability_previews_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("capability-previews")
}

fn capability_executions_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("capability-executions")
}

fn media_artifacts_dir(workspace_root: &Path) -> PathBuf {
    mobile_root(workspace_root).join("media-artifacts")
}

fn manifest_path(workspace_root: &Path, node_id: &str) -> PathBuf {
    nodes_dir(workspace_root).join(format!("{node_id}.json"))
}

fn command_path(workspace_root: &Path, command_id: &str) -> PathBuf {
    commands_dir(workspace_root).join(format!("{command_id}.json"))
}

fn pairing_path(workspace_root: &Path, pairing_id: &str) -> PathBuf {
    pairings_dir(workspace_root).join(format!("{pairing_id}.json"))
}

fn app_session_path(workspace_root: &Path, session_id: &str) -> PathBuf {
    app_sessions_dir(workspace_root).join(format!("{session_id}.json"))
}

fn notification_path(workspace_root: &Path, notification_id: &str) -> PathBuf {
    notifications_dir(workspace_root).join(format!("{notification_id}.json"))
}

fn inbox_message_path(workspace_root: &Path, message_id: &str) -> PathBuf {
    inbox_dir(workspace_root).join(format!("{message_id}.json"))
}

fn outbox_message_path(workspace_root: &Path, message_id: &str) -> PathBuf {
    outbox_dir(workspace_root).join(format!("{message_id}.json"))
}

fn runtime_path(workspace_root: &Path, node_id: &str) -> PathBuf {
    runtime_dir(workspace_root).join(format!("{node_id}.json"))
}

fn sync_conflict_path(workspace_root: &Path, conflict_id: &str) -> PathBuf {
    sync_conflicts_dir(workspace_root).join(format!("{conflict_id}.json"))
}

fn capability_preview_path(workspace_root: &Path, preview_id: &str) -> PathBuf {
    capability_previews_dir(workspace_root).join(format!("{preview_id}.json"))
}

fn capability_execution_path(workspace_root: &Path, execution_id: &str) -> PathBuf {
    capability_executions_dir(workspace_root).join(format!("{execution_id}.json"))
}

fn media_artifact_path(workspace_root: &Path, artifact_id: &str) -> PathBuf {
    media_artifacts_dir(workspace_root).join(format!("{artifact_id}.json"))
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

fn write_pairing_record(workspace_root: &Path, record: &MobilePairingRecord) -> Result<()> {
    fs::create_dir_all(pairings_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            pairings_dir(workspace_root).display()
        )
    })?;
    let path = pairing_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record).context("failed to serialize mobile pairing record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_app_session_record(workspace_root: &Path, record: &MobileAppSessionRecord) -> Result<()> {
    fs::create_dir_all(app_sessions_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            app_sessions_dir(workspace_root).display()
        )
    })?;
    let path = app_session_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile app session record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_sync_conflict_record(
    workspace_root: &Path,
    record: &MobileSyncConflictRecord,
) -> Result<()> {
    fs::create_dir_all(sync_conflicts_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            sync_conflicts_dir(workspace_root).display()
        )
    })?;
    let path = sync_conflict_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile sync conflict record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_capability_preview_record(
    workspace_root: &Path,
    record: &MobileCapabilityPreviewRecord,
) -> Result<()> {
    fs::create_dir_all(capability_previews_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            capability_previews_dir(workspace_root).display()
        )
    })?;
    let path = capability_preview_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile capability preview record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_capability_execution_record(
    workspace_root: &Path,
    record: &MobileCapabilityExecutionRecord,
) -> Result<()> {
    fs::create_dir_all(capability_executions_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            capability_executions_dir(workspace_root).display()
        )
    })?;
    let path = capability_execution_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile capability execution record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_media_artifact_record(
    workspace_root: &Path,
    record: &MobileMediaArtifactRecord,
) -> Result<()> {
    fs::create_dir_all(media_artifacts_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            media_artifacts_dir(workspace_root).display()
        )
    })?;
    let path = media_artifact_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile media artifact record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_notification_record(
    workspace_root: &Path,
    record: &MobileNotificationRecord,
) -> Result<()> {
    fs::create_dir_all(notifications_dir(workspace_root)).with_context(|| {
        format!(
            "failed to create {}",
            notifications_dir(workspace_root).display()
        )
    })?;
    let path = notification_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile notification record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_inbound_message_record(
    workspace_root: &Path,
    record: &MobileInboundMessageRecord,
) -> Result<()> {
    fs::create_dir_all(inbox_dir(workspace_root))
        .with_context(|| format!("failed to create {}", inbox_dir(workspace_root).display()))?;
    let path = inbox_message_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile inbound message record")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_outbound_message_record(
    workspace_root: &Path,
    record: &MobileOutboundMessageRecord,
) -> Result<()> {
    fs::create_dir_all(outbox_dir(workspace_root))
        .with_context(|| format!("failed to create {}", outbox_dir(workspace_root).display()))?;
    let path = outbox_message_path(workspace_root, &record.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(record)
            .context("failed to serialize mobile outbound message record")?,
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

fn runtime_activity_entries(runtime: &MobileNodeRuntimeState) -> Vec<MobileNodeActivityEntry> {
    let mut entries = Vec::new();
    if let Some(created_at) = runtime.last_heartbeat_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "runtime_heartbeat".to_string(),
            id: runtime.node_id.clone(),
            status: runtime.runtime_status.clone(),
            created_at: created_at.clone(),
            summary: format!(
                "app={} network={} reachable={} battery={}",
                runtime.app_state,
                runtime.network,
                runtime.reachable,
                runtime
                    .battery_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "-".to_string())
            ),
        });
    }
    if let Some(created_at) = runtime.push_token_updated_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "push_registration".to_string(),
            id: runtime.node_id.clone(),
            status: if runtime.push_token_present {
                "registered".to_string()
            } else {
                "missing_token".to_string()
            },
            created_at: created_at.clone(),
            summary: format!(
                "provider={} notifications_authorized={}",
                runtime.push_provider.as_deref().unwrap_or("-"),
                runtime.notifications_authorized
            ),
        });
    }
    if let Some(created_at) = runtime.wake_requested_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "wake_request".to_string(),
            id: runtime
                .last_wake_command_id
                .clone()
                .unwrap_or_else(|| runtime.node_id.clone()),
            status: runtime.wake_state.clone(),
            created_at: created_at.clone(),
            summary: runtime
                .wake_reason
                .clone()
                .unwrap_or_else(|| "wake request recorded".to_string()),
        });
    }
    if let Some(created_at) = runtime.rehydrate_requested_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "rehydrate_request".to_string(),
            id: runtime
                .last_rehydrate_command_id
                .clone()
                .unwrap_or_else(|| runtime.node_id.clone()),
            status: runtime.rehydrate_state.clone(),
            created_at: created_at.clone(),
            summary: format!(
                "{} pending_changes={}",
                runtime
                    .rehydrate_reason
                    .clone()
                    .unwrap_or_else(|| "rehydrate request recorded".to_string()),
                runtime
                    .rehydrate_pending_change_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            ),
        });
    }
    if let Some(created_at) = runtime.last_sync_requested_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "sync_request".to_string(),
            id: runtime.node_id.clone(),
            status: runtime.sync_state.clone(),
            created_at: created_at.clone(),
            summary: format!(
                "pending_changes={} result={}",
                runtime
                    .pending_change_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                runtime.last_sync_result.as_deref().unwrap_or("-")
            ),
        });
    }
    if let Some(created_at) = runtime.last_sync_at.as_ref() {
        entries.push(MobileNodeActivityEntry {
            kind: "sync_result".to_string(),
            id: runtime.node_id.clone(),
            status: runtime.sync_state.clone(),
            created_at: created_at.clone(),
            summary: runtime
                .last_sync_result
                .clone()
                .unwrap_or_else(|| "sync completed".to_string()),
        });
    }
    entries
}

fn summarize_body(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let summarized = trimmed.chars().take(limit).collect::<String>();
    format!("{summarized}...")
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

fn capability_aliases(capability: &str) -> &'static [&'static str] {
    match capability {
        "camera" => &["camera"],
        "screen_recording" => &["screen_recording", "screen"],
        "location" => &["location"],
        "contacts" => &["contacts"],
        "calendar" => &["calendar"],
        "photos" => &["photos", "camera_roll"],
        "canvas" => &["canvas"],
        "sms" => &["sms"],
        "notifications" => &["notifications"],
        _ => &[],
    }
}

fn derive_app_session_status(runtime: &MobileNodeRuntimeState) -> String {
    if !runtime.reachable && runtime.last_heartbeat_at.is_some() {
        "disconnected".to_string()
    } else if runtime.reachable && runtime.app_state == "active" {
        "active".to_string()
    } else if runtime.reachable && runtime.app_state == "background" {
        "background".to_string()
    } else if runtime.reachable {
        "reachable".to_string()
    } else {
        "registered".to_string()
    }
}

fn duration_secs_between(start: &str, end: &str) -> Option<u64> {
    let start = DateTime::parse_from_rfc3339(start).ok()?;
    let end = DateTime::parse_from_rfc3339(end).ok()?;
    let duration = end.signed_duration_since(start);
    (duration.num_seconds() >= 0).then_some(duration.num_seconds() as u64)
}

fn record_app_session_heartbeat(
    workspace_root: &Path,
    runtime: &MobileNodeRuntimeState,
    entry_reason: &str,
) -> Result<()> {
    let status = derive_app_session_status(runtime);
    let now = runtime
        .last_heartbeat_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let open_session = list_app_session_data(workspace_root, Some(&runtime.node_id), None, None)?
        .into_iter()
        .find(|record| record.ended_at.is_none());

    match open_session {
        Some(mut record) if record.status == status => {
            record.last_seen_at = now.clone();
            record.network = Some(runtime.network.clone());
            record.battery_percent = runtime.battery_percent;
            write_app_session_record(workspace_root, &record)?;
        }
        Some(mut record) => {
            record.ended_at = Some(now.clone());
            record.duration_secs = duration_secs_between(&record.started_at, &now);
            record.exit_reason = Some("heartbeat_transition".to_string());
            write_app_session_record(workspace_root, &record)?;

            write_app_session_record(
                workspace_root,
                &MobileAppSessionRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    node_id: runtime.node_id.clone(),
                    status,
                    started_at: now.clone(),
                    last_seen_at: now,
                    ended_at: None,
                    duration_secs: None,
                    network: Some(runtime.network.clone()),
                    battery_percent: runtime.battery_percent,
                    entry_reason: Some(entry_reason.to_string()),
                    exit_reason: None,
                },
            )?;
        }
        None => {
            write_app_session_record(
                workspace_root,
                &MobileAppSessionRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    node_id: runtime.node_id.clone(),
                    status,
                    started_at: now.clone(),
                    last_seen_at: now,
                    ended_at: None,
                    duration_secs: None,
                    network: Some(runtime.network.clone()),
                    battery_percent: runtime.battery_percent,
                    entry_reason: Some(entry_reason.to_string()),
                    exit_reason: None,
                },
            )?;
        }
    }

    Ok(())
}

fn capability_notes(capability: &str) -> Vec<String> {
    match capability {
        "camera" => vec!["bounded photo-capture preview only".to_string()],
        "screen_recording" => vec!["bounded clip preview only".to_string()],
        "location" => vec!["bounded coarse location preview".to_string()],
        "contacts" => vec!["bounded query preview only".to_string()],
        "calendar" => vec!["bounded range query preview only".to_string()],
        "photos" => vec!["bounded picker preview only".to_string()],
        "canvas" => vec!["bounded sketch/canvas preview only".to_string()],
        "sms" => vec!["bounded draft preview only".to_string()],
        "notifications" => vec!["reuses the shipped notification preview lane".to_string()],
        _ => Vec::new(),
    }
}

fn is_preview_supported_capability(capability: &str) -> bool {
    !capability_aliases(capability).is_empty()
}

fn normalize_capability_name(capability: &str) -> String {
    capability.trim().to_lowercase().replace('-', "_")
}

fn build_media_artifact_record(
    record: &MobileCapabilityExecutionRecord,
) -> Option<MobileMediaArtifactRecord> {
    let (media_kind, mime, artifact_kind, summary) = match record.capability.as_str() {
        "camera" => (
            "image",
            "image/jpeg",
            "camera_capture_receipt",
            "bounded camera capture receipt",
        ),
        "screen_recording" => (
            "video",
            "video/mp4",
            "screen_recording_receipt",
            "bounded screen recording receipt",
        ),
        "photos" => (
            "image",
            "image/jpeg",
            "photo_picker_receipt",
            "bounded photo selection receipt",
        ),
        "canvas" => (
            "image",
            "image/png",
            "canvas_export_receipt",
            "bounded canvas export receipt",
        ),
        _ => return None,
    };

    Some(MobileMediaArtifactRecord {
        id: format!("artifact-{}", record.id),
        execution_id: record.id.clone(),
        node_id: record.node_id.clone(),
        capability: record.capability.clone(),
        media_kind: media_kind.to_string(),
        mime: mime.to_string(),
        status: "captured".to_string(),
        created_at: record.created_at.clone(),
        requested_by: record.requested_by.clone(),
        summary: summary.to_string(),
        artifact: json!({
            "kind": artifact_kind,
            "execution_id": record.id,
            "result": record.result,
        }),
    })
}

fn has_capability(advertised: &[String], capability: &str) -> bool {
    let normalized = advertised
        .iter()
        .map(|entry| normalize_capability_name(entry))
        .collect::<Vec<_>>();
    capability_aliases(capability)
        .iter()
        .any(|alias| normalized.iter().any(|entry| entry == alias))
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
    fn pairing_history_and_unpair_round_trip() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-lifecycle".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "IPHONE_LIFECYCLE_TOKEN".to_string(),
                device_name: Some("Lifecycle iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "camera".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let paired = list_pairing_data(temp.path(), Some("iphone-lifecycle"), Some(8))
            .expect("list pairings");
        assert_eq!(paired.len(), 1);
        assert_eq!(paired[0].kind, "paired");

        let result = unpair_node_data(
            temp.path(),
            "iphone-lifecycle",
            MobileUnpairRequest {
                requested_by: Some("operator".to_string()),
                reason: Some("retired device".to_string()),
                remove_runtime_state: true,
            },
        )
        .expect("unpair");
        assert!(result.removed_manifest);
        assert_eq!(result.pairing.kind, "unpaired");
        assert_eq!(result.pairing.requested_by.as_deref(), Some("operator"));

        let pairings = list_pairing_data(temp.path(), Some("iphone-lifecycle"), Some(8))
            .expect("list updated pairings");
        assert_eq!(pairings.len(), 2);
        assert_eq!(pairings[0].kind, "unpaired");
        assert_eq!(pairings[1].kind, "paired");
        assert!(!manifest_path(temp.path(), "iphone-lifecycle").exists());
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
    fn command_timeline_events_capture_receipt_transitions() {
        let record = MobileCommandRecord {
            id: "command-timeline-test".to_string(),
            node_id: "iphone-command".to_string(),
            command: DeviceCommandKind::PushNotification,
            required_capability: "notifications".to_string(),
            approval_required: true,
            status: "executed".to_string(),
            payload: Value::Null,
            result: Value::Null,
            created_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .expect("created_at")
                .with_timezone(&Utc),
            approved_by: Some("operator".to_string()),
            decided_reason: Some("approved".to_string()),
            approved_at: Some(
                DateTime::parse_from_rfc3339("2026-01-01T00:01:00Z")
                    .expect("approved_at")
                    .with_timezone(&Utc),
            ),
            executed_at: Some(
                DateTime::parse_from_rfc3339("2026-01-01T00:02:00Z")
                    .expect("executed_at")
                    .with_timezone(&Utc),
            ),
        };

        let events = command_timeline_events(&record);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, "command_dispatched");
        assert_eq!(events[1].kind, "command_approved");
        assert_eq!(events[2].kind, "command_executed");
    }

    #[tokio::test]
    async fn command_metrics_counts_receipts() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_COMMAND_TOKEN", "token");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-commands".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_COMMAND_TOKEN".to_string(),
                device_name: Some("Commands iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-commands".to_string(),
                command: DeviceCommandKind::PushNotification,
                payload: json!({
                    "title": "Pending",
                    "body": "Awaiting approval"
                }),
                approved_by: None,
                require_approval: Some(true),
            },
        )
        .await
        .expect("dispatch pending command");

        dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-commands".to_string(),
                command: DeviceCommandKind::SyncNow,
                payload: json!({
                    "pending_change_count": 2
                }),
                approved_by: Some("operator".to_string()),
                require_approval: Some(false),
            },
        )
        .await
        .expect("dispatch executed command");

        let metrics = command_metrics_data(temp.path(), Some("iphone-commands"), Some(10))
            .expect("command metrics");
        assert_eq!(metrics.metrics.total_commands, 2);
        assert_eq!(metrics.metrics.pending_approval_commands, 1);
        assert_eq!(metrics.metrics.executed_commands, 1);
        assert_eq!(
            metrics
                .metrics
                .by_command_kind
                .get("push_notification")
                .copied(),
            Some(1)
        );
        assert_eq!(
            metrics.metrics.by_command_kind.get("sync_now").copied(),
            Some(1)
        );
        unsafe {
            std::env::remove_var("MOBILE_COMMAND_TOKEN");
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

    #[test]
    fn register_push_updates_runtime_receipt() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-push".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_PUSH_TOKEN".to_string(),
                device_name: Some("Push iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let push = register_push_data(
            temp.path(),
            "iphone-push",
            MobilePushRegistrationRequest {
                push_provider: Some("apns".to_string()),
                push_token_present: Some(true),
                notifications_authorized: Some(true),
            },
        )
        .expect("register push");

        assert!(push.push_token_present);
        assert!(push.notifications_authorized);
        assert_eq!(push.push_provider.as_deref(), Some("apns"));
        assert!(push.push_token_updated_at.is_some());
    }

    #[test]
    fn report_sync_updates_runtime_receipt() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-sync".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_SYNC_TOKEN".to_string(),
                device_name: Some("Sync iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let sync = report_sync_data(
            temp.path(),
            "iphone-sync",
            MobileSyncReportRequest {
                sync_state: Some("synced".to_string()),
                pending_change_count: Some(0),
                last_sync_result: Some("ok".to_string()),
            },
        )
        .expect("report sync");

        assert_eq!(sync.sync_state, "synced");
        assert_eq!(sync.pending_change_count, Some(0));
        assert_eq!(sync.last_sync_result.as_deref(), Some("ok"));
        assert!(sync.last_sync_at.is_some());
        assert_eq!(sync.pending_conflict_count, 0);
        assert_eq!(sync.resolved_conflict_count, 0);
    }

    #[test]
    fn heartbeat_creates_and_rotates_app_sessions() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-app-session".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_APP_SESSION_TOKEN".to_string(),
                device_name: Some("Session iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        heartbeat_node_data(
            temp.path(),
            "iphone-app-session",
            MobileHeartbeatRequest {
                app_state: Some("active".to_string()),
                network: Some("wifi".to_string()),
                reachable: Some(true),
                push_token_present: Some(true),
                battery_percent: Some(90),
                metadata: Value::Null,
            },
        )
        .expect("active heartbeat");
        heartbeat_node_data(
            temp.path(),
            "iphone-app-session",
            MobileHeartbeatRequest {
                app_state: Some("background".to_string()),
                network: Some("wifi".to_string()),
                reachable: Some(true),
                push_token_present: Some(true),
                battery_percent: Some(88),
                metadata: Value::Null,
            },
        )
        .expect("background heartbeat");

        let sessions =
            list_app_session_data(temp.path(), Some("iphone-app-session"), None, Some(8))
                .expect("sessions");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].status, "background");
        assert!(sessions[0].ended_at.is_none());
        assert_eq!(sessions[1].status, "active");
        assert!(sessions[1].ended_at.is_some());
    }

    #[test]
    fn sync_conflicts_can_be_reported_and_resolved() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-conflicts".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_CONFLICT_TOKEN".to_string(),
                device_name: Some("Conflict iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let conflict = report_sync_conflict_data(
            temp.path(),
            MobileSyncConflictReportRequest {
                node_id: "iphone-conflicts".to_string(),
                item_key: "calendar:event:42".to_string(),
                conflict_type: "calendar_conflict".to_string(),
                summary: Some("calendar event changed on both ends".to_string()),
                details: Some("server and device both edited the same event".to_string()),
                resolution_hint: Some("prefer_server".to_string()),
            },
        )
        .expect("report conflict");

        let pending = list_sync_conflict_data(
            temp.path(),
            Some("iphone-conflicts"),
            Some("pending"),
            Some(8),
        )
        .expect("list conflicts");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, conflict.id);

        let sync = node_sync_state_data(temp.path(), "iphone-conflicts").expect("sync");
        assert_eq!(sync.pending_conflict_count, 1);
        assert_eq!(sync.resolved_conflict_count, 0);

        let resolved = resolve_sync_conflict_data(
            temp.path(),
            &conflict.id,
            MobileSyncConflictResolveRequest {
                resolved_by: "operator".to_string(),
                resolution: Some("prefer_server".to_string()),
            },
        )
        .expect("resolve conflict");
        assert_eq!(resolved.status, "resolved");
        assert_eq!(resolved.resolved_by.as_deref(), Some("operator"));

        let sync = node_sync_state_data(temp.path(), "iphone-conflicts").expect("sync");
        assert_eq!(sync.pending_conflict_count, 0);
        assert_eq!(sync.resolved_conflict_count, 1);
    }

    #[tokio::test]
    async fn mobile_activity_aggregates_runtime_and_receipts() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_ACTIVITY_TOKEN", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-activity".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_ACTIVITY_TOKEN".to_string(),
                device_name: Some("Activity iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec![
                    "mobile".to_string(),
                    "camera".to_string(),
                    "notifications".to_string(),
                    "sms".to_string(),
                ],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        heartbeat_node_data(
            temp.path(),
            "iphone-activity",
            MobileHeartbeatRequest {
                app_state: Some("active".to_string()),
                network: Some("wifi".to_string()),
                reachable: Some(true),
                push_token_present: Some(true),
                battery_percent: Some(91),
                metadata: Value::Null,
            },
        )
        .expect("heartbeat");
        register_push_data(
            temp.path(),
            "iphone-activity",
            MobilePushRegistrationRequest {
                push_provider: Some("apns".to_string()),
                push_token_present: Some(true),
                notifications_authorized: Some(true),
            },
        )
        .expect("push registration");
        report_sync_data(
            temp.path(),
            "iphone-activity",
            MobileSyncReportRequest {
                sync_state: Some("synced".to_string()),
                pending_change_count: Some(0),
                last_sync_result: Some("ok".to_string()),
            },
        )
        .expect("sync report");
        report_sync_conflict_data(
            temp.path(),
            MobileSyncConflictReportRequest {
                node_id: "iphone-activity".to_string(),
                item_key: "contacts:alice".to_string(),
                conflict_type: "contacts_conflict".to_string(),
                summary: Some("contact changed on device and server".to_string()),
                details: None,
                resolution_hint: Some("manual_review".to_string()),
            },
        )
        .expect("sync conflict");
        report_inbound_message_data(
            temp.path(),
            MobileInboundMessageReportRequest {
                node_id: "iphone-activity".to_string(),
                source: "ops-room".to_string(),
                target: "assistant".to_string(),
                content: "ping from mobile".to_string(),
                content_type: None,
                metadata: Value::Null,
            },
        )
        .expect("report inbound");
        send_outbound_message_data(
            temp.path(),
            MobileOutboundMessageSendRequest {
                node_id: "iphone-activity".to_string(),
                target: "ops-room".to_string(),
                content: "pong to mobile".to_string(),
                content_type: None,
                requested_by: Some("tester".to_string()),
                metadata: Value::Null,
            },
        )
        .await
        .expect("send outbound");
        send_notification_data(
            temp.path(),
            MobileNotificationSendRequest {
                node_id: "iphone-activity".to_string(),
                title: "Heads up".to_string(),
                body: "Activity notification".to_string(),
                priority: Some("normal".to_string()),
                notification_type: Some("message".to_string()),
                data: HashMap::new(),
                requested_by: Some("tester".to_string()),
            },
        )
        .await
        .expect("send notification");
        dispatch_command_data(
            temp.path(),
            MobileCommandDispatchRequest {
                node_id: "iphone-activity".to_string(),
                command: DeviceCommandKind::SendMessage,
                payload: json!({"target":"ops-room","content":"command path"}),
                approved_by: Some("tester".to_string()),
                require_approval: Some(false),
            },
        )
        .await
        .expect("dispatch command");
        execute_capability_data(
            temp.path(),
            MobileCapabilityExecuteRequest {
                node_id: "iphone-activity".to_string(),
                capability: "camera".to_string(),
                target: Some("activity-roll".to_string()),
                query: None,
                note: Some("capture for activity timeline".to_string()),
                requested_by: Some("tester".to_string()),
            },
        )
        .expect("execute capability");

        let activity =
            node_activity_data(temp.path(), "iphone-activity", Some(16)).expect("activity");
        let kinds = activity
            .entries
            .iter()
            .map(|entry| entry.kind.as_str())
            .collect::<Vec<_>>();

        assert!(kinds.contains(&"node_paired"));
        assert!(kinds.contains(&"app_session"));
        assert!(kinds.contains(&"runtime_heartbeat"));
        assert!(kinds.contains(&"push_registration"));
        assert!(kinds.contains(&"sync_result"));
        assert!(kinds.contains(&"sync_conflict"));
        assert!(kinds.contains(&"notification"));
        assert!(kinds.contains(&"inbound_message"));
        assert!(kinds.contains(&"outbound_message"));
        assert!(kinds.contains(&"capability_execution"));
        assert!(kinds.contains(&"media_artifact"));
        assert!(kinds.contains(&"command"));
        assert!(activity.entry_count >= 7);
        unsafe {
            std::env::remove_var("MOBILE_ACTIVITY_TOKEN");
        }
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

    #[test]
    fn node_capabilities_reports_advertised_preview_lanes() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-capabilities".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_CAPABILITY_TOKEN".to_string(),
                device_name: Some("Capability iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec![
                    "mobile".to_string(),
                    "camera".to_string(),
                    "location".to_string(),
                    "photos".to_string(),
                ],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let result = node_capabilities_data(temp.path(), "iphone-capabilities")
            .expect("capability inventory");
        assert_eq!(result.node_id, "iphone-capabilities");
        assert!(
            result
                .capabilities
                .iter()
                .any(|entry| entry.capability == "camera" && entry.advertised)
        );
        assert!(
            result
                .capabilities
                .iter()
                .any(|entry| entry.capability == "location" && entry.advertised)
        );
        assert!(
            result
                .capabilities
                .iter()
                .any(|entry| entry.capability == "calendar" && !entry.advertised)
        );
    }

    #[test]
    fn preview_capability_persists_bounded_receipt() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-preview".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_PREVIEW_TOKEN".to_string(),
                device_name: Some("Preview iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "camera".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = preview_capability_data(
            temp.path(),
            MobileCapabilityPreviewRequest {
                node_id: "iphone-preview".to_string(),
                capability: "camera".to_string(),
                target: None,
                query: None,
                note: Some("operator preview".to_string()),
            },
        )
        .expect("capability preview");

        assert_eq!(record.node_id, "iphone-preview");
        assert_eq!(record.capability, "camera");
        assert_eq!(record.preview["preview"]["kind"], "camera_capture_preview");
        assert!(capability_preview_path(temp.path(), &record.id).exists());
    }

    #[test]
    fn execute_capability_persists_bounded_receipt() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-execution".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_EXECUTION_TOKEN".to_string(),
                device_name: Some("Execution iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "camera".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = execute_capability_data(
            temp.path(),
            MobileCapabilityExecuteRequest {
                node_id: "iphone-execution".to_string(),
                capability: "camera".to_string(),
                target: Some("camera_roll".to_string()),
                query: None,
                note: Some("operator execution".to_string()),
                requested_by: Some("tester".to_string()),
            },
        )
        .expect("capability execution");

        assert_eq!(record.node_id, "iphone-execution");
        assert_eq!(record.capability, "camera");
        assert_eq!(record.status, "executed");
        assert_eq!(record.result["preview"]["kind"], "camera_capture_execution");
        assert!(capability_execution_path(temp.path(), &record.id).exists());

        let inspected =
            inspect_capability_execution_data(temp.path(), &record.id).expect("inspect execution");
        assert_eq!(inspected.id, record.id);

        let executions = list_capability_execution_data(
            temp.path(),
            Some("iphone-execution"),
            Some("camera"),
            Some(10),
        )
        .expect("list executions");
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].id, record.id);

        let artifacts = list_media_artifact_data(
            temp.path(),
            Some("iphone-execution"),
            Some("camera"),
            Some(10),
        )
        .expect("list artifacts");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].execution_id, record.id);
        assert_eq!(artifacts[0].media_kind, "image");

        let inspected_artifact =
            inspect_media_artifact_data(temp.path(), &artifacts[0].id).expect("inspect artifact");
        assert_eq!(inspected_artifact.id, artifacts[0].id);
    }

    #[tokio::test]
    async fn send_notification_persists_runtime_receipt() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_NOTIFY_TOKEN", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-notify".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_NOTIFY_TOKEN".to_string(),
                device_name: Some("Notify iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");
        register_push_data(
            temp.path(),
            "iphone-notify",
            MobilePushRegistrationRequest {
                push_provider: Some("apns".to_string()),
                push_token_present: Some(true),
                notifications_authorized: Some(true),
            },
        )
        .expect("register push");

        let record = send_notification_data(
            temp.path(),
            MobileNotificationSendRequest {
                node_id: "iphone-notify".to_string(),
                title: "Studio alert".to_string(),
                body: "Track export finished".to_string(),
                priority: Some("high".to_string()),
                notification_type: Some("update".to_string()),
                data: HashMap::from([(String::from("job"), String::from("export"))]),
                requested_by: Some("test".to_string()),
            },
        )
        .await
        .expect("send notification");

        assert_eq!(record.status, "dispatched");
        assert!(record.command_id.is_some());

        let runtime = node_runtime_data(temp.path(), "iphone-notify").expect("runtime");
        assert_eq!(runtime.pending_notification_count, 1);
        assert!(runtime.last_notification_at.is_some());
        unsafe {
            std::env::remove_var("MOBILE_NOTIFY_TOKEN");
        }
    }

    #[tokio::test]
    async fn acknowledge_notification_updates_runtime_counters() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_NOTIFY_ACK_TOKEN", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-notify-ack".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_NOTIFY_ACK_TOKEN".to_string(),
                device_name: Some("Ack iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = send_notification_data(
            temp.path(),
            MobileNotificationSendRequest {
                node_id: "iphone-notify-ack".to_string(),
                title: "Ops".to_string(),
                body: "Acknowledge me".to_string(),
                priority: None,
                notification_type: None,
                data: HashMap::new(),
                requested_by: Some("test".to_string()),
            },
        )
        .await
        .expect("send notification");

        let acknowledged = acknowledge_notification_data(
            temp.path(),
            &record.id,
            MobileNotificationAckRequest {
                acknowledged_by: "operator".to_string(),
            },
        )
        .expect("acknowledge");

        assert_eq!(acknowledged.status, "acknowledged");
        assert_eq!(acknowledged.acknowledged_by.as_deref(), Some("operator"));

        let runtime = node_runtime_data(temp.path(), "iphone-notify-ack").expect("runtime");
        assert_eq!(runtime.pending_notification_count, 0);
        assert_eq!(runtime.delivered_notification_count, 1);
        unsafe {
            std::env::remove_var("MOBILE_NOTIFY_ACK_TOKEN");
        }
    }

    #[test]
    fn report_inbound_message_persists_runtime_receipt() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-inbox".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_INBOX_TOKEN".to_string(),
                device_name: Some("Inbox iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string(), "notifications".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = report_inbound_message_data(
            temp.path(),
            MobileInboundMessageReportRequest {
                node_id: "iphone-inbox".to_string(),
                source: "+14165550123".to_string(),
                target: "main".to_string(),
                content: "Studio status changed".to_string(),
                content_type: Some("text/plain".to_string()),
                metadata: json!({ "transport": "sms" }),
            },
        )
        .expect("report inbound message");

        assert_eq!(record.status, "reported");
        assert_eq!(record.source, "+14165550123");
        assert_eq!(record.preview["target"], "main");

        let runtime = node_runtime_data(temp.path(), "iphone-inbox").expect("runtime");
        assert_eq!(runtime.pending_inbound_message_count, 1);
        assert!(runtime.last_inbound_message_at.is_some());
    }

    #[test]
    fn acknowledge_inbound_message_updates_runtime_counters() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-inbox-ack".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_INBOX_ACK_TOKEN".to_string(),
                device_name: Some("Inbox Ack iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = report_inbound_message_data(
            temp.path(),
            MobileInboundMessageReportRequest {
                node_id: "iphone-inbox-ack".to_string(),
                source: "ops-room".to_string(),
                target: "main".to_string(),
                content: "Ack me".to_string(),
                content_type: None,
                metadata: Value::Null,
            },
        )
        .expect("report inbound message");

        let acknowledged = acknowledge_inbound_message_data(
            temp.path(),
            &record.id,
            MobileInboundMessageAckRequest {
                acknowledged_by: "operator".to_string(),
            },
        )
        .expect("acknowledge inbound message");

        assert_eq!(acknowledged.status, "acknowledged");
        assert_eq!(acknowledged.acknowledged_by.as_deref(), Some("operator"));

        let runtime = node_runtime_data(temp.path(), "iphone-inbox-ack").expect("runtime");
        assert_eq!(runtime.pending_inbound_message_count, 0);
        assert_eq!(runtime.acknowledged_inbound_message_count, 1);
    }

    #[tokio::test]
    async fn send_outbound_message_persists_runtime_receipt() {
        let temp = tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("MOBILE_OUTBOX_TOKEN", "secret");
        }
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-outbox".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_OUTBOX_TOKEN".to_string(),
                device_name: Some("Outbox iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = send_outbound_message_data(
            temp.path(),
            MobileOutboundMessageSendRequest {
                node_id: "iphone-outbox".to_string(),
                target: "ops-room".to_string(),
                content: "Outbound hello".to_string(),
                content_type: Some("text/plain".to_string()),
                requested_by: Some("test".to_string()),
                metadata: json!({ "transport": "chat" }),
            },
        )
        .await
        .expect("send outbound message");

        assert!(matches!(record.status.as_str(), "queued" | "dispatched"));
        if record.status == "dispatched" {
            assert!(record.command_id.is_some());
        }

        let runtime = node_runtime_data(temp.path(), "iphone-outbox").expect("runtime");
        assert_eq!(runtime.pending_outbound_message_count, 1);
        assert!(runtime.last_outbound_message_at.is_some());
        unsafe {
            std::env::remove_var("MOBILE_OUTBOX_TOKEN");
        }
    }

    #[test]
    fn acknowledge_outbound_message_updates_runtime_counters() {
        let temp = tempdir().expect("tempdir");
        pair_node_data(
            temp.path(),
            MobilePairRequest {
                id: "iphone-outbox-ack".to_string(),
                gateway_url: "wss://example.com/gateway".to_string(),
                auth_token_env: "MOBILE_OUTBOX_ACK_TOKEN".to_string(),
                device_name: Some("Outbox Ack iPhone".to_string()),
                platform: Some("ios".to_string()),
                capabilities: vec!["mobile".to_string()],
                enabled: true,
                sync: None,
                notifications: None,
                metadata: Value::Null,
            },
        )
        .expect("pair node");

        let record = report_inbound_message_data(
            temp.path(),
            MobileInboundMessageReportRequest {
                node_id: "iphone-outbox-ack".to_string(),
                source: "ops-room".to_string(),
                target: "main".to_string(),
                content: "seed".to_string(),
                content_type: None,
                metadata: Value::Null,
            },
        )
        .expect("seed runtime");
        assert_eq!(record.status, "reported");

        let outbound = MobileOutboundMessageRecord {
            id: "outbound-ack".to_string(),
            node_id: "iphone-outbox-ack".to_string(),
            target: "ops-room".to_string(),
            status: "dispatched".to_string(),
            content_type: Some("text/plain".to_string()),
            content_preview: "Ack me".to_string(),
            bytes: 6,
            created_at: Utc::now().to_rfc3339(),
            dispatched_at: Some(Utc::now().to_rfc3339()),
            acknowledged_at: None,
            acknowledged_by: None,
            command_id: None,
            metadata: Value::Null,
            preview: json!({ "target": "ops-room" }),
        };
        write_outbound_message_record(temp.path(), &outbound).expect("write outbound");

        let mut runtime = node_runtime_data(temp.path(), "iphone-outbox-ack").expect("runtime");
        runtime.pending_outbound_message_count = 1;
        save_runtime_state(temp.path(), &runtime).expect("save runtime");

        let acknowledged = acknowledge_outbound_message_data(
            temp.path(),
            "outbound-ack",
            MobileOutboundMessageAckRequest {
                acknowledged_by: "operator".to_string(),
            },
        )
        .expect("acknowledge outbound message");

        assert_eq!(acknowledged.status, "acknowledged");
        assert_eq!(acknowledged.acknowledged_by.as_deref(), Some("operator"));

        let runtime = node_runtime_data(temp.path(), "iphone-outbox-ack").expect("runtime");
        assert_eq!(runtime.pending_outbound_message_count, 0);
        assert_eq!(runtime.acknowledged_outbound_message_count, 1);
    }
}
