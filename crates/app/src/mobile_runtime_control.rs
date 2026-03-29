use crate::mobile_runtime_status::{MobileNodeRuntimeState, MobileRuntimeStatusService};
use chrono::{DateTime, Utc};
use openrustclaw_mobile::protocol::{
    MobileCommandDecisionRequest, MobileCommandDispatchRequest, MobileCommandRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNotificationSendRequest {
    pub node_id: String,
    pub title: String,
    pub body: String,
    pub priority: Option<String>,
    pub notification_type: Option<String>,
    pub data: HashMap<String, String>,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNotificationRecord {
    pub id: String,
    pub node_id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub priority: Option<String>,
    pub notification_type: Option<String>,
    pub data: HashMap<String, String>,
    pub created_at: String,
    pub dispatched_at: Option<String>,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub command_id: Option<String>,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileNotificationAckRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileInboundMessageReportRequest {
    pub node_id: String,
    pub source: String,
    pub target: String,
    pub content: String,
    pub content_type: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileInboundMessageRecord {
    pub id: String,
    pub node_id: String,
    pub source: String,
    pub target: String,
    pub status: String,
    pub content_type: Option<String>,
    pub content_preview: String,
    pub bytes: usize,
    pub created_at: String,
    pub reported_at: Option<String>,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub metadata: Value,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileInboundMessageAckRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileOutboundMessageSendRequest {
    pub node_id: String,
    pub target: String,
    pub content: String,
    pub content_type: Option<String>,
    pub requested_by: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileOutboundMessageRecord {
    pub id: String,
    pub node_id: String,
    pub target: String,
    pub status: String,
    pub content_type: Option<String>,
    pub content_preview: String,
    pub bytes: usize,
    pub created_at: String,
    pub dispatched_at: Option<String>,
    pub acknowledged_at: Option<String>,
    pub acknowledged_by: Option<String>,
    pub command_id: Option<String>,
    pub metadata: Value,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileOutboundMessageAckRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileWakeRequest {
    pub requested_by: Option<String>,
    pub reason: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileRehydrateRequest {
    pub requested_by: Option<String>,
    pub reason: Option<String>,
    pub pending_change_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNodeRuntimeActionResult {
    pub node_id: String,
    pub runtime: MobileNodeRuntimeState,
    pub command: Option<MobileCommandRecord>,
    pub preview: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MobileCommandEvent {
    pub index: usize,
    pub kind: String,
    pub observed_at: String,
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

#[derive(Debug, Clone, Default)]
pub struct MobileRuntimeControlService {
    status_service: MobileRuntimeStatusService,
}

impl MobileRuntimeControlService {
    pub fn new() -> Self {
        Self {
            status_service: MobileRuntimeStatusService::new(),
        }
    }

    pub fn notification_record(
        &self,
        id: String,
        request: MobileNotificationSendRequest,
        preview: Value,
        created_at: String,
    ) -> MobileNotificationRecord {
        MobileNotificationRecord {
            id,
            node_id: request.node_id,
            title: request.title,
            body: request.body,
            status: "queued".to_string(),
            priority: request.priority,
            notification_type: request.notification_type,
            data: request.data,
            created_at,
            dispatched_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            command_id: None,
            preview,
        }
    }

    pub fn mark_notification_dispatched(
        &self,
        mut record: MobileNotificationRecord,
        command_id: String,
        dispatched_at: String,
    ) -> MobileNotificationRecord {
        record.status = "dispatched".to_string();
        record.dispatched_at = Some(dispatched_at);
        record.command_id = Some(command_id);
        record
    }

    pub fn acknowledge_notification(
        &self,
        mut record: MobileNotificationRecord,
        mut runtime: MobileNodeRuntimeState,
        request: MobileNotificationAckRequest,
        acknowledged_at: String,
    ) -> (MobileNotificationRecord, MobileNodeRuntimeState) {
        if record.status == "acknowledged" {
            return (record, runtime);
        }
        record.status = "acknowledged".to_string();
        record.acknowledged_by = Some(request.acknowledged_by);
        record.acknowledged_at = Some(acknowledged_at);
        runtime.pending_notification_count = runtime.pending_notification_count.saturating_sub(1);
        runtime.delivered_notification_count =
            runtime.delivered_notification_count.saturating_add(1);
        self.status_service.refresh_runtime_status(&mut runtime);
        (record, runtime)
    }

    pub fn inbound_message_record(
        &self,
        id: String,
        request: MobileInboundMessageReportRequest,
        content_type: String,
        preview: Value,
        bytes: usize,
        created_at: String,
    ) -> MobileInboundMessageRecord {
        MobileInboundMessageRecord {
            id,
            node_id: request.node_id,
            source: request.source,
            target: request.target,
            status: "reported".to_string(),
            content_type: Some(content_type),
            content_preview: request.content,
            bytes,
            created_at: created_at.clone(),
            reported_at: Some(created_at),
            acknowledged_at: None,
            acknowledged_by: None,
            metadata: request.metadata,
            preview,
        }
    }

    pub fn acknowledge_inbound_message(
        &self,
        mut record: MobileInboundMessageRecord,
        mut runtime: MobileNodeRuntimeState,
        request: MobileInboundMessageAckRequest,
        acknowledged_at: String,
    ) -> (MobileInboundMessageRecord, MobileNodeRuntimeState) {
        if record.status == "acknowledged" {
            return (record, runtime);
        }
        record.status = "acknowledged".to_string();
        record.acknowledged_by = Some(request.acknowledged_by);
        record.acknowledged_at = Some(acknowledged_at);
        runtime.pending_inbound_message_count =
            runtime.pending_inbound_message_count.saturating_sub(1);
        runtime.acknowledged_inbound_message_count =
            runtime.acknowledged_inbound_message_count.saturating_add(1);
        self.status_service.refresh_runtime_status(&mut runtime);
        (record, runtime)
    }

    pub fn outbound_message_record(
        &self,
        id: String,
        request: MobileOutboundMessageSendRequest,
        content_type: String,
        preview: Value,
        bytes: usize,
        created_at: String,
    ) -> MobileOutboundMessageRecord {
        MobileOutboundMessageRecord {
            id,
            node_id: request.node_id,
            target: request.target,
            status: "queued".to_string(),
            content_type: Some(content_type),
            content_preview: request.content,
            bytes,
            created_at,
            dispatched_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            command_id: None,
            metadata: request.metadata,
            preview,
        }
    }

    pub fn mark_outbound_message_dispatched(
        &self,
        mut record: MobileOutboundMessageRecord,
        command_id: String,
        dispatched_at: String,
    ) -> MobileOutboundMessageRecord {
        record.status = "dispatched".to_string();
        record.dispatched_at = Some(dispatched_at);
        record.command_id = Some(command_id);
        record
    }

    pub fn acknowledge_outbound_message(
        &self,
        mut record: MobileOutboundMessageRecord,
        mut runtime: MobileNodeRuntimeState,
        request: MobileOutboundMessageAckRequest,
        acknowledged_at: String,
    ) -> (MobileOutboundMessageRecord, MobileNodeRuntimeState) {
        if record.status == "acknowledged" {
            return (record, runtime);
        }
        record.status = "acknowledged".to_string();
        record.acknowledged_by = Some(request.acknowledged_by);
        record.acknowledged_at = Some(acknowledged_at);
        runtime.pending_outbound_message_count =
            runtime.pending_outbound_message_count.saturating_sub(1);
        runtime.acknowledged_outbound_message_count = runtime
            .acknowledged_outbound_message_count
            .saturating_add(1);
        self.status_service.refresh_runtime_status(&mut runtime);
        (record, runtime)
    }

    pub fn dispatch_command_record(
        &self,
        request: MobileCommandDispatchRequest,
        node_id: String,
        required_capability: String,
        approval_required: bool,
        created_at: DateTime<Utc>,
    ) -> MobileCommandRecord {
        let approved_by = request.approved_by.clone();
        MobileCommandRecord {
            id: uuid::Uuid::new_v4().to_string(),
            node_id,
            command: request.command,
            required_capability,
            approval_required,
            status: if approval_required && approved_by.is_none() {
                "pending_approval".to_string()
            } else {
                "approved".to_string()
            },
            payload: request.payload,
            result: Value::Null,
            created_at,
            approved_by: approved_by.clone(),
            decided_reason: None,
            approved_at: approved_by.as_ref().map(|_| created_at),
            executed_at: None,
        }
    }

    pub fn approve_command(
        &self,
        mut record: MobileCommandRecord,
        request: MobileCommandDecisionRequest,
        result: Value,
        approved_at: DateTime<Utc>,
        executed_at: DateTime<Utc>,
    ) -> MobileCommandRecord {
        record.status = "approved".to_string();
        record.approved_by = Some(request.decided_by);
        record.decided_reason = request.reason;
        record.approved_at = Some(approved_at);
        record.result = result;
        record.status = "executed".to_string();
        record.executed_at = Some(executed_at);
        record
    }

    pub fn reject_command(
        &self,
        mut record: MobileCommandRecord,
        request: MobileCommandDecisionRequest,
        decided_at: DateTime<Utc>,
    ) -> MobileCommandRecord {
        record.status = "rejected".to_string();
        record.approved_by = Some(request.decided_by);
        record.decided_reason = request.reason;
        record.approved_at = Some(decided_at);
        record
    }

    pub fn command_timeline_events(
        &self,
        command: &MobileCommandRecord,
    ) -> Vec<MobileCommandEvent> {
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

    pub fn command_metrics(&self, commands: &[MobileCommandRecord]) -> MobileCommandMetricsSummary {
        let mut by_command_kind: HashMap<String, usize> = HashMap::new();
        let mut pending_approval_commands = 0usize;
        let mut approved_commands = 0usize;
        let mut executed_commands = 0usize;
        let mut rejected_commands = 0usize;
        let mut execution_latencies = Vec::new();

        for command in commands {
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
            if let Some(latency) = self.command_execution_latency_secs(command) {
                execution_latencies.push(latency as f64);
            }
        }

        let avg_execution_latency_secs = if execution_latencies.is_empty() {
            None
        } else {
            Some(execution_latencies.iter().sum::<f64>() / execution_latencies.len() as f64)
        };

        MobileCommandMetricsSummary {
            total_commands: commands.len(),
            pending_approval_commands,
            approved_commands,
            executed_commands,
            rejected_commands,
            by_command_kind,
            avg_execution_latency_secs,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wake_node(
        &self,
        node_id: &str,
        device_name: &str,
        mut runtime: MobileNodeRuntimeState,
        request: MobileWakeRequest,
        ready_for_runtime: bool,
        notifications_capability: bool,
        command: Option<MobileCommandRecord>,
        now: String,
    ) -> MobileNodeRuntimeActionResult {
        runtime.wake_requested_at = Some(now);
        runtime.wake_requested_by = request.requested_by.clone();
        runtime.wake_reason = request.reason.clone();
        runtime.wake_state = "requested".to_string();
        if let Some(command) = command.as_ref() {
            runtime.wake_state = "dispatched".to_string();
            runtime.last_wake_command_id = Some(command.id.clone());
        }
        self.status_service.refresh_runtime_status(&mut runtime);

        MobileNodeRuntimeActionResult {
            node_id: node_id.to_string(),
            runtime,
            command,
            preview: serde_json::json!({
                "requested": true,
                "ready_for_runtime": ready_for_runtime,
                "notifications_capability": notifications_capability,
                "title": request.title.unwrap_or_else(|| format!("Wake {device_name}")),
                "body": request.body.unwrap_or_else(|| "Operator requested a bounded wake ping".to_string()),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate_node(
        &self,
        node_id: &str,
        mut runtime: MobileNodeRuntimeState,
        request: MobileRehydrateRequest,
        pending_change_count: usize,
        preview: Value,
        command: Option<MobileCommandRecord>,
        now: String,
    ) -> MobileNodeRuntimeActionResult {
        runtime.rehydrate_requested_at = Some(now);
        runtime.rehydrate_requested_by = request.requested_by.clone();
        runtime.rehydrate_reason = request.reason.clone();
        runtime.rehydrate_pending_change_count = Some(pending_change_count);
        runtime.rehydrate_state = "requested".to_string();
        if let Some(command) = command.as_ref() {
            runtime.rehydrate_state = "synced".to_string();
            runtime.last_rehydrate_command_id = Some(command.id.clone());
        }
        self.status_service.refresh_runtime_status(&mut runtime);

        MobileNodeRuntimeActionResult {
            node_id: node_id.to_string(),
            runtime,
            command,
            preview,
        }
    }

    fn command_execution_latency_secs(&self, command: &MobileCommandRecord) -> Option<u64> {
        let executed_at = command.executed_at.as_ref()?;
        let latency = executed_at
            .signed_duration_since(command.created_at)
            .num_seconds();
        Some(latency.max(0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use openrustclaw_mobile::protocol::DeviceCommandKind;
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
    fn wake_and_rehydrate_update_runtime_transitions() {
        let service = MobileRuntimeControlService::new();
        let wake = service.wake_node(
            "iphone",
            "Phone",
            sample_runtime(),
            MobileWakeRequest {
                requested_by: Some("operator".to_string()),
                reason: Some("attention".to_string()),
                title: None,
                body: None,
            },
            true,
            true,
            None,
            "2026-03-28T20:00:00Z".to_string(),
        );
        assert_eq!(wake.runtime.wake_state, "requested");
        assert_eq!(wake.runtime.runtime_status, "wake_requested");

        let rehydrate = service.rehydrate_node(
            "iphone",
            sample_runtime(),
            MobileRehydrateRequest {
                requested_by: Some("operator".to_string()),
                reason: Some("resume".to_string()),
                pending_change_count: Some(3),
            },
            3,
            json!({"should_sync": true}),
            None,
            "2026-03-28T20:00:01Z".to_string(),
        );
        assert_eq!(rehydrate.runtime.rehydrate_state, "requested");
        assert_eq!(rehydrate.runtime.runtime_status, "rehydrate_requested");
    }

    #[test]
    fn command_metrics_and_events_track_lifecycle() {
        let service = MobileRuntimeControlService::new();
        let command = MobileCommandRecord {
            id: "cmd-1".to_string(),
            node_id: "iphone".to_string(),
            command: DeviceCommandKind::PushNotification,
            required_capability: "notifications".to_string(),
            approval_required: false,
            status: "executed".to_string(),
            payload: json!({}),
            result: json!({"ok": true}),
            created_at: Utc.with_ymd_and_hms(2026, 3, 28, 20, 0, 0).unwrap(),
            approved_by: Some("operator".to_string()),
            decided_reason: None,
            approved_at: Some(Utc.with_ymd_and_hms(2026, 3, 28, 20, 0, 1).unwrap()),
            executed_at: Some(Utc.with_ymd_and_hms(2026, 3, 28, 20, 0, 2).unwrap()),
        };

        let events = service.command_timeline_events(&command);
        let metrics = service.command_metrics(&[command]);

        assert_eq!(events.len(), 2);
        assert_eq!(metrics.total_commands, 1);
        assert_eq!(metrics.executed_commands, 1);
        assert_eq!(metrics.avg_execution_latency_secs, Some(2.0));
    }
}
