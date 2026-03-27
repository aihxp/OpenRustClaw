use chrono::{Duration, Utc};
use openrustclaw_cli::commands::inspect::voice_operator_report_summary;
use openrustclaw_cli::commands::talk::{TalkSessionReceipt, TalkSessionStateSnapshot, TalkSessionTurn};
use openrustclaw_cli::commands::voice_runtime::{VoiceSessionRecord, VoiceSessionTurn};
use openrustclaw_core::config::AppConfig;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_json(path: &std::path::Path, value: &impl serde::Serialize) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

#[tokio::test]
async fn voice_operator_report_surfaces_attention_across_voice_talk_and_calls() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let now = Utc::now();

    let stale_voice_session = VoiceSessionRecord {
        id: "voice-stale".to_string(),
        status: "active".to_string(),
        live_state: "listening".to_string(),
        stt_provider: "openai".to_string(),
        tts_provider: "openai".to_string(),
        tts_voice: "alloy".to_string(),
        assistant_prompt: None,
        end_reason: None,
        created_at: (now - Duration::minutes(20)).to_rfc3339(),
        last_activity_at: (now - Duration::minutes(10)).to_rfc3339(),
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
            created_at: (now - Duration::minutes(20)).to_rfc3339(),
            synthesized_output_path: None,
        }],
    };
    write_json(
        &workspace
            .path()
            .join(".claw/voice/sessions/voice-stale.json"),
        &stale_voice_session,
    )?;

    let talk_error = TalkSessionReceipt {
        id: "talk-error".to_string(),
        status: "error".to_string(),
        requested_provider: "openai".to_string(),
        requested_model: Some("gpt-4o-mini-transcribe".to_string()),
        wake_word: "hey claw".to_string(),
        silence_timeout_secs: 4,
        max_utterance_secs: 20,
        barge_in: true,
        created_at: (now - Duration::minutes(9)).to_rfc3339(),
        last_activity_at: (now - Duration::minutes(4)).to_rfc3339(),
        closed_at: Some((now - Duration::minutes(3)).to_rfc3339()),
        final_state: Some("transcription_failed".to_string()),
        turn_count: 1,
        snapshots: vec![TalkSessionStateSnapshot {
            observed_at: (now - Duration::minutes(5)).to_rfc3339(),
            state: "listening".to_string(),
            turn_count: 1,
        }],
        turns: vec![TalkSessionTurn {
            index: 1,
            user_text: "call support".to_string(),
            agent_response: "I could not hear that clearly.".to_string(),
            captured_at: (now - Duration::minutes(5)).to_rfc3339(),
        }],
    };
    write_json(
        &workspace.path().join(".claw/talk/sessions/talk-error.json"),
        &talk_error,
    )?;

    let stale_voice_call = json!({
        "calls": [{
            "call_id": "call-stale",
            "plugin_id": "support-line",
            "skill_name": "support",
            "status": "active",
            "health": "active",
            "remote": "+15551234567",
            "greeting_text": "Hello from OpenRustClaw",
            "greeting_audio_path": null,
            "reason": null,
            "metadata": { "suite": "phase15" },
            "start_hook_output": null,
            "end_hook_output": null,
            "reconnect_hook_output": null,
            "started_at": (now - Duration::minutes(25)).to_rfc3339(),
            "last_seen_at": (now - Duration::minutes(12)).to_rfc3339(),
            "reconnect_count": 1,
            "last_reconnected_at": (now - Duration::minutes(15)).to_rfc3339(),
            "stale_after_secs": 30,
            "ended_at": null
        }]
    });
    write_json(
        &workspace.path().join(".claw/control/skill-voice-calls.json"),
        &stale_voice_call,
    )?;

    let mut config = AppConfig::default();
    config.voice.enabled = true;
    config.voice.stt.api_key_env = Some("PATH".to_string());
    config.voice.tts.api_key_env = Some("PATH".to_string());

    let report = voice_operator_report_summary(&config, workspace.path(), 12, Some(30)).await?;

    assert_eq!(report.status, "attention");
    assert!(report.voice_status.enabled);
    assert_eq!(report.voice_lane.stale_sessions, 1);
    assert_eq!(report.talk_lane.error_sessions, 1);
    assert_eq!(report.bounded_call_lane.stale_calls, 1);
    assert!(
        report
            .attention_signals
            .iter()
            .any(|signal| signal.kind == "voice_session_stale")
    );
    assert!(
        report
            .attention_signals
            .iter()
            .any(|signal| signal.kind == "talk_error")
    );
    assert!(
        report
            .attention_signals
            .iter()
            .any(|signal| signal.kind == "voice_call_stale")
    );
    assert!(
        report
            .recent_activity
            .iter()
            .any(|entry| entry.kind == "voice_session" && entry.label == "voice-stale")
    );
    assert!(
        report
            .recent_activity
            .iter()
            .any(|entry| entry.kind == "talk_session" && entry.label == "talk-error")
    );
    assert!(
        report
            .recent_activity
            .iter()
            .any(|entry| entry.kind == "voice_call" && entry.label == "call-stale")
    );

    Ok(())
}
