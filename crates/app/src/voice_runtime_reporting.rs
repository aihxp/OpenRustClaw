use crate::voice_runtime_lifecycle::VoiceSessionRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionTranscriptTurn {
    pub index: usize,
    pub role: String,
    pub text: String,
    pub created_at: String,
    pub synthesized_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionTranscript {
    pub session_id: String,
    pub status: String,
    pub live_state: String,
    pub turn_count: usize,
    pub last_activity_at: String,
    pub closed_at: Option<String>,
    pub turns: Vec<VoiceSessionTranscriptTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionArtifact {
    pub index: usize,
    pub role: String,
    pub created_at: String,
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceArtifactSnapshot {
    pub index: usize,
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionArtifacts {
    pub session_id: String,
    pub status: String,
    pub live_state: String,
    pub artifact_count: usize,
    pub total_bytes: u64,
    pub artifacts: Vec<VoiceSessionArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionEvent {
    pub index: usize,
    pub kind: String,
    pub created_at: String,
    pub summary: String,
    pub synthesized_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionEvents {
    pub session_id: String,
    pub status: String,
    pub live_state: String,
    pub event_count: usize,
    pub events: Vec<VoiceSessionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceSessionMetrics {
    pub session_id: String,
    pub status: String,
    pub live_state: String,
    pub turn_count: usize,
    pub user_turn_count: usize,
    pub assistant_turn_count: usize,
    pub transcript_chars: usize,
    pub user_chars: usize,
    pub assistant_chars: usize,
    pub artifact_count: usize,
    pub artifact_bytes: u64,
    pub reconnect_count: usize,
    pub pause_count: usize,
    pub interrupted_count: usize,
    pub idle_secs: u64,
    pub duration_secs: Option<u64>,
    pub avg_user_turn_chars: Option<f64>,
    pub avg_assistant_turn_chars: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceMetricsSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub total_turns: usize,
    pub total_artifacts: usize,
    pub total_artifact_bytes: u64,
    pub total_transcript_chars: usize,
    pub avg_turns_per_session: f64,
    pub avg_session_duration_secs: Option<f64>,
    pub sessions: Vec<VoiceSessionMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceOutcomeRecord {
    pub session_id: String,
    pub status: String,
    pub live_state: String,
    pub outcome_label: String,
    pub detail: String,
    pub attention_needed: bool,
    pub stale: bool,
    pub idle_secs: u64,
    pub turn_count: usize,
    pub artifact_count: usize,
    pub end_reason: Option<String>,
    pub last_activity_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceOutcomesSummary {
    pub stale_after_secs: u64,
    pub total_sessions: usize,
    pub attention_needed: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub outcomes: Vec<VoiceOutcomeRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct VoiceRuntimeReportingService;

impl VoiceRuntimeReportingService {
    pub fn new() -> Self {
        Self
    }

    pub fn session_transcript(&self, session: VoiceSessionRecord) -> VoiceSessionTranscript {
        VoiceSessionTranscript {
            session_id: session.id,
            status: session.status,
            live_state: session.live_state,
            turn_count: session.turns.len(),
            last_activity_at: session.last_activity_at,
            closed_at: session.closed_at,
            turns: session
                .turns
                .into_iter()
                .enumerate()
                .map(|(index, turn)| VoiceSessionTranscriptTurn {
                    index,
                    role: turn.role,
                    text: turn.text,
                    created_at: turn.created_at,
                    synthesized_output_path: turn.synthesized_output_path,
                })
                .collect(),
        }
    }

    pub fn session_artifacts(
        &self,
        session: &VoiceSessionRecord,
        snapshots: Vec<VoiceArtifactSnapshot>,
    ) -> VoiceSessionArtifacts {
        let total_bytes = snapshots.iter().filter_map(|entry| entry.bytes).sum();
        let artifacts = snapshots
            .into_iter()
            .filter_map(|snapshot| {
                let turn = session.turns.get(snapshot.index)?;
                Some(VoiceSessionArtifact {
                    index: snapshot.index,
                    role: turn.role.clone(),
                    created_at: turn.created_at.clone(),
                    path: snapshot.path,
                    exists: snapshot.exists,
                    bytes: snapshot.bytes,
                })
            })
            .collect::<Vec<_>>();

        VoiceSessionArtifacts {
            session_id: session.id.clone(),
            status: session.status.clone(),
            live_state: session.live_state.clone(),
            artifact_count: artifacts.len(),
            total_bytes,
            artifacts,
        }
    }

    pub fn session_events(&self, session: &VoiceSessionRecord) -> VoiceSessionEvents {
        let mut events = Vec::new();
        events.push(VoiceSessionEvent {
            index: 0,
            kind: "session_started".to_string(),
            created_at: session.created_at.clone(),
            summary: format!(
                "session started with stt={} tts={} voice={}",
                session.stt_provider, session.tts_provider, session.tts_voice
            ),
            synthesized_output_path: None,
        });
        events.extend(session.turns.iter().map(|turn| VoiceSessionEvent {
            index: 0,
            kind: format!("{}_turn", turn.role),
            created_at: turn.created_at.clone(),
            summary: self.truncate_voice_event_summary(&turn.text, 96),
            synthesized_output_path: turn.synthesized_output_path.clone(),
        }));
        if let Some(created_at) = session.last_reconnected_at.as_ref() {
            events.push(VoiceSessionEvent {
                index: 0,
                kind: "session_reconnected".to_string(),
                created_at: created_at.clone(),
                summary: format!("reconnect_count={}", session.reconnect_count),
                synthesized_output_path: None,
            });
        }
        if let Some(created_at) = session.last_paused_at.as_ref() {
            events.push(VoiceSessionEvent {
                index: 0,
                kind: "session_paused".to_string(),
                created_at: created_at.clone(),
                summary: format!("pause_count={}", session.pause_count),
                synthesized_output_path: None,
            });
        }
        if let Some(created_at) = session.last_resumed_at.as_ref() {
            events.push(VoiceSessionEvent {
                index: 0,
                kind: "session_resumed".to_string(),
                created_at: created_at.clone(),
                summary: "session resumed".to_string(),
                synthesized_output_path: None,
            });
        }
        if let Some(created_at) = session.last_interrupted_at.as_ref() {
            events.push(VoiceSessionEvent {
                index: 0,
                kind: "session_interrupted".to_string(),
                created_at: created_at.clone(),
                summary: format!("interrupted_count={}", session.interrupted_count),
                synthesized_output_path: None,
            });
        }
        if let Some(created_at) = session.closed_at.as_ref() {
            events.push(VoiceSessionEvent {
                index: 0,
                kind: "session_ended".to_string(),
                created_at: created_at.clone(),
                summary: session
                    .end_reason
                    .clone()
                    .unwrap_or_else(|| "session ended".to_string()),
                synthesized_output_path: None,
            });
        }
        events.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        for (index, event) in events.iter_mut().enumerate() {
            event.index = index;
        }

        VoiceSessionEvents {
            session_id: session.id.clone(),
            status: session.status.clone(),
            live_state: session.live_state.clone(),
            event_count: events.len(),
            events,
        }
    }

    pub fn session_metrics(
        &self,
        session: VoiceSessionRecord,
        artifact_bytes: u64,
        idle_secs: u64,
        duration_secs: Option<u64>,
    ) -> VoiceSessionMetrics {
        let turn_count = session.turns.len();
        let user_turn_count = session
            .turns
            .iter()
            .filter(|turn| turn.role == "user")
            .count();
        let assistant_turn_count = session
            .turns
            .iter()
            .filter(|turn| turn.role == "assistant")
            .count();
        let user_chars = session
            .turns
            .iter()
            .filter(|turn| turn.role == "user")
            .map(|turn| turn.text.chars().count())
            .sum::<usize>();
        let assistant_chars = session
            .turns
            .iter()
            .filter(|turn| turn.role == "assistant")
            .map(|turn| turn.text.chars().count())
            .sum::<usize>();
        let transcript_chars = user_chars + assistant_chars;
        let artifact_count = session
            .turns
            .iter()
            .filter(|turn| turn.synthesized_output_path.is_some())
            .count();

        VoiceSessionMetrics {
            session_id: session.id,
            status: session.status,
            live_state: session.live_state,
            turn_count,
            user_turn_count,
            assistant_turn_count,
            transcript_chars,
            user_chars,
            assistant_chars,
            artifact_count,
            artifact_bytes,
            reconnect_count: session.reconnect_count,
            pause_count: session.pause_count,
            interrupted_count: session.interrupted_count,
            idle_secs,
            duration_secs,
            avg_user_turn_chars: (user_turn_count > 0)
                .then(|| user_chars as f64 / user_turn_count as f64),
            avg_assistant_turn_chars: (assistant_turn_count > 0)
                .then(|| assistant_chars as f64 / assistant_turn_count as f64),
        }
    }

    pub fn metrics_summary(&self, sessions: Vec<VoiceSessionMetrics>) -> VoiceMetricsSummary {
        let total_sessions = sessions.len();
        let active_sessions = sessions
            .iter()
            .filter(|entry| entry.status == "active")
            .count();
        let ended_sessions = sessions
            .iter()
            .filter(|entry| entry.status == "ended")
            .count();
        let total_turns = sessions.iter().map(|entry| entry.turn_count).sum();
        let total_artifacts = sessions.iter().map(|entry| entry.artifact_count).sum();
        let total_artifact_bytes = sessions.iter().map(|entry| entry.artifact_bytes).sum();
        let total_transcript_chars = sessions.iter().map(|entry| entry.transcript_chars).sum();
        let avg_turns_per_session = if total_sessions == 0 {
            0.0
        } else {
            total_turns as f64 / total_sessions as f64
        };
        let durations = sessions
            .iter()
            .filter_map(|entry| entry.duration_secs.map(|value| value as f64))
            .collect::<Vec<_>>();

        VoiceMetricsSummary {
            total_sessions,
            active_sessions,
            ended_sessions,
            total_turns,
            total_artifacts,
            total_artifact_bytes,
            total_transcript_chars,
            avg_turns_per_session,
            avg_session_duration_secs: (!durations.is_empty())
                .then(|| durations.iter().sum::<f64>() / durations.len() as f64),
            sessions,
        }
    }

    pub fn outcome(
        &self,
        session: &VoiceSessionRecord,
        stale_after_secs: u64,
        idle_secs: u64,
        has_missing_artifact: bool,
    ) -> VoiceOutcomeRecord {
        let stale = session.status == "active" && idle_secs >= stale_after_secs;
        let artifact_count = session
            .turns
            .iter()
            .filter(|turn| turn.synthesized_output_path.is_some())
            .count();
        let (outcome_label, detail, attention_needed) = if stale {
            (
                "stale".to_string(),
                format!("idle for {idle_secs}s; intervention recommended"),
                true,
            )
        } else if session.status == "ended" {
            (
                "ended".to_string(),
                session
                    .end_reason
                    .clone()
                    .unwrap_or_else(|| "session ended".to_string()),
                false,
            )
        } else if session.live_state == "paused" {
            (
                "paused".to_string(),
                "session paused and awaiting resume".to_string(),
                true,
            )
        } else if session.live_state == "interrupted" {
            (
                "interrupted".to_string(),
                "session interrupted and awaiting resume".to_string(),
                true,
            )
        } else if has_missing_artifact {
            (
                "artifact_gap".to_string(),
                "session references missing synthesized output artifacts".to_string(),
                true,
            )
        } else if session.status == "active" {
            (
                "active".to_string(),
                format!(
                    "{} turns, {} audio artifacts, idle {}s",
                    session.turns.len(),
                    artifact_count,
                    idle_secs
                ),
                false,
            )
        } else {
            (
                session.status.clone(),
                format!(
                    "status={} live_state={}",
                    session.status, session.live_state
                ),
                true,
            )
        };

        VoiceOutcomeRecord {
            session_id: session.id.clone(),
            status: session.status.clone(),
            live_state: session.live_state.clone(),
            outcome_label,
            detail,
            attention_needed,
            stale,
            idle_secs,
            turn_count: session.turns.len(),
            artifact_count,
            end_reason: session.end_reason.clone(),
            last_activity_at: session.last_activity_at.clone(),
            closed_at: session.closed_at.clone(),
        }
    }

    pub fn outcomes_summary(
        &self,
        stale_after_secs: u64,
        total_sessions: usize,
        mut outcomes: Vec<VoiceOutcomeRecord>,
        limit: usize,
    ) -> VoiceOutcomesSummary {
        outcomes.sort_by(|left, right| {
            right
                .last_activity_at
                .cmp(&left.last_activity_at)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        outcomes.truncate(limit.max(1));
        let attention_needed = outcomes
            .iter()
            .filter(|entry| entry.attention_needed)
            .count();
        let active_sessions = outcomes
            .iter()
            .filter(|entry| entry.status == "active")
            .count();
        let ended_sessions = outcomes
            .iter()
            .filter(|entry| entry.status == "ended")
            .count();

        VoiceOutcomesSummary {
            stale_after_secs,
            total_sessions,
            attention_needed,
            active_sessions,
            ended_sessions,
            outcomes,
        }
    }

    fn truncate_voice_event_summary(&self, text: &str, limit: usize) -> String {
        let trimmed = text.trim();
        if trimmed.chars().count() <= limit {
            return trimmed.to_string();
        }
        let summarized = trimmed.chars().take(limit).collect::<String>();
        format!("{summarized}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_runtime_lifecycle::VoiceSessionTurn;

    fn sample_session() -> VoiceSessionRecord {
        VoiceSessionRecord {
            id: "voice-1".to_string(),
            status: "active".to_string(),
            live_state: "listening".to_string(),
            stt_provider: "openai".to_string(),
            tts_provider: "openai".to_string(),
            tts_voice: "alloy".to_string(),
            assistant_prompt: None,
            end_reason: None,
            created_at: "2026-03-28T20:00:00Z".to_string(),
            last_activity_at: "2026-03-28T20:00:02Z".to_string(),
            closed_at: None,
            reconnect_count: 0,
            last_reconnected_at: None,
            pause_count: 0,
            interrupted_count: 0,
            last_paused_at: None,
            last_resumed_at: None,
            last_interrupted_at: None,
            turns: vec![
                VoiceSessionTurn {
                    role: "user".to_string(),
                    text: "hello".to_string(),
                    created_at: "2026-03-28T20:00:01Z".to_string(),
                    synthesized_output_path: None,
                },
                VoiceSessionTurn {
                    role: "assistant".to_string(),
                    text: "hi".to_string(),
                    created_at: "2026-03-28T20:00:02Z".to_string(),
                    synthesized_output_path: Some("voice/out.mp3".to_string()),
                },
            ],
        }
    }

    #[test]
    fn reporting_services_shape_transcripts_and_metrics() {
        let service = VoiceRuntimeReportingService::new();
        let transcript = service.session_transcript(sample_session());
        assert_eq!(transcript.turn_count, 2);

        let metrics = service.session_metrics(sample_session(), 42, 5, Some(2));
        assert_eq!(metrics.artifact_count, 1);
        assert_eq!(metrics.transcript_chars, 7);

        let summary = service.metrics_summary(vec![metrics]);
        assert_eq!(summary.total_sessions, 1);
        assert_eq!(summary.total_artifacts, 1);
    }

    #[test]
    fn outcomes_surface_attention_for_stale_sessions() {
        let service = VoiceRuntimeReportingService::new();
        let mut stale = sample_session();
        stale.last_activity_at = "2026-03-28T19:00:00Z".to_string();
        let outcome = service.outcome(&stale, 300, 3600, false);
        assert!(outcome.attention_needed);
        assert_eq!(outcome.outcome_label, "stale");
    }
}
