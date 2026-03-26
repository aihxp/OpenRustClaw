use openrustclaw_cli::commands::inspect::{
    ToolExecutionRecord, append_tool_execution_record, new_tool_execution_record,
    tool_execution_history,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn tool_execution_history_persists_and_filters_records() -> TestResult {
    let workspace = tempfile::tempdir()?;

    let success = new_tool_execution_record(
        "memory_timeline",
        "mcp_tool",
        "success",
        "success",
        14,
        None,
        None,
        Some(serde_json::json!({"limit": 5})),
        Some(serde_json::json!({"entries": []})),
    );
    let failure = ToolExecutionRecord {
        created_at: "2026-03-26T20:00:00Z".to_string(),
        ..new_tool_execution_record(
            "compiled_skill_execute",
            "mcp_tool",
            "failure",
            "timeout",
            9000,
            Some("tool timed out".to_string()),
            None,
            None,
            None,
        )
    };

    append_tool_execution_record(workspace.path(), &success)?;
    append_tool_execution_record(workspace.path(), &failure)?;

    let report = tool_execution_history(
        workspace.path(),
        10,
        Some("mcp_tool"),
        Some("failure"),
        None,
    )?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].tool_name, "compiled_skill_execute");
    assert_eq!(report.entries[0].status_detail, "timeout");
    assert_eq!(report.entries[0].error.as_deref(), Some("tool timed out"));

    Ok(())
}

#[test]
fn tool_execution_history_filters_by_tool_name_for_email_activity() -> TestResult {
    let workspace = tempfile::tempdir()?;

    let gmail = ToolExecutionRecord {
        created_at: "2026-03-26T20:15:00Z".to_string(),
        ..new_tool_execution_record(
            "channels.gmail_pubsub.ingress",
            "runtime_tool",
            "success",
            "success",
            22,
            None,
            None,
            Some(serde_json::json!({"body_bytes": 48})),
            Some(serde_json::json!({
                "email_address": "user@example.com",
                "processed_count": 1,
                "skipped_count": 0,
                "entries": [{
                    "message_id": "msg-1",
                    "thread_id": "thread-1",
                    "from": "sender@example.com",
                    "subject": "Quarterly update",
                    "received_at": "2026-03-26T20:14:00Z",
                    "attachment_count": 1,
                    "unread": true
                }]
            })),
        )
    };
    let unrelated = new_tool_execution_record(
        "memory_timeline",
        "runtime_tool",
        "success",
        "success",
        8,
        None,
        None,
        None,
        None,
    );

    append_tool_execution_record(workspace.path(), &gmail)?;
    append_tool_execution_record(workspace.path(), &unrelated)?;

    let report = tool_execution_history(
        workspace.path(),
        10,
        Some("runtime_tool"),
        Some("success"),
        Some("channels.gmail_pubsub.ingress"),
    )?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].tool_name, "channels.gmail_pubsub.ingress");
    assert_eq!(
        report.entries[0]
            .result_preview
            .as_ref()
            .and_then(|value| value.get("processed_count")),
        Some(&serde_json::json!(1))
    );

    Ok(())
}
