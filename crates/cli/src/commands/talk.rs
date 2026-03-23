//! Talk Mode command - continuous voice conversation.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use openrustclaw_voice::start_live_audio_capture;
use openrustclaw_voice::{
    SimpleWakeDetector, SpeechToText, SttConfig, TalkConfig, TalkModeBuilder, TalkState,
    TextToSpeech, TtsConfig, WakeWordConfig,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionStateSnapshot {
    pub observed_at: String,
    pub state: String,
    pub turn_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionTurn {
    pub index: usize,
    pub user_text: String,
    pub agent_response: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionReceipt {
    pub id: String,
    pub status: String,
    pub requested_provider: String,
    #[serde(default)]
    pub requested_model: Option<String>,
    pub wake_word: String,
    pub silence_timeout_secs: u64,
    pub max_utterance_secs: u64,
    pub barge_in: bool,
    pub created_at: String,
    pub last_activity_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub final_state: Option<String>,
    #[serde(default)]
    pub turn_count: usize,
    #[serde(default)]
    pub snapshots: Vec<TalkSessionStateSnapshot>,
    #[serde(default)]
    pub turns: Vec<TalkSessionTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionSummary {
    pub id: String,
    pub status: String,
    pub requested_provider: String,
    #[serde(default)]
    pub requested_model: Option<String>,
    pub wake_word: String,
    pub created_at: String,
    pub last_activity_at: String,
    pub turn_count: usize,
    #[serde(default)]
    pub final_state: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionList {
    pub sessions: Vec<TalkSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkRuntimeStatus {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    #[serde(default)]
    pub latest_session: Option<TalkSessionSummary>,
    pub sessions: Vec<TalkSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkRuntimeListRequest {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionEvent {
    pub index: usize,
    pub kind: String,
    pub observed_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionEvents {
    pub session_id: String,
    pub status: String,
    pub turn_count: usize,
    pub event_count: usize,
    pub events: Vec<TalkSessionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkSessionMetrics {
    pub session_id: String,
    pub status: String,
    pub turn_count: usize,
    pub snapshot_count: usize,
    pub event_count: usize,
    #[serde(default)]
    pub final_state: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkRuntimeMetrics {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub error_sessions: usize,
    pub total_turns: usize,
    pub total_events: usize,
    pub avg_turns_per_session: f64,
    pub sessions: Vec<TalkSessionMetrics>,
}

/// Run the Talk Mode voice conversation.
pub async fn run(
    provider: &str,
    model: Option<&str>,
    wake_word: &str,
    silence_timeout_secs: u64,
    max_utterance_secs: u64,
    enable_barge_in: bool,
) -> Result<()> {
    let workspace_root =
        std::env::current_dir().context("Failed to determine current workspace root")?;
    run_with_workspace_root(
        &workspace_root,
        provider,
        model,
        wake_word,
        silence_timeout_secs,
        max_utterance_secs,
        enable_barge_in,
    )
    .await
}

/// Inspect the current talk runtime status and recent receipts.
pub async fn status(workspace_root: &Path, limit: usize) -> Result<()> {
    let payload = runtime_status(workspace_root, limit).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// List recent talk receipts.
pub async fn sessions(workspace_root: &Path, limit: usize) -> Result<()> {
    let payload = list_talk_sessions(workspace_root, limit).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Inspect one talk receipt.
pub async fn inspect(workspace_root: &Path, session_id: &str) -> Result<()> {
    let payload = inspect_talk_session(workspace_root, session_id).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Inspect the derived event timeline for one talk receipt.
pub async fn events(workspace_root: &Path, session_id: &str) -> Result<()> {
    let payload = talk_session_events_data(workspace_root, session_id).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Inspect derived metrics for one talk receipt.
pub async fn session_metrics(workspace_root: &Path, session_id: &str) -> Result<()> {
    let payload = talk_session_metrics_data(workspace_root, session_id).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Inspect aggregate talk runtime metrics.
pub async fn metrics(workspace_root: &Path) -> Result<()> {
    let payload = runtime_metrics_data(workspace_root).await?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

async fn run_with_workspace_root(
    workspace_root: &Path,
    provider: &str,
    model: Option<&str>,
    wake_word: &str,
    silence_timeout_secs: u64,
    max_utterance_secs: u64,
    enable_barge_in: bool,
) -> Result<()> {
    print_banner();

    println!("Requested provider: {}", provider);
    println!("Requested model: {}", model.unwrap_or("<none>"));
    println!("Wake word: \"{}\"", wake_word);
    println!("Silence timeout: {}s", silence_timeout_secs);
    println!("Max utterance: {}s", max_utterance_secs);
    println!(
        "Barge-in: {}",
        if enable_barge_in {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    let session_id = format!("talk-{}", Uuid::new_v4());
    let receipt_path = talk_session_path(workspace_root, &session_id);
    println!("Talk receipt: {}", receipt_path.display());
    println!();

    // Create voice components.
    let wake = Arc::new(SimpleWakeDetector::new(WakeWordConfig::with_wake_word(
        wake_word,
    )));
    let stt = Arc::new(
        SpeechToText::new(SttConfig::default())
            .map_err(|e| anyhow::anyhow!("Failed to create STT: {}", e))?,
    );
    let tts = Arc::new(
        TextToSpeech::new(TtsConfig::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create TTS: {}", e))?,
    );

    let _audio_capture = start_live_audio_capture(&wake)
        .map_err(|error| anyhow::anyhow!("Failed to start live audio capture: {}", error))?;

    let config = TalkConfig::with_wake_word(wake_word)
        .with_silence_timeout(Duration::from_secs(silence_timeout_secs))
        .with_max_utterance_duration(Duration::from_secs(max_utterance_secs))
        .with_barge_in(enable_barge_in)
        .with_session_timeout(Some(Duration::from_secs(300)));

    let talk_mode = Arc::new(
        TalkModeBuilder::new()
            .wake(wake)
            .stt(stt)
            .tts(tts)
            .config(config)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Talk Mode: {}", e))?,
    );

    let receipt = Arc::new(Mutex::new(TalkSessionReceipt {
        id: session_id.clone(),
        status: "active".to_string(),
        requested_provider: provider.to_string(),
        requested_model: model.map(|value| value.to_string()),
        wake_word: wake_word.to_string(),
        silence_timeout_secs,
        max_utterance_secs,
        barge_in: enable_barge_in,
        created_at: now_rfc3339(),
        last_activity_at: now_rfc3339(),
        closed_at: None,
        final_state: None,
        turn_count: 0,
        snapshots: vec![TalkSessionStateSnapshot {
            observed_at: now_rfc3339(),
            state: TalkState::Idle.to_string(),
            turn_count: 0,
        }],
        turns: vec![],
    }));
    {
        let receipt_guard = receipt.lock().await;
        save_talk_session(workspace_root, &receipt_guard).await?;
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let receipt_poll = receipt.clone();
    let talk_mode_poll = talk_mode.clone();
    let workspace_poll = workspace_root.to_path_buf();
    let stop_poll = stop_flag.clone();
    let poll_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        let mut last_state = TalkState::Idle;
        let mut last_turn_count = 0usize;

        loop {
            ticker.tick().await;
            if stop_poll.load(Ordering::SeqCst) {
                break;
            }

            let state = talk_mode_poll.current_state().await;
            let history = talk_mode_poll.history().await;
            let turn_count = history.len();
            let now = now_rfc3339();
            let mut receipt = receipt_poll.lock().await;

            receipt.last_activity_at = now.clone();
            receipt.turn_count = turn_count;

            if state != last_state {
                receipt.snapshots.push(TalkSessionStateSnapshot {
                    observed_at: now.clone(),
                    state: state.to_string(),
                    turn_count,
                });
                last_state = state;
            }

            if turn_count != last_turn_count || receipt.turns.len() != turn_count {
                receipt.turns = history
                    .iter()
                    .enumerate()
                    .map(|(index, turn)| TalkSessionTurn {
                        index,
                        user_text: turn.user_text.clone(),
                        agent_response: turn.agent_response.clone(),
                        captured_at: now.clone(),
                    })
                    .collect();
                last_turn_count = turn_count;
            }

            if let Err(error) = save_talk_session(&workspace_poll, &receipt).await {
                warn!(error = %error, "Failed to persist talk receipt snapshot");
            }
        }
    });

    println!("Talk Mode started!");
    println!("Say \"{}\" to start a conversation.", wake_word);
    println!();
    println!("Commands:");
    println!("  Ctrl+C - Stop Talk Mode");
    println!();

    let talk_mode_for_ctrlc = talk_mode.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Ctrl+C received, stopping...");
        talk_mode_for_ctrlc.stop().await;
    });

    let result = match talk_mode.run().await {
        Ok(()) => {
            println!("\nTalk Mode ended.");
            Ok(())
        }
        Err(e) => {
            error!("Talk Mode error: {}", e);
            Err(anyhow::anyhow!("Talk Mode failed: {}", e))
        }
    };

    stop_flag.store(true, Ordering::SeqCst);
    let final_state = talk_mode.current_state().await;
    let history = talk_mode.history().await;
    {
        let mut receipt = receipt.lock().await;
        receipt.status = if result.is_ok() {
            "ended".to_string()
        } else {
            "error".to_string()
        };
        receipt.last_activity_at = now_rfc3339();
        receipt.closed_at = Some(now_rfc3339());
        receipt.final_state = Some(final_state.to_string());
        receipt.turn_count = history.len();
        receipt.turns = history
            .iter()
            .enumerate()
            .map(|(index, turn)| TalkSessionTurn {
                index,
                user_text: turn.user_text.clone(),
                agent_response: turn.agent_response.clone(),
                captured_at: now_rfc3339(),
            })
            .collect();
        receipt.snapshots.push(TalkSessionStateSnapshot {
            observed_at: now_rfc3339(),
            state: final_state.to_string(),
            turn_count: history.len(),
        });
        save_talk_session(workspace_root, &receipt).await?;
    }

    let _ = poll_handle.await;
    result
}

/// Talk runtime status and recent receipts.
pub async fn runtime_status(workspace_root: &Path, limit: usize) -> Result<TalkRuntimeStatus> {
    let sessions = load_talk_sessions(workspace_root).await?;
    let total_sessions = sessions.len();
    let active_sessions = sessions
        .iter()
        .filter(|entry| entry.status == "active")
        .count();
    let ended_sessions = sessions
        .iter()
        .filter(|entry| entry.status == "ended")
        .count();
    let latest_session = sessions.first().map(|entry| entry.summary());
    let sessions = sessions
        .into_iter()
        .take(limit.max(1))
        .map(|entry| entry.summary())
        .collect::<Vec<_>>();

    Ok(TalkRuntimeStatus {
        total_sessions,
        active_sessions,
        ended_sessions,
        latest_session,
        sessions,
    })
}

/// Aggregate talk runtime metrics and per-session summaries.
pub async fn runtime_metrics_data(workspace_root: &Path) -> Result<TalkRuntimeMetrics> {
    let sessions = load_talk_sessions(workspace_root).await?;
    let total_sessions = sessions.len();
    let active_sessions = sessions
        .iter()
        .filter(|entry| entry.status == "active")
        .count();
    let ended_sessions = sessions
        .iter()
        .filter(|entry| entry.status == "ended")
        .count();
    let error_sessions = sessions
        .iter()
        .filter(|entry| entry.status == "error")
        .count();
    let total_turns = sessions.iter().map(|entry| entry.turn_count).sum::<usize>();
    let total_events = sessions
        .iter()
        .map(|entry| entry.timeline_events().len())
        .sum::<usize>();
    let avg_turns_per_session = if total_sessions == 0 {
        0.0
    } else {
        total_turns as f64 / total_sessions as f64
    };

    Ok(TalkRuntimeMetrics {
        total_sessions,
        active_sessions,
        ended_sessions,
        error_sessions,
        total_turns,
        total_events,
        avg_turns_per_session,
        sessions: sessions.into_iter().map(|entry| entry.metrics()).collect(),
    })
}

/// List recent talk receipts.
pub async fn list_talk_sessions(workspace_root: &Path, limit: usize) -> Result<TalkSessionList> {
    let sessions = load_talk_sessions(workspace_root).await?;
    Ok(TalkSessionList {
        sessions: sessions
            .into_iter()
            .take(limit.max(1))
            .map(|entry| entry.summary())
            .collect(),
    })
}

/// Inspect one talk receipt.
pub async fn inspect_talk_session(
    workspace_root: &Path,
    session_id: &str,
) -> Result<TalkSessionReceipt> {
    let path = talk_session_path(workspace_root, session_id);
    let bytes = fs::read(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn talk_session_events_data(
    workspace_root: &Path,
    session_id: &str,
) -> Result<TalkSessionEvents> {
    let session = inspect_talk_session(workspace_root, session_id).await?;
    let events = session.timeline_events();
    Ok(TalkSessionEvents {
        session_id: session.id,
        status: session.status,
        turn_count: session.turn_count,
        event_count: events.len(),
        events,
    })
}

pub async fn talk_session_metrics_data(
    workspace_root: &Path,
    session_id: &str,
) -> Result<TalkSessionMetrics> {
    let session = inspect_talk_session(workspace_root, session_id).await?;
    Ok(session.metrics())
}

impl TalkSessionReceipt {
    fn summary(&self) -> TalkSessionSummary {
        TalkSessionSummary {
            id: self.id.clone(),
            status: self.status.clone(),
            requested_provider: self.requested_provider.clone(),
            requested_model: self.requested_model.clone(),
            wake_word: self.wake_word.clone(),
            created_at: self.created_at.clone(),
            last_activity_at: self.last_activity_at.clone(),
            turn_count: self.turn_count,
            final_state: self.final_state.clone(),
            closed_at: self.closed_at.clone(),
        }
    }

    fn metrics(&self) -> TalkSessionMetrics {
        let duration_secs = self
            .closed_at
            .as_ref()
            .and_then(|closed_at| duration_secs_between(&self.created_at, closed_at));
        TalkSessionMetrics {
            session_id: self.id.clone(),
            status: self.status.clone(),
            turn_count: self.turn_count,
            snapshot_count: self.snapshots.len(),
            event_count: self.timeline_events().len(),
            final_state: self.final_state.clone(),
            closed_at: self.closed_at.clone(),
            duration_secs,
        }
    }

    fn timeline_events(&self) -> Vec<TalkSessionEvent> {
        let mut events = Vec::new();
        let mut index = 0usize;

        events.push(TalkSessionEvent {
            index,
            kind: "session_started".to_string(),
            observed_at: self.created_at.clone(),
            summary: format!(
                "Talk session {} started with wake word {}",
                self.id, self.wake_word
            ),
        });
        index += 1;

        for window in self.snapshots.windows(2) {
            let from = &window[0];
            let to = &window[1];
            if from.state != to.state {
                events.push(TalkSessionEvent {
                    index,
                    kind: "state_changed".to_string(),
                    observed_at: to.observed_at.clone(),
                    summary: format!(
                        "State changed from {} to {} at turn {}",
                        from.state, to.state, to.turn_count
                    ),
                });
                index += 1;
            }
        }

        for turn in &self.turns {
            events.push(TalkSessionEvent {
                index,
                kind: "turn_recorded".to_string(),
                observed_at: turn.captured_at.clone(),
                summary: format!(
                    "Turn {} recorded for user text {:?} and assistant response {:?}",
                    turn.index, turn.user_text, turn.agent_response
                ),
            });
            index += 1;
        }

        if let Some(closed_at) = self.closed_at.as_ref() {
            events.push(TalkSessionEvent {
                index,
                kind: "session_ended".to_string(),
                observed_at: closed_at.clone(),
                summary: format!(
                    "Talk session ended in {} with {} turns",
                    self.final_state.as_deref().unwrap_or(self.status.as_str()),
                    self.turn_count
                ),
            });
        }

        events
    }
}

fn duration_secs_between(start: &str, end: &str) -> Option<u64> {
    let start = DateTime::parse_from_rfc3339(start).ok()?;
    let end = DateTime::parse_from_rfc3339(end).ok()?;
    Some(end.signed_duration_since(start).num_seconds().max(0) as u64)
}

async fn load_talk_sessions(workspace_root: &Path) -> Result<Vec<TalkSessionReceipt>> {
    let dir = talk_sessions_dir(workspace_root);
    let mut sessions = Vec::new();

    match fs::read_dir(&dir).await {
        Ok(mut entries) => {
            while let Some(entry) = entries
                .next_entry()
                .await
                .with_context(|| format!("failed to scan {}", dir.display()))?
            {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let bytes = fs::read(&path)
                    .await
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let receipt: TalkSessionReceipt = serde_json::from_slice(&bytes)
                    .with_context(|| format!("failed to parse {}", path.display()))?;
                sessions.push(receipt);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read talk sessions from {}", dir.display()));
        }
    }

    sessions.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.last_activity_at.cmp(&a.last_activity_at))
            .then_with(|| b.id.cmp(&a.id))
    });

    Ok(sessions)
}

async fn save_talk_session(workspace_root: &Path, session: &TalkSessionReceipt) -> Result<()> {
    let dir = talk_sessions_dir(workspace_root);
    fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("failed to create {}", dir.display()))?;
    let path = talk_session_path(workspace_root, &session.id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(session).context("failed to serialize talk session")?,
    )
    .await
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn talk_sessions_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".claw").join("talk").join("sessions")
}

fn talk_session_path(workspace_root: &Path, session_id: &str) -> PathBuf {
    talk_sessions_dir(workspace_root).join(format!("{session_id}.json"))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// Print the Talk Mode banner.
fn print_banner() {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           OpenRustClaw Talk Mode                         ║");
    println!("║           Continuous Voice Conversation                  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn talk_receipts_round_trip() {
        let temp = tempdir().expect("tempdir");
        let receipt = TalkSessionReceipt {
            id: "talk-session-test".to_string(),
            status: "ended".to_string(),
            requested_provider: "anthropic".to_string(),
            requested_model: Some("claude".to_string()),
            wake_word: "Hey Assistant".to_string(),
            silence_timeout_secs: 3,
            max_utterance_secs: 30,
            barge_in: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_activity_at: "2026-01-01T00:00:10Z".to_string(),
            closed_at: Some("2026-01-01T00:00:11Z".to_string()),
            final_state: Some("idle".to_string()),
            turn_count: 1,
            snapshots: vec![TalkSessionStateSnapshot {
                observed_at: "2026-01-01T00:00:05Z".to_string(),
                state: "listening".to_string(),
                turn_count: 1,
            }],
            turns: vec![TalkSessionTurn {
                index: 0,
                user_text: "hello".to_string(),
                agent_response: "hi".to_string(),
                captured_at: "2026-01-01T00:00:06Z".to_string(),
            }],
        };

        save_talk_session(temp.path(), &receipt)
            .await
            .expect("save receipt");

        let list = list_talk_sessions(temp.path(), 20)
            .await
            .expect("list receipts");
        assert_eq!(list.sessions.len(), 1);
        assert_eq!(list.sessions[0].id, "talk-session-test");
        assert_eq!(list.sessions[0].turn_count, 1);

        let inspected = inspect_talk_session(temp.path(), "talk-session-test")
            .await
            .expect("inspect receipt");
        assert_eq!(inspected.requested_provider, "anthropic");
        assert_eq!(inspected.turns[0].user_text, "hello");

        let status = runtime_status(temp.path(), 20).await.expect("status");
        assert_eq!(status.total_sessions, 1);
        assert_eq!(status.ended_sessions, 1);
        assert!(status.latest_session.is_some());

        let metrics = runtime_metrics_data(temp.path()).await.expect("metrics");
        assert_eq!(metrics.total_sessions, 1);
        assert_eq!(metrics.total_turns, 1);
        assert_eq!(metrics.total_events, 3);

        let events = talk_session_events_data(temp.path(), "talk-session-test")
            .await
            .expect("events");
        assert_eq!(events.event_count, 3);
        assert_eq!(events.events[0].kind, "session_started");
        assert_eq!(events.events[1].kind, "turn_recorded");
        assert_eq!(events.events[2].kind, "session_ended");
    }
}
