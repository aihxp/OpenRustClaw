//! Multi-model and multi-claw orchestration helpers.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};
use openrustclaw_providers::{
    AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider,
    openrouter::RouteStrategy,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{control, runtime};

pub const DEFAULT_RUNS_DIR: &str = ".claw/control/orchestration-runs";

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
    pub reflection_notes: Vec<String>,
    #[serde(default)]
    pub reflection_candidates: Vec<ReflectionCandidate>,
    #[serde(default)]
    pub supervision: Option<SupervisionSummary>,
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
    pub final_claw_id: String,
    pub final_provider: String,
    pub final_model: String,
    pub receipt_path: String,
}

#[derive(Debug, Deserialize)]
struct PlannerResponse {
    #[serde(default)]
    final_mode: Option<String>,
    #[serde(default)]
    direct_response: Option<String>,
    #[serde(default)]
    delegations: Vec<PlannerDelegation>,
}

#[derive(Debug, Deserialize)]
struct PlannerDelegation {
    claw_id: String,
    instruction: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
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

struct ExecutableModel {
    provider: Arc<dyn LlmProvider>,
    decision: ResolvedModelDecision,
}

pub fn runs_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNS_DIR)
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
    let config = runtime::load_effective_config("config/default.toml", workspace_root)?;
    let control_root = control::control_root_for(workspace_root);
    let registry = control::load_registry(control_root)?;
    control::validate_registry(&registry)?;
    let routing = resolve_routing(&registry, &config, &request)?;
    let runtime_mode = request.mode.to_lowercase();

    let mut record = if runtime_mode == "orchestrated"
        || (runtime_mode == "auto" && routing.execution_mode == "orchestrated")
    {
        run_orchestrated(
            request.clone(),
            &registry,
            &config,
            &routing,
            workspace_root,
        )
        .await?
    } else {
        run_direct(
            request.clone(),
            &registry,
            &config,
            &routing,
            workspace_root,
        )
        .await?
    };
    record.receipt_path = save_run_record(workspace_root, &record)?
        .display()
        .to_string();
    Ok(record)
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
            final_claw_id: record.final_claw_id,
            final_provider: record.final_provider,
            final_model: record.final_model,
            receipt_path: record.receipt_path,
        });
    }

    runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    runs.truncate(limit);
    Ok(runs)
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

pub fn read_run_supervision(workspace_root: &Path, receipt_id: &str) -> Result<serde_json::Value> {
    let run = read_run(workspace_root, receipt_id)?;
    Ok(serde_json::json!({
        "run_id": run.run_id,
        "mode": run.mode,
        "supervision": run.supervision,
        "reflection_notes": run.reflection_notes,
        "reflection_candidates": run.reflection_candidates,
        "checkpoints": run.checkpoints,
    }))
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

    let mut warnings = Vec::new();
    let (claw_id, route_source) = if let Some(claw_id) = &request.claw_id {
        (claw_id.clone(), "explicit_claw".to_string())
    } else if let Some(task_id) = &request.task_id {
        if let Some(claw_id) = runtime.task_assignments.get(task_id) {
            (claw_id.clone(), "task_assignment".to_string())
        } else if let Some(category) = &request.category {
            if let Some(claw_id) = runtime.category_assignments.get(category) {
                (claw_id.clone(), "category_assignment".to_string())
            } else if runtime.mode == "orchestrated" {
                (
                    runtime
                        .orchestrator_claw_id
                        .clone()
                        .or_else(|| runtime.default_claw_id.clone())
                        .context("No orchestrator/default claw configured")?,
                    "orchestrated_default".to_string(),
                )
            } else {
                (
                    runtime
                        .default_claw_id
                        .clone()
                        .or_else(|| first_enabled_claw_id(registry))
                        .context("No default claw configured")?,
                    "default_claw".to_string(),
                )
            }
        } else if runtime.mode == "orchestrated" {
            (
                runtime
                    .orchestrator_claw_id
                    .clone()
                    .or_else(|| runtime.default_claw_id.clone())
                    .context("No orchestrator/default claw configured")?,
                "orchestrated_default".to_string(),
            )
        } else {
            (
                runtime
                    .default_claw_id
                    .clone()
                    .or_else(|| first_enabled_claw_id(registry))
                    .context("No default claw configured")?,
                "default_claw".to_string(),
            )
        }
    } else if let Some(category) = &request.category {
        if let Some(claw_id) = runtime.category_assignments.get(category) {
            (claw_id.clone(), "category_assignment".to_string())
        } else if runtime.mode == "orchestrated" {
            (
                runtime
                    .orchestrator_claw_id
                    .clone()
                    .or_else(|| runtime.default_claw_id.clone())
                    .context("No orchestrator/default claw configured")?,
                "orchestrated_default".to_string(),
            )
        } else {
            (
                runtime
                    .default_claw_id
                    .clone()
                    .or_else(|| first_enabled_claw_id(registry))
                    .context("No default claw configured")?,
                "default_claw".to_string(),
            )
        }
    } else if runtime.mode == "orchestrated" && request.mode != "direct" {
        (
            runtime
                .orchestrator_claw_id
                .clone()
                .or_else(|| runtime.default_claw_id.clone())
                .context("No orchestrator/default claw configured")?,
            "orchestrated_default".to_string(),
        )
    } else {
        (
            runtime
                .default_claw_id
                .clone()
                .or_else(|| first_enabled_claw_id(registry))
                .context("No default claw configured")?,
            "default_claw".to_string(),
        )
    };

    let claw = registry
        .claws
        .get(&claw_id)
        .with_context(|| format!("Unknown claw '{}'", claw_id))?;
    let model = resolve_model_with_fallback(&claw.model_profile_id, registry, config)?;
    warnings.extend(model.decision.warnings.clone());
    let applied_lessons = matching_lessons(
        registry,
        request,
        claw,
        &runtime,
        &model.decision.selected_profile_id,
        &model.decision.provider,
    );
    let steering_notes = build_steering_notes(&runtime.autonomy, &applied_lessons);
    let available_workers = registry
        .claws
        .values()
        .filter(|candidate| candidate.enabled && candidate.id != claw.id)
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();

    Ok(RoutingDecision {
        execution_mode: runtime.mode,
        route_source,
        task_id: request.task_id.clone(),
        category: request.category.clone(),
        selected_claw_id: claw.id.clone(),
        selected_claw_role: claw.role.clone(),
        selected_agent_profile_id: claw.agent_profile_id.clone(),
        selected_model_profile_id: claw.model_profile_id.clone(),
        selected_model: model.decision,
        available_workers,
        autonomy: runtime.autonomy,
        allow_shared_context: runtime.allow_shared_context,
        isolation_mode: runtime.isolation_mode,
        applied_lessons,
        steering_notes,
        warnings,
    })
}

fn first_enabled_claw_id(registry: &control::ControlRegistry) -> Option<String> {
    registry
        .claws
        .values()
        .find(|claw| claw.enabled)
        .map(|claw| claw.id.clone())
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

fn excerpt(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
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
    let mut candidates = Vec::new();
    if routing.selected_model.fallback_path.len() > 1 {
        candidates.push(ReflectionCandidate {
            kind: "fallback_path".to_string(),
            signal: format!(
                "model profile '{}' required fallback path {}",
                routing.selected_model.requested_profile_id,
                routing.selected_model.fallback_path.join(" -> ")
            ),
            recommendation:
                "preflight this provider/model lane or promote a healthier fallback for this route"
                    .to_string(),
            rationale: Some(
                "The primary model profile was unavailable or unsupported at run time.".to_string(),
            ),
            confidence: Some(0.82),
            claw_id: Some(routing.selected_claw_id.clone()),
            model_profile_id: Some(routing.selected_model_profile_id.clone()),
            provider: Some(routing.selected_model.provider.clone()),
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
                signal: format!("worker '{}' returned {} open questions", worker.claw_id, worker.questions.len()),
                recommendation: "surface blockers explicitly or add a follow-up worker turn instead of treating the run as fully complete".to_string(),
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

fn summarize_supervision(
    checkpoints: &[OrchestrationCheckpoint],
    worker_results: &[WorkerResultEnvelope],
    reflection_candidates: &[ReflectionCandidate],
) -> SupervisionSummary {
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
    SupervisionSummary {
        checkpoint_count: checkpoints.len(),
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
) -> Result<OrchestrationRunRecord> {
    let claw = registry
        .claws
        .get(&routing.selected_claw_id)
        .with_context(|| format!("Unknown claw '{}'", routing.selected_claw_id))?;
    let executable = resolve_model_with_fallback(&claw.model_profile_id, registry, config)?;
    let agent_profile = registry
        .agent_profiles
        .get(&claw.agent_profile_id)
        .with_context(|| format!("Unknown agent profile '{}'", claw.agent_profile_id))?;
    let prompt = execute_completion(
        executable.provider.clone(),
        build_claw_system_prompt(
            claw,
            agent_profile,
            routing,
            workspace_root,
            "direct_response",
        ),
        request.prompt.clone(),
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
        Some(excerpt(&prompt.message.content, 240)),
    )];
    let reflection_candidates = build_reflection_candidates(routing, &[]);
    let supervision = summarize_supervision(&checkpoints, &[], &reflection_candidates);

    Ok(OrchestrationRunRecord {
        run_id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        mode: "direct".to_string(),
        request,
        routing: routing.clone(),
        delegations: Vec::new(),
        worker_results: Vec::new(),
        checkpoints,
        reflection_notes: routing.steering_notes.clone(),
        reflection_candidates,
        supervision: Some(supervision),
        final_output: prompt.message.content,
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
) -> Result<OrchestrationRunRecord> {
    let orchestrator = registry
        .claws
        .get(&routing.selected_claw_id)
        .with_context(|| format!("Unknown claw '{}'", routing.selected_claw_id))?;
    let orchestrator_agent = registry
        .agent_profiles
        .get(&orchestrator.agent_profile_id)
        .with_context(|| format!("Unknown agent profile '{}'", orchestrator.agent_profile_id))?;
    let orchestrator_model =
        resolve_model_with_fallback(&orchestrator.model_profile_id, registry, config)?;
    let worker_candidates = registry
        .claws
        .values()
        .filter(|claw| claw.enabled && claw.id != orchestrator.id)
        .collect::<Vec<_>>();
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

    if worker_candidates.is_empty() {
        return run_direct(request, registry, config, routing, workspace_root).await;
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
    let planner_response = execute_completion(
        orchestrator_model.provider.clone(),
        planner_system,
        planner_user,
    )
    .await?;
    checkpoints.push(make_checkpoint(
        "planner",
        "completed",
        "orchestrator planner completed",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        Some(excerpt(&request.prompt, 240)),
        Some(excerpt(&planner_response.message.content, 240)),
    ));
    let plan = parse_json_payload::<PlannerResponse>(&planner_response.message.content).unwrap_or(
        PlannerResponse {
            final_mode: Some("answer_directly".to_string()),
            direct_response: Some(planner_response.message.content.clone()),
            delegations: Vec::new(),
        },
    );

    if matches!(plan.final_mode.as_deref(), Some("answer_directly")) || plan.delegations.is_empty()
    {
        let reflection_candidates = build_reflection_candidates(routing, &[]);
        let supervision = summarize_supervision(&checkpoints, &[], &reflection_candidates);
        return Ok(OrchestrationRunRecord {
            run_id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            mode: "orchestrated".to_string(),
            request,
            routing: routing.clone(),
            delegations: Vec::new(),
            worker_results: Vec::new(),
            checkpoints,
            reflection_notes: routing.steering_notes.clone(),
            reflection_candidates,
            supervision: Some(supervision),
            final_output: plan
                .direct_response
                .unwrap_or(planner_response.message.content),
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
        let worker_model = resolve_model_with_fallback(&worker.model_profile_id, registry, config)?;

        delegations.push(DelegationTask {
            id: Uuid::new_v4().to_string(),
            claw_id: worker.id.clone(),
            instruction: item.instruction.clone(),
            reason: item.reason.clone(),
        });

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
            "Original request:\n{}\n\nAssigned subtask:\n{}\n\nReason for assignment:\n{}",
            request.prompt, item.instruction, item.reason
        );
        let worker_response =
            execute_completion(worker_model.provider.clone(), worker_system, worker_user).await?;
        let parsed = parse_json_payload::<WorkerEnvelopeResponse>(&worker_response.message.content)
            .unwrap_or(WorkerEnvelopeResponse {
                status: "completed".to_string(),
                summary: worker_response.message.content.chars().take(240).collect(),
                full_output: Some(worker_response.message.content.clone()),
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
            Some(excerpt(&item.instruction, 240)),
            Some(excerpt(&worker_response.message.content, 240)),
        ));
        worker_results.push(WorkerResultEnvelope {
            claw_id: worker.id.clone(),
            agent_profile_id: worker.agent_profile_id.clone(),
            model_profile_id: worker_model.decision.selected_profile_id.clone(),
            provider: worker_model.decision.provider.clone(),
            model: worker_model.decision.model.clone(),
            status: parsed.status,
            summary: parsed.summary,
            full_output: parsed
                .full_output
                .unwrap_or_else(|| worker_response.message.content.clone()),
            questions: parsed.questions,
            confidence: parsed.confidence,
            next_step_recommendation: parsed.next_step_recommendation,
        });
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
    let synthesis_input_excerpt = excerpt(&synthesis_user, 240);
    let final_response = execute_completion(
        orchestrator_model.provider.clone(),
        synthesis_system,
        synthesis_user,
    )
    .await?;
    checkpoints.push(make_checkpoint(
        "synthesis",
        "completed",
        "orchestrator synthesized final response",
        Some(orchestrator.id.as_str()),
        Some(orchestrator_model.decision.selected_profile_id.as_str()),
        Some(orchestrator_model.decision.provider.as_str()),
        Some(orchestrator_model.decision.model.as_str()),
        Some(synthesis_input_excerpt),
        Some(excerpt(&final_response.message.content, 240)),
    ));
    let reflection_candidates = build_reflection_candidates(routing, &worker_results);
    let mut reflection_notes = routing.steering_notes.clone();
    if !reflection_candidates.is_empty() {
        reflection_notes.push(format!(
            "{} reflection candidate(s) generated for operator review",
            reflection_candidates.len()
        ));
    }
    let supervision = summarize_supervision(&checkpoints, &worker_results, &reflection_candidates);

    Ok(OrchestrationRunRecord {
        run_id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        mode: "orchestrated".to_string(),
        request,
        routing: routing.clone(),
        delegations,
        worker_results,
        checkpoints,
        reflection_notes,
        reflection_candidates,
        supervision: Some(supervision),
        final_output: final_response.message.content,
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
) -> Result<openrustclaw_core::types::CompletionResponse> {
    provider
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
        .map_err(Into::into)
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
            },
        )
        .expect("resolve");
        assert_eq!(decision.selected_claw_id, "orchestrator");
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
                autonomy: control::AutonomyPolicy::default(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                applied_lessons: vec![],
                steering_notes: vec![],
                warnings: vec![],
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
            reflection_notes: vec![],
            reflection_candidates: vec![],
            supervision: None,
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
            },
            delegations: vec![],
            worker_results: vec![],
            checkpoints: vec![],
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
}
