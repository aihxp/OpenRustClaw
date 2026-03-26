use chrono::{Duration, Utc};
use openrustclaw_cli::commands::inspect::{
    ToolExecutionRecord, append_tool_execution_record, new_tool_execution_record,
    tool_execution_history,
};
use openrustclaw_cli::commands::voice_runtime::{
    VoiceSessionRecord, VoiceSessionTurn, voice_session_outcomes,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_voice_session(
    workspace_root: &std::path::Path,
    session: &VoiceSessionRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = workspace_root.join(".claw").join("voice").join("sessions");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{}.json", session.id)),
        serde_json::to_vec_pretty(session)?,
    )?;
    Ok(())
}

#[tokio::test]
async fn communications_audit_surfaces_cover_recent_email_and_voice_state() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let now = Utc::now();

    let gmail = ToolExecutionRecord {
        created_at: now.to_rfc3339(),
        ..new_tool_execution_record(
            "channels.gmail_pubsub.ingress",
            "runtime_tool",
            "success",
            "success",
            18,
            None,
            None,
            Some(serde_json::json!({"body_bytes": 64})),
            Some(serde_json::json!({
                "email_address": "ops@example.com",
                "processed_count": 1,
                "skipped_count": 0,
                "entries": [{
                    "message_id": "msg-55",
                    "thread_id": "thread-22",
                    "from": "customer@example.com",
                    "subject": "Need help",
                    "received_at": now.to_rfc3339(),
                    "attachment_count": 0,
                    "unread": true
                }]
            })),
        )
    };
    append_tool_execution_record(workspace.path(), &gmail)?;

    let voice = VoiceSessionRecord {
        id: "voice-attention".to_string(),
        status: "active".to_string(),
        live_state: "interrupted".to_string(),
        stt_provider: "openai".to_string(),
        tts_provider: "openai".to_string(),
        tts_voice: "alloy".to_string(),
        assistant_prompt: None,
        end_reason: None,
        created_at: (now - Duration::minutes(3)).to_rfc3339(),
        last_activity_at: now.to_rfc3339(),
        closed_at: None,
        reconnect_count: 0,
        last_reconnected_at: None,
        pause_count: 0,
        interrupted_count: 1,
        last_paused_at: None,
        last_resumed_at: None,
        last_interrupted_at: Some(now.to_rfc3339()),
        turns: vec![VoiceSessionTurn {
            role: "assistant".to_string(),
            text: "checking status".to_string(),
            created_at: (now - Duration::minutes(1)).to_rfc3339(),
            synthesized_output_path: None,
        }],
    };
    write_voice_session(workspace.path(), &voice)?;

    let email_report = tool_execution_history(
        workspace.path(),
        10,
        Some("runtime_tool"),
        Some("success"),
        Some("channels.gmail_pubsub.ingress"),
    )?;
    assert_eq!(email_report.entries.len(), 1);
    assert_eq!(
        email_report.entries[0]
            .result_preview
            .as_ref()
            .and_then(|value| value.get("email_address")),
        Some(&serde_json::json!("ops@example.com"))
    );

    let voice_report = voice_session_outcomes(workspace.path(), Some(30), Some(10)).await?;
    assert_eq!(voice_report.outcomes.len(), 1);
    assert_eq!(voice_report.outcomes[0].outcome_label, "interrupted");
    assert!(voice_report.outcomes[0].attention_needed);

    Ok(())
}
