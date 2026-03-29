//! Multi-model and multi-claw orchestration helpers.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use openrustclaw_app::orchestration_reporting::{
    ActiveSupervisionSnapshot as AppActiveSupervisionSnapshot,
    OrchestrationActorResourceSummary as AppOrchestrationActorResourceSummary,
    OrchestrationReportingService, OrchestrationResourceTotals as AppOrchestrationResourceTotals,
    OrchestrationSupervisionSummary as AppOrchestrationSupervisionSummary,
    OrchestrationTraceEntrySummary as AppOrchestrationTraceEntrySummary,
    ReflectionCandidate as AppReflectionCandidate,
    ReflectionRoutingContext as AppReflectionRoutingContext,
    ReflectionWorkerResult as AppReflectionWorkerResult,
};
use openrustclaw_app::orchestration_routing::{
    OrchestrationAutonomyPolicy as AppOrchestrationAutonomyPolicy,
    OrchestrationClawRouteSpec as AppOrchestrationClawRouteSpec,
    OrchestrationLifecycleSummary as AppOrchestrationLifecycleSummary,
    OrchestrationRequest as AppOrchestrationRequest,
    OrchestrationRequestOverrides as AppOrchestrationRequestOverrides, OrchestrationRoutingService,
    OrchestrationRuntimeSpec as AppOrchestrationRuntimeSpec,
};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};
use openrustclaw_providers::{
    AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider,
    openrouter::RouteStrategy,
};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use super::{control, runtime};

pub const DEFAULT_RUNS_DIR: &str = ".claw/control/orchestration-runs";
pub const DEFAULT_ACTIVE_RUNS_DIR: &str = ".claw/control/orchestration-active";

fn default_active_status() -> String {
    "queued".to_string()
}

fn default_lifecycle_state() -> String {
    "queued".to_string()
}

fn default_transcript_role() -> String {
    "assistant".to_string()
}

fn default_run_mode() -> String {
    "auto".to_string()
}

fn default_status_completed() -> String {
    "completed".to_string()
}

fn default_checkpoint_status_completed() -> String {
    "completed".to_string()
}

fn default_reflection_kind() -> String {
    "lesson_candidate".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestrationRequestOverrides {
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub worker_model_profile_id: Option<String>,
    #[serde(default)]
    pub autonomy_level: Option<String>,
    #[serde(default)]
    pub max_delegations: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub approval_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestrationRequest {
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default = "default_run_mode")]
    pub mode: String,
    #[serde(default)]
    pub overrides: OrchestrationRequestOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModelDecision {
    pub requested_profile_id: String,
    pub selected_profile_id: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub fallback_path: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub execution_mode: String,
    pub route_source: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub selected_claw_id: String,
    pub selected_claw_role: String,
    pub selected_agent_profile_id: String,
    pub selected_model_profile_id: String,
    pub selected_model: ResolvedModelDecision,
    #[serde(default)]
    pub available_workers: Vec<String>,
    #[serde(default)]
    pub autonomy: control::AutonomyPolicy,
    pub allow_shared_context: bool,
    pub isolation_mode: String,
    #[serde(default)]
    pub applied_lessons: Vec<control::DecisionLessonSpec>,
    #[serde(default)]
    pub steering_notes: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub request_overrides: OrchestrationRequestOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationTask {
    pub id: String,
    pub claw_id: String,
    pub instruction: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResultEnvelope {
    pub claw_id: String,
    pub agent_profile_id: String,
    pub model_profile_id: String,
    pub provider: String,
    pub model: String,
    #[serde(default = "default_status_completed")]
    pub status: String,
    pub summary: String,
    pub full_output: String,
    #[serde(default)]
    pub questions: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub next_step_recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationCheckpoint {
    pub checkpoint_id: String,
    pub created_at: String,
    pub stage: String,
    #[serde(default = "default_checkpoint_status_completed")]
    pub status: String,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub note: String,
    #[serde(default)]
    pub input_excerpt: Option<String>,
    #[serde(default)]
    pub output_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionCandidate {
    #[serde(default = "default_reflection_kind")]
    pub kind: String,
    pub signal: String,
    pub recommendation: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionSummary {
    pub checkpoint_count: usize,
    pub worker_count: usize,
    pub needs_input_count: usize,
    pub failed_count: usize,
    #[serde(default)]
    pub low_confidence_workers: Vec<String>,
    pub reflection_candidate_count: usize,
    pub escalation_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTraceEntry {
    pub trace_id: String,
    pub created_at: String,
    pub stage: String,
    pub actor_type: String,
    pub actor_id: String,
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    #[serde(default = "default_checkpoint_status_completed")]
    pub status: String,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub note: String,
    #[serde(default)]
    pub input_excerpt: Option<String>,
    #[serde(default)]
    pub output_excerpt: Option<String>,
    #[serde(default)]
    pub estimated_input_tokens: Option<usize>,
    #[serde(default)]
    pub estimated_output_tokens: Option<usize>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRelationship {
    pub relationship_id: String,
    pub parent_run_id: String,
    pub child_id: String,
    pub child_kind: String,
    #[serde(default)]
    pub delegation_id: Option<String>,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTranscriptEntry {
    pub transcript_id: String,
    pub created_at: String,
    pub stage: String,
    pub actor_type: String,
    pub actor_id: String,
    #[serde(default = "default_transcript_role")]
    pub role: String,
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    #[serde(default)]
    pub delegation_id: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRunEvent {
    pub event_id: String,
    pub created_at: String,
    pub stage: String,
    pub actor_type: String,
    pub actor_id: String,
    #[serde(default = "default_active_status")]
    pub status: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionLifecycleSummary {
    #[serde(default = "default_lifecycle_state")]
    pub state: String,
    #[serde(default)]
    pub intervention_required: bool,
    #[serde(default)]
    pub escalation_requested: bool,
    #[serde(default)]
    pub rollback_requested: bool,
    #[serde(default)]
    pub rollback_reference: Option<String>,
    #[serde(default)]
    pub decision_count: usize,
    #[serde(default)]
    pub last_decision_at: Option<String>,
}

impl Default for SupervisionLifecycleSummary {
    fn default() -> Self {
        Self {
            state: default_lifecycle_state(),
            intervention_required: false,
            escalation_requested: false,
            rollback_requested: false,
            rollback_reference: None,
            decision_count: 0,
            last_decision_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRunDecisionRecord {
    pub decision_id: String,
    pub created_at: String,
    pub action: String,
    pub requested_by: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub rollback_reference: Option<String>,
    pub resulting_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveRunInterventionRequest {
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub rollback_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOrchestrationRun {
    pub run_id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default = "default_active_status")]
    pub status: String,
    pub request: OrchestrationRequest,
    #[serde(default)]
    pub routing: Option<RoutingDecision>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub current_actor_type: Option<String>,
    #[serde(default)]
    pub current_actor_id: Option<String>,
    #[serde(default)]
    pub current_note: Option<String>,
    #[serde(default)]
    pub lifecycle: SupervisionLifecycleSummary,
    #[serde(default)]
    pub pause_requested: bool,
    #[serde(default)]
    pub kill_requested: bool,
    #[serde(default)]
    pub checkpoint_count: usize,
    #[serde(default)]
    pub trace_count: usize,
    #[serde(default)]
    pub worker_count: usize,
    #[serde(default)]
    pub relationship_count: usize,
    #[serde(default)]
    pub resource_totals: Option<OrchestrationResourceTotals>,
    #[serde(default)]
    pub receipt_id: Option<String>,
    #[serde(default)]
    pub receipt_path: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PromoteReflectionInput {
    pub lesson_id: Option<String>,
    pub active: bool,
    pub signal: Option<String>,
    pub recommendation: Option<String>,
    pub rationale: Option<String>,
    pub confidence: Option<f32>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub claw_id: Option<String>,
    pub model_profile_id: Option<String>,
    pub provider: Option<String>,
    pub autonomy_level: Option<String>,
    pub execution_mode: Option<String>,
}

impl Default for PromoteReflectionInput {
    fn default() -> Self {
        Self {
            lesson_id: None,
            active: true,
            signal: None,
            recommendation: None,
            rationale: None,
            confidence: None,
            source: None,
            category: None,
            claw_id: None,
            model_profile_id: None,
            provider: None,
            autonomy_level: None,
            execution_mode: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRunRecord {
    pub run_id: String,
    pub created_at: String,
    pub mode: String,
    pub request: OrchestrationRequest,
    pub routing: RoutingDecision,
    #[serde(default)]
    pub delegations: Vec<DelegationTask>,
    #[serde(default)]
    pub worker_results: Vec<WorkerResultEnvelope>,
    #[serde(default)]
    pub checkpoints: Vec<OrchestrationCheckpoint>,
    #[serde(default)]
    pub trace: Vec<OrchestrationTraceEntry>,
    #[serde(default)]
    pub relationships: Vec<OrchestrationRelationship>,
    #[serde(default)]
    pub transcript: Vec<OrchestrationTranscriptEntry>,
    #[serde(default)]
    pub reflection_notes: Vec<String>,
    #[serde(default)]
    pub reflection_candidates: Vec<ReflectionCandidate>,
    #[serde(default)]
    pub supervision: Option<SupervisionSummary>,
    #[serde(default)]
    pub lifecycle: SupervisionLifecycleSummary,
    #[serde(default)]
    pub decision_history: Vec<ActiveRunDecisionRecord>,
    pub final_output: String,
    pub final_claw_id: String,
    pub final_model_profile_id: String,
    pub final_provider: String,
    pub final_model: String,
    pub receipt_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRunSummary {
    pub receipt_id: String,
    pub run_id: String,
    pub created_at: String,
    pub mode: String,
    pub route_source: String,
    pub approval_policy: String,
    pub final_claw_id: String,
    pub final_provider: String,
    pub final_model: String,
    pub worker_count: usize,
    pub failed_count: usize,
    pub needs_input_count: usize,
    pub escalation_recommended: bool,
    pub lifecycle_state: String,
    pub decision_count: usize,
    pub receipt_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRouteSummary {
    pub execution_mode: String,
    pub route_source: String,
    pub selected_claw_id: String,
    pub selected_claw_role: String,
    pub selected_model_profile_id: String,
    pub provider: String,
    pub model: String,
    pub approval_policy: String,
    #[serde(default)]
    pub autonomy_level: String,
    #[serde(default)]
    pub available_workers: Vec<String>,
    #[serde(default)]
    pub max_delegations: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptWorkerSummary {
    pub claw_id: String,
    pub status: String,
    pub summary: String,
    pub provider: String,
    pub model: String,
    pub model_profile_id: String,
    pub question_count: usize,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub next_step_recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptDelegationSummary {
    pub id: String,
    pub claw_id: String,
    pub reason: String,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSupervisionReport {
    pub run_id: String,
    pub mode: String,
    pub route: ReceiptRouteSummary,
    #[serde(default)]
    pub supervision: Option<SupervisionSummary>,
    pub lifecycle: SupervisionLifecycleSummary,
    pub resource_totals: OrchestrationResourceTotals,
    #[serde(default)]
    pub delegations: Vec<ReceiptDelegationSummary>,
    #[serde(default)]
    pub workers: Vec<ReceiptWorkerSummary>,
    #[serde(default)]
    pub checkpoints: Vec<OrchestrationCheckpoint>,
    #[serde(default)]
    pub relationships: Vec<OrchestrationRelationship>,
    #[serde(default)]
    pub reflection_notes: Vec<String>,
    #[serde(default)]
    pub reflection_candidates: Vec<ReflectionCandidate>,
    #[serde(default)]
    pub decision_history: Vec<ActiveRunDecisionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRunSupervisionReport {
    pub run: ActiveOrchestrationRun,
    #[serde(default)]
    pub recent_events: Vec<ActiveRunEvent>,
    #[serde(default)]
    pub attention_signals: Vec<String>,
    #[serde(default)]
    pub decision_history: Vec<ActiveRunDecisionRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlannerResponse {
    #[serde(default)]
    final_mode: Option<String>,
    #[serde(default)]
    direct_response: Option<String>,
    #[serde(default)]
    delegations: Vec<PlannerDelegation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlannerDelegation {
    claw_id: String,
    instruction: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlanReviewResponse {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerEnvelopeResponse {
    #[serde(default = "default_status_completed")]
    status: String,
    summary: String,
    #[serde(default)]
    full_output: Option<String>,
    #[serde(default)]
    questions: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    next_step_recommendation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FollowUpReviewResponse {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    instruction: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

struct ExecutableModel {
    provider: Arc<dyn LlmProvider>,
    decision: ResolvedModelDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResourceTotals {
    pub trace_count: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationActorResourceSummary {
    pub actor_type: String,
    pub actor_id: String,
    pub stage_count: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionTelemetry {
    response: openrustclaw_core::types::CompletionResponse,
    estimated_input_tokens: usize,
    estimated_output_tokens: usize,
    duration_ms: u64,
}

pub fn runs_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNS_DIR)
}

pub fn active_runs_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_ACTIVE_RUNS_DIR)
}

fn active_run_path(workspace_root: &Path, run_id: &str) -> PathBuf {
    active_runs_root_for(workspace_root).join(format!("{run_id}.json"))
}

fn active_run_events_path(workspace_root: &Path, run_id: &str) -> PathBuf {
    active_runs_root_for(workspace_root).join(format!("{run_id}.events.jsonl"))
}

fn active_run_decisions_path(workspace_root: &Path, run_id: &str) -> PathBuf {
    active_runs_root_for(workspace_root).join(format!("{run_id}.decisions.jsonl"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveRunMonitor {
    workspace_root: PathBuf,
    run_id: String,
}

impl ActiveRunMonitor {
    fn new(workspace_root: &Path, run_id: &str) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            run_id: run_id.to_string(),
        }
    }

    fn read_snapshot(&self) -> Result<ActiveOrchestrationRun> {
        read_active_run(&self.workspace_root, &self.run_id)
    }

    fn mutate_snapshot<F>(&self, mutate: F) -> Result<ActiveOrchestrationRun>
    where
        F: FnOnce(&mut ActiveOrchestrationRun),
    {
        let path = active_run_path(&self.workspace_root, &self.run_id);
        let mut snapshot = self.read_snapshot()?;
        mutate(&mut snapshot);
        snapshot.updated_at = Utc::now().to_rfc3339();
        fs::write(
            &path,
            serde_json::to_vec_pretty(&snapshot).context("Failed to encode active run snapshot")?,
        )
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
        Ok(snapshot)
    }

    fn append_event(
        &self,
        stage: &str,
        actor_type: &str,
        actor_id: &str,
        status: &str,
        note: impl Into<String>,
    ) -> Result<()> {
        let path = active_run_events_path(&self.workspace_root, &self.run_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }
        let event = ActiveRunEvent {
            event_id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            stage: stage.to_string(),
            actor_type: actor_type.to_string(),
            actor_id: actor_id.to_string(),
            status: status.to_string(),
            note: note.into(),
        };
        let mut encoded =
            serde_json::to_string(&event).context("Failed to encode active run event")?;
        encoded.push('\n');
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("Failed to open '{}'", path.display()))?;
        file.write_all(encoded.as_bytes())
            .with_context(|| format!("Failed to append '{}'", path.display()))?;
        Ok(())
    }

    fn append_decision(
        &self,
        action: &str,
        requested_by: &str,
        reason: Option<&str>,
        rollback_reference: Option<&str>,
        resulting_state: &str,
    ) -> Result<ActiveRunDecisionRecord> {
        let path = active_run_decisions_path(&self.workspace_root, &self.run_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }
        let record = ActiveRunDecisionRecord {
            decision_id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            action: action.to_string(),
            requested_by: requested_by.to_string(),
            reason: reason.map(ToString::to_string),
            rollback_reference: rollback_reference.map(ToString::to_string),
            resulting_state: resulting_state.to_string(),
        };
        let mut encoded =
            serde_json::to_string(&record).context("Failed to encode active run decision")?;
        encoded.push('\n');
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("Failed to open '{}'", path.display()))?;
        file.write_all(encoded.as_bytes())
            .with_context(|| format!("Failed to append '{}'", path.display()))?;
        Ok(record)
    }

    fn update_stage(
        &self,
        status: &str,
        stage: &str,
        actor_type: &str,
        actor_id: &str,
        note: impl Into<String>,
    ) -> Result<()> {
        let note = note.into();
        self.mutate_snapshot(|snapshot| {
            snapshot.status = status.to_string();
            snapshot.current_stage = Some(stage.to_string());
            snapshot.current_actor_type = Some(actor_type.to_string());
            snapshot.current_actor_id = Some(actor_id.to_string());
            snapshot.current_note = Some(note.clone());
            if snapshot.started_at.is_none() && status == "running" {
                snapshot.started_at = Some(Utc::now().to_rfc3339());
            }
            if !snapshot.lifecycle.intervention_required {
                snapshot.lifecycle.state = match status {
                    "running" => "running".to_string(),
                    "paused" => "paused".to_string(),
                    "completed" => "completed".to_string(),
                    "failed" => "failed".to_string(),
                    "killed" => "killed".to_string(),
                    other => other.to_string(),
                };
            }
        })?;
        self.append_event(stage, actor_type, actor_id, status, note)?;
        Ok(())
    }

    fn update_from_record(&self, record: &OrchestrationRunRecord) -> Result<()> {
        let receipt_id = Path::new(&record.receipt_path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string());
        self.mutate_snapshot(|snapshot| {
            snapshot.checkpoint_count = record.checkpoints.len();
            snapshot.trace_count = record.trace.len();
            snapshot.worker_count = record.worker_results.len();
            snapshot.relationship_count = record.relationships.len();
            snapshot.resource_totals = Some(summarize_trace_resources(&record.trace));
            snapshot.receipt_id = receipt_id.clone();
            snapshot.receipt_path = Some(record.receipt_path.clone());
        })?;
        Ok(())
    }

    async fn checkpoint(
        &self,
        status: &str,
        stage: &str,
        actor_type: &str,
        actor_id: &str,
        note: &str,
    ) -> Result<()> {
        self.update_stage(status, stage, actor_type, actor_id, note)?;
        self.wait_if_paused(stage, actor_type, actor_id).await
    }

    async fn wait_if_paused(&self, stage: &str, actor_type: &str, actor_id: &str) -> Result<()> {
        loop {
            let snapshot = self.read_snapshot()?;
            if snapshot.kill_requested {
                self.update_stage(
                    "killed",
                    stage,
                    actor_type,
                    actor_id,
                    "kill requested by operator",
                )?;
                return Err(anyhow::anyhow!("orchestration run killed by operator"));
            }
            if snapshot.pause_requested {
                if snapshot.status != "paused" {
                    self.update_stage("paused", stage, actor_type, actor_id, "paused by operator")?;
                }
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            if snapshot.status == "paused" {
                self.update_stage(
                    "running",
                    stage,
                    actor_type,
                    actor_id,
                    "resumed by operator",
                )?;
            }
            return Ok(());
        }
    }

    fn finalize_success(&self, record: &OrchestrationRunRecord) -> Result<ActiveOrchestrationRun> {
        self.update_from_record(record)?;
        let snapshot = self.mutate_snapshot(|snapshot| {
            snapshot.status = "completed".to_string();
            snapshot.finished_at = Some(Utc::now().to_rfc3339());
            snapshot.current_stage = Some("completed".to_string());
            snapshot.current_actor_type = Some("orchestrator".to_string());
            snapshot.current_actor_id = Some(record.final_claw_id.clone());
            snapshot.current_note = Some("run completed".to_string());
            snapshot.last_error = None;
            snapshot.lifecycle.state = "completed".to_string();
            snapshot.lifecycle.intervention_required = false;
        })?;
        self.append_event(
            "completed",
            "orchestrator",
            record.final_claw_id.as_str(),
            "completed",
            "run completed",
        )?;
        Ok(snapshot)
    }

    fn finalize_failure(&self, error: &str) -> Result<ActiveOrchestrationRun> {
        let snapshot = self.mutate_snapshot(|snapshot| {
            snapshot.status = if snapshot.lifecycle.rollback_requested {
                "rolled_back".to_string()
            } else if snapshot.kill_requested {
                "killed".to_string()
            } else {
                "failed".to_string()
            };
            snapshot.finished_at = Some(Utc::now().to_rfc3339());
            snapshot.current_stage = Some(snapshot.status.clone());
            snapshot.current_actor_type = Some("orchestrator".to_string());
            snapshot.current_actor_id = snapshot
                .current_actor_id
                .clone()
                .or_else(|| Some("orchestrator".to_string()));
            snapshot.current_note = Some(error.to_string());
            snapshot.last_error = Some(error.to_string());
            snapshot.lifecycle.state = snapshot.status.clone();
        })?;
        self.append_event(
            snapshot.current_stage.as_deref().unwrap_or("failed"),
            "orchestrator",
            snapshot
                .current_actor_id
                .as_deref()
                .unwrap_or("orchestrator"),
            snapshot.status.as_str(),
            error,
        )?;
        Ok(snapshot)
    }
}

pub fn resolve(request: OrchestrationRequest, workspace_root: &Path) -> Result<RoutingDecision> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let control_root = control::control_root_for(workspace_root);
    let registry = control::load_registry(control_root)?;
    control::validate_registry(&registry)?;
    resolve_routing(&registry, &config, &request)
}

pub async fn run(
    request: OrchestrationRequest,
    workspace_root: &Path,
) -> Result<OrchestrationRunRecord> {
    run_internal(request, workspace_root, None).await
}

pub async fn submit(
    request: OrchestrationRequest,
    workspace_root: &Path,
) -> Result<ActiveOrchestrationRun> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let control_root = control::control_root_for(workspace_root);
    let registry = control::load_registry(control_root)?;
    control::validate_registry(&registry)?;
    let routing = resolve_routing(&registry, &config, &request)?;
    let run_id = Uuid::new_v4().to_string();
    let snapshot = ActiveOrchestrationRun {
        run_id: run_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        started_at: None,
        finished_at: None,
        status: "queued".to_string(),
        request,
        routing: Some(routing),
        current_stage: Some("queued".to_string()),
        current_actor_type: Some("orchestrator".to_string()),
        current_actor_id: None,
        current_note: Some("queued for background execution".to_string()),
        lifecycle: SupervisionLifecycleSummary::default(),
        pause_requested: false,
        kill_requested: false,
        checkpoint_count: 0,
        trace_count: 0,
        worker_count: 0,
        relationship_count: 0,
        resource_totals: None,
        receipt_id: None,
        receipt_path: None,
        last_error: None,
    };
    write_active_run(workspace_root, &snapshot)?;

    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    std::process::Command::new(current_exe)
        .arg("orchestrate")
        .arg("worker-run")
        .arg("--run-id")
        .arg(&run_id)
        .current_dir(workspace_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn background orchestration worker")?;

    Ok(snapshot)
}

async fn run_internal(
    request: OrchestrationRequest,
    workspace_root: &Path,
    monitor: Option<&ActiveRunMonitor>,
) -> Result<OrchestrationRunRecord> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let control_root = control::control_root_for(workspace_root);
    let registry = control::load_registry(control_root)?;
    control::validate_registry(&registry)?;
    let routing = resolve_routing(&registry, &config, &request)?;
    let runtime_mode = request.mode.to_lowercase();

    if let Some(monitor) = monitor {
        monitor.mutate_snapshot(|snapshot| {
            snapshot.routing = Some(routing.clone());
        })?;
        monitor
            .checkpoint(
                "running",
                "routing",
                "orchestrator",
                routing.selected_claw_id.as_str(),
                "resolved orchestration route",
            )
            .await?;
    }

    let mut record = if runtime_mode == "orchestrated"
        || (runtime_mode == "auto" && routing.execution_mode == "orchestrated")
    {
        run_orchestrated(
            request.clone(),
            &registry,
            &config,
            &routing,
            workspace_root,
            monitor,
        )
        .await?
    } else {
        run_direct(
            request.clone(),
            &registry,
            &config,
            &routing,
            workspace_root,
            monitor,
        )
        .await?
    };
    if let Some(monitor) = monitor {
        let snapshot = monitor.read_snapshot()?;
        record.lifecycle = lifecycle_summary_for(
            workspace_root,
            &snapshot.run_id,
            Some(snapshot.lifecycle.clone()),
        )?;
        record.decision_history =
            read_active_run_decisions(workspace_root, &snapshot.run_id, usize::MAX)?;
    }
    record.receipt_path = save_run_record(workspace_root, &record)?
        .display()
        .to_string();
    Ok(record)
}

pub async fn worker_run(run_id: &str, workspace_root: &Path) -> Result<ActiveOrchestrationRun> {
    let monitor = ActiveRunMonitor::new(workspace_root, run_id);
    let snapshot = monitor.read_snapshot()?;
    monitor.update_stage(
        "running",
        "routing",
        "orchestrator",
        snapshot
            .routing
            .as_ref()
            .map(|routing| routing.selected_claw_id.as_str())
            .unwrap_or("pending"),
        "background orchestration worker started",
    )?;
    match run_internal(snapshot.request.clone(), workspace_root, Some(&monitor)).await {
        Ok(record) => monitor.finalize_success(&record),
        Err(error) => {
            monitor.finalize_failure(&error.to_string())?;
            Err(error)
        }
    }
}

pub fn list_runs(workspace_root: &Path, limit: usize) -> Result<Vec<OrchestrationRunSummary>> {
    let runs_root = runs_root_for(workspace_root);
    if !runs_root.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in fs::read_dir(&runs_root)
        .with_context(|| format!("Failed to read '{}'", runs_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let record: OrchestrationRunRecord = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("Failed to decode '{}'", path.display()))?;
        runs.push(OrchestrationRunSummary {
            receipt_id: path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default(),
            run_id: record.run_id,
            created_at: record.created_at,
            mode: record.mode,
            route_source: record.routing.route_source,
            approval_policy: record.routing.autonomy.approval_policy.clone(),
            final_claw_id: record.final_claw_id,
            final_provider: record.final_provider,
            final_model: record.final_model,
            worker_count: record.worker_results.len(),
            failed_count: record
                .worker_results
                .iter()
                .filter(|worker| worker.status == "failed")
                .count(),
            needs_input_count: record
                .worker_results
                .iter()
                .filter(|worker| worker.status == "needs_input")
                .count(),
            escalation_recommended: record
                .supervision
                .as_ref()
                .map(|summary| summary.escalation_recommended)
                .unwrap_or(false),
            lifecycle_state: record.lifecycle.state.clone(),
            decision_count: record.decision_history.len(),
            receipt_path: record.receipt_path,
        });
    }

    runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    runs.truncate(limit);
    Ok(runs)
}

pub fn write_active_run(workspace_root: &Path, run: &ActiveOrchestrationRun) -> Result<PathBuf> {
    let active_root = active_runs_root_for(workspace_root);
    fs::create_dir_all(&active_root)
        .with_context(|| format!("Failed to create '{}'", active_root.display()))?;
    let path = active_run_path(workspace_root, &run.run_id);
    fs::write(
        &path,
        serde_json::to_vec_pretty(run).context("Failed to encode active orchestration run")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(path)
}

pub fn list_active_runs(
    workspace_root: &Path,
    active_only: bool,
    limit: usize,
) -> Result<Vec<ActiveOrchestrationRun>> {
    let active_root = active_runs_root_for(workspace_root);
    if !active_root.exists() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(&active_root)
        .with_context(|| format!("Failed to read '{}'", active_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let run: ActiveOrchestrationRun = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("Failed to decode '{}'", path.display()))?;
        if active_only
            && matches!(
                run.status.as_str(),
                "completed" | "failed" | "killed" | "rolled_back"
            )
        {
            continue;
        }
        runs.push(run);
    }
    runs.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    runs.truncate(limit);
    Ok(runs)
}

pub fn read_active_run(workspace_root: &Path, run_id: &str) -> Result<ActiveOrchestrationRun> {
    if run_id.contains('/') || run_id.contains('\\') {
        anyhow::bail!("invalid active run id");
    }
    let path = active_run_path(workspace_root, run_id);
    let bytes = fs::read(&path).with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to decode '{}'", path.display()))
}

pub fn read_active_run_events(
    workspace_root: &Path,
    run_id: &str,
    limit: usize,
) -> Result<Vec<ActiveRunEvent>> {
    if run_id.contains('/') || run_id.contains('\\') {
        anyhow::bail!("invalid active run id");
    }
    let path = active_run_events_path(workspace_root, run_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut events = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        events.push(
            serde_json::from_str::<ActiveRunEvent>(line)
                .with_context(|| format!("Failed to decode event line in '{}'", path.display()))?,
        );
    }
    if events.len() > limit {
        events = events.split_off(events.len() - limit);
    }
    Ok(events)
}

pub fn read_active_run_decisions(
    workspace_root: &Path,
    run_id: &str,
    limit: usize,
) -> Result<Vec<ActiveRunDecisionRecord>> {
    if run_id.contains('/') || run_id.contains('\\') {
        anyhow::bail!("invalid active run id");
    }
    let path = active_run_decisions_path(workspace_root, run_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut decisions = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        decisions.push(
            serde_json::from_str::<ActiveRunDecisionRecord>(line).with_context(|| {
                format!("Failed to decode decision line in '{}'", path.display())
            })?,
        );
    }
    if decisions.len() > limit {
        decisions = decisions.split_off(decisions.len() - limit);
    }
    Ok(decisions)
}

fn lifecycle_summary_for(
    workspace_root: &Path,
    run_id: &str,
    base: Option<SupervisionLifecycleSummary>,
) -> Result<SupervisionLifecycleSummary> {
    let mut lifecycle = base.unwrap_or_default();
    let decisions = read_active_run_decisions(workspace_root, run_id, usize::MAX)?;
    lifecycle.decision_count = decisions.len();
    lifecycle.last_decision_at = decisions.last().map(|record| record.created_at.clone());
    Ok(lifecycle)
}

fn apply_active_lifecycle_action(
    workspace_root: &Path,
    run_id: &str,
    action: &str,
    requested_by: &str,
    reason: Option<&str>,
    rollback_reference: Option<&str>,
) -> Result<ActiveOrchestrationRun> {
    let monitor = ActiveRunMonitor::new(workspace_root, run_id);
    let transition = OrchestrationRoutingService::new().intervention_transition(
        action,
        reason,
        rollback_reference,
    )?;
    monitor.mutate_snapshot(|snapshot| {
        snapshot.pause_requested = transition.pause_requested;
        snapshot.kill_requested = transition.kill_requested;
        snapshot.lifecycle = supervision_lifecycle_from_app(&transition.lifecycle);
        snapshot.current_stage = Some(transition.stage.clone());
        snapshot.current_note = Some(transition.note.clone());
    })?;
    monitor.append_event(
        transition.stage.as_str(),
        "operator",
        requested_by,
        transition.status.as_str(),
        transition.note.clone(),
    )?;
    let decision = monitor.append_decision(
        action,
        requested_by,
        reason,
        rollback_reference,
        &transition.resulting_state,
    )?;
    let updated = monitor.mutate_snapshot(|snapshot| {
        snapshot.lifecycle.decision_count += 1;
        snapshot.lifecycle.last_decision_at = Some(decision.created_at.clone());
    })?;
    Ok(updated)
}

fn app_orchestration_request(request: &OrchestrationRequest) -> AppOrchestrationRequest {
    AppOrchestrationRequest {
        prompt: request.prompt.clone(),
        task_id: request.task_id.clone(),
        category: request.category.clone(),
        claw_id: request.claw_id.clone(),
        mode: request.mode.clone(),
        overrides: AppOrchestrationRequestOverrides {
            model_profile_id: request.overrides.model_profile_id.clone(),
            worker_model_profile_id: request.overrides.worker_model_profile_id.clone(),
            autonomy_level: request.overrides.autonomy_level.clone(),
            max_delegations: request.overrides.max_delegations,
            max_iterations: request.overrides.max_iterations,
            max_runtime_secs: request.overrides.max_runtime_secs,
            approval_policy: request.overrides.approval_policy.clone(),
        },
    }
}

fn app_runtime_spec_from_control(
    runtime: &control::RuntimeModeSpec,
) -> AppOrchestrationRuntimeSpec {
    AppOrchestrationRuntimeSpec {
        mode: runtime.mode.clone(),
        default_claw_id: runtime.default_claw_id.clone(),
        orchestrator_claw_id: runtime.orchestrator_claw_id.clone(),
        task_assignments: runtime
            .task_assignments
            .iter()
            .map(|(task_id, claw_id)| (task_id.clone(), claw_id.clone()))
            .collect(),
        category_assignments: runtime
            .category_assignments
            .iter()
            .map(|(category, claw_id)| (category.clone(), claw_id.clone()))
            .collect(),
        allow_shared_context: runtime.allow_shared_context,
        isolation_mode: runtime.isolation_mode.clone(),
        autonomy: AppOrchestrationAutonomyPolicy {
            autonomy_level: runtime.autonomy.autonomy_level.clone(),
            yolo_mode: runtime.autonomy.yolo_mode,
            steering_enabled: runtime.autonomy.steering_enabled,
            decision_learning_enabled: runtime.autonomy.decision_learning_enabled,
            critic_enabled: runtime.autonomy.critic_enabled,
            max_delegations: runtime.autonomy.max_delegations,
            max_iterations: runtime.autonomy.max_iterations,
            max_runtime_secs: runtime.autonomy.max_runtime_secs,
            max_lesson_hints: runtime.autonomy.max_lesson_hints,
            approval_policy: runtime.autonomy.approval_policy.clone(),
        },
    }
}

fn app_claw_route_specs(registry: &control::ControlRegistry) -> Vec<AppOrchestrationClawRouteSpec> {
    registry
        .claws
        .values()
        .map(|claw| AppOrchestrationClawRouteSpec {
            id: claw.id.clone(),
            role: claw.role.clone(),
            enabled: claw.enabled,
        })
        .collect()
}

fn control_autonomy_policy_from_app(
    autonomy: &AppOrchestrationAutonomyPolicy,
) -> control::AutonomyPolicy {
    control::AutonomyPolicy {
        autonomy_level: autonomy.autonomy_level.clone(),
        yolo_mode: autonomy.yolo_mode,
        steering_enabled: autonomy.steering_enabled,
        decision_learning_enabled: autonomy.decision_learning_enabled,
        critic_enabled: autonomy.critic_enabled,
        max_delegations: autonomy.max_delegations,
        max_iterations: autonomy.max_iterations,
        max_runtime_secs: autonomy.max_runtime_secs,
        max_lesson_hints: autonomy.max_lesson_hints,
        approval_policy: autonomy.approval_policy.clone(),
    }
}

fn supervision_lifecycle_from_app(
    lifecycle: &AppOrchestrationLifecycleSummary,
) -> SupervisionLifecycleSummary {
    SupervisionLifecycleSummary {
        state: lifecycle.state.clone(),
        intervention_required: lifecycle.intervention_required,
        escalation_requested: lifecycle.escalation_requested,
        rollback_requested: lifecycle.rollback_requested,
        rollback_reference: lifecycle.rollback_reference.clone(),
        ..SupervisionLifecycleSummary::default()
    }
}

fn app_trace_entries(trace: &[OrchestrationTraceEntry]) -> Vec<AppOrchestrationTraceEntrySummary> {
    trace
        .iter()
        .map(|entry| AppOrchestrationTraceEntrySummary {
            actor_type: entry.actor_type.clone(),
            actor_id: entry.actor_id.clone(),
            estimated_input_tokens: entry.estimated_input_tokens,
            estimated_output_tokens: entry.estimated_output_tokens,
            duration_ms: entry.duration_ms,
        })
        .collect()
}

fn cli_resource_totals_from_app(
    totals: &AppOrchestrationResourceTotals,
) -> OrchestrationResourceTotals {
    OrchestrationResourceTotals {
        trace_count: totals.trace_count,
        estimated_input_tokens: totals.estimated_input_tokens,
        estimated_output_tokens: totals.estimated_output_tokens,
        duration_ms: totals.duration_ms,
    }
}

fn app_resource_totals_from_cli(
    totals: &OrchestrationResourceTotals,
) -> AppOrchestrationResourceTotals {
    AppOrchestrationResourceTotals {
        trace_count: totals.trace_count,
        estimated_input_tokens: totals.estimated_input_tokens,
        estimated_output_tokens: totals.estimated_output_tokens,
        duration_ms: totals.duration_ms,
    }
}

fn cli_actor_summary_from_app(
    summary: &AppOrchestrationActorResourceSummary,
) -> OrchestrationActorResourceSummary {
    OrchestrationActorResourceSummary {
        actor_type: summary.actor_type.clone(),
        actor_id: summary.actor_id.clone(),
        stage_count: summary.stage_count,
        estimated_input_tokens: summary.estimated_input_tokens,
        estimated_output_tokens: summary.estimated_output_tokens,
        duration_ms: summary.duration_ms,
    }
}

fn app_actor_summary_from_cli(
    summary: &OrchestrationActorResourceSummary,
) -> AppOrchestrationActorResourceSummary {
    AppOrchestrationActorResourceSummary {
        actor_type: summary.actor_type.clone(),
        actor_id: summary.actor_id.clone(),
        stage_count: summary.stage_count,
        estimated_input_tokens: summary.estimated_input_tokens,
        estimated_output_tokens: summary.estimated_output_tokens,
        duration_ms: summary.duration_ms,
    }
}

fn app_reflection_worker_from_cli(worker: &WorkerResultEnvelope) -> AppReflectionWorkerResult {
    AppReflectionWorkerResult {
        claw_id: worker.claw_id.clone(),
        status: worker.status.clone(),
        summary: worker.summary.clone(),
        questions: worker.questions.clone(),
        confidence: worker.confidence,
        next_step_recommendation: worker.next_step_recommendation.clone(),
        model_profile_id: worker.model_profile_id.clone(),
        provider: worker.provider.clone(),
    }
}

fn cli_reflection_candidate_from_app(candidate: &AppReflectionCandidate) -> ReflectionCandidate {
    ReflectionCandidate {
        kind: candidate.kind.clone(),
        signal: candidate.signal.clone(),
        recommendation: candidate.recommendation.clone(),
        rationale: candidate.rationale.clone(),
        confidence: candidate.confidence,
        claw_id: candidate.claw_id.clone(),
        model_profile_id: candidate.model_profile_id.clone(),
        provider: candidate.provider.clone(),
    }
}

fn app_reflection_candidate_from_cli(candidate: &ReflectionCandidate) -> AppReflectionCandidate {
    AppReflectionCandidate {
        kind: candidate.kind.clone(),
        signal: candidate.signal.clone(),
        recommendation: candidate.recommendation.clone(),
        rationale: candidate.rationale.clone(),
        confidence: candidate.confidence,
        claw_id: candidate.claw_id.clone(),
        model_profile_id: candidate.model_profile_id.clone(),
        provider: candidate.provider.clone(),
    }
}

fn cli_supervision_summary_from_app(
    summary: &AppOrchestrationSupervisionSummary,
) -> SupervisionSummary {
    SupervisionSummary {
        checkpoint_count: summary.checkpoint_count,
        worker_count: summary.worker_count,
        needs_input_count: summary.needs_input_count,
        failed_count: summary.failed_count,
        low_confidence_workers: summary.low_confidence_workers.clone(),
        reflection_candidate_count: summary.reflection_candidate_count,
        escalation_recommended: summary.escalation_recommended,
    }
}

pub fn pause_active_run(workspace_root: &Path, run_id: &str) -> Result<ActiveOrchestrationRun> {
    apply_active_lifecycle_action(workspace_root, run_id, "pause", "operator", None, None)
}

pub fn resume_active_run(workspace_root: &Path, run_id: &str) -> Result<ActiveOrchestrationRun> {
    apply_active_lifecycle_action(workspace_root, run_id, "resume", "operator", None, None)
}

pub fn kill_active_run(workspace_root: &Path, run_id: &str) -> Result<ActiveOrchestrationRun> {
    apply_active_lifecycle_action(workspace_root, run_id, "kill", "operator", None, None)
}

pub fn escalate_active_run(
    workspace_root: &Path,
    run_id: &str,
    request: ActiveRunInterventionRequest,
) -> Result<ActiveOrchestrationRun> {
    apply_active_lifecycle_action(
        workspace_root,
        run_id,
        "escalate",
        request.requested_by.as_deref().unwrap_or("operator"),
        request.reason.as_deref(),
        None,
    )
}

pub fn rollback_active_run(
    workspace_root: &Path,
    run_id: &str,
    request: ActiveRunInterventionRequest,
) -> Result<ActiveOrchestrationRun> {
    apply_active_lifecycle_action(
        workspace_root,
        run_id,
        "rollback",
        request.requested_by.as_deref().unwrap_or("operator"),
        request.reason.as_deref(),
        request.rollback_reference.as_deref(),
    )
}

pub fn read_run(workspace_root: &Path, receipt_id: &str) -> Result<OrchestrationRunRecord> {
    if receipt_id.contains('/') || receipt_id.contains('\\') {
        anyhow::bail!("invalid receipt id");
    }
    let path = runs_root_for(workspace_root).join(receipt_id);
    let bytes = fs::read(&path).with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to decode '{}'", path.display()))
}

pub fn read_run_checkpoints(
    workspace_root: &Path,
    receipt_id: &str,
) -> Result<Vec<OrchestrationCheckpoint>> {
    Ok(read_run(workspace_root, receipt_id)?.checkpoints)
}

pub fn read_run_trace(workspace_root: &Path, receipt_id: &str) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    Ok(OrchestrationReportingService::new().trace_payload(
        &run.run_id,
        serde_json::to_value(run.trace)?,
        serde_json::to_value(run.relationships)?,
    ))
}

pub fn read_run_transcript(workspace_root: &Path, receipt_id: &str) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    Ok(OrchestrationReportingService::new().transcript_payload(
        &run.run_id,
        serde_json::to_value(run.transcript)?,
        serde_json::to_value(run.relationships)?,
    ))
}

pub fn read_run_resources(workspace_root: &Path, receipt_id: &str) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    let totals = summarize_trace_resources(&run.trace);
    let actors = summarize_actor_resources(&run.trace);
    Ok(OrchestrationReportingService::new().resources_payload(
        &run.run_id,
        &app_resource_totals_from_cli(&totals),
        &actors
            .iter()
            .map(app_actor_summary_from_cli)
            .collect::<Vec<_>>(),
        serde_json::to_value(run.trace)?,
    ))
}

pub fn read_run_supervision(workspace_root: &Path, receipt_id: &str) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    let report = ReceiptSupervisionReport {
        run_id: run.run_id,
        mode: run.mode,
        route: ReceiptRouteSummary {
            execution_mode: run.routing.execution_mode,
            route_source: run.routing.route_source,
            selected_claw_id: run.routing.selected_claw_id,
            selected_claw_role: run.routing.selected_claw_role,
            selected_model_profile_id: run.routing.selected_model_profile_id,
            provider: run.routing.selected_model.provider,
            model: run.routing.selected_model.model,
            approval_policy: run.routing.autonomy.approval_policy.clone(),
            autonomy_level: run.routing.autonomy.autonomy_level.clone(),
            available_workers: run.routing.available_workers,
            max_delegations: run.routing.request_overrides.max_delegations,
            max_iterations: run.routing.request_overrides.max_iterations,
            max_runtime_secs: run.routing.request_overrides.max_runtime_secs,
        },
        supervision: run.supervision,
        lifecycle: run.lifecycle,
        resource_totals: summarize_trace_resources(&run.trace),
        delegations: run
            .delegations
            .into_iter()
            .map(|delegation| ReceiptDelegationSummary {
                id: delegation.id,
                claw_id: delegation.claw_id,
                reason: delegation.reason,
                instruction: delegation.instruction,
            })
            .collect(),
        workers: run
            .worker_results
            .into_iter()
            .map(|worker| ReceiptWorkerSummary {
                claw_id: worker.claw_id,
                status: worker.status,
                summary: worker.summary,
                provider: worker.provider,
                model: worker.model,
                model_profile_id: worker.model_profile_id,
                question_count: worker.questions.len(),
                confidence: worker.confidence,
                next_step_recommendation: worker.next_step_recommendation,
            })
            .collect(),
        checkpoints: run.checkpoints,
        relationships: run.relationships,
        reflection_notes: run.reflection_notes,
        reflection_candidates: run.reflection_candidates,
        decision_history: run.decision_history,
    };
    Ok(OrchestrationReportingService::new()
        .receipt_supervision_payload(serde_json::to_value(report)?))
}

pub fn read_active_run_supervision(
    workspace_root: &Path,
    run_id: &str,
    event_limit: usize,
) -> Result<serde_json::Value> {
    let run = read_active_run(workspace_root, run_id)?;
    let recent_events = read_active_run_events(workspace_root, run_id, event_limit.max(1))?;
    let decision_history = read_active_run_decisions(workspace_root, run_id, event_limit.max(1))?;
    let attention_signals = OrchestrationReportingService::new().attention_signals(
        &AppActiveSupervisionSnapshot {
            status: run.status.clone(),
            pause_requested: run.pause_requested,
            kill_requested: run.kill_requested,
            lifecycle_state: run.lifecycle.state.clone(),
            intervention_required: run.lifecycle.intervention_required,
            escalation_requested: run.lifecycle.escalation_requested,
            rollback_requested: run.lifecycle.rollback_requested,
            last_error: run.last_error.clone(),
        },
        &recent_events
            .iter()
            .map(|event| event.status.clone())
            .collect::<Vec<_>>(),
    );
    Ok(
        OrchestrationReportingService::new().active_supervision_payload(serde_json::to_value(
            ActiveRunSupervisionReport {
                run,
                recent_events,
                attention_signals,
                decision_history,
            },
        )?),
    )
}

pub fn promote_reflection_candidate(
    workspace_root: &Path,
    receipt_id: &str,
    candidate_index: usize,
    input: PromoteReflectionInput,
) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    let candidate = run
        .reflection_candidates
        .get(candidate_index)
        .cloned()
        .with_context(|| {
            format!(
                "reflection candidate {} not found for receipt '{}'",
                candidate_index, receipt_id
            )
        })?;

    let lesson_id = input
        .lesson_id
        .unwrap_or_else(|| make_lesson_id(&run.run_id, candidate_index, &candidate.kind));
    let signal = input.signal.unwrap_or(candidate.signal.clone());
    let recommendation = input
        .recommendation
        .unwrap_or(candidate.recommendation.clone());
    let rationale = input.rationale.or(candidate.rationale.clone());
    let confidence = input.confidence.or(candidate.confidence).unwrap_or(0.7);
    let source = input
        .source
        .unwrap_or_else(|| "reflection_candidate".to_string());
    let category = input.category.or(run.request.category.clone());
    let claw_id = input
        .claw_id
        .or(candidate.claw_id.clone())
        .or(Some(run.routing.selected_claw_id.clone()));
    let model_profile_id = input
        .model_profile_id
        .or(candidate.model_profile_id.clone())
        .or(Some(run.routing.selected_model_profile_id.clone()));
    let provider = input
        .provider
        .or(candidate.provider.clone())
        .or(Some(run.routing.selected_model.provider.clone()));
    let autonomy_level = input
        .autonomy_level
        .or(Some(run.routing.autonomy.autonomy_level.clone()));
    let execution_mode = input
        .execution_mode
        .or(Some(run.routing.execution_mode.clone()));
    let control_root = control::control_root_for(workspace_root);
    let control_root_str = control_root.to_string_lossy().to_string();

    control::create_lesson(
        Some(&control_root_str),
        control::NewLessonInput {
            id: &lesson_id,
            active: input.active,
            signal: &signal,
            recommendation: &recommendation,
            rationale: rationale.as_deref(),
            confidence,
            source: Some(&source),
            task_id: run.request.task_id.as_deref(),
            category: category.as_deref(),
            claw_id: claw_id.as_deref(),
            model_profile_id: model_profile_id.as_deref(),
            provider: provider.as_deref(),
            autonomy_level: autonomy_level.as_deref(),
            execution_mode: execution_mode.as_deref(),
        },
    )?;

    let registry = control::load_registry(control_root)?;
    let lesson =
        registry.lessons.get(&lesson_id).cloned().with_context(|| {
            format!("Promoted lesson '{}' was not found after write", lesson_id)
        })?;
    Ok(serde_json::json!({
        "status": "ok",
        "receipt_id": receipt_id,
        "candidate_index": candidate_index,
        "lesson": lesson,
    }))
}

fn resolve_routing(
    registry: &control::ControlRegistry,
    config: &AppConfig,
    request: &OrchestrationRequest,
) -> Result<RoutingDecision> {
    let runtime = registry
        .runtime
        .clone()
        .unwrap_or(control::RuntimeModeSpec {
            mode: "solo_claw".to_string(),
            default_claw_id: None,
            orchestrator_claw_id: None,
            task_assignments: BTreeMap::new(),
            category_assignments: BTreeMap::new(),
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            autonomy: control::AutonomyPolicy::default(),
            metadata: serde_json::json!({}),
        });
    let route = OrchestrationRoutingService::new().resolve_route(
        &app_runtime_spec_from_control(&runtime),
        &app_claw_route_specs(registry),
        &app_orchestration_request(request),
    )?;
    let mut resolved_runtime = runtime.clone();
    resolved_runtime.autonomy = control_autonomy_policy_from_app(&route.autonomy);
    let mut warnings = Vec::new();
    let claw = registry
        .claws
        .get(&route.selected_claw_id)
        .with_context(|| format!("Unknown claw '{}'", route.selected_claw_id))?;
    let selected_model_profile_id = request
        .overrides
        .model_profile_id
        .clone()
        .unwrap_or_else(|| claw.model_profile_id.clone());
    let model = resolve_model_with_fallback(&selected_model_profile_id, registry, config)?;
    warnings.extend(model.decision.warnings.clone());
    if request.overrides.model_profile_id.is_some() {
        warnings.push(format!(
            "per-run model override requested '{}'",
            selected_model_profile_id
        ));
    }
    if request.overrides.worker_model_profile_id.is_some() {
        warnings.push(format!(
            "per-run worker model override requested '{}'",
            request
                .overrides
                .worker_model_profile_id
                .as_deref()
                .unwrap_or_default()
        ));
    }
    let applied_lessons = matching_lessons(
        registry,
        request,
        claw,
        &resolved_runtime,
        &model.decision.selected_profile_id,
        &model.decision.provider,
    );
    let steering_notes = build_steering_notes(&resolved_runtime.autonomy, &applied_lessons);

    Ok(RoutingDecision {
        execution_mode: route.execution_mode,
        route_source: route.route_source,
        task_id: request.task_id.clone(),
        category: request.category.clone(),
        selected_claw_id: claw.id.clone(),
        selected_claw_role: route.selected_claw_role,
        selected_agent_profile_id: claw.agent_profile_id.clone(),
        selected_model_profile_id,
        selected_model: model.decision,
        available_workers: route.available_workers,
        autonomy: control_autonomy_policy_from_app(&route.autonomy),
        allow_shared_context: route.allow_shared_context,
        isolation_mode: route.isolation_mode,
        applied_lessons,
        steering_notes,
        warnings,
        request_overrides: request.overrides.clone(),
    })
}

fn matching_lessons(
    registry: &control::ControlRegistry,
    request: &OrchestrationRequest,
    claw: &control::ClawSpec,
    runtime: &control::RuntimeModeSpec,
    model_profile_id: &str,
    provider: &str,
) -> Vec<control::DecisionLessonSpec> {
    let mut matches = registry
        .lessons
        .values()
        .filter(|lesson| lesson.active)
        .filter(|lesson| {
            scope_matches(lesson.scope.task_id.as_deref(), request.task_id.as_deref())
                && scope_matches(
                    lesson.scope.category.as_deref(),
                    request.category.as_deref(),
                )
                && scope_matches(lesson.scope.claw_id.as_deref(), Some(claw.id.as_str()))
                && scope_matches(
                    lesson.scope.model_profile_id.as_deref(),
                    Some(model_profile_id),
                )
                && scope_matches(lesson.scope.provider.as_deref(), Some(provider))
                && scope_matches(
                    lesson.scope.autonomy_level.as_deref(),
                    Some(runtime.autonomy.autonomy_level.as_str()),
                )
                && scope_matches(
                    lesson.scope.execution_mode.as_deref(),
                    Some(runtime.mode.as_str()),
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    matches
        .into_iter()
        .take(runtime.autonomy.max_lesson_hints)
        .collect()
}

fn scope_matches(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => actual == Some(expected),
        None => true,
    }
}

fn build_steering_notes(
    autonomy: &control::AutonomyPolicy,
    lessons: &[control::DecisionLessonSpec],
) -> Vec<String> {
    let mut notes = vec![format!(
        "autonomy={} approval_policy={} max_delegations={} max_iterations={} max_runtime_secs={}",
        autonomy.autonomy_level,
        autonomy.approval_policy,
        autonomy.max_delegations,
        autonomy.max_iterations,
        autonomy.max_runtime_secs
    )];
    if autonomy.yolo_mode {
        notes.push(
            "yolo mode is enabled; keep actions bounded by explicit budgets, approvals, and kill switches"
                .to_string(),
        );
    }
    if !autonomy.steering_enabled {
        notes.push("steering is disabled; use lessons as soft guidance only".to_string());
    }
    if !autonomy.decision_learning_enabled {
        notes.push("decision learning is disabled for this run".to_string());
    }
    if !autonomy.critic_enabled {
        notes.push("critic loop is disabled; favor conservative execution".to_string());
    }
    for lesson in lessons {
        notes.push(format!("lesson {}: {}", lesson.id, lesson.recommendation));
    }
    notes
}

#[allow(clippy::too_many_arguments)]
fn make_checkpoint(
    stage: &str,
    status: &str,
    note: impl Into<String>,
    claw_id: Option<&str>,
    model_profile_id: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    input_excerpt: Option<String>,
    output_excerpt: Option<String>,
) -> OrchestrationCheckpoint {
    OrchestrationCheckpoint {
        checkpoint_id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        stage: stage.to_string(),
        status: status.to_string(),
        claw_id: claw_id.map(ToString::to_string),
        model_profile_id: model_profile_id.map(ToString::to_string),
        provider: provider.map(ToString::to_string),
        model: model.map(ToString::to_string),
        note: note.into(),
        input_excerpt,
        output_excerpt,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_trace_entry(
    stage: &str,
    actor_type: &str,
    actor_id: &str,
    parent_trace_id: Option<&str>,
    status: &str,
    claw_id: Option<&str>,
    model_profile_id: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    note: impl Into<String>,
    input_excerpt: Option<String>,
    output_excerpt: Option<String>,
    estimated_input_tokens: Option<usize>,
    estimated_output_tokens: Option<usize>,
    duration_ms: Option<u64>,
) -> OrchestrationTraceEntry {
    OrchestrationTraceEntry {
        trace_id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        stage: stage.to_string(),
        actor_type: actor_type.to_string(),
        actor_id: actor_id.to_string(),
        parent_trace_id: parent_trace_id.map(ToString::to_string),
        status: status.to_string(),
        claw_id: claw_id.map(ToString::to_string),
        model_profile_id: model_profile_id.map(ToString::to_string),
        provider: provider.map(ToString::to_string),
        model: model.map(ToString::to_string),
        note: note.into(),
        input_excerpt,
        output_excerpt,
        estimated_input_tokens,
        estimated_output_tokens,
        duration_ms,
    }
}

fn make_relationship(
    parent_run_id: &str,
    child_id: &str,
    child_kind: &str,
    delegation_id: Option<&str>,
    claw_id: Option<&str>,
    model_profile_id: Option<&str>,
    stage: &str,
) -> OrchestrationRelationship {
    OrchestrationRelationship {
        relationship_id: Uuid::new_v4().to_string(),
        parent_run_id: parent_run_id.to_string(),
        child_id: child_id.to_string(),
        child_kind: child_kind.to_string(),
        delegation_id: delegation_id.map(ToString::to_string),
        claw_id: claw_id.map(ToString::to_string),
        model_profile_id: model_profile_id.map(ToString::to_string),
        stage: stage.to_string(),
    }
}

fn make_transcript_entry(
    stage: &str,
    actor_type: &str,
    actor_id: &str,
    role: &str,
    content: impl Into<String>,
    parent_trace_id: Option<&str>,
    delegation_id: Option<&str>,
) -> OrchestrationTranscriptEntry {
    OrchestrationTranscriptEntry {
        transcript_id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        stage: stage.to_string(),
        actor_type: actor_type.to_string(),
        actor_id: actor_id.to_string(),
        role: role.to_string(),
        parent_trace_id: parent_trace_id.map(ToString::to_string),
        delegation_id: delegation_id.map(ToString::to_string),
        content: content.into(),
    }
}

fn should_follow_up(
    routing: &RoutingDecision,
    worker: &WorkerResultEnvelope,
    follow_up_count: usize,
) -> bool {
    if !routing.autonomy.steering_enabled {
        return false;
    }
    if follow_up_count.saturating_add(1) >= routing.autonomy.max_iterations {
        return false;
    }
    if worker.status == "failed" || worker.status == "needs_input" {
        return true;
    }
    if !worker.questions.is_empty() {
        return true;
    }
    matches!(worker.confidence, Some(confidence) if confidence < 0.6)
}

fn should_run_selective_critic(
    routing: &RoutingDecision,
    worker_results: &[WorkerResultEnvelope],
) -> bool {
    if !routing.autonomy.critic_enabled {
        return false;
    }
    worker_results.iter().any(|worker| {
        worker.status != "completed"
            || !worker.questions.is_empty()
            || matches!(worker.confidence, Some(confidence) if confidence < 0.7)
    })
}

fn ensure_runtime_budget(started_at: Instant, routing: &RoutingDecision) -> Result<()> {
    if started_at.elapsed().as_secs() >= routing.autonomy.max_runtime_secs {
        anyhow::bail!(
            "orchestration runtime budget exceeded {}s",
            routing.autonomy.max_runtime_secs
        );
    }
    Ok(())
}

fn excerpt(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn estimate_tokens(input: &str) -> usize {
    let chars = input.chars().count();
    chars.div_ceil(4)
}

fn summarize_trace_resources(trace: &[OrchestrationTraceEntry]) -> OrchestrationResourceTotals {
    cli_resource_totals_from_app(
        &OrchestrationReportingService::new().summarize_trace_resources(&app_trace_entries(trace)),
    )
}

fn summarize_actor_resources(
    trace: &[OrchestrationTraceEntry],
) -> Vec<OrchestrationActorResourceSummary> {
    OrchestrationReportingService::new()
        .summarize_actor_resources(&app_trace_entries(trace))
        .iter()
        .map(cli_actor_summary_from_app)
        .collect()
}

fn make_lesson_id(run_id: &str, candidate_index: usize, kind: &str) -> String {
    format!(
        "reflection-{}-{}-{}",
        simple_slug(run_id),
        candidate_index,
        simple_slug(kind)
    )
}

fn simple_slug(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch.to_ascii_lowercase()
        } else if !last_dash {
            last_dash = true;
            '-'
        } else {
            continue;
        };
        slug.push(next);
    }
    slug.trim_matches('-').to_string()
}

fn build_reflection_candidates(
    routing: &RoutingDecision,
    worker_results: &[WorkerResultEnvelope],
) -> Vec<ReflectionCandidate> {
    OrchestrationReportingService::new()
        .build_reflection_candidates(
            &AppReflectionRoutingContext {
                selected_claw_id: routing.selected_claw_id.clone(),
                selected_model_profile_id: routing.selected_model_profile_id.clone(),
                requested_model_profile_id: routing.selected_model.requested_profile_id.clone(),
                provider: routing.selected_model.provider.clone(),
                fallback_path: routing.selected_model.fallback_path.clone(),
            },
            &worker_results
                .iter()
                .map(app_reflection_worker_from_cli)
                .collect::<Vec<_>>(),
        )
        .iter()
        .map(cli_reflection_candidate_from_app)
        .collect()
}

fn summarize_supervision(
    checkpoints: &[OrchestrationCheckpoint],
    worker_results: &[WorkerResultEnvelope],
    reflection_candidates: &[ReflectionCandidate],
) -> SupervisionSummary {
    cli_supervision_summary_from_app(
        &OrchestrationReportingService::new().summarize_supervision(
            checkpoints.len(),
            &worker_results
                .iter()
                .map(app_reflection_worker_from_cli)
                .collect::<Vec<_>>(),
            &reflection_candidates
                .iter()
                .map(app_reflection_candidate_from_cli)
                .collect::<Vec<_>>(),
        ),
    )
}

fn resolve_model_with_fallback(
    requested_profile_id: &str,
    registry: &control::ControlRegistry,
    config: &AppConfig,
) -> Result<ExecutableModel> {
    let mut queue = VecDeque::from([requested_profile_id.to_string()]);
    let mut visited = HashSet::new();
    let mut attempted = Vec::new();
    let mut warnings = Vec::new();

    while let Some(profile_id) = queue.pop_front() {
        if !visited.insert(profile_id.clone()) {
            continue;
        }

        let profile = registry
            .model_profiles
            .get(&profile_id)
            .with_context(|| format!("Unknown model profile '{}'", profile_id))?;
        attempted.push(profile.id.clone());
        match instantiate_provider_for_profile(profile, config) {
            Ok(provider) => {
                return Ok(ExecutableModel {
                    provider,
                    decision: ResolvedModelDecision {
                        requested_profile_id: requested_profile_id.to_string(),
                        selected_profile_id: profile.id.clone(),
                        provider: profile.provider.clone(),
                        model: profile.model.clone(),
                        fallback_path: attempted,
                        warnings,
                    },
                });
            }
            Err(error) => {
                warnings.push(format!("{}: {}", profile.id, error));
                for fallback in &profile.fallback_order {
                    queue.push_back(fallback.clone());
                }
            }
        }
    }

    anyhow::bail!(
        "Failed to resolve executable model profile '{}': {}",
        requested_profile_id,
        warnings.join("; ")
    )
}

fn instantiate_provider_for_profile(
    profile: &control::ModelProfileSpec,
    config: &AppConfig,
) -> Result<Arc<dyn LlmProvider>> {
    match profile.provider.to_lowercase().as_str() {
        "anthropic" => {
            let env_name = config
                .providers
                .anthropic
                .api_key_env
                .as_deref()
                .unwrap_or("ANTHROPIC_API_KEY");
            let api_key = std::env::var(env_name)
                .with_context(|| format!("{} environment variable not set", env_name))?;
            Ok(Arc::new(AnthropicProvider::new(
                api_key,
                profile.model.clone(),
            )))
        }
        "openai" => {
            let env_name = config
                .providers
                .openai
                .api_key_env
                .as_deref()
                .unwrap_or("OPENAI_API_KEY");
            let api_key = std::env::var(env_name)
                .with_context(|| format!("{} environment variable not set", env_name))?;
            Ok(Arc::new(OpenAiProvider::new(
                api_key,
                profile.model.clone(),
            )))
        }
        "openrouter" => {
            let env_name = config
                .providers
                .openrouter
                .api_key_env
                .as_deref()
                .unwrap_or("OPENROUTER_API_KEY");
            let api_key = std::env::var(env_name)
                .with_context(|| format!("{} environment variable not set", env_name))?;
            let strategy = match config.providers.openrouter.route_strategy.as_str() {
                "price" => RouteStrategy::Price,
                "throughput" => RouteStrategy::Throughput,
                "web_search" | "online" => RouteStrategy::WebSearch,
                _ => RouteStrategy::Quality,
            };
            Ok(Arc::new(OpenRouterProvider::with_strategy(
                api_key,
                profile.model.clone(),
                strategy,
            )))
        }
        "ollama" => Ok(Arc::new(OllamaProvider::with_base_url(
            profile.model.clone(),
            config.providers.ollama.base_url.clone(),
        ))),
        "gemini" | "google" => {
            let api_key = std::env::var("GEMINI_API_KEY")
                .or_else(|_| std::env::var("GOOGLE_API_KEY"))
                .context("GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set")?;
            Ok(Arc::new(GeminiProvider::new(
                api_key,
                profile.model.clone(),
            )))
        }
        other => anyhow::bail!("unsupported provider '{}'", other),
    }
}

async fn run_direct(
    request: OrchestrationRequest,
    registry: &control::ControlRegistry,
    config: &AppConfig,
    routing: &RoutingDecision,
    workspace_root: &Path,
    monitor: Option<&ActiveRunMonitor>,
) -> Result<OrchestrationRunRecord> {
    let run_id = Uuid::new_v4().to_string();
    let claw = registry
        .claws
        .get(&routing.selected_claw_id)
        .with_context(|| format!("Unknown claw '{}'", routing.selected_claw_id))?;
    let executable =
        resolve_model_with_fallback(&routing.selected_model_profile_id, registry, config)?;
    let agent_profile = registry
        .agent_profiles
        .get(&claw.agent_profile_id)
        .with_context(|| format!("Unknown agent profile '{}'", claw.agent_profile_id))?;
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "running",
                "direct_response",
                "claw",
                claw.id.as_str(),
                "starting direct response",
            )
            .await?;
    }
    let system_prompt = build_claw_system_prompt(
        claw,
        agent_profile,
        routing,
        workspace_root,
        "direct_response",
    );
    let user_prompt = request.prompt.clone();
    let prompt = execute_completion(
        executable.provider.clone(),
        system_prompt.clone(),
        user_prompt.clone(),
    )
    .await?;
    let checkpoints = vec![make_checkpoint(
        "direct_response",
        "completed",
        "direct run completed",
        Some(claw.id.as_str()),
        Some(executable.decision.selected_profile_id.as_str()),
        Some(executable.decision.provider.as_str()),
        Some(executable.decision.model.as_str()),
        Some(excerpt(&request.prompt, 240)),
        Some(excerpt(&prompt.response.message.content, 240)),
    )];
    let trace = vec![make_trace_entry(
        "direct_response",
        "claw",
        claw.id.as_str(),
        None,
        "completed",
        Some(claw.id.as_str()),
        Some(executable.decision.selected_profile_id.as_str()),
        Some(executable.decision.provider.as_str()),
        Some(executable.decision.model.as_str()),
        "direct run completed",
        Some(excerpt(&request.prompt, 240)),
        Some(excerpt(&prompt.response.message.content, 240)),
        Some(prompt.estimated_input_tokens),
        Some(prompt.estimated_output_tokens),
        Some(prompt.duration_ms),
    )];
    let transcript = vec![
        make_transcript_entry(
            "direct_response",
            "claw",
            claw.id.as_str(),
            "system",
            system_prompt,
            None,
            None,
        ),
        make_transcript_entry(
            "direct_response",
            "operator",
            "request",
            "user",
            user_prompt,
            None,
            None,
        ),
        make_transcript_entry(
            "direct_response",
            "claw",
            claw.id.as_str(),
            "assistant",
            prompt.response.message.content.clone(),
            trace.first().map(|entry| entry.trace_id.as_str()),
            None,
        ),
    ];
    let reflection_candidates = build_reflection_candidates(routing, &[]);
    let supervision = summarize_supervision(&checkpoints, &[], &reflection_candidates);
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "completed",
                "direct_response",
                "claw",
                claw.id.as_str(),
                "direct response completed",
            )
            .await?;
    }

    Ok(OrchestrationRunRecord {
        run_id,
        created_at: Utc::now().to_rfc3339(),
        mode: "direct".to_string(),
        request,
        routing: routing.clone(),
        delegations: Vec::new(),
        worker_results: Vec::new(),
        checkpoints,
        trace,
        relationships: Vec::new(),
        transcript,
        reflection_notes: routing.steering_notes.clone(),
        reflection_candidates,
        supervision: Some(supervision),
        lifecycle: SupervisionLifecycleSummary::default(),
        decision_history: Vec::new(),
        final_output: prompt.response.message.content,
        final_claw_id: claw.id.clone(),
        final_model_profile_id: executable.decision.selected_profile_id.clone(),
        final_provider: executable.decision.provider.clone(),
        final_model: executable.decision.model.clone(),
        receipt_path: String::new(),
    })
}

async fn run_orchestrated(
    request: OrchestrationRequest,
    registry: &control::ControlRegistry,
    config: &AppConfig,
    routing: &RoutingDecision,
    workspace_root: &Path,
    monitor: Option<&ActiveRunMonitor>,
) -> Result<OrchestrationRunRecord> {
    let run_id = Uuid::new_v4().to_string();
    let started_at = Instant::now();
    let orchestrator = registry
        .claws
        .get(&routing.selected_claw_id)
        .with_context(|| format!("Unknown claw '{}'", routing.selected_claw_id))?;
    let orchestrator_agent = registry
        .agent_profiles
        .get(&orchestrator.agent_profile_id)
        .with_context(|| format!("Unknown agent profile '{}'", orchestrator.agent_profile_id))?;
    let orchestrator_model =
        resolve_model_with_fallback(&routing.selected_model_profile_id, registry, config)?;
    let worker_candidates = registry
        .claws
        .values()
        .filter(|claw| claw.enabled && claw.id != orchestrator.id)
        .collect::<Vec<_>>();
    let mut trace = Vec::new();
    let mut relationships = Vec::new();
    let mut transcript = vec![make_transcript_entry(
        "routing",
        "operator",
        "request",
        "user",
        request.prompt.clone(),
        None,
        None,
    )];
    let mut checkpoints = vec![make_checkpoint(
        "routing",
        "completed",
        format!(
            "resolved route {} -> {} ({})",
            routing.route_source, routing.selected_claw_id, routing.selected_model.model
        ),
        Some(orchestrator.id.as_str()),
        Some(routing.selected_model.selected_profile_id.as_str()),
        Some(routing.selected_model.provider.as_str()),
        Some(routing.selected_model.model.as_str()),
        Some(excerpt(&request.prompt, 240)),
        None,
    )];
    trace.push(make_trace_entry(
        "routing",
        "orchestrator",
        orchestrator.id.as_str(),
        None,
        "completed",
        Some(orchestrator.id.as_str()),
        Some(routing.selected_model.selected_profile_id.as_str()),
        Some(routing.selected_model.provider.as_str()),
        Some(routing.selected_model.model.as_str()),
        format!(
            "resolved route {} -> {} ({})",
            routing.route_source, routing.selected_claw_id, routing.selected_model.model
        ),
        Some(excerpt(&request.prompt, 240)),
        None,
        None,
        None,
        None,
    ));
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "running",
                "routing",
                "orchestrator",
                orchestrator.id.as_str(),
                "orchestrated run started",
            )
            .await?;
    }

    if worker_candidates.is_empty() {
        return run_direct(request, registry, config, routing, workspace_root, monitor).await;
    }

    let available_workers = worker_candidates
        .iter()
        .map(|claw| {
            serde_json::json!({
                "id": claw.id,
                "role": claw.role,
                "categories": claw.task_categories,
                "model_profile_id": claw.model_profile_id,
                "agent_profile_id": claw.agent_profile_id,
            })
        })
        .collect::<Vec<_>>();
    let max_delegations = routing.autonomy.max_delegations.max(1);
    let planner_system = format!(
        "{}\nReturn JSON only with this schema:\n{{\"final_mode\":\"delegate|answer_directly\",\"direct_response\":\"...\",\"delegations\":[{{\"claw_id\":\"worker-id\",\"instruction\":\"bounded instruction\",\"reason\":\"why this worker\"}}]}}\nIf delegation is needed, keep it to at most {} worker tasks and only use these claw ids: {}.",
        build_claw_system_prompt(
            orchestrator,
            orchestrator_agent,
            routing,
            workspace_root,
            "orchestrator_planner",
        ),
        max_delegations,
        available_workers
            .iter()
            .filter_map(|item| item["id"].as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let planner_user = format!(
        "Original request:\n{}\n\nAvailable workers:\n{}",
        request.prompt,
        serde_json::to_string_pretty(&available_workers)?
    );
    transcript.push(make_transcript_entry(
        "planner",
        "orchestrator",
        orchestrator.id.as_str(),
        "system",
        planner_system.clone(),
        trace.last().map(|entry| entry.trace_id.as_str()),
        None,
    ));
    transcript.push(make_transcript_entry(
        "planner",
        "operator",
        "request",
        "user",
        planner_user.clone(),
        trace.last().map(|entry| entry.trace_id.as_str()),
        None,
    ));
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "running",
                "planner",
                "orchestrator",
                orchestrator.id.as_str(),
                "planner deciding worker delegation",
            )
            .await?;
    }
    ensure_runtime_budget(started_at, routing)?;
    let planner_response = execute_completion(
        orchestrator_model.provider.clone(),
        planner_system,
        planner_user,
    )
    .await?;
    let planner_trace = make_trace_entry(
        "planner",
        "orchestrator",
        orchestrator.id.as_str(),
        trace.last().map(|entry| entry.trace_id.as_str()),
        "completed",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        "orchestrator planner completed",
        Some(excerpt(&request.prompt, 240)),
        Some(excerpt(&planner_response.response.message.content, 240)),
        Some(planner_response.estimated_input_tokens),
        Some(planner_response.estimated_output_tokens),
        Some(planner_response.duration_ms),
    );
    let planner_trace_id = planner_trace.trace_id.clone();
    trace.push(planner_trace);
    transcript.push(make_transcript_entry(
        "planner",
        "orchestrator",
        orchestrator.id.as_str(),
        "assistant",
        planner_response.response.message.content.clone(),
        Some(planner_trace_id.as_str()),
        None,
    ));
    checkpoints.push(make_checkpoint(
        "planner",
        "completed",
        "orchestrator planner completed",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        Some(excerpt(&request.prompt, 240)),
        Some(excerpt(&planner_response.response.message.content, 240)),
    ));
    let plan = parse_json_payload::<PlannerResponse>(&planner_response.response.message.content)
        .unwrap_or(PlannerResponse {
            final_mode: Some("answer_directly".to_string()),
            direct_response: Some(planner_response.response.message.content.clone()),
            delegations: Vec::new(),
        });

    let mut reflection_notes = routing.steering_notes.clone();
    if routing.autonomy.critic_enabled && !plan.delegations.is_empty() {
        let review_system = format!(
            "{}\nReturn JSON only with this schema:\n{{\"action\":\"continue|answer_directly\",\"note\":\"brief reasoning\"}}\nDo not rewrite the whole plan unless it is clearly over-scoped or unsafe.",
            build_claw_system_prompt(
                orchestrator,
                orchestrator_agent,
                routing,
                workspace_root,
                "plan_review",
            )
        );
        let review_user = format!(
            "Original request:\n{}\n\nPlanner output:\n{}",
            request.prompt,
            serde_json::to_string_pretty(&plan)?
        );
        transcript.push(make_transcript_entry(
            "plan_review",
            "critic",
            orchestrator.id.as_str(),
            "system",
            review_system.clone(),
            Some(planner_trace_id.as_str()),
            None,
        ));
        transcript.push(make_transcript_entry(
            "plan_review",
            "critic",
            orchestrator.id.as_str(),
            "user",
            review_user.clone(),
            Some(planner_trace_id.as_str()),
            None,
        ));
        if let Some(monitor) = monitor {
            monitor
                .checkpoint(
                    "running",
                    "plan_review",
                    "critic",
                    orchestrator.id.as_str(),
                    "critic reviewing delegation plan",
                )
                .await?;
        }
        ensure_runtime_budget(started_at, routing)?;
        let review_response = execute_completion(
            orchestrator_model.provider.clone(),
            review_system,
            review_user,
        )
        .await?;
        let review =
            parse_json_payload::<PlanReviewResponse>(&review_response.response.message.content)
                .unwrap_or(PlanReviewResponse {
                    action: Some("continue".to_string()),
                    note: Some(review_response.response.message.content.clone()),
                });
        trace.push(make_trace_entry(
            "plan_review",
            "critic",
            orchestrator.id.as_str(),
            Some(planner_trace_id.as_str()),
            "completed",
            Some(orchestrator.id.as_str()),
            Some(orchestrator_model.decision.selected_profile_id.as_str()),
            Some(orchestrator_model.decision.provider.as_str()),
            Some(orchestrator_model.decision.model.as_str()),
            "critic reviewed delegation plan",
            Some(excerpt(&request.prompt, 240)),
            Some(excerpt(&review_response.response.message.content, 240)),
            Some(review_response.estimated_input_tokens),
            Some(review_response.estimated_output_tokens),
            Some(review_response.duration_ms),
        ));
        checkpoints.push(make_checkpoint(
            "plan_review",
            "completed",
            "critic reviewed delegation plan",
            Some(orchestrator.id.as_str()),
            Some(orchestrator_model.decision.selected_profile_id.as_str()),
            Some(orchestrator_model.decision.provider.as_str()),
            Some(orchestrator_model.decision.model.as_str()),
            Some(excerpt(&request.prompt, 240)),
            Some(excerpt(&review_response.response.message.content, 240)),
        ));
        transcript.push(make_transcript_entry(
            "plan_review",
            "critic",
            orchestrator.id.as_str(),
            "assistant",
            review_response.response.message.content.clone(),
            trace.last().map(|entry| entry.trace_id.as_str()),
            None,
        ));
        if let Some(note) = review.note
            && !note.trim().is_empty()
        {
            reflection_notes.push(format!("critic: {}", note.trim()));
        }
        if matches!(review.action.as_deref(), Some("answer_directly")) {
            let reflection_candidates = build_reflection_candidates(routing, &[]);
            let supervision = summarize_supervision(&checkpoints, &[], &reflection_candidates);
            return Ok(OrchestrationRunRecord {
                run_id,
                created_at: Utc::now().to_rfc3339(),
                mode: "orchestrated".to_string(),
                request,
                routing: routing.clone(),
                delegations: Vec::new(),
                worker_results: Vec::new(),
                checkpoints,
                trace,
                relationships,
                transcript,
                reflection_notes,
                reflection_candidates,
                supervision: Some(supervision),
                lifecycle: SupervisionLifecycleSummary::default(),
                decision_history: Vec::new(),
                final_output: plan
                    .direct_response
                    .unwrap_or_else(|| "critic requested direct answer".to_string()),
                final_claw_id: orchestrator.id.clone(),
                final_model_profile_id: orchestrator_model.decision.selected_profile_id.clone(),
                final_provider: orchestrator_model.decision.provider.clone(),
                final_model: orchestrator_model.decision.model.clone(),
                receipt_path: String::new(),
            });
        }
    }

    if matches!(plan.final_mode.as_deref(), Some("answer_directly")) || plan.delegations.is_empty()
    {
        let reflection_candidates = build_reflection_candidates(routing, &[]);
        let supervision = summarize_supervision(&checkpoints, &[], &reflection_candidates);
        return Ok(OrchestrationRunRecord {
            run_id,
            created_at: Utc::now().to_rfc3339(),
            mode: "orchestrated".to_string(),
            request,
            routing: routing.clone(),
            delegations: Vec::new(),
            worker_results: Vec::new(),
            checkpoints,
            trace,
            relationships,
            transcript,
            reflection_notes,
            reflection_candidates,
            supervision: Some(supervision),
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: Vec::new(),
            final_output: plan
                .direct_response
                .unwrap_or(planner_response.response.message.content),
            final_claw_id: orchestrator.id.clone(),
            final_model_profile_id: orchestrator_model.decision.selected_profile_id.clone(),
            final_provider: orchestrator_model.decision.provider.clone(),
            final_model: orchestrator_model.decision.model.clone(),
            receipt_path: String::new(),
        });
    }

    let mut delegations = Vec::new();
    let mut worker_results = Vec::new();
    for item in plan.delegations.into_iter().take(max_delegations) {
        let worker = registry
            .claws
            .get(&item.claw_id)
            .with_context(|| format!("Planner selected unknown claw '{}'", item.claw_id))?;
        let worker_agent = registry
            .agent_profiles
            .get(&worker.agent_profile_id)
            .with_context(|| format!("Unknown agent profile '{}'", worker.agent_profile_id))?;
        let worker_model = resolve_model_with_fallback(
            request
                .overrides
                .worker_model_profile_id
                .as_deref()
                .unwrap_or(worker.model_profile_id.as_str()),
            registry,
            config,
        )?;

        delegations.push(DelegationTask {
            id: Uuid::new_v4().to_string(),
            claw_id: worker.id.clone(),
            instruction: item.instruction.clone(),
            reason: item.reason.clone(),
        });
        let delegation_id = delegations
            .last()
            .map(|task| task.id.clone())
            .unwrap_or_default();
        relationships.push(make_relationship(
            &run_id,
            worker.id.as_str(),
            "delegated_task",
            Some(delegation_id.as_str()),
            Some(worker.id.as_str()),
            Some(worker_model.decision.requested_profile_id.as_str()),
            "worker_execution",
        ));
        let mut follow_up_count = 0usize;
        let mut instruction = item.instruction.clone();
        let worker_result;
        loop {
            if let Some(monitor) = monitor {
                monitor
                    .checkpoint(
                        "running",
                        "worker_execution",
                        "worker",
                        worker.id.as_str(),
                        format!("worker '{}' executing delegated task", worker.id).as_str(),
                    )
                    .await?;
            }
            ensure_runtime_budget(started_at, routing)?;
            let worker_system = format!(
                "{}\nReturn JSON only with this schema:\n{{\"status\":\"completed|needs_input|failed\",\"summary\":\"one paragraph\",\"full_output\":\"detailed output\",\"questions\":[\"...\"],\"confidence\":0.0,\"next_step_recommendation\":\"...\"}}",
                build_claw_system_prompt(
                    worker,
                    worker_agent,
                    routing,
                    workspace_root,
                    "worker_execution"
                ),
            );
            let worker_user = format!(
                "Original request:\n{}\n\nAssigned subtask:\n{}\n\nReason for assignment:\n{}\n\nFollow-up round: {}",
                request.prompt, instruction, item.reason, follow_up_count
            );
            transcript.push(make_transcript_entry(
                "worker_execution",
                "worker",
                worker.id.as_str(),
                "system",
                worker_system.clone(),
                Some(planner_trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            transcript.push(make_transcript_entry(
                "worker_execution",
                "orchestrator",
                orchestrator.id.as_str(),
                "user",
                worker_user.clone(),
                Some(planner_trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            let worker_response =
                execute_completion(worker_model.provider.clone(), worker_system, worker_user)
                    .await?;
            let parsed = parse_json_payload::<WorkerEnvelopeResponse>(
                &worker_response.response.message.content,
            )
            .unwrap_or(WorkerEnvelopeResponse {
                status: "completed".to_string(),
                summary: worker_response
                    .response
                    .message
                    .content
                    .chars()
                    .take(240)
                    .collect(),
                full_output: Some(worker_response.response.message.content.clone()),
                questions: Vec::new(),
                confidence: None,
                next_step_recommendation: None,
            });
            checkpoints.push(make_checkpoint(
                "worker_execution",
                &parsed.status,
                format!("worker '{}' completed delegated step", worker.id),
                Some(worker.id.as_str()),
                Some(worker_model.decision.selected_profile_id.as_str()),
                Some(worker_model.decision.provider.as_str()),
                Some(worker_model.decision.model.as_str()),
                Some(excerpt(&instruction, 240)),
                Some(excerpt(&worker_response.response.message.content, 240)),
            ));
            let worker_trace = make_trace_entry(
                "worker_execution",
                "worker",
                worker.id.as_str(),
                Some(planner_trace_id.as_str()),
                &parsed.status,
                Some(worker.id.as_str()),
                Some(worker_model.decision.selected_profile_id.as_str()),
                Some(worker_model.decision.provider.as_str()),
                Some(worker_model.decision.model.as_str()),
                format!("worker '{}' completed delegated step", worker.id),
                Some(excerpt(&instruction, 240)),
                Some(excerpt(&worker_response.response.message.content, 240)),
                Some(worker_response.estimated_input_tokens),
                Some(worker_response.estimated_output_tokens),
                Some(worker_response.duration_ms),
            );
            let worker_trace_id = worker_trace.trace_id.clone();
            trace.push(worker_trace);
            transcript.push(make_transcript_entry(
                "worker_execution",
                "worker",
                worker.id.as_str(),
                "assistant",
                worker_response.response.message.content.clone(),
                Some(worker_trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            let current = WorkerResultEnvelope {
                claw_id: worker.id.clone(),
                agent_profile_id: worker.agent_profile_id.clone(),
                model_profile_id: worker_model.decision.selected_profile_id.clone(),
                provider: worker_model.decision.provider.clone(),
                model: worker_model.decision.model.clone(),
                status: parsed.status,
                summary: parsed.summary,
                full_output: parsed
                    .full_output
                    .unwrap_or_else(|| worker_response.response.message.content.clone()),
                questions: parsed.questions,
                confidence: parsed.confidence,
                next_step_recommendation: parsed.next_step_recommendation,
            };
            if !should_follow_up(routing, &current, follow_up_count) {
                worker_result = Some(current);
                break;
            }

            if let Some(monitor) = monitor {
                monitor
                    .checkpoint(
                        "running",
                        "follow_up_review",
                        "orchestrator",
                        orchestrator.id.as_str(),
                        format!("reviewing follow-up need for worker '{}'", worker.id).as_str(),
                    )
                    .await?;
            }
            ensure_runtime_budget(started_at, routing)?;
            let follow_up_system = format!(
                "{}\nReturn JSON only with this schema:\n{{\"action\":\"follow_up|accept|escalate\",\"instruction\":\"optional new instruction\",\"note\":\"brief reasoning\"}}\nOnly request another worker round if it materially improves answer quality; do not over-steer high-confidence completed work.",
                build_claw_system_prompt(
                    orchestrator,
                    orchestrator_agent,
                    routing,
                    workspace_root,
                    "follow_up_review",
                )
            );
            let follow_up_user = format!(
                "Original request:\n{}\n\nWorker result:\n{}\n\nCurrent instruction:\n{}",
                request.prompt,
                serde_json::to_string_pretty(&current)?,
                instruction
            );
            transcript.push(make_transcript_entry(
                "follow_up_review",
                "critic",
                orchestrator.id.as_str(),
                "system",
                follow_up_system.clone(),
                Some(worker_trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            transcript.push(make_transcript_entry(
                "follow_up_review",
                "critic",
                orchestrator.id.as_str(),
                "user",
                follow_up_user.clone(),
                Some(worker_trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            let follow_up_response = execute_completion(
                orchestrator_model.provider.clone(),
                follow_up_system,
                follow_up_user,
            )
            .await?;
            let follow_up = parse_json_payload::<FollowUpReviewResponse>(
                &follow_up_response.response.message.content,
            )
            .unwrap_or(FollowUpReviewResponse {
                action: Some("accept".to_string()),
                instruction: None,
                note: Some(follow_up_response.response.message.content.clone()),
            });
            trace.push(make_trace_entry(
                "follow_up_review",
                "critic",
                orchestrator.id.as_str(),
                Some(worker_trace_id.as_str()),
                "completed",
                Some(orchestrator.id.as_str()),
                Some(orchestrator_model.decision.selected_profile_id.as_str()),
                Some(orchestrator_model.decision.provider.as_str()),
                Some(orchestrator_model.decision.model.as_str()),
                format!("reviewed follow-up need for worker '{}'", worker.id),
                Some(excerpt(&instruction, 240)),
                Some(excerpt(&follow_up_response.response.message.content, 240)),
                Some(follow_up_response.estimated_input_tokens),
                Some(follow_up_response.estimated_output_tokens),
                Some(follow_up_response.duration_ms),
            ));
            checkpoints.push(make_checkpoint(
                "follow_up_review",
                "completed",
                format!("reviewed follow-up need for worker '{}'", worker.id),
                Some(orchestrator.id.as_str()),
                Some(orchestrator_model.decision.selected_profile_id.as_str()),
                Some(orchestrator_model.decision.provider.as_str()),
                Some(orchestrator_model.decision.model.as_str()),
                Some(excerpt(&instruction, 240)),
                Some(excerpt(&follow_up_response.response.message.content, 240)),
            ));
            transcript.push(make_transcript_entry(
                "follow_up_review",
                "critic",
                orchestrator.id.as_str(),
                "assistant",
                follow_up_response.response.message.content.clone(),
                trace.last().map(|entry| entry.trace_id.as_str()),
                Some(delegation_id.as_str()),
            ));
            match follow_up.action.as_deref() {
                Some("follow_up")
                    if follow_up_count.saturating_add(1) < routing.autonomy.max_iterations =>
                {
                    follow_up_count += 1;
                    instruction = follow_up
                        .instruction
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| {
                            format!(
                                "{}\n\nAddress the open questions and improve confidence before returning.",
                                current.full_output
                            )
                        });
                    if let Some(note) = follow_up.note
                        && !note.trim().is_empty()
                    {
                        reflection_notes.push(format!(
                            "follow-up {} for worker '{}': {}",
                            follow_up_count,
                            worker.id,
                            note.trim()
                        ));
                    }
                    continue;
                }
                Some("escalate") => {
                    let mut escalated = current;
                    escalated.status = "needs_input".to_string();
                    if escalated.next_step_recommendation.is_none() {
                        escalated.next_step_recommendation = follow_up.note.clone();
                    }
                    worker_result = Some(escalated);
                    break;
                }
                _ => {
                    worker_result = Some(current);
                    break;
                }
            }
        }

        if let Some(result) = worker_result {
            worker_results.push(result);
        }
    }

    if should_run_selective_critic(routing, &worker_results) {
        if let Some(monitor) = monitor {
            monitor
                .checkpoint(
                    "running",
                    "quality_review",
                    "critic",
                    orchestrator.id.as_str(),
                    "selective critic reviewing worker results",
                )
                .await?;
        }
        ensure_runtime_budget(started_at, routing)?;
        let critic_system = format!(
            "{}\nReturn JSON only with this schema:\n{{\"action\":\"proceed|surface_blockers\",\"note\":\"brief reasoning\"}}\nReview the worker set only when confidence, failures, or open questions warrant it.",
            build_claw_system_prompt(
                orchestrator,
                orchestrator_agent,
                routing,
                workspace_root,
                "quality_review",
            )
        );
        let critic_user = format!(
            "Original request:\n{}\n\nWorker results:\n{}",
            request.prompt,
            serde_json::to_string_pretty(&worker_results)?
        );
        transcript.push(make_transcript_entry(
            "quality_review",
            "critic",
            orchestrator.id.as_str(),
            "system",
            critic_system.clone(),
            Some(planner_trace_id.as_str()),
            None,
        ));
        transcript.push(make_transcript_entry(
            "quality_review",
            "critic",
            orchestrator.id.as_str(),
            "user",
            critic_user.clone(),
            Some(planner_trace_id.as_str()),
            None,
        ));
        let critic_response = execute_completion(
            orchestrator_model.provider.clone(),
            critic_system,
            critic_user,
        )
        .await?;
        let critic_review =
            parse_json_payload::<PlanReviewResponse>(&critic_response.response.message.content)
                .unwrap_or(PlanReviewResponse {
                    action: Some("proceed".to_string()),
                    note: Some(critic_response.response.message.content.clone()),
                });
        trace.push(make_trace_entry(
            "quality_review",
            "critic",
            orchestrator.id.as_str(),
            Some(planner_trace_id.as_str()),
            "completed",
            Some(orchestrator.id.as_str()),
            Some(orchestrator_model.decision.selected_profile_id.as_str()),
            Some(orchestrator_model.decision.provider.as_str()),
            Some(orchestrator_model.decision.model.as_str()),
            "selective critic reviewed worker outputs",
            Some(excerpt(&request.prompt, 240)),
            Some(excerpt(&critic_response.response.message.content, 240)),
            Some(critic_response.estimated_input_tokens),
            Some(critic_response.estimated_output_tokens),
            Some(critic_response.duration_ms),
        ));
        checkpoints.push(make_checkpoint(
            "quality_review",
            "completed",
            "selective critic reviewed worker outputs",
            Some(orchestrator.id.as_str()),
            Some(orchestrator_model.decision.selected_profile_id.as_str()),
            Some(orchestrator_model.decision.provider.as_str()),
            Some(orchestrator_model.decision.model.as_str()),
            Some(excerpt(&request.prompt, 240)),
            Some(excerpt(&critic_response.response.message.content, 240)),
        ));
        transcript.push(make_transcript_entry(
            "quality_review",
            "critic",
            orchestrator.id.as_str(),
            "assistant",
            critic_response.response.message.content.clone(),
            trace.last().map(|entry| entry.trace_id.as_str()),
            None,
        ));
        if let Some(note) = critic_review.note
            && !note.trim().is_empty()
        {
            reflection_notes.push(format!("quality review: {}", note.trim()));
        }
    }

    let synthesis_system = format!(
        "{}\nSynthesize the final user-facing answer from the worker results. If workers asked questions, surface only the necessary blockers. Do not mention hidden orchestration unless relevant.",
        build_claw_system_prompt(
            orchestrator,
            orchestrator_agent,
            routing,
            workspace_root,
            "orchestrator_synthesis",
        )
    );
    let synthesis_user = format!(
        "Original request:\n{}\n\nWorker results:\n{}",
        request.prompt,
        serde_json::to_string_pretty(&worker_results)?
    );
    transcript.push(make_transcript_entry(
        "synthesis",
        "orchestrator",
        orchestrator.id.as_str(),
        "system",
        synthesis_system.clone(),
        Some(planner_trace_id.as_str()),
        None,
    ));
    transcript.push(make_transcript_entry(
        "synthesis",
        "orchestrator",
        orchestrator.id.as_str(),
        "user",
        synthesis_user.clone(),
        Some(planner_trace_id.as_str()),
        None,
    ));
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "running",
                "synthesis",
                "orchestrator",
                orchestrator.id.as_str(),
                "synthesizing final response",
            )
            .await?;
    }
    ensure_runtime_budget(started_at, routing)?;
    let synthesis_input_excerpt = excerpt(&synthesis_user, 240);
    let final_response = execute_completion(
        orchestrator_model.provider.clone(),
        synthesis_system,
        synthesis_user,
    )
    .await?;
    trace.push(make_trace_entry(
        "synthesis",
        "orchestrator",
        orchestrator.id.as_str(),
        Some(planner_trace_id.as_str()),
        "completed",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        "orchestrator synthesized final response",
        Some(synthesis_input_excerpt.clone()),
        Some(excerpt(&final_response.response.message.content, 240)),
        Some(final_response.estimated_input_tokens),
        Some(final_response.estimated_output_tokens),
        Some(final_response.duration_ms),
    ));
    transcript.push(make_transcript_entry(
        "synthesis",
        "orchestrator",
        orchestrator.id.as_str(),
        "assistant",
        final_response.response.message.content.clone(),
        trace.last().map(|entry| entry.trace_id.as_str()),
        None,
    ));
    checkpoints.push(make_checkpoint(
        "synthesis",
        "completed",
        "orchestrator synthesized final response",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        Some(synthesis_input_excerpt),
        Some(excerpt(&final_response.response.message.content, 240)),
    ));
    let reflection_candidates = build_reflection_candidates(routing, &worker_results);
    if !reflection_candidates.is_empty() {
        reflection_notes.push(format!(
            "{} reflection candidate(s) generated for operator review",
            reflection_candidates.len()
        ));
    }
    let supervision = summarize_supervision(&checkpoints, &worker_results, &reflection_candidates);
    if let Some(monitor) = monitor {
        monitor
            .checkpoint(
                "completed",
                "synthesis",
                "orchestrator",
                orchestrator.id.as_str(),
                "orchestrated run completed",
            )
            .await?;
    }

    Ok(OrchestrationRunRecord {
        run_id,
        created_at: Utc::now().to_rfc3339(),
        mode: "orchestrated".to_string(),
        request,
        routing: routing.clone(),
        delegations,
        worker_results,
        checkpoints,
        trace,
        relationships,
        transcript,
        reflection_notes,
        reflection_candidates,
        supervision: Some(supervision),
        lifecycle: SupervisionLifecycleSummary::default(),
        decision_history: Vec::new(),
        final_output: final_response.response.message.content,
        final_claw_id: orchestrator.id.clone(),
        final_model_profile_id: orchestrator_model.decision.selected_profile_id.clone(),
        final_provider: orchestrator_model.decision.provider.clone(),
        final_model: orchestrator_model.decision.model.clone(),
        receipt_path: String::new(),
    })
}

fn build_claw_system_prompt(
    claw: &control::ClawSpec,
    agent_profile: &control::AgentProfileSpec,
    routing: &RoutingDecision,
    workspace_root: &Path,
    mode: &str,
) -> String {
    let lesson_hints = if routing.applied_lessons.is_empty() {
        "Decision lessons: none".to_string()
    } else {
        let joined = routing
            .applied_lessons
            .iter()
            .map(|lesson| {
                format!(
                    "{} => {} (confidence {:.2})",
                    lesson.signal, lesson.recommendation, lesson.confidence
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!("Decision lessons: {joined}")
    };
    let steering_notes = if routing.steering_notes.is_empty() {
        "-".to_string()
    } else {
        routing.steering_notes.join(" | ")
    };
    format!(
        "You are Claw '{claw_id}' operating in OpenRustClaw.\n\
Role: {role}\n\
Agent profile: {agent_profile_id}\n\
Memory scope: {memory_scope}\n\
Output policy: {output_policy}\n\
Execution mode: {execution_mode}\n\
Isolation mode: {isolation_mode}\n\
Shared context allowed: {shared_context}\n\
Autonomy level: {autonomy_level}\n\
Yolo mode: {yolo_mode}\n\
Approval policy: {approval_policy}\n\
Steering enabled: {steering_enabled}\n\
Decision learning enabled: {decision_learning_enabled}\n\
Critic enabled: {critic_enabled}\n\
Max delegations: {max_delegations}\n\
Max iterations: {max_iterations}\n\
Max runtime secs: {max_runtime_secs}\n\
Workspace: {workspace}\n\
Task category: {category}\n\
Task id: {task_id}\n\
Run mode: {mode}\n\
Steering notes: {steering_notes}\n\
{lesson_hints}\n\
Stay within your assigned responsibility. Cross-task contamination is disallowed unless shared context is explicitly enabled. Respect the autonomy policy as risk limits, not as a replacement for sound model judgment.",
        claw_id = claw.id,
        role = claw.role,
        agent_profile_id = agent_profile.id,
        memory_scope = claw.memory_scope,
        output_policy = agent_profile.output_policy,
        execution_mode = routing.execution_mode,
        isolation_mode = routing.isolation_mode,
        shared_context = routing.allow_shared_context,
        autonomy_level = routing.autonomy.autonomy_level,
        yolo_mode = routing.autonomy.yolo_mode,
        approval_policy = routing.autonomy.approval_policy,
        steering_enabled = routing.autonomy.steering_enabled,
        decision_learning_enabled = routing.autonomy.decision_learning_enabled,
        critic_enabled = routing.autonomy.critic_enabled,
        max_delegations = routing.autonomy.max_delegations,
        max_iterations = routing.autonomy.max_iterations,
        max_runtime_secs = routing.autonomy.max_runtime_secs,
        workspace = workspace_root.display(),
        category = routing.category.as_deref().unwrap_or("-"),
        task_id = routing.task_id.as_deref().unwrap_or("-"),
        mode = mode,
        steering_notes = steering_notes,
        lesson_hints = lesson_hints,
    )
}

async fn execute_completion(
    provider: Arc<dyn LlmProvider>,
    system_prompt: String,
    user_prompt: String,
) -> Result<CompletionTelemetry> {
    let estimated_input_tokens =
        estimate_tokens(&system_prompt).saturating_add(estimate_tokens(&user_prompt));
    let started_at = Instant::now();
    let response = provider
        .complete(CompletionRequest {
            messages: vec![Message::user(user_prompt)],
            model: Some(provider.model_id().to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.2),
            tools: None,
            system_prompt: Some(system_prompt),
            stream: false,
        })
        .await
        .map_err(anyhow::Error::from)?;
    let duration_ms = started_at.elapsed().as_millis() as u64;
    let estimated_output_tokens = estimate_tokens(&response.message.content);
    Ok(CompletionTelemetry {
        response,
        estimated_input_tokens,
        estimated_output_tokens,
        duration_ms,
    })
}

fn parse_json_payload<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    if let Ok(parsed) = serde_json::from_str::<T>(raw) {
        return Ok(parsed);
    }
    let trimmed = raw.trim();
    if let Some(fenced) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|body| body.strip_suffix("```"))
        && let Ok(parsed) = serde_json::from_str::<T>(fenced.trim())
    {
        return Ok(parsed);
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let candidate = &trimmed[start..=end];
        if let Ok(parsed) = serde_json::from_str::<T>(candidate) {
            return Ok(parsed);
        }
    }
    anyhow::bail!("Failed to parse JSON payload")
}

fn save_run_record(workspace_root: &Path, record: &OrchestrationRunRecord) -> Result<PathBuf> {
    let runs_root = runs_root_for(workspace_root);
    fs::create_dir_all(&runs_root)
        .with_context(|| format!("Failed to create '{}'", runs_root.display()))?;
    let path = runs_root.join(format!(
        "{}-{}.json",
        Utc::now().format("%Y%m%d%H%M%S"),
        record.run_id
    ));
    fs::write(
        &path,
        serde_json::to_vec_pretty(record).context("Failed to encode orchestration run record")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> control::ControlRegistry {
        let mut registry = control::ControlRegistry::default();
        registry.agent_profiles.insert(
            "default".to_string(),
            control::AgentProfileSpec {
                id: "default".to_string(),
                name: None,
                extends: vec![],
                model_profile_id: Some("primary".to_string()),
                thinking_level: None,
                timeout_secs: 90,
                tool_allow: vec![],
                tool_deny: vec![],
                memory_scope: "workspace_shared".to_string(),
                output_policy: "standard".to_string(),
                metadata: serde_json::json!({}),
            },
        );
        registry.agent_profiles.insert(
            "orchestrator".to_string(),
            control::AgentProfileSpec {
                id: "orchestrator".to_string(),
                name: None,
                extends: vec![],
                model_profile_id: Some("primary".to_string()),
                thinking_level: None,
                timeout_secs: 90,
                tool_allow: vec![],
                tool_deny: vec![],
                memory_scope: "workspace_shared".to_string(),
                output_policy: "delegating".to_string(),
                metadata: serde_json::json!({}),
            },
        );
        registry.model_profiles.insert(
            "primary".to_string(),
            control::ModelProfileSpec {
                id: "primary".to_string(),
                provider: "unsupported".to_string(),
                model: "x".to_string(),
                context_window: None,
                max_output_tokens: None,
                supports_tools: true,
                supports_vision: false,
                role_tags: vec![],
                artifact_preferences: vec![],
                fallback_order: vec!["local".to_string()],
                latency_hint: None,
                cost_hint: None,
                reasoning_hint: None,
                metadata: serde_json::json!({}),
            },
        );
        registry.model_profiles.insert(
            "local".to_string(),
            control::ModelProfileSpec {
                id: "local".to_string(),
                provider: "ollama".to_string(),
                model: "llama3.1".to_string(),
                context_window: None,
                max_output_tokens: None,
                supports_tools: true,
                supports_vision: false,
                role_tags: vec![],
                artifact_preferences: vec![],
                fallback_order: vec![],
                latency_hint: None,
                cost_hint: None,
                reasoning_hint: None,
                metadata: serde_json::json!({}),
            },
        );
        registry.claws.insert(
            "main".to_string(),
            control::ClawSpec {
                id: "main".to_string(),
                name: None,
                agent_profile_id: "default".to_string(),
                model_profile_id: "primary".to_string(),
                role: "primary".to_string(),
                memory_scope: "workspace_shared".to_string(),
                task_categories: vec!["general".to_string()],
                tool_allow: vec![],
                tool_deny: vec![],
                enabled: true,
                metadata: serde_json::json!({}),
            },
        );
        registry.claws.insert(
            "orchestrator".to_string(),
            control::ClawSpec {
                id: "orchestrator".to_string(),
                name: None,
                agent_profile_id: "orchestrator".to_string(),
                model_profile_id: "primary".to_string(),
                role: "orchestrator".to_string(),
                memory_scope: "workspace_shared".to_string(),
                task_categories: vec!["routing".to_string()],
                tool_allow: vec![],
                tool_deny: vec![],
                enabled: true,
                metadata: serde_json::json!({}),
            },
        );
        registry.runtime = Some(control::RuntimeModeSpec {
            mode: "orchestrated".to_string(),
            default_claw_id: Some("main".to_string()),
            orchestrator_claw_id: Some("orchestrator".to_string()),
            task_assignments: BTreeMap::from([("task-1".to_string(), "main".to_string())]),
            category_assignments: BTreeMap::from([("code".to_string(), "main".to_string())]),
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            autonomy: control::AutonomyPolicy::default(),
            metadata: serde_json::json!({}),
        });
        registry
    }

    #[test]
    fn resolves_explicit_task_assignment_before_default() {
        let registry = sample_registry();
        let config = AppConfig::default();
        let decision = resolve_routing(
            &registry,
            &config,
            &OrchestrationRequest {
                prompt: "test".to_string(),
                task_id: Some("task-1".to_string()),
                category: Some("code".to_string()),
                claw_id: None,
                mode: "auto".to_string(),
                overrides: OrchestrationRequestOverrides::default(),
            },
        )
        .expect("resolve");
        assert_eq!(decision.selected_claw_id, "main");
        assert_eq!(decision.route_source, "task_assignment");
    }

    #[test]
    fn resolves_orchestrator_when_mode_is_orchestrated() {
        let registry = sample_registry();
        let config = AppConfig::default();
        let decision = resolve_routing(
            &registry,
            &config,
            &OrchestrationRequest {
                prompt: "test".to_string(),
                task_id: None,
                category: None,
                claw_id: None,
                mode: "auto".to_string(),
                overrides: OrchestrationRequestOverrides::default(),
            },
        )
        .expect("resolve");
        assert_eq!(decision.selected_claw_id, "orchestrator");
    }

    #[test]
    fn request_overrides_adjust_model_and_autonomy() {
        let registry = sample_registry();
        let config = AppConfig::default();
        let decision = resolve_routing(
            &registry,
            &config,
            &OrchestrationRequest {
                prompt: "test".to_string(),
                task_id: None,
                category: None,
                claw_id: Some("main".to_string()),
                mode: "auto".to_string(),
                overrides: OrchestrationRequestOverrides {
                    model_profile_id: Some("local".to_string()),
                    worker_model_profile_id: Some("local".to_string()),
                    autonomy_level: Some("managed".to_string()),
                    max_delegations: Some(2),
                    max_iterations: Some(9),
                    max_runtime_secs: Some(45),
                    approval_policy: Some("side_effects".to_string()),
                },
            },
        )
        .expect("resolve");
        assert_eq!(decision.selected_model_profile_id, "local");
        assert_eq!(decision.selected_model.requested_profile_id, "local");
        assert_eq!(decision.autonomy.autonomy_level, "managed");
        assert_eq!(decision.autonomy.max_delegations, 2);
        assert_eq!(decision.autonomy.max_iterations, 9);
        assert_eq!(decision.autonomy.max_runtime_secs, 45);
        assert_eq!(decision.autonomy.approval_policy, "side_effects");
        assert_eq!(
            decision
                .request_overrides
                .worker_model_profile_id
                .as_deref(),
            Some("local")
        );
    }

    #[test]
    fn model_resolution_uses_fallback_order() {
        let registry = sample_registry();
        let config = AppConfig::default();
        let resolved = resolve_model_with_fallback("primary", &registry, &config).expect("model");
        assert_eq!(resolved.decision.selected_profile_id, "local");
        assert_eq!(resolved.decision.fallback_path, vec!["primary", "local"]);
    }

    #[test]
    fn parses_json_from_fenced_block() {
        let parsed = parse_json_payload::<PlannerResponse>(
            "```json\n{\"final_mode\":\"answer_directly\",\"direct_response\":\"ok\"}\n```",
        )
        .expect("json");
        assert_eq!(parsed.final_mode.as_deref(), Some("answer_directly"));
        assert_eq!(parsed.direct_response.as_deref(), Some("ok"));
    }

    #[test]
    fn list_runs_returns_saved_receipts() {
        let root = tempfile::tempdir().unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "direct".to_string(),
            request: OrchestrationRequest::default(),
            routing: RoutingDecision {
                execution_mode: "solo_claw".to_string(),
                route_source: "default_claw".to_string(),
                task_id: None,
                category: None,
                selected_claw_id: "claw-a".to_string(),
                selected_claw_role: "primary".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "primary".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "primary".to_string(),
                    selected_profile_id: "primary".to_string(),
                    provider: "openrouter".to_string(),
                    model: "test".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec![],
                autonomy: control::AutonomyPolicy {
                    autonomy_level: "managed".to_string(),
                    approval_policy: "side_effects".to_string(),
                    ..control::AutonomyPolicy::default()
                },
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides::default(),
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            trace: vec![],
            relationships: vec![],
            reflection_notes: vec![],
            reflection_candidates: vec![],
            supervision: None,
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            transcript: vec![],
            final_output: "ok".to_string(),
            final_claw_id: "claw-a".to_string(),
            final_model_profile_id: "primary".to_string(),
            final_provider: "openrouter".to_string(),
            final_model: "test".to_string(),
            receipt_path: "receipt.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("receipt.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let runs = list_runs(root.path(), 10).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].receipt_id, "receipt.json");
        assert_eq!(runs[0].final_claw_id, "claw-a");
        let loaded = read_run(root.path(), "receipt.json").unwrap();
        assert_eq!(loaded.run_id, "run-1");
    }

    #[test]
    fn read_run_trace_returns_trace_and_relationships() {
        let root = tempfile::tempdir().unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "orchestrated".to_string(),
            request: OrchestrationRequest::default(),
            routing: RoutingDecision {
                execution_mode: "orchestrated".to_string(),
                route_source: "default_claw".to_string(),
                task_id: None,
                category: None,
                selected_claw_id: "claw-a".to_string(),
                selected_claw_role: "orchestrator".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "primary".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "primary".to_string(),
                    selected_profile_id: "primary".to_string(),
                    provider: "openrouter".to_string(),
                    model: "test".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec!["worker-a".to_string()],
                autonomy: control::AutonomyPolicy::default(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides::default(),
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            trace: vec![OrchestrationTraceEntry {
                trace_id: "trace-1".to_string(),
                created_at: "2026-03-19T00:00:01Z".to_string(),
                stage: "planner".to_string(),
                actor_type: "orchestrator".to_string(),
                actor_id: "claw-a".to_string(),
                parent_trace_id: None,
                status: "completed".to_string(),
                claw_id: Some("claw-a".to_string()),
                model_profile_id: Some("primary".to_string()),
                provider: Some("openrouter".to_string()),
                model: Some("test".to_string()),
                note: "planner completed".to_string(),
                input_excerpt: Some("input".to_string()),
                output_excerpt: Some("output".to_string()),
                estimated_input_tokens: Some(42),
                estimated_output_tokens: Some(18),
                duration_ms: Some(125),
            }],
            relationships: vec![OrchestrationRelationship {
                relationship_id: "relationship-1".to_string(),
                parent_run_id: "run-1".to_string(),
                child_id: "worker-a".to_string(),
                child_kind: "worker".to_string(),
                delegation_id: Some("delegation-1".to_string()),
                claw_id: Some("worker-a".to_string()),
                model_profile_id: Some("worker-profile".to_string()),
                stage: "worker_execution".to_string(),
            }],
            reflection_notes: vec![],
            reflection_candidates: vec![],
            supervision: None,
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            transcript: vec![],
            final_output: "ok".to_string(),
            final_claw_id: "claw-a".to_string(),
            final_model_profile_id: "primary".to_string(),
            final_provider: "openrouter".to_string(),
            final_model: "test".to_string(),
            receipt_path: "trace.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("trace.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let payload = read_run_trace(root.path(), "trace.json").unwrap();
        assert_eq!(payload["run_id"], "run-1");
        assert_eq!(payload["trace"][0]["trace_id"], "trace-1");
        assert_eq!(payload["relationships"][0]["child_id"], "worker-a");
    }

    #[test]
    fn read_run_transcript_returns_entries() {
        let root = tempfile::tempdir().unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "orchestrated".to_string(),
            request: OrchestrationRequest::default(),
            routing: RoutingDecision {
                execution_mode: "orchestrated".to_string(),
                route_source: "default_claw".to_string(),
                task_id: None,
                category: None,
                selected_claw_id: "main".to_string(),
                selected_claw_role: "orchestrator".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "primary".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "primary".to_string(),
                    selected_profile_id: "primary".to_string(),
                    provider: "openrouter".to_string(),
                    model: "test".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec!["worker-a".to_string()],
                autonomy: control::AutonomyPolicy::default(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides::default(),
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            trace: vec![],
            relationships: vec![],
            transcript: vec![OrchestrationTranscriptEntry {
                transcript_id: "tx-1".to_string(),
                created_at: "2026-03-19T00:00:01Z".to_string(),
                stage: "planner".to_string(),
                actor_type: "orchestrator".to_string(),
                actor_id: "main".to_string(),
                role: "assistant".to_string(),
                parent_trace_id: None,
                delegation_id: None,
                content: "plan".to_string(),
            }],
            reflection_notes: vec![],
            reflection_candidates: vec![],
            supervision: None,
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            final_output: "ok".to_string(),
            final_claw_id: "main".to_string(),
            final_model_profile_id: "primary".to_string(),
            final_provider: "openrouter".to_string(),
            final_model: "test".to_string(),
            receipt_path: "transcript.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("transcript.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let payload = read_run_transcript(root.path(), "transcript.json").unwrap();
        assert_eq!(payload["transcript"][0]["transcript_id"], "tx-1");
    }

    #[test]
    fn read_run_resources_summarizes_trace_metrics() {
        let root = tempfile::tempdir().unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "orchestrated".to_string(),
            request: OrchestrationRequest::default(),
            routing: RoutingDecision {
                execution_mode: "orchestrated".to_string(),
                route_source: "default_claw".to_string(),
                task_id: None,
                category: None,
                selected_claw_id: "main".to_string(),
                selected_claw_role: "orchestrator".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "primary".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "primary".to_string(),
                    selected_profile_id: "primary".to_string(),
                    provider: "openrouter".to_string(),
                    model: "test".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec![],
                autonomy: control::AutonomyPolicy::default(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides::default(),
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            trace: vec![
                OrchestrationTraceEntry {
                    trace_id: "trace-1".to_string(),
                    created_at: "2026-03-19T00:00:01Z".to_string(),
                    stage: "planner".to_string(),
                    actor_type: "orchestrator".to_string(),
                    actor_id: "main".to_string(),
                    parent_trace_id: None,
                    status: "completed".to_string(),
                    claw_id: Some("main".to_string()),
                    model_profile_id: Some("primary".to_string()),
                    provider: Some("openrouter".to_string()),
                    model: Some("test".to_string()),
                    note: "planner completed".to_string(),
                    input_excerpt: Some("input".to_string()),
                    output_excerpt: Some("output".to_string()),
                    estimated_input_tokens: Some(40),
                    estimated_output_tokens: Some(12),
                    duration_ms: Some(120),
                },
                OrchestrationTraceEntry {
                    trace_id: "trace-2".to_string(),
                    created_at: "2026-03-19T00:00:02Z".to_string(),
                    stage: "worker_execution".to_string(),
                    actor_type: "worker".to_string(),
                    actor_id: "worker-a".to_string(),
                    parent_trace_id: Some("trace-1".to_string()),
                    status: "completed".to_string(),
                    claw_id: Some("worker-a".to_string()),
                    model_profile_id: Some("worker-profile".to_string()),
                    provider: Some("openrouter".to_string()),
                    model: Some("test-worker".to_string()),
                    note: "worker completed".to_string(),
                    input_excerpt: Some("worker input".to_string()),
                    output_excerpt: Some("worker output".to_string()),
                    estimated_input_tokens: Some(24),
                    estimated_output_tokens: Some(10),
                    duration_ms: Some(80),
                },
            ],
            relationships: vec![],
            reflection_notes: vec![],
            reflection_candidates: vec![],
            supervision: None,
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            transcript: vec![],
            final_output: "ok".to_string(),
            final_claw_id: "main".to_string(),
            final_model_profile_id: "primary".to_string(),
            final_provider: "openrouter".to_string(),
            final_model: "test".to_string(),
            receipt_path: "resources.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("resources.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let payload = read_run_resources(root.path(), "resources.json").unwrap();
        assert_eq!(payload["totals"]["trace_count"], 2);
        assert_eq!(payload["totals"]["estimated_input_tokens"], 64);
        assert_eq!(payload["totals"]["estimated_output_tokens"], 22);
        assert_eq!(payload["totals"]["duration_ms"], 200);
        assert_eq!(payload["actors"][0]["actor_id"], "main");
        assert_eq!(payload["actors"][1]["actor_id"], "worker-a");
    }

    #[test]
    fn read_run_supervision_returns_worker_and_delegation_details() {
        let root = tempfile::tempdir().unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-12".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "orchestrated".to_string(),
            request: OrchestrationRequest::default(),
            routing: RoutingDecision {
                execution_mode: "orchestrated".to_string(),
                route_source: "default_claw".to_string(),
                task_id: None,
                category: Some("code".to_string()),
                selected_claw_id: "main".to_string(),
                selected_claw_role: "orchestrator".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "primary".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "primary".to_string(),
                    selected_profile_id: "primary".to_string(),
                    provider: "openrouter".to_string(),
                    model: "test".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec!["worker-a".to_string(), "worker-b".to_string()],
                autonomy: control::AutonomyPolicy {
                    autonomy_level: "managed".to_string(),
                    approval_policy: "side_effects".to_string(),
                    ..control::AutonomyPolicy::default()
                },
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides {
                    model_profile_id: None,
                    worker_model_profile_id: None,
                    autonomy_level: Some("managed".to_string()),
                    max_delegations: Some(2),
                    max_iterations: Some(5),
                    max_runtime_secs: Some(180),
                    approval_policy: Some("side_effects".to_string()),
                },
            },
            delegations: vec![DelegationTask {
                id: "delegation-1".to_string(),
                claw_id: "worker-a".to_string(),
                instruction: "inspect the failing test path".to_string(),
                reason: "worker owns the code area".to_string(),
            }],
            worker_results: vec![WorkerResultEnvelope {
                claw_id: "worker-a".to_string(),
                agent_profile_id: "worker".to_string(),
                model_profile_id: "worker-profile".to_string(),
                provider: "openrouter".to_string(),
                model: "gpt-test".to_string(),
                status: "needs_input".to_string(),
                summary: "needs more context".to_string(),
                full_output: "needs more context".to_string(),
                questions: vec!["which failing test matters most?".to_string()],
                confidence: Some(0.55),
                next_step_recommendation: Some("narrow the failing scope".to_string()),
            }],
            checkpoints: vec![],
            trace: vec![],
            relationships: vec![],
            transcript: vec![],
            reflection_notes: vec!["consider narrowing delegation scope".to_string()],
            reflection_candidates: vec![],
            supervision: Some(SupervisionSummary {
                checkpoint_count: 2,
                worker_count: 1,
                needs_input_count: 1,
                failed_count: 0,
                low_confidence_workers: vec!["worker-a".to_string()],
                reflection_candidate_count: 0,
                escalation_recommended: true,
            }),
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            final_output: "blocked on worker input".to_string(),
            final_claw_id: "main".to_string(),
            final_model_profile_id: "primary".to_string(),
            final_provider: "openrouter".to_string(),
            final_model: "test".to_string(),
            receipt_path: "supervision.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("supervision.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let payload = read_run_supervision(root.path(), "supervision.json").unwrap();
        assert_eq!(payload["route"]["approval_policy"], "side_effects");
        assert_eq!(payload["delegations"][0]["claw_id"], "worker-a");
        assert_eq!(payload["workers"][0]["status"], "needs_input");
        assert_eq!(payload["workers"][0]["question_count"], 1);
        assert_eq!(payload["supervision"]["escalation_recommended"], true);
        assert_eq!(payload["lifecycle"]["state"], "queued");
        assert_eq!(payload["decision_history"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn read_active_run_supervision_includes_recent_events_and_attention_signals() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = ActiveOrchestrationRun {
            run_id: "run-live".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            updated_at: "2026-03-19T00:00:00Z".to_string(),
            started_at: Some("2026-03-19T00:00:01Z".to_string()),
            finished_at: None,
            status: "running".to_string(),
            request: OrchestrationRequest::default(),
            routing: None,
            current_stage: Some("worker_execution".to_string()),
            current_actor_type: Some("worker".to_string()),
            current_actor_id: Some("worker-a".to_string()),
            current_note: Some("waiting for worker output".to_string()),
            lifecycle: SupervisionLifecycleSummary {
                state: "pause_requested".to_string(),
                intervention_required: true,
                escalation_requested: true,
                rollback_requested: false,
                rollback_reference: None,
                decision_count: 0,
                last_decision_at: None,
            },
            pause_requested: true,
            kill_requested: false,
            checkpoint_count: 2,
            trace_count: 3,
            worker_count: 1,
            relationship_count: 1,
            resource_totals: Some(OrchestrationResourceTotals {
                trace_count: 3,
                estimated_input_tokens: 90,
                estimated_output_tokens: 30,
                duration_ms: 450,
            }),
            receipt_id: None,
            receipt_path: None,
            last_error: Some("worker timeout risk".to_string()),
        };
        write_active_run(root.path(), &snapshot).unwrap();

        let monitor = ActiveRunMonitor::new(root.path(), "run-live");
        monitor
            .append_event(
                "worker_execution",
                "worker",
                "worker-a",
                "running",
                "worker is still running",
            )
            .unwrap();

        let payload = read_active_run_supervision(root.path(), "run-live", 10).unwrap();
        assert_eq!(payload["run"]["run_id"], "run-live");
        assert_eq!(payload["recent_events"][0]["actor_id"], "worker-a");
        assert!(
            payload["attention_signals"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some("pause requested by operator"))
        );
        assert_eq!(payload["run"]["lifecycle"]["state"], "pause_requested");
    }

    #[test]
    fn lifecycle_actions_record_decisions_and_rolled_back_state() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = ActiveOrchestrationRun {
            run_id: "run-decision".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            updated_at: "2026-03-19T00:00:00Z".to_string(),
            started_at: Some("2026-03-19T00:00:01Z".to_string()),
            finished_at: None,
            status: "running".to_string(),
            request: OrchestrationRequest::default(),
            routing: None,
            current_stage: Some("worker_execution".to_string()),
            current_actor_type: Some("worker".to_string()),
            current_actor_id: Some("worker-a".to_string()),
            current_note: Some("running".to_string()),
            lifecycle: SupervisionLifecycleSummary {
                state: "running".to_string(),
                ..SupervisionLifecycleSummary::default()
            },
            pause_requested: false,
            kill_requested: false,
            checkpoint_count: 0,
            trace_count: 0,
            worker_count: 0,
            relationship_count: 0,
            resource_totals: None,
            receipt_id: None,
            receipt_path: None,
            last_error: None,
        };
        write_active_run(root.path(), &snapshot).unwrap();

        let escalated = escalate_active_run(
            root.path(),
            "run-decision",
            ActiveRunInterventionRequest {
                requested_by: Some("owner-1".to_string()),
                reason: Some("worker needs approval".to_string()),
                rollback_reference: None,
            },
        )
        .unwrap();
        assert_eq!(escalated.lifecycle.state, "escalated");
        assert!(escalated.lifecycle.intervention_required);

        let rollback = rollback_active_run(
            root.path(),
            "run-decision",
            ActiveRunInterventionRequest {
                requested_by: Some("owner-1".to_string()),
                reason: Some("unsafe path".to_string()),
                rollback_reference: Some("receipt-17.json".to_string()),
            },
        )
        .unwrap();
        assert!(rollback.kill_requested);
        assert!(rollback.lifecycle.rollback_requested);

        let decisions = read_active_run_decisions(root.path(), "run-decision", 10).unwrap();
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].action, "escalate");
        assert_eq!(decisions[1].action, "rollback");
        assert_eq!(
            decisions[1].rollback_reference.as_deref(),
            Some("receipt-17.json")
        );

        let monitor = ActiveRunMonitor::new(root.path(), "run-decision");
        let finalized = monitor
            .finalize_failure("rollback requested by operator")
            .unwrap();
        assert_eq!(finalized.status, "rolled_back");

        let payload = read_active_run_supervision(root.path(), "run-decision", 10).unwrap();
        assert_eq!(payload["run"]["status"], "rolled_back");
        assert_eq!(payload["run"]["lifecycle"]["state"], "rolled_back");
        assert_eq!(payload["decision_history"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn reflection_candidates_capture_failed_worker() {
        let routing = RoutingDecision {
            execution_mode: "orchestrated".to_string(),
            route_source: "orchestrated_default".to_string(),
            task_id: None,
            category: Some("code".to_string()),
            selected_claw_id: "orchestrator".to_string(),
            selected_claw_role: "orchestrator".to_string(),
            selected_agent_profile_id: "orchestrator".to_string(),
            selected_model_profile_id: "primary".to_string(),
            selected_model: ResolvedModelDecision {
                requested_profile_id: "primary".to_string(),
                selected_profile_id: "local".to_string(),
                provider: "ollama".to_string(),
                model: "llama3.1".to_string(),
                fallback_path: vec!["primary".to_string(), "local".to_string()],
                warnings: vec![],
            },
            available_workers: vec!["worker-a".to_string()],
            autonomy: control::AutonomyPolicy::default(),
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            applied_lessons: vec![],
            steering_notes: vec![],
            warnings: vec![],
            request_overrides: OrchestrationRequestOverrides::default(),
        };
        let worker_results = vec![WorkerResultEnvelope {
            claw_id: "worker-a".to_string(),
            agent_profile_id: "default".to_string(),
            model_profile_id: "primary".to_string(),
            provider: "openrouter".to_string(),
            model: "model-a".to_string(),
            status: "failed".to_string(),
            summary: "failed to complete".to_string(),
            full_output: "failed to complete".to_string(),
            questions: vec!["need credentials".to_string()],
            confidence: Some(0.4),
            next_step_recommendation: Some("ask for credentials".to_string()),
        }];
        let candidates = build_reflection_candidates(&routing, &worker_results);
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
    fn promote_reflection_candidate_writes_decision_lesson() {
        let root = tempfile::tempdir().unwrap();
        let control_root = control::control_root_for(root.path());
        control::init(Some(control_root.to_str().unwrap())).unwrap();
        let record = OrchestrationRunRecord {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            mode: "orchestrated".to_string(),
            request: OrchestrationRequest {
                prompt: "test".to_string(),
                task_id: Some("task-1".to_string()),
                category: Some("code".to_string()),
                claw_id: None,
                mode: "auto".to_string(),
                overrides: OrchestrationRequestOverrides::default(),
            },
            routing: RoutingDecision {
                execution_mode: "orchestrated".to_string(),
                route_source: "orchestrated_default".to_string(),
                task_id: Some("task-1".to_string()),
                category: Some("code".to_string()),
                selected_claw_id: "main".to_string(),
                selected_claw_role: "primary".to_string(),
                selected_agent_profile_id: "default".to_string(),
                selected_model_profile_id: "core-groq".to_string(),
                selected_model: ResolvedModelDecision {
                    requested_profile_id: "core-groq".to_string(),
                    selected_profile_id: "core-groq".to_string(),
                    provider: "groq".to_string(),
                    model: "llama-3.3-70b-versatile".to_string(),
                    fallback_path: vec![],
                    warnings: vec![],
                },
                available_workers: vec!["orchestrator".to_string()],
                autonomy: control::AutonomyPolicy::default(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
                request_overrides: OrchestrationRequestOverrides::default(),
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            trace: vec![],
            relationships: vec![],
            reflection_notes: vec![],
            reflection_candidates: vec![ReflectionCandidate {
                kind: "worker_status".to_string(),
                signal: "worker failed".to_string(),
                recommendation: "route a safer model".to_string(),
                rationale: Some("worker returned failed".to_string()),
                confidence: Some(0.8),
                claw_id: Some("main".to_string()),
                model_profile_id: Some("core-groq".to_string()),
                provider: Some("groq".to_string()),
            }],
            supervision: Some(SupervisionSummary {
                checkpoint_count: 0,
                worker_count: 0,
                needs_input_count: 0,
                failed_count: 1,
                low_confidence_workers: vec![],
                reflection_candidate_count: 1,
                escalation_recommended: true,
            }),
            lifecycle: SupervisionLifecycleSummary::default(),
            decision_history: vec![],
            transcript: vec![],
            final_output: "ok".to_string(),
            final_claw_id: "main".to_string(),
            final_model_profile_id: "core-groq".to_string(),
            final_provider: "groq".to_string(),
            final_model: "llama-3.3-70b-versatile".to_string(),
            receipt_path: "test.json".to_string(),
        };
        let path = runs_root_for(root.path()).join("test.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let result = promote_reflection_candidate(
            root.path(),
            "test.json",
            0,
            PromoteReflectionInput::default(),
        )
        .unwrap();
        assert_eq!(result["status"], "ok");
        let registry = control::load_registry(control::control_root_for(root.path())).unwrap();
        assert_eq!(registry.lessons.len(), 1);
        let lesson = registry.lessons.values().next().unwrap();
        assert_eq!(lesson.signal, "worker failed");
        assert_eq!(lesson.recommendation, "route a safer model");
        assert_eq!(lesson.scope.category.as_deref(), Some("code"));
    }

    #[test]
    fn active_run_state_updates_support_pause_resume_and_kill() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = ActiveOrchestrationRun {
            run_id: "run-1".to_string(),
            created_at: "2026-03-19T00:00:00Z".to_string(),
            updated_at: "2026-03-19T00:00:00Z".to_string(),
            started_at: None,
            finished_at: None,
            status: "queued".to_string(),
            request: OrchestrationRequest::default(),
            routing: None,
            current_stage: Some("queued".to_string()),
            current_actor_type: Some("orchestrator".to_string()),
            current_actor_id: Some("main".to_string()),
            current_note: Some("queued".to_string()),
            lifecycle: SupervisionLifecycleSummary::default(),
            pause_requested: false,
            kill_requested: false,
            checkpoint_count: 0,
            trace_count: 0,
            worker_count: 0,
            relationship_count: 0,
            resource_totals: None,
            receipt_id: None,
            receipt_path: None,
            last_error: None,
        };
        write_active_run(root.path(), &snapshot).unwrap();
        let listed = list_active_runs(root.path(), true, 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].run_id, "run-1");

        let paused = pause_active_run(root.path(), "run-1").unwrap();
        assert!(paused.pause_requested);
        let resumed = resume_active_run(root.path(), "run-1").unwrap();
        assert!(!resumed.pause_requested);
        let killed = kill_active_run(root.path(), "run-1").unwrap();
        assert!(killed.kill_requested);
    }
}
