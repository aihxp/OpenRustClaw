use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeManifest {
    pub version: u32,
    pub node: MobileNodeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeSpec {
    pub id: String,
    pub gateway_url: String,
    pub auth_token_env: String,
    pub device_name: String,
    pub enabled: bool,
    pub platform: String,
    pub capabilities: Vec<String>,
    pub sync: MobileSyncSpec,
    pub notifications: MobileNotificationSpec,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileSyncSpec {
    pub mode: String,
    pub priority: String,
    pub conflict_resolution: String,
    pub max_sync_interval_secs: u64,
    pub min_battery_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNotificationSpec {
    pub enabled: bool,
    pub apns_enabled: bool,
    pub fcm_enabled: bool,
    pub show_badge: bool,
    pub play_sound: bool,
    pub sound_name: Option<String>,
    pub vibration: bool,
    pub batch_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeRuntimeState {
    pub node_id: String,
    pub runtime_status: String,
    pub app_state: String,
    pub network: String,
    pub reachable: bool,
    pub push_token_present: bool,
    pub notifications_authorized: bool,
    pub push_provider: Option<String>,
    pub push_token_updated_at: Option<String>,
    pub battery_percent: Option<u8>,
    pub last_heartbeat_at: Option<String>,
    pub wake_state: String,
    pub wake_requested_at: Option<String>,
    pub wake_requested_by: Option<String>,
    pub wake_reason: Option<String>,
    pub last_wake_command_id: Option<String>,
    pub rehydrate_state: String,
    pub rehydrate_requested_at: Option<String>,
    pub rehydrate_requested_by: Option<String>,
    pub rehydrate_reason: Option<String>,
    pub rehydrate_pending_change_count: Option<usize>,
    pub last_rehydrate_command_id: Option<String>,
    pub sync_state: String,
    pub pending_change_count: Option<usize>,
    pub last_sync_requested_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_sync_result: Option<String>,
    pub pending_notification_count: usize,
    pub delivered_notification_count: usize,
    pub last_notification_at: Option<String>,
    pub pending_inbound_message_count: usize,
    pub acknowledged_inbound_message_count: usize,
    pub last_inbound_message_at: Option<String>,
    pub pending_outbound_message_count: usize,
    pub acknowledged_outbound_message_count: usize,
    pub last_outbound_message_at: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MobileAppSessionMetricsSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub by_status: HashMap<String, usize>,
    pub avg_duration_secs: Option<f64>,
    pub newest_session_at: Option<String>,
    pub oldest_session_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodePushState {
    pub node_id: String,
    pub push_token_present: bool,
    pub notifications_authorized: bool,
    pub push_provider: Option<String>,
    pub push_token_updated_at: Option<String>,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeSyncState {
    pub node_id: String,
    pub sync_state: String,
    pub pending_change_count: Option<usize>,
    pub pending_conflict_count: usize,
    pub resolved_conflict_count: usize,
    pub last_sync_requested_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_sync_result: Option<String>,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeActivityEntry {
    pub kind: String,
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MobileCommandMetricsSummary {
    pub total_commands: usize,
    pub pending_approval_commands: usize,
    pub approved_commands: usize,
    pub executed_commands: usize,
    pub rejected_commands: usize,
    pub by_command_kind: HashMap<String, usize>,
    pub avg_execution_latency_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub active_app_sessions: usize,
    pub ended_app_sessions: usize,
    pub sync_conflicts: usize,
    pub notifications: usize,
    pub inbox_messages: usize,
    pub outbox_messages: usize,
    pub commands: usize,
    pub capability_executions: usize,
    pub media_artifacts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileOperatorAttentionSignal {
    pub kind: String,
    pub severity: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MobileNodeOperatorReport {
    pub status: String,
    pub manifest: MobileNodeManifest,
    pub node_status: MobileNodeStatus,
    pub summary: MobileNodeSummary,
    pub runtime: MobileNodeRuntimeState,
    pub push: MobileNodePushState,
    pub sync: MobileNodeSyncState,
    pub app_session_metrics: MobileAppSessionMetricsSummary,
    pub command_metrics: MobileCommandMetricsSummary,
    pub recent_activity: Vec<MobileNodeActivityEntry>,
    pub attention_signals: Vec<MobileOperatorAttentionSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MobileNodeOperatorState {
    pub manifest: MobileNodeManifest,
    pub node_status: MobileNodeStatus,
    pub summary: MobileNodeSummary,
    pub runtime: MobileNodeRuntimeState,
    pub push: MobileNodePushState,
    pub sync: MobileNodeSyncState,
    pub app_session_metrics: MobileAppSessionMetricsSummary,
    pub command_metrics: MobileCommandMetricsSummary,
    pub recent_activity: Vec<MobileNodeActivityEntry>,
}

pub trait MobileNodeOperatorSource {
    fn load_mobile_node_operator_state(&self) -> Result<MobileNodeOperatorState>;
}

pub struct MobileNodeOperatorService<S> {
    source: S,
}

impl<S> MobileNodeOperatorService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> MobileNodeOperatorService<S>
where
    S: MobileNodeOperatorSource,
{
    pub fn report(&self) -> Result<MobileNodeOperatorReport> {
        let state = self.source.load_mobile_node_operator_state()?;
        let attention_signals = mobile_attention_signals(
            &state.node_status,
            &state.summary,
            &state.runtime,
            &state.push,
            &state.sync,
            &state.app_session_metrics,
            &state.command_metrics,
        );

        Ok(MobileNodeOperatorReport {
            status: "ok".to_string(),
            manifest: state.manifest,
            node_status: state.node_status,
            summary: state.summary,
            runtime: state.runtime,
            push: state.push,
            sync: state.sync,
            app_session_metrics: state.app_session_metrics,
            command_metrics: state.command_metrics,
            recent_activity: state.recent_activity,
            attention_signals,
        })
    }
}

fn mobile_attention_signals(
    node_status: &MobileNodeStatus,
    summary: &MobileNodeSummary,
    runtime: &MobileNodeRuntimeState,
    push: &MobileNodePushState,
    sync: &MobileNodeSyncState,
    app_session_metrics: &MobileAppSessionMetricsSummary,
    command_metrics: &MobileCommandMetricsSummary,
) -> Vec<MobileOperatorAttentionSignal> {
    let mut signals = Vec::new();

    if node_status.readiness != "ready_for_runtime" {
        signals.push(MobileOperatorAttentionSignal {
            kind: "node_readiness".to_string(),
            severity: "high".to_string(),
            summary: format!("node readiness is {}", node_status.readiness),
        });
    }
    if !runtime.reachable {
        signals.push(MobileOperatorAttentionSignal {
            kind: "connectivity".to_string(),
            severity: "high".to_string(),
            summary: "node is not currently reachable".to_string(),
        });
    }
    if !push.push_token_present {
        signals.push(MobileOperatorAttentionSignal {
            kind: "push_registration".to_string(),
            severity: "medium".to_string(),
            summary: "push token is missing".to_string(),
        });
    }
    if !push.notifications_authorized {
        signals.push(MobileOperatorAttentionSignal {
            kind: "notification_authorization".to_string(),
            severity: "medium".to_string(),
            summary: "device notifications are not authorized".to_string(),
        });
    }
    if sync.pending_conflict_count > 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "sync_conflicts".to_string(),
            severity: "high".to_string(),
            summary: format!(
                "{} sync conflict(s) need resolution",
                sync.pending_conflict_count
            ),
        });
    }
    if command_metrics.pending_approval_commands > 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "pending_approval".to_string(),
            severity: "high".to_string(),
            summary: format!(
                "{} mobile command(s) are waiting for approval",
                command_metrics.pending_approval_commands
            ),
        });
    }
    if matches!(runtime.wake_state.as_str(), "requested" | "dispatched") {
        signals.push(MobileOperatorAttentionSignal {
            kind: "wake_request".to_string(),
            severity: "medium".to_string(),
            summary: format!("wake request is {}", runtime.wake_state),
        });
    }
    if runtime.rehydrate_state == "requested" {
        signals.push(MobileOperatorAttentionSignal {
            kind: "rehydrate_request".to_string(),
            severity: "medium".to_string(),
            summary: "rehydrate has been requested but not completed".to_string(),
        });
    }
    if runtime.pending_notification_count > 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "notifications_pending".to_string(),
            severity: "low".to_string(),
            summary: format!(
                "{} notification(s) remain pending acknowledgement",
                runtime.pending_notification_count
            ),
        });
    }
    if runtime.pending_inbound_message_count > 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "inbox_backlog".to_string(),
            severity: "low".to_string(),
            summary: format!(
                "{} inbound mobile message(s) remain unacknowledged",
                runtime.pending_inbound_message_count
            ),
        });
    }
    if runtime.pending_outbound_message_count > 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "outbox_backlog".to_string(),
            severity: "low".to_string(),
            summary: format!(
                "{} outbound mobile message(s) remain unacknowledged",
                runtime.pending_outbound_message_count
            ),
        });
    }
    if summary.capability_executions > 0 && summary.media_artifacts == 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "capability_artifacts".to_string(),
            severity: "low".to_string(),
            summary: "capability executions exist without derived media artifacts yet".to_string(),
        });
    }
    if runtime.reachable && app_session_metrics.active_sessions == 0 {
        signals.push(MobileOperatorAttentionSignal {
            kind: "app_sessions".to_string(),
            severity: "low".to_string(),
            summary: "node is reachable but has no active app sessions".to_string(),
        });
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        state: MobileNodeOperatorState,
    }

    impl MobileNodeOperatorSource for StubSource {
        fn load_mobile_node_operator_state(&self) -> Result<MobileNodeOperatorState> {
            Ok(self.state.clone())
        }
    }

    #[test]
    fn report_builds_attention_signals_from_mobile_state() -> Result<()> {
        let service = MobileNodeOperatorService::new(StubSource {
            state: MobileNodeOperatorState {
                manifest: MobileNodeManifest {
                    version: 1,
                    node: MobileNodeSpec {
                        id: "ios-1".to_string(),
                        gateway_url: "wss://example.com/gateway".to_string(),
                        auth_token_env: "MOBILE_TOKEN".to_string(),
                        device_name: "Test iPhone".to_string(),
                        enabled: true,
                        platform: "ios".to_string(),
                        capabilities: vec!["mobile".to_string()],
                        sync: MobileSyncSpec {
                            mode: "push_pull".to_string(),
                            priority: "balanced".to_string(),
                            conflict_resolution: "manual".to_string(),
                            max_sync_interval_secs: 60,
                            min_battery_percent: 20,
                        },
                        notifications: MobileNotificationSpec {
                            enabled: true,
                            apns_enabled: true,
                            fcm_enabled: false,
                            show_badge: true,
                            play_sound: true,
                            sound_name: None,
                            vibration: true,
                            batch_interval_secs: 30,
                        },
                        metadata: Value::Null,
                    },
                },
                node_status: MobileNodeStatus {
                    id: "ios-1".to_string(),
                    enabled: true,
                    platform: "ios".to_string(),
                    gateway_url: "wss://example.com/gateway".to_string(),
                    device_name: "Test iPhone".to_string(),
                    capabilities: vec!["mobile".to_string()],
                    auth_token_env: "MOBILE_TOKEN".to_string(),
                    auth_token_present: true,
                    readiness: "ready_for_runtime".to_string(),
                    sync: MobileSyncSpec {
                        mode: "push_pull".to_string(),
                        priority: "balanced".to_string(),
                        conflict_resolution: "manual".to_string(),
                        max_sync_interval_secs: 60,
                        min_battery_percent: 20,
                    },
                    notifications: MobileNotificationSpec {
                        enabled: true,
                        apns_enabled: true,
                        fcm_enabled: false,
                        show_badge: true,
                        play_sound: true,
                        sound_name: None,
                        vibration: true,
                        batch_interval_secs: 30,
                    },
                },
                summary: MobileNodeSummary {
                    node_id: "ios-1".to_string(),
                    runtime_status: "online".to_string(),
                    app_state: "active".to_string(),
                    network: "wifi".to_string(),
                    reachable: false,
                    push_token_present: false,
                    notifications_authorized: false,
                    wake_state: "idle".to_string(),
                    rehydrate_state: "requested".to_string(),
                    sync_state: "degraded".to_string(),
                    battery_percent: Some(85),
                    pairings: 1,
                    app_sessions: 0,
                    active_app_sessions: 0,
                    ended_app_sessions: 0,
                    sync_conflicts: 2,
                    notifications: 0,
                    inbox_messages: 1,
                    outbox_messages: 1,
                    commands: 1,
                    capability_executions: 1,
                    media_artifacts: 0,
                },
                runtime: MobileNodeRuntimeState {
                    node_id: "ios-1".to_string(),
                    runtime_status: "online".to_string(),
                    app_state: "active".to_string(),
                    network: "wifi".to_string(),
                    reachable: false,
                    push_token_present: false,
                    notifications_authorized: false,
                    push_provider: None,
                    push_token_updated_at: None,
                    battery_percent: Some(85),
                    last_heartbeat_at: None,
                    wake_state: "idle".to_string(),
                    wake_requested_at: None,
                    wake_requested_by: None,
                    wake_reason: None,
                    last_wake_command_id: None,
                    rehydrate_state: "requested".to_string(),
                    rehydrate_requested_at: None,
                    rehydrate_requested_by: None,
                    rehydrate_reason: None,
                    rehydrate_pending_change_count: None,
                    last_rehydrate_command_id: None,
                    sync_state: "degraded".to_string(),
                    pending_change_count: Some(3),
                    last_sync_requested_at: None,
                    last_sync_at: None,
                    last_sync_result: Some("conflicted".to_string()),
                    pending_notification_count: 1,
                    delivered_notification_count: 0,
                    last_notification_at: None,
                    pending_inbound_message_count: 1,
                    acknowledged_inbound_message_count: 0,
                    last_inbound_message_at: None,
                    pending_outbound_message_count: 1,
                    acknowledged_outbound_message_count: 0,
                    last_outbound_message_at: None,
                    metadata: Value::Null,
                },
                push: MobileNodePushState {
                    node_id: "ios-1".to_string(),
                    push_token_present: false,
                    notifications_authorized: false,
                    push_provider: None,
                    push_token_updated_at: None,
                    runtime_status: "online".to_string(),
                },
                sync: MobileNodeSyncState {
                    node_id: "ios-1".to_string(),
                    sync_state: "degraded".to_string(),
                    pending_change_count: Some(3),
                    pending_conflict_count: 2,
                    resolved_conflict_count: 0,
                    last_sync_requested_at: None,
                    last_sync_at: None,
                    last_sync_result: Some("conflicted".to_string()),
                    runtime_status: "online".to_string(),
                },
                app_session_metrics: MobileAppSessionMetricsSummary {
                    total_sessions: 0,
                    active_sessions: 0,
                    ended_sessions: 0,
                    by_status: HashMap::new(),
                    avg_duration_secs: None,
                    newest_session_at: None,
                    oldest_session_at: None,
                },
                command_metrics: MobileCommandMetricsSummary {
                    total_commands: 1,
                    pending_approval_commands: 1,
                    approved_commands: 0,
                    executed_commands: 0,
                    rejected_commands: 0,
                    by_command_kind: HashMap::new(),
                    avg_execution_latency_secs: None,
                },
                recent_activity: vec![MobileNodeActivityEntry {
                    kind: "runtime_heartbeat".to_string(),
                    id: "hb-1".to_string(),
                    status: "ok".to_string(),
                    created_at: "2026-03-28T00:00:00Z".to_string(),
                    summary: "heartbeat".to_string(),
                }],
            },
        });

        let report = service.report()?;
        assert_eq!(report.status, "ok");
        assert_eq!(report.recent_activity.len(), 1);
        assert!(
            report
                .attention_signals
                .iter()
                .any(|signal| signal.kind == "connectivity")
        );
        assert!(
            report
                .attention_signals
                .iter()
                .any(|signal| signal.kind == "sync_conflicts")
        );
        assert!(
            report
                .attention_signals
                .iter()
                .any(|signal| signal.kind == "pending_approval")
        );
        Ok(())
    }
}
