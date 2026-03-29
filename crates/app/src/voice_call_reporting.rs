use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceCallRecord {
    pub call_id: String,
    pub plugin_id: String,
    pub skill_name: String,
    pub status: String,
    #[serde(default = "default_voice_call_health")]
    pub health: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_hook_output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_hook_output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconnect_hook_output: Option<serde_json::Value>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub reconnect_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconnected_at: Option<String>,
    #[serde(default = "default_voice_call_stale_after_secs")]
    pub stale_after_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoiceCallHealthSummary {
    pub total: usize,
    pub active: usize,
    pub stale: usize,
    pub ended: usize,
    pub reaped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_active_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoiceCallMetricsSummary {
    pub total: usize,
    pub active: usize,
    pub stale: usize,
    pub ended: usize,
    pub reaped: usize,
    pub reconnects: usize,
    pub with_greeting_audio: usize,
    pub with_start_hook_output: usize,
    pub with_end_hook_output: usize,
    pub with_reconnect_hook_output: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoiceCallArtifact {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoiceCallEvent {
    pub kind: String,
    pub observed_at: String,
    pub call_id: String,
    pub plugin_id: String,
    pub skill_name: String,
    pub status: String,
    pub health: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub details: serde_json::Value,
}

pub struct VoiceCallReportingService;

impl VoiceCallReportingService {
    pub fn refresh_health(call: &mut VoiceCallRecord, now: DateTime<Utc>) {
        if call.ended_at.is_some() {
            if call.status == "reaped" {
                call.health = "reaped".to_string();
            } else {
                call.health = "ended".to_string();
            }
            return;
        }

        let reference = call
            .last_seen_at
            .as_deref()
            .and_then(parse_rfc3339_utc)
            .or_else(|| parse_rfc3339_utc(&call.started_at))
            .unwrap_or(now);
        let stale_after_secs = call.stale_after_secs.max(1);
        let age = now.signed_duration_since(reference).num_seconds().max(0) as u64;
        call.health = if age > stale_after_secs {
            "stale".to_string()
        } else {
            "active".to_string()
        };
    }

    pub fn refresh_all_health(calls: &mut [VoiceCallRecord], now: DateTime<Utc>) {
        for call in calls {
            Self::refresh_health(call, now);
        }
    }

    pub fn health_summary(calls: &[VoiceCallRecord]) -> VoiceCallHealthSummary {
        let mut active = 0usize;
        let mut stale = 0usize;
        let mut ended = 0usize;
        let mut reaped = 0usize;
        let mut oldest_active: Option<(DateTime<Utc>, String)> = None;

        for call in calls {
            match call.health.as_str() {
                "active" => {
                    active += 1;
                    if let Some(started_at) = parse_rfc3339_utc(&call.started_at) {
                        let replace = oldest_active
                            .as_ref()
                            .map(|(current, _)| started_at < *current)
                            .unwrap_or(true);
                        if replace {
                            oldest_active = Some((started_at, call.call_id.clone()));
                        }
                    }
                }
                "stale" => stale += 1,
                "reaped" => reaped += 1,
                _ => ended += 1,
            }
        }

        VoiceCallHealthSummary {
            total: calls.len(),
            active,
            stale,
            ended,
            reaped,
            oldest_active_call_id: oldest_active.map(|(_, id)| id),
        }
    }

    pub fn metrics_summary(calls: &[VoiceCallRecord]) -> VoiceCallMetricsSummary {
        let mut summary = VoiceCallMetricsSummary {
            total: calls.len(),
            active: 0,
            stale: 0,
            ended: 0,
            reaped: 0,
            reconnects: 0,
            with_greeting_audio: 0,
            with_start_hook_output: 0,
            with_end_hook_output: 0,
            with_reconnect_hook_output: 0,
        };

        for call in calls {
            summary.reconnects += call.reconnect_count;
            if call.greeting_audio_path.is_some() {
                summary.with_greeting_audio += 1;
            }
            if call.start_hook_output.is_some() {
                summary.with_start_hook_output += 1;
            }
            if call.end_hook_output.is_some() {
                summary.with_end_hook_output += 1;
            }
            if call.reconnect_hook_output.is_some() {
                summary.with_reconnect_hook_output += 1;
            }
            match call.health.as_str() {
                "active" => summary.active += 1,
                "stale" => summary.stale += 1,
                "reaped" => summary.reaped += 1,
                _ => summary.ended += 1,
            }
        }

        summary
    }

    pub fn artifacts_for_record(call: &VoiceCallRecord) -> Vec<VoiceCallArtifact> {
        let mut artifacts = Vec::new();
        if let Some(path) = call.greeting_audio_path.clone() {
            artifacts.push(VoiceCallArtifact {
                kind: "greeting_audio".to_string(),
                path: Some(path),
                output: None,
                observed_at: call.started_at.clone(),
            });
        }
        if let Some(output) = call.start_hook_output.clone() {
            artifacts.push(VoiceCallArtifact {
                kind: "start_hook_output".to_string(),
                path: None,
                output: Some(output),
                observed_at: call.started_at.clone(),
            });
        }
        if let Some(output) = call.reconnect_hook_output.clone() {
            artifacts.push(VoiceCallArtifact {
                kind: "reconnect_hook_output".to_string(),
                path: None,
                output: Some(output),
                observed_at: call.last_reconnected_at.clone().unwrap_or_else(|| {
                    call.last_seen_at
                        .clone()
                        .unwrap_or_else(|| call.started_at.clone())
                }),
            });
        }
        if let Some(output) = call.end_hook_output.clone() {
            artifacts.push(VoiceCallArtifact {
                kind: "end_hook_output".to_string(),
                path: None,
                output: Some(output),
                observed_at: call.ended_at.clone().unwrap_or_else(|| {
                    call.last_seen_at
                        .clone()
                        .unwrap_or_else(|| call.started_at.clone())
                }),
            });
        }
        artifacts
    }

    pub fn events_for_record(call: &VoiceCallRecord) -> Vec<VoiceCallEvent> {
        let mut events = Vec::new();
        events.push(VoiceCallEvent {
            kind: "started".to_string(),
            observed_at: call.started_at.clone(),
            call_id: call.call_id.clone(),
            plugin_id: call.plugin_id.clone(),
            skill_name: call.skill_name.clone(),
            status: call.status.clone(),
            health: call.health.clone(),
            remote: call.remote.clone(),
            reason: None,
            details: serde_json::json!({
                "greeting_text": call.greeting_text,
                "stale_after_secs": call.stale_after_secs,
                "reconnect_count": call.reconnect_count,
            }),
        });

        if call.reconnect_count > 0 {
            events.push(VoiceCallEvent {
                kind: "reconnected".to_string(),
                observed_at: call.last_reconnected_at.clone().unwrap_or_else(|| {
                    call.last_seen_at
                        .clone()
                        .unwrap_or_else(|| call.started_at.clone())
                }),
                call_id: call.call_id.clone(),
                plugin_id: call.plugin_id.clone(),
                skill_name: call.skill_name.clone(),
                status: call.status.clone(),
                health: call.health.clone(),
                remote: call.remote.clone(),
                reason: None,
                details: serde_json::json!({
                    "reconnect_count": call.reconnect_count,
                    "reconnect_hook_output": call.reconnect_hook_output,
                }),
            });
        }

        events.push(VoiceCallEvent {
            kind: "heartbeat".to_string(),
            observed_at: call
                .last_seen_at
                .clone()
                .unwrap_or_else(|| call.started_at.clone()),
            call_id: call.call_id.clone(),
            plugin_id: call.plugin_id.clone(),
            skill_name: call.skill_name.clone(),
            status: call.status.clone(),
            health: call.health.clone(),
            remote: call.remote.clone(),
            reason: call.reason.clone(),
            details: serde_json::json!({
                "last_seen_at": call.last_seen_at,
                "ended_at": call.ended_at,
                "stale_after_secs": call.stale_after_secs,
            }),
        });

        if call.ended_at.is_some() {
            events.push(VoiceCallEvent {
                kind: if call.status == "reaped" {
                    "reaped".to_string()
                } else {
                    "ended".to_string()
                },
                observed_at: call.ended_at.clone().unwrap_or_else(|| {
                    call.last_seen_at
                        .clone()
                        .unwrap_or_else(|| call.started_at.clone())
                }),
                call_id: call.call_id.clone(),
                plugin_id: call.plugin_id.clone(),
                skill_name: call.skill_name.clone(),
                status: call.status.clone(),
                health: call.health.clone(),
                remote: call.remote.clone(),
                reason: call.reason.clone(),
                details: serde_json::json!({
                    "reason": call.reason,
                    "end_hook_output": call.end_hook_output,
                }),
            });
        }

        events.sort_by(|left, right| {
            left.observed_at
                .cmp(&right.observed_at)
                .then_with(|| left.kind.cmp(&right.kind))
        });
        events
    }
}

fn default_voice_call_stale_after_secs() -> u64 {
    300
}

fn default_voice_call_health() -> String {
    "active".to_string()
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::{VoiceCallRecord, VoiceCallReportingService};
    use chrono::{DateTime, Utc};

    #[test]
    fn health_summary_and_events_cover_lifecycle() {
        let mut calls = vec![VoiceCallRecord {
            call_id: "call-1".to_string(),
            plugin_id: "plugin-1".to_string(),
            skill_name: "demo".to_string(),
            status: "active".to_string(),
            health: "active".to_string(),
            remote: Some("+15551230000".to_string()),
            greeting_text: None,
            greeting_audio_path: None,
            reason: None,
            metadata: serde_json::json!({}),
            start_hook_output: None,
            end_hook_output: None,
            reconnect_hook_output: None,
            started_at: "2026-03-28T10:00:00Z".to_string(),
            last_seen_at: Some("2026-03-28T10:10:00Z".to_string()),
            reconnect_count: 1,
            last_reconnected_at: Some("2026-03-28T10:05:00Z".to_string()),
            stale_after_secs: 60,
            ended_at: None,
        }];

        VoiceCallReportingService::refresh_all_health(
            &mut calls,
            DateTime::parse_from_rfc3339("2026-03-28T10:12:00Z")
                .expect("time")
                .with_timezone(&Utc),
        );
        assert_eq!(calls[0].health, "stale");
        assert_eq!(VoiceCallReportingService::health_summary(&calls).stale, 1);
        assert_eq!(
            VoiceCallReportingService::metrics_summary(&calls).reconnects,
            1
        );
        assert_eq!(
            VoiceCallReportingService::events_for_record(&calls[0]).len(),
            3
        );
    }
}
