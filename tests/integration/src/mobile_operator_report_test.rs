use openrustclaw_cli::commands::mobile::{
    DeviceCommandKind, MobileCommandDispatchRequest, MobileHeartbeatRequest, MobilePairRequest,
    MobilePushRegistrationRequest, MobileSyncConflictReportRequest, dispatch_command_data,
    heartbeat_node_data, mobile_node_report_data, pair_node_data, register_push_data,
    report_sync_conflict_data,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn mobile_operator_report_surfaces_attention_and_recent_activity() -> TestResult {
    let workspace = tempfile::tempdir()?;

    pair_node_data(
        workspace.path(),
        MobilePairRequest {
            id: "report-node".to_string(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            auth_token_env: "PATH".to_string(),
            device_name: Some("Report Phone".to_string()),
            platform: Some("ios".to_string()),
            capabilities: vec!["mobile".to_string(), "notifications".to_string()],
            enabled: true,
            sync: None,
            notifications: None,
            metadata: json!({ "suite": "phase13" }),
        },
    )?;

    heartbeat_node_data(
        workspace.path(),
        "report-node",
        MobileHeartbeatRequest {
            app_state: Some("active".to_string()),
            network: Some("wifi".to_string()),
            reachable: Some(true),
            push_token_present: Some(false),
            battery_percent: Some(78),
            metadata: json!({}),
        },
    )?;

    register_push_data(
        workspace.path(),
        "report-node",
        MobilePushRegistrationRequest {
            push_provider: Some("apns".to_string()),
            push_token_present: Some(true),
            notifications_authorized: Some(false),
        },
    )?;

    report_sync_conflict_data(
        workspace.path(),
        MobileSyncConflictReportRequest {
            node_id: "report-node".to_string(),
            item_key: "contacts/42".to_string(),
            conflict_type: "contact_merge".to_string(),
            summary: Some("contact changed on device and server".to_string()),
            details: None,
            resolution_hint: Some("operator review".to_string()),
        },
    )?;

    let pending = dispatch_command_data(
        workspace.path(),
        MobileCommandDispatchRequest {
            node_id: "report-node".to_string(),
            command: DeviceCommandKind::PushNotification,
            payload: json!({
                "title": "Approval needed",
                "body": "Mobile operator report coverage"
            }),
            approved_by: None,
            require_approval: Some(true),
        },
    )
    .await?;
    assert_eq!(pending.status, "pending_approval");

    let report = mobile_node_report_data(workspace.path(), "report-node", Some(12))?;

    assert_eq!(report.summary.node_id, "report-node");
    assert_eq!(report.summary.sync_conflicts, 1);
    assert_eq!(report.command_metrics.pending_approval_commands, 1);
    assert_eq!(report.push.push_provider.as_deref(), Some("apns"));
    assert!(!report.push.notifications_authorized);
    assert!(
        report
            .attention_signals
            .iter()
            .any(|signal| signal.kind == "pending_approval")
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
            .any(|signal| signal.kind == "notification_authorization")
    );
    assert!(
        report
            .recent_activity
            .iter()
            .any(|entry| entry.kind == "sync_conflict")
    );
    assert!(
        report
            .recent_activity
            .iter()
            .any(|entry| entry.kind == "runtime_heartbeat")
    );

    Ok(())
}
