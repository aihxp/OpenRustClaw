use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub last_notification_at: Option<String>,
    pub pending_notification_count: usize,
    pub delivered_notification_count: usize,
    pub last_inbound_message_at: Option<String>,
    pub pending_inbound_message_count: usize,
    pub acknowledged_inbound_message_count: usize,
    pub last_outbound_message_at: Option<String>,
    pub pending_outbound_message_count: usize,
    pub acknowledged_outbound_message_count: usize,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileHeartbeatRequest {
    pub app_state: Option<String>,
    pub network: Option<String>,
    pub reachable: Option<bool>,
    pub push_token_present: Option<bool>,
    pub battery_percent: Option<u8>,
    pub metadata: Value,
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
pub struct MobilePushRegistrationRequest {
    pub push_provider: Option<String>,
    pub push_token_present: Option<bool>,
    pub notifications_authorized: Option<bool>,
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
pub struct MobileSyncReportRequest {
    pub sync_state: Option<String>,
    pub pending_change_count: Option<usize>,
    pub last_sync_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNodeActivityEntry {
    pub kind: String,
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub summary: String,
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
pub struct MobileNodeSummaryCounts {
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
pub struct MobileMetricsSummary {
    pub total_nodes: usize,
    pub paired_records: usize,
    pub unpaired_records: usize,
    pub reachable_nodes: usize,
    pub push_token_present_nodes: usize,
    pub notifications_authorized_nodes: usize,
    pub waking_nodes: usize,
    pub rehydrate_pending_nodes: usize,
    pub total_app_sessions: usize,
    pub active_app_sessions: usize,
    pub ended_app_sessions: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileMetricsCounts {
    pub paired_records: usize,
    pub unpaired_records: usize,
    pub total_app_sessions: usize,
    pub active_app_sessions: usize,
    pub ended_app_sessions: usize,
    pub commands: usize,
    pub sync_conflicts: usize,
    pub capability_executions: usize,
    pub media_artifacts: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MobileRuntimeStatusService;

impl MobileRuntimeStatusService {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh_runtime_status(&self, runtime: &mut MobileNodeRuntimeState) {
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

    pub fn heartbeat(
        &self,
        mut runtime: MobileNodeRuntimeState,
        request: MobileHeartbeatRequest,
        now: String,
    ) -> MobileNodeRuntimeState {
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
            if runtime.rehydrate_state == "requested" {
                runtime.rehydrate_state = "ready".to_string();
            }
        }
        self.refresh_runtime_status(&mut runtime);
        runtime
    }

    pub fn push_state(
        &self,
        node_id: &str,
        runtime: &MobileNodeRuntimeState,
    ) -> MobileNodePushState {
        MobileNodePushState {
            node_id: node_id.to_string(),
            push_token_present: runtime.push_token_present,
            notifications_authorized: runtime.notifications_authorized,
            push_provider: runtime.push_provider.clone(),
            push_token_updated_at: runtime.push_token_updated_at.clone(),
            runtime_status: runtime.runtime_status.clone(),
        }
    }

    pub fn register_push(
        &self,
        mut runtime: MobileNodeRuntimeState,
        request: MobilePushRegistrationRequest,
        now: String,
    ) -> MobileNodeRuntimeState {
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
        self.refresh_runtime_status(&mut runtime);
        runtime
    }

    pub fn sync_state(
        &self,
        node_id: &str,
        runtime: &MobileNodeRuntimeState,
        pending_conflict_count: usize,
        resolved_conflict_count: usize,
    ) -> MobileNodeSyncState {
        MobileNodeSyncState {
            node_id: node_id.to_string(),
            sync_state: runtime.sync_state.clone(),
            pending_change_count: runtime.pending_change_count,
            pending_conflict_count,
            resolved_conflict_count,
            last_sync_requested_at: runtime.last_sync_requested_at.clone(),
            last_sync_at: runtime.last_sync_at.clone(),
            last_sync_result: runtime.last_sync_result.clone(),
            runtime_status: runtime.runtime_status.clone(),
        }
    }

    pub fn report_sync(
        &self,
        mut runtime: MobileNodeRuntimeState,
        request: MobileSyncReportRequest,
        now: String,
    ) -> MobileNodeRuntimeState {
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
        self.refresh_runtime_status(&mut runtime);
        runtime
    }

    pub fn runtime_activity_entries(
        &self,
        runtime: &MobileNodeRuntimeState,
    ) -> Vec<MobileNodeActivityEntry> {
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

    pub fn node_summary(
        &self,
        node_id: &str,
        runtime: &MobileNodeRuntimeState,
        counts: MobileNodeSummaryCounts,
    ) -> MobileNodeSummary {
        MobileNodeSummary {
            node_id: node_id.to_string(),
            runtime_status: runtime.runtime_status.clone(),
            app_state: runtime.app_state.clone(),
            network: runtime.network.clone(),
            reachable: runtime.reachable,
            push_token_present: runtime.push_token_present,
            notifications_authorized: runtime.notifications_authorized,
            wake_state: runtime.wake_state.clone(),
            rehydrate_state: runtime.rehydrate_state.clone(),
            sync_state: runtime.sync_state.clone(),
            battery_percent: runtime.battery_percent,
            pairings: counts.pairings,
            app_sessions: counts.app_sessions,
            active_app_sessions: counts.active_app_sessions,
            ended_app_sessions: counts.ended_app_sessions,
            sync_conflicts: counts.sync_conflicts,
            notifications: counts.notifications,
            inbox_messages: counts.inbox_messages,
            outbox_messages: counts.outbox_messages,
            commands: counts.commands,
            capability_executions: counts.capability_executions,
            media_artifacts: counts.media_artifacts,
        }
    }

    pub fn metrics(
        &self,
        runtimes: &[MobileNodeRuntimeState],
        counts: MobileMetricsCounts,
    ) -> MobileMetricsSummary {
        let mut summary = MobileMetricsSummary {
            total_nodes: runtimes.len(),
            paired_records: counts.paired_records,
            unpaired_records: counts.unpaired_records,
            reachable_nodes: 0,
            push_token_present_nodes: 0,
            notifications_authorized_nodes: 0,
            waking_nodes: 0,
            rehydrate_pending_nodes: 0,
            total_app_sessions: counts.total_app_sessions,
            active_app_sessions: counts.active_app_sessions,
            ended_app_sessions: counts.ended_app_sessions,
            pending_notifications: 0,
            delivered_notifications: 0,
            pending_inbound_messages: 0,
            acknowledged_inbound_messages: 0,
            pending_outbound_messages: 0,
            acknowledged_outbound_messages: 0,
            commands: counts.commands,
            sync_conflicts: counts.sync_conflicts,
            capability_executions: counts.capability_executions,
            media_artifacts: counts.media_artifacts,
        };

        for runtime in runtimes {
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

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_runtime() -> MobileNodeRuntimeState {
        MobileNodeRuntimeState {
            node_id: "iphone".to_string(),
            runtime_status: "registered".to_string(),
            app_state: "inactive".to_string(),
            network: "offline".to_string(),
            reachable: false,
            push_token_present: false,
            notifications_authorized: false,
            push_provider: None,
            push_token_updated_at: None,
            battery_percent: None,
            last_heartbeat_at: None,
            wake_state: "idle".to_string(),
            wake_requested_at: None,
            wake_requested_by: None,
            wake_reason: None,
            last_wake_command_id: None,
            rehydrate_state: "idle".to_string(),
            rehydrate_requested_at: None,
            rehydrate_requested_by: None,
            rehydrate_reason: None,
            rehydrate_pending_change_count: None,
            last_rehydrate_command_id: None,
            sync_state: "idle".to_string(),
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

    #[test]
    fn heartbeat_acknowledges_requested_runtime_state() {
        let service = MobileRuntimeStatusService::new();
        let mut runtime = sample_runtime();
        runtime.wake_state = "requested".to_string();
        runtime.rehydrate_state = "requested".to_string();
        let updated = service.heartbeat(
            runtime,
            MobileHeartbeatRequest {
                app_state: Some("active".to_string()),
                network: Some("wifi".to_string()),
                reachable: Some(true),
                push_token_present: Some(true),
                battery_percent: Some(120),
                metadata: json!({"source": "heartbeat"}),
            },
            "2026-03-28T20:00:00Z".to_string(),
        );

        assert_eq!(updated.runtime_status, "active");
        assert_eq!(updated.wake_state, "acknowledged");
        assert_eq!(updated.rehydrate_state, "ready");
        assert_eq!(updated.battery_percent, Some(100));
        assert_eq!(updated.metadata["source"], "heartbeat");
    }

    #[test]
    fn metrics_rolls_up_runtime_receipts() {
        let service = MobileRuntimeStatusService::new();
        let mut runtime = sample_runtime();
        runtime.reachable = true;
        runtime.push_token_present = true;
        runtime.notifications_authorized = true;
        runtime.wake_state = "requested".to_string();
        runtime.pending_notification_count = 2;
        runtime.pending_inbound_message_count = 1;
        runtime.acknowledged_outbound_message_count = 3;

        let summary = service.metrics(
            &[runtime],
            MobileMetricsCounts {
                paired_records: 4,
                unpaired_records: 1,
                total_app_sessions: 5,
                active_app_sessions: 2,
                ended_app_sessions: 3,
                commands: 6,
                sync_conflicts: 7,
                capability_executions: 8,
                media_artifacts: 9,
            },
        );

        assert_eq!(summary.total_nodes, 1);
        assert_eq!(summary.reachable_nodes, 1);
        assert_eq!(summary.push_token_present_nodes, 1);
        assert_eq!(summary.notifications_authorized_nodes, 1);
        assert_eq!(summary.waking_nodes, 1);
        assert_eq!(summary.pending_notifications, 2);
        assert_eq!(summary.pending_inbound_messages, 1);
        assert_eq!(summary.acknowledged_outbound_messages, 3);
        assert_eq!(summary.commands, 6);
    }
}
