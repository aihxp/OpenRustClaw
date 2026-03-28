use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationTraceEntrySummary {
    pub actor_type: String,
    pub actor_id: String,
    pub estimated_input_tokens: Option<usize>,
    pub estimated_output_tokens: Option<usize>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationResourceTotals {
    pub trace_count: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationActorResourceSummary {
    pub actor_type: String,
    pub actor_id: String,
    pub stage_count: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflectionRoutingContext {
    pub selected_claw_id: String,
    pub selected_model_profile_id: String,
    pub requested_model_profile_id: String,
    pub provider: String,
    pub fallback_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflectionWorkerResult {
    pub claw_id: String,
    pub status: String,
    pub summary: String,
    pub questions: Vec<String>,
    pub confidence: Option<f32>,
    pub next_step_recommendation: Option<String>,
    pub model_profile_id: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflectionCandidate {
    pub kind: String,
    pub signal: String,
    pub recommendation: String,
    pub rationale: Option<String>,
    pub confidence: Option<f32>,
    pub claw_id: Option<String>,
    pub model_profile_id: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestrationSupervisionSummary {
    pub checkpoint_count: usize,
    pub worker_count: usize,
    pub needs_input_count: usize,
    pub failed_count: usize,
    pub low_confidence_workers: Vec<String>,
    pub reflection_candidate_count: usize,
    pub escalation_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveSupervisionSnapshot {
    pub status: String,
    pub pause_requested: bool,
    pub kill_requested: bool,
    pub lifecycle_state: String,
    pub intervention_required: bool,
    pub escalation_requested: bool,
    pub rollback_requested: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OrchestrationReportingService;

impl OrchestrationReportingService {
    pub fn new() -> Self {
        Self
    }

    pub fn summarize_trace_resources(
        &self,
        trace: &[OrchestrationTraceEntrySummary],
    ) -> OrchestrationResourceTotals {
        OrchestrationResourceTotals {
            trace_count: trace.len(),
            estimated_input_tokens: trace
                .iter()
                .map(|entry| entry.estimated_input_tokens.unwrap_or(0))
                .sum(),
            estimated_output_tokens: trace
                .iter()
                .map(|entry| entry.estimated_output_tokens.unwrap_or(0))
                .sum(),
            duration_ms: trace
                .iter()
                .map(|entry| entry.duration_ms.unwrap_or(0))
                .sum(),
        }
    }

    pub fn summarize_actor_resources(
        &self,
        trace: &[OrchestrationTraceEntrySummary],
    ) -> Vec<OrchestrationActorResourceSummary> {
        let mut by_actor: BTreeMap<(String, String), OrchestrationActorResourceSummary> =
            BTreeMap::new();
        for entry in trace {
            let key = (entry.actor_type.clone(), entry.actor_id.clone());
            let summary =
                by_actor
                    .entry(key.clone())
                    .or_insert_with(|| OrchestrationActorResourceSummary {
                        actor_type: key.0.clone(),
                        actor_id: key.1.clone(),
                        stage_count: 0,
                        estimated_input_tokens: 0,
                        estimated_output_tokens: 0,
                        duration_ms: 0,
                    });
            summary.stage_count += 1;
            summary.estimated_input_tokens += entry.estimated_input_tokens.unwrap_or(0);
            summary.estimated_output_tokens += entry.estimated_output_tokens.unwrap_or(0);
            summary.duration_ms += entry.duration_ms.unwrap_or(0);
        }
        by_actor.into_values().collect()
    }

    pub fn build_reflection_candidates(
        &self,
        routing: &ReflectionRoutingContext,
        worker_results: &[ReflectionWorkerResult],
    ) -> Vec<ReflectionCandidate> {
        let mut candidates = Vec::new();
        if routing.fallback_path.len() > 1 {
            candidates.push(ReflectionCandidate {
                kind: "fallback_path".to_string(),
                signal: format!(
                    "model profile '{}' required fallback path {}",
                    routing.requested_model_profile_id,
                    routing.fallback_path.join(" -> ")
                ),
                recommendation:
                    "preflight this provider/model lane or promote a healthier fallback for this route"
                        .to_string(),
                rationale: Some(
                    "The primary model profile was unavailable or unsupported at run time."
                        .to_string(),
                ),
                confidence: Some(0.82),
                claw_id: Some(routing.selected_claw_id.clone()),
                model_profile_id: Some(routing.selected_model_profile_id.clone()),
                provider: Some(routing.provider.clone()),
            });
        }

        for worker in worker_results {
            if worker.status == "failed" || worker.status == "needs_input" {
                candidates.push(ReflectionCandidate {
                    kind: "worker_status".to_string(),
                    signal: format!("worker '{}' returned status '{}'", worker.claw_id, worker.status),
                    recommendation: if worker.status == "needs_input" {
                        "tighten task framing or add an approval/escalation checkpoint before delegation".to_string()
                    } else {
                        "capture a task-specific lesson and consider revising worker/model routing".to_string()
                    },
                    rationale: worker.next_step_recommendation.clone(),
                    confidence: worker.confidence.or(Some(0.7)),
                    claw_id: Some(worker.claw_id.clone()),
                    model_profile_id: Some(worker.model_profile_id.clone()),
                    provider: Some(worker.provider.clone()),
                });
            }
            if let Some(confidence) = worker.confidence
                && confidence < 0.6
            {
                candidates.push(ReflectionCandidate {
                    kind: "low_confidence".to_string(),
                    signal: format!(
                        "worker '{}' reported low confidence {:.2}",
                        worker.claw_id, confidence
                    ),
                    recommendation:
                        "route a critic or human review step before exposing this output as final"
                            .to_string(),
                    rationale: Some(worker.summary.clone()),
                    confidence: Some(confidence),
                    claw_id: Some(worker.claw_id.clone()),
                    model_profile_id: Some(worker.model_profile_id.clone()),
                    provider: Some(worker.provider.clone()),
                });
            }
            if !worker.questions.is_empty() {
                candidates.push(ReflectionCandidate {
                    kind: "open_questions".to_string(),
                    signal: format!(
                        "worker '{}' returned {} open questions",
                        worker.claw_id,
                        worker.questions.len()
                    ),
                    recommendation:
                        "surface blockers explicitly or add a follow-up worker turn instead of treating the run as fully complete"
                            .to_string(),
                    rationale: Some(worker.questions.join(" | ")),
                    confidence: worker.confidence.or(Some(0.65)),
                    claw_id: Some(worker.claw_id.clone()),
                    model_profile_id: Some(worker.model_profile_id.clone()),
                    provider: Some(worker.provider.clone()),
                });
            }
        }

        candidates
    }

    pub fn summarize_supervision(
        &self,
        checkpoint_count: usize,
        worker_results: &[ReflectionWorkerResult],
        reflection_candidates: &[ReflectionCandidate],
    ) -> OrchestrationSupervisionSummary {
        let needs_input_count = worker_results
            .iter()
            .filter(|worker| worker.status == "needs_input")
            .count();
        let failed_count = worker_results
            .iter()
            .filter(|worker| worker.status == "failed")
            .count();
        let low_confidence_workers = worker_results
            .iter()
            .filter_map(|worker| match worker.confidence {
                Some(confidence) if confidence < 0.6 => Some(worker.claw_id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        OrchestrationSupervisionSummary {
            checkpoint_count,
            worker_count: worker_results.len(),
            needs_input_count,
            failed_count,
            low_confidence_workers,
            reflection_candidate_count: reflection_candidates.len(),
            escalation_recommended: failed_count > 0
                || needs_input_count > 0
                || !reflection_candidates.is_empty(),
        }
    }

    pub fn attention_signals(
        &self,
        snapshot: &ActiveSupervisionSnapshot,
        recent_event_statuses: &[String],
    ) -> Vec<String> {
        let mut attention_signals = Vec::new();
        if snapshot.intervention_required {
            attention_signals.push(format!(
                "operator intervention required: {}",
                snapshot.lifecycle_state
            ));
        }
        if snapshot.pause_requested {
            attention_signals.push("pause requested by operator".to_string());
        }
        if snapshot.kill_requested {
            attention_signals.push("kill requested by operator".to_string());
        }
        if matches!(snapshot.status.as_str(), "failed" | "killed") {
            attention_signals.push(format!("run is {}", snapshot.status));
        }
        if let Some(error) = snapshot.last_error.as_deref()
            && !error.trim().is_empty()
        {
            attention_signals.push(format!("last error: {error}"));
        }
        if recent_event_statuses
            .iter()
            .any(|status| matches!(status.as_str(), "failed" | "killed"))
        {
            attention_signals.push("recent events include failed or killed status".to_string());
        }
        if snapshot.rollback_requested {
            attention_signals.push("rollback requested for this run".to_string());
        }
        if snapshot.escalation_requested {
            attention_signals.push("run has been escalated for operator review".to_string());
        }
        attention_signals
    }

    pub fn trace_payload(&self, run_id: &str, trace: Value, relationships: Value) -> Value {
        json!({
            "run_id": run_id,
            "trace": trace,
            "relationships": relationships,
        })
    }

    pub fn transcript_payload(
        &self,
        run_id: &str,
        transcript: Value,
        relationships: Value,
    ) -> Value {
        json!({
            "run_id": run_id,
            "transcript": transcript,
            "relationships": relationships,
        })
    }

    pub fn resources_payload(
        &self,
        run_id: &str,
        totals: &OrchestrationResourceTotals,
        actors: &[OrchestrationActorResourceSummary],
        trace: Value,
    ) -> Value {
        json!({
            "run_id": run_id,
            "totals": totals,
            "actors": actors,
            "trace": trace,
        })
    }

    pub fn receipt_supervision_payload(&self, report: Value) -> Value {
        report
    }

    pub fn active_supervision_payload(&self, report: Value) -> Value {
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_totals_accumulate_trace_costs() {
        let service = OrchestrationReportingService::new();
        let totals = service.summarize_trace_resources(&[
            OrchestrationTraceEntrySummary {
                actor_type: "orchestrator".to_string(),
                actor_id: "main".to_string(),
                estimated_input_tokens: Some(10),
                estimated_output_tokens: Some(4),
                duration_ms: Some(20),
            },
            OrchestrationTraceEntrySummary {
                actor_type: "worker".to_string(),
                actor_id: "worker-a".to_string(),
                estimated_input_tokens: Some(5),
                estimated_output_tokens: Some(6),
                duration_ms: Some(30),
            },
        ]);
        assert_eq!(totals.trace_count, 2);
        assert_eq!(totals.estimated_input_tokens, 15);
        assert_eq!(totals.estimated_output_tokens, 10);
        assert_eq!(totals.duration_ms, 50);
    }

    #[test]
    fn reflection_candidates_capture_fallback_and_worker_gaps() {
        let service = OrchestrationReportingService::new();
        let candidates = service.build_reflection_candidates(
            &ReflectionRoutingContext {
                selected_claw_id: "orchestrator".to_string(),
                selected_model_profile_id: "primary".to_string(),
                requested_model_profile_id: "primary".to_string(),
                provider: "openrouter".to_string(),
                fallback_path: vec!["primary".to_string(), "local".to_string()],
            },
            &[ReflectionWorkerResult {
                claw_id: "worker-a".to_string(),
                status: "needs_input".to_string(),
                summary: "needs approval".to_string(),
                questions: vec!["approve?".to_string()],
                confidence: Some(0.4),
                next_step_recommendation: Some("ask operator".to_string()),
                model_profile_id: "worker-primary".to_string(),
                provider: "openrouter".to_string(),
            }],
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind == "fallback_path")
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind == "worker_status")
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind == "low_confidence")
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind == "open_questions")
        );
    }

    #[test]
    fn attention_signals_surface_intervention_and_recent_failures() {
        let service = OrchestrationReportingService::new();
        let signals = service.attention_signals(
            &ActiveSupervisionSnapshot {
                status: "running".to_string(),
                pause_requested: true,
                kill_requested: false,
                lifecycle_state: "pause_requested".to_string(),
                intervention_required: true,
                escalation_requested: true,
                rollback_requested: false,
                last_error: Some("worker timeout risk".to_string()),
            },
            &["running".to_string(), "failed".to_string()],
        );
        assert!(
            signals
                .iter()
                .any(|signal| signal == "pause requested by operator")
        );
        assert!(
            signals
                .iter()
                .any(|signal| signal == "run has been escalated for operator review")
        );
        assert!(
            signals
                .iter()
                .any(|signal| signal == "recent events include failed or killed status")
        );
    }
}
