use chrono::{Duration, Utc};
use openrustclaw_cli::commands::voice_runtime::{
    VoiceSessionRecord, VoiceSessionTurn, voice_session_outcomes,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn voice_sessions_dir(workspace_root: &std::path::Path) -> std::path::PathBuf {
    workspace_root.join(".claw").join("voice").join("sessions")
}

fn write_session(
    workspace_root: &std::path::Path,
    session: &VoiceSessionRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = voice_sessions_dir(workspace_root);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{}.json", session.id)),
        serde_json::to_vec_pretty(session)?,
    )?;
    Ok(())
}

#[tokio::test]
async fn voice_outcomes_distinguish_stale_paused_and_ended_sessions() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let now = Utc::now();

    let stale = VoiceSessionRecord {
        id: "voice-stale".to_string(),
        status: "active".to_string(),
        live_state: "listening".to_string(),
        stt_provider: "openai".to_string(),
        tts_provider: "openai".to_string(),
        tts_voice: "alloy".to_string(),
        assistant_prompt: None,
        end_reason: None,
        created_at: (now - Duration::hours(2)).to_rfc3339(),
        last_activity_at: (now - Duration::minutes(45)).to_rfc3339(),
        closed_at: None,
        reconnect_count: 0,
        last_reconnected_at: None,
        pause_count: 0,
        interrupted_count: 0,
        last_paused_at: None,
        last_resumed_at: None,
        last_interrupted_at: None,
        turns: vec![VoiceSessionTurn {
            role: "user".to_string(),
            text: "hello".to_string(),
            created_at: (now - Duration::hours(2)).to_rfc3339(),
            synthesized_output_path: None,
        }],
    };
    let paused = VoiceSessionRecord {
        id: "voice-paused".to_string(),
        status: "active".to_string(),
        live_state: "paused".to_string(),
        stt_provider: "openai".to_string(),
        tts_provider: "openai".to_string(),
        tts_voice: "nova".to_string(),
        assistant_prompt: None,
        end_reason: None,
        created_at: (now - Duration::minutes(20)).to_rfc3339(),
        last_activity_at: now.to_rfc3339(),
        closed_at: None,
        reconnect_count: 0,
        last_reconnected_at: None,
        pause_count: 1,
        interrupted_count: 0,
        last_paused_at: Some(now.to_rfc3339()),
        last_resumed_at: None,
        last_interrupted_at: None,
        turns: vec![],
    };
    let ended = VoiceSessionRecord {
        id: "voice-ended".to_string(),
        status: "ended".to_string(),
        live_state: "ended".to_string(),
        stt_provider: "openai".to_string(),
        tts_provider: "openai".to_string(),
        tts_voice: "alloy".to_string(),
        assistant_prompt: None,
        end_reason: Some("completed".to_string()),
        created_at: (now - Duration::minutes(10)).to_rfc3339(),
        last_activity_at: (now - Duration::minutes(5)).to_rfc3339(),
        closed_at: Some((now - Duration::minutes(4)).to_rfc3339()),
        reconnect_count: 0,
        last_reconnected_at: None,
        pause_count: 0,
        interrupted_count: 0,
        last_paused_at: None,
        last_resumed_at: None,
        last_interrupted_at: None,
        turns: vec![VoiceSessionTurn {
            role: "assistant".to_string(),
            text: "done".to_string(),
            created_at: (now - Duration::minutes(5)).to_rfc3339(),
            synthesized_output_path: None,
        }],
    };

    write_session(workspace.path(), &stale)?;
    write_session(workspace.path(), &paused)?;
    write_session(workspace.path(), &ended)?;

    let report = voice_session_outcomes(workspace.path(), Some(30), Some(10)).await?;
    assert_eq!(report.stale_after_secs, 30);
    assert_eq!(report.total_sessions, 3);
    assert_eq!(report.active_sessions, 2);
    assert_eq!(report.ended_sessions, 1);
    assert_eq!(report.attention_needed, 2);

    let stale_entry = report
        .outcomes
        .iter()
        .find(|entry| entry.session_id == "voice-stale")
        .expect("stale outcome");
    assert_eq!(stale_entry.outcome_label, "stale");
    assert!(stale_entry.attention_needed);
    assert!(stale_entry.stale);

    let paused_entry = report
        .outcomes
        .iter()
        .find(|entry| entry.session_id == "voice-paused")
        .expect("paused outcome");
    assert_eq!(paused_entry.outcome_label, "paused");
    assert!(paused_entry.attention_needed);

    let ended_entry = report
        .outcomes
        .iter()
        .find(|entry| entry.session_id == "voice-ended")
        .expect("ended outcome");
    assert_eq!(ended_entry.outcome_label, "ended");
    assert_eq!(ended_entry.detail, "completed");
    assert!(!ended_entry.attention_needed);

    Ok(())
}
