//! File-backed control-plane registry for agent/model/claw profiles and runtime mode.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use openrustclaw_app::agent_backend_catalog::AgentBackendCatalogService;
use openrustclaw_app::agent_backend_control::AgentBackendControlService;
use openrustclaw_app::agent_fabric_registry::{
    AgentFabricAdvertisement, AgentFabricRegistry, AgentFabricRegistryService,
    EnrollRemoteHostRequest, FabricRouteSignal, TrustedRemoteHostRecord,
};
use openrustclaw_app::agent_route_policy::{
    AgentRoutePolicyService, DelegatedRouteDecision, DelegatedRouteRequest,
    RemoteRouteExecutionEnvelope,
};
use openrustclaw_app::autonomy_lessons_control::AutonomyLessonRequest;
use openrustclaw_app::control_registry as app_control_registry;
use openrustclaw_app::learning_review::{LearningReviewService, LearningReviewSource};
use openrustclaw_app::skill_proposals::{SkillProposalService, SkillProposalSource};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::{Error as CoreError, Result as CoreResult};
use openrustclaw_core::types::{
    LearningCandidate, LearningCandidateCreateRequest, LearningCandidateImpact,
    LearningCandidatePromotionRequest, LearningCandidateQuarantineRequest,
    LearningCandidateReviewAction, LearningCandidateReviewRequest,
    LearningCandidateRollbackRequest, LearningCandidateSourceKind, LearningCandidateStatus,
    SkillProposal, SkillProposalCreateRequest, SkillProposalInstallRequest,
    SkillProposalQuarantineRequest, SkillProposalReviewAction, SkillProposalReviewRequest,
    SkillProposalRollbackRequest, SkillProposalSourceKind, SkillProposalStatus,
    SkillProposalVerificationReport, SkillProposalVerificationStatus, SkillProposalVerifyRequest,
    SkillSource,
};
use openrustclaw_db::{SqliteLearningStore, SqliteSkillProposalStore, init_pool, run_migrations};
use openrustclaw_skills::{CompiledSkillStatus, compile_skill_to_dir, remove_compiled_artifact};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use walkdir::WalkDir;

use super::runtime;

pub const DEFAULT_CONTROL_DIR: &str = ".claw/control";

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_timeout_secs() -> u64 {
    90
}

fn default_memory_scope() -> String {
    "workspace_shared".to_string()
}

fn default_output_policy() -> String {
    "standard".to_string()
}

fn default_claw_role() -> String {
    "worker".to_string()
}

fn default_runtime_mode() -> String {
    "solo_claw".to_string()
}

fn default_isolation_mode() -> String {
    "strict".to_string()
}

fn default_autonomy_level() -> String {
    "managed".to_string()
}

fn default_approval_policy() -> String {
    "side_effects".to_string()
}

fn default_steering_enabled() -> bool {
    true
}

fn default_decision_learning_enabled() -> bool {
    true
}

fn default_critic_enabled() -> bool {
    true
}

fn default_max_delegations() -> usize {
    4
}

fn default_max_iterations() -> usize {
    8
}

fn default_max_runtime_secs() -> u64 {
    600
}

fn default_max_lesson_hints() -> usize {
    5
}

fn default_confidence() -> f32 {
    0.7
}

fn default_lesson_source() -> String {
    "operator".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub profile: AgentProfileSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileSpec {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub extends: Vec<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub thinking_level: Option<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub tool_allow: Vec<String>,
    #[serde(default)]
    pub tool_deny: Vec<String>,
    #[serde(default = "default_memory_scope")]
    pub memory_scope: String,
    #[serde(default = "default_output_policy")]
    pub output_policy: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfileManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub model: ModelProfileSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfileSpec {
    pub id: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub context_window: Option<usize>,
    #[serde(default)]
    pub max_output_tokens: Option<usize>,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub role_tags: Vec<String>,
    #[serde(default)]
    pub artifact_preferences: Vec<String>,
    #[serde(default)]
    pub fallback_order: Vec<String>,
    #[serde(default)]
    pub latency_hint: Option<String>,
    #[serde(default)]
    pub cost_hint: Option<String>,
    #[serde(default)]
    pub reasoning_hint: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub claw: ClawSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawSpec {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub agent_profile_id: String,
    pub model_profile_id: String,
    #[serde(default = "default_claw_role")]
    pub role: String,
    #[serde(default = "default_memory_scope")]
    pub memory_scope: String,
    #[serde(default)]
    pub task_categories: Vec<String>,
    #[serde(default)]
    pub tool_allow: Vec<String>,
    #[serde(default)]
    pub tool_deny: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModeManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub runtime: RuntimeModeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModeSpec {
    #[serde(default = "default_runtime_mode")]
    pub mode: String,
    #[serde(default)]
    pub default_claw_id: Option<String>,
    #[serde(default)]
    pub orchestrator_claw_id: Option<String>,
    #[serde(default)]
    pub task_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub category_assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub allow_shared_context: bool,
    #[serde(default = "default_isolation_mode")]
    pub isolation_mode: String,
    #[serde(default)]
    pub autonomy: AutonomyPolicy,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyPolicy {
    #[serde(default = "default_autonomy_level")]
    pub autonomy_level: String,
    #[serde(default)]
    pub yolo_mode: bool,
    #[serde(default = "default_steering_enabled")]
    pub steering_enabled: bool,
    #[serde(default = "default_decision_learning_enabled")]
    pub decision_learning_enabled: bool,
    #[serde(default = "default_critic_enabled")]
    pub critic_enabled: bool,
    #[serde(default = "default_max_delegations")]
    pub max_delegations: usize,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_max_runtime_secs")]
    pub max_runtime_secs: u64,
    #[serde(default = "default_max_lesson_hints")]
    pub max_lesson_hints: usize,
    #[serde(default = "default_approval_policy")]
    pub approval_policy: String,
}

impl Default for AutonomyPolicy {
    fn default() -> Self {
        Self {
            autonomy_level: default_autonomy_level(),
            yolo_mode: false,
            steering_enabled: default_steering_enabled(),
            decision_learning_enabled: default_decision_learning_enabled(),
            critic_enabled: default_critic_enabled(),
            max_delegations: default_max_delegations(),
            max_iterations: default_max_iterations(),
            max_runtime_secs: default_max_runtime_secs(),
            max_lesson_hints: default_max_lesson_hints(),
            approval_policy: default_approval_policy(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLessonManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub lesson: DecisionLessonSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionLessonScope {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub autonomy_level: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLessonSpec {
    pub id: String,
    #[serde(default = "default_true")]
    pub active: bool,
    pub signal: String,
    pub recommendation: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default = "default_lesson_source")]
    pub source: String,
    #[serde(default)]
    pub scope: DecisionLessonScope,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Default)]
pub struct ControlRegistry {
    pub agent_profiles: BTreeMap<String, AgentProfileSpec>,
    pub model_profiles: BTreeMap<String, ModelProfileSpec>,
    pub claws: BTreeMap<String, ClawSpec>,
    pub lessons: BTreeMap<String, DecisionLessonSpec>,
    pub runtime: Option<RuntimeModeSpec>,
}

fn app_agent_profile(profile: &AgentProfileSpec) -> app_control_registry::AgentProfileSpec {
    app_control_registry::AgentProfileSpec {
        id: profile.id.clone(),
        extends: profile.extends.clone(),
        model_profile_id: profile.model_profile_id.clone(),
        memory_scope: profile.memory_scope.clone(),
        output_policy: profile.output_policy.clone(),
    }
}

fn app_model_profile(profile: &ModelProfileSpec) -> app_control_registry::ModelProfileSpec {
    app_control_registry::ModelProfileSpec {
        id: profile.id.clone(),
        provider: profile.provider.clone(),
        model: profile.model.clone(),
        role_tags: profile.role_tags.clone(),
        artifact_preferences: profile.artifact_preferences.clone(),
        fallback_order: profile.fallback_order.clone(),
    }
}

fn app_claw_spec(claw: &ClawSpec) -> app_control_registry::ClawSpec {
    app_control_registry::ClawSpec {
        id: claw.id.clone(),
        name: claw.name.clone(),
        role: claw.role.clone(),
        agent_profile_id: claw.agent_profile_id.clone(),
        model_profile_id: claw.model_profile_id.clone(),
        memory_scope: claw.memory_scope.clone(),
        task_categories: claw.task_categories.clone(),
        enabled: claw.enabled,
    }
}

fn app_autonomy_policy(policy: &AutonomyPolicy) -> app_control_registry::AutonomyPolicy {
    app_control_registry::AutonomyPolicy {
        autonomy_level: policy.autonomy_level.clone(),
        yolo_mode: policy.yolo_mode,
        steering_enabled: policy.steering_enabled,
        decision_learning_enabled: policy.decision_learning_enabled,
        critic_enabled: policy.critic_enabled,
        max_delegations: policy.max_delegations,
        max_iterations: policy.max_iterations,
        max_runtime_secs: policy.max_runtime_secs,
        max_lesson_hints: policy.max_lesson_hints,
        approval_policy: policy.approval_policy.clone(),
    }
}

fn app_runtime_spec(runtime: &RuntimeModeSpec) -> app_control_registry::RuntimeModeSpec {
    app_control_registry::RuntimeModeSpec {
        mode: runtime.mode.clone(),
        default_claw_id: runtime.default_claw_id.clone(),
        orchestrator_claw_id: runtime.orchestrator_claw_id.clone(),
        task_assignments: runtime
            .task_assignments
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        category_assignments: runtime
            .category_assignments
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        allow_shared_context: runtime.allow_shared_context,
        isolation_mode: runtime.isolation_mode.clone(),
        autonomy: app_autonomy_policy(&runtime.autonomy),
    }
}

fn app_lesson_scope(scope: &DecisionLessonScope) -> app_control_registry::DecisionLessonScope {
    app_control_registry::DecisionLessonScope {
        category: scope.category.clone(),
        claw_id: scope.claw_id.clone(),
        model_profile_id: scope.model_profile_id.clone(),
        provider: scope.provider.clone(),
        autonomy_level: scope.autonomy_level.clone(),
        execution_mode: scope.execution_mode.clone(),
    }
}

fn app_lesson(lesson: &DecisionLessonSpec) -> app_control_registry::DecisionLessonSpec {
    app_control_registry::DecisionLessonSpec {
        id: lesson.id.clone(),
        active: lesson.active,
        signal: lesson.signal.clone(),
        recommendation: lesson.recommendation.clone(),
        confidence: lesson.confidence,
        source: lesson.source.clone(),
        scope: app_lesson_scope(&lesson.scope),
    }
}

fn app_registry_snapshot(
    registry: &ControlRegistry,
) -> app_control_registry::ControlRegistrySnapshot {
    app_control_registry::ControlRegistrySnapshot {
        agent_profiles: registry
            .agent_profiles
            .values()
            .map(app_agent_profile)
            .collect(),
        model_profiles: registry
            .model_profiles
            .values()
            .map(app_model_profile)
            .collect(),
        claws: registry.claws.values().map(app_claw_spec).collect(),
        lessons: registry.lessons.values().map(app_lesson).collect(),
        runtime: registry.runtime.as_ref().map(app_runtime_spec),
    }
}

pub fn control_root_for(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(DEFAULT_CONTROL_DIR)
}

fn agents_dir(root: &Path) -> PathBuf {
    root.join("agents")
}

fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

fn claws_dir(root: &Path) -> PathBuf {
    root.join("claws")
}

fn lessons_dir(root: &Path) -> PathBuf {
    root.join("lessons")
}

fn runtime_path(root: &Path) -> PathBuf {
    root.join("runtime.yaml")
}

fn runtime_artifact_path(root: &Path) -> PathBuf {
    root.join("CLAW_RUNTIME.md")
}

fn agent_fabric_path(root: &Path) -> PathBuf {
    root.join("agent-fabric.json")
}

fn agent_route_receipts_path(root: &Path) -> PathBuf {
    root.join("agent-route-decisions.jsonl")
}

fn skill_proposals_dir(root: &Path) -> PathBuf {
    root.join("skill-proposals")
}

pub fn resolve_root(root: Option<&str>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to determine current workspace root")?;
    Ok(match root {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        }
        None => control_root_for(cwd),
    })
}

pub fn init(root: Option<&str>) -> Result<()> {
    let root = resolve_root(root)?;
    fs::create_dir_all(agents_dir(&root))
        .with_context(|| format!("Failed to create '{}'", agents_dir(&root).display()))?;
    fs::create_dir_all(models_dir(&root))
        .with_context(|| format!("Failed to create '{}'", models_dir(&root).display()))?;
    fs::create_dir_all(claws_dir(&root))
        .with_context(|| format!("Failed to create '{}'", claws_dir(&root).display()))?;
    fs::create_dir_all(lessons_dir(&root))
        .with_context(|| format!("Failed to create '{}'", lessons_dir(&root).display()))?;

    write_if_missing(
        &agents_dir(&root).join("default.yaml"),
        &serde_yaml::to_string(&AgentProfileManifest {
            version: 1,
            profile: AgentProfileSpec {
                id: "default".to_string(),
                name: Some("Default Operator".to_string()),
                extends: vec![],
                model_profile_id: Some("core-groq".to_string()),
                thinking_level: Some("balanced".to_string()),
                timeout_secs: 90,
                tool_allow: vec![],
                tool_deny: vec![],
                memory_scope: "workspace_shared".to_string(),
                output_policy: "standard".to_string(),
                metadata: serde_json::json!({
                    "purpose": "default interactive claw"
                }),
            },
        })?,
    )?;
    write_if_missing(
        &agents_dir(&root).join("orchestrator.yaml"),
        &serde_yaml::to_string(&AgentProfileManifest {
            version: 1,
            profile: AgentProfileSpec {
                id: "orchestrator".to_string(),
                name: Some("Quarterback".to_string()),
                extends: vec!["default".to_string()],
                model_profile_id: Some("control-openrouter".to_string()),
                thinking_level: Some("planner".to_string()),
                timeout_secs: 120,
                tool_allow: vec!["memory_search".to_string(), "session_list".to_string()],
                tool_deny: vec![],
                memory_scope: "workspace_shared".to_string(),
                output_policy: "delegating".to_string(),
                metadata: serde_json::json!({
                    "purpose": "quarterback/orchestrator claw"
                }),
            },
        })?,
    )?;

    write_if_missing(
        &models_dir(&root).join("core-groq.yaml"),
        &serde_yaml::to_string(&ModelProfileManifest {
            version: 1,
            model: ModelProfileSpec {
                id: "core-groq".to_string(),
                provider: "groq".to_string(),
                model: "llama-3.3-70b-versatile".to_string(),
                context_window: Some(131_072),
                max_output_tokens: Some(4096),
                supports_tools: true,
                supports_vision: false,
                role_tags: vec!["core_model".to_string(), "low_latency".to_string()],
                artifact_preferences: vec!["AGENTS.md".to_string(), "AI.md".to_string()],
                fallback_order: vec!["control-openrouter".to_string(), "local-ollama".to_string()],
                latency_hint: Some("low".to_string()),
                cost_hint: Some("low".to_string()),
                reasoning_hint: Some("balanced".to_string()),
                metadata: serde_json::json!({
                    "recommended_role": "core_model",
                    "byok": true
                }),
            },
        })?,
    )?;
    write_if_missing(
        &models_dir(&root).join("control-openrouter.yaml"),
        &serde_yaml::to_string(&ModelProfileManifest {
            version: 1,
            model: ModelProfileSpec {
                id: "control-openrouter".to_string(),
                provider: "openrouter".to_string(),
                model: "openrouter/auto".to_string(),
                context_window: Some(128_000),
                max_output_tokens: Some(4096),
                supports_tools: true,
                supports_vision: true,
                role_tags: vec!["control_plane_model".to_string(), "fallback".to_string()],
                artifact_preferences: vec![
                    "AGENTS.md".to_string(),
                    ".github/copilot-instructions.md".to_string(),
                ],
                fallback_order: vec!["local-ollama".to_string()],
                latency_hint: Some("variable".to_string()),
                cost_hint: Some("free_or_low_cost".to_string()),
                reasoning_hint: Some("broad_compatibility".to_string()),
                metadata: serde_json::json!({
                    "recommended_role": "control_plane_model",
                    "byok": true
                }),
            },
        })?,
    )?;
    write_if_missing(
        &models_dir(&root).join("local-ollama.yaml"),
        &serde_yaml::to_string(&ModelProfileManifest {
            version: 1,
            model: ModelProfileSpec {
                id: "local-ollama".to_string(),
                provider: "ollama".to_string(),
                model: "llama3.1".to_string(),
                context_window: Some(128_000),
                max_output_tokens: Some(4096),
                supports_tools: true,
                supports_vision: false,
                role_tags: vec!["offline_fallback".to_string()],
                artifact_preferences: vec!["Modelfile".to_string(), "AGENTS.md".to_string()],
                fallback_order: vec![],
                latency_hint: Some("local".to_string()),
                cost_hint: Some("free".to_string()),
                reasoning_hint: Some("offline".to_string()),
                metadata: serde_json::json!({
                    "recommended_role": "offline_fallback",
                    "byok": false
                }),
            },
        })?,
    )?;
    write_detected_delegated_model_profiles(&root)?;

    write_if_missing(
        &claws_dir(&root).join("main.yaml"),
        &serde_yaml::to_string(&ClawManifest {
            version: 1,
            claw: ClawSpec {
                id: "main".to_string(),
                name: Some("Main Claw".to_string()),
                agent_profile_id: "default".to_string(),
                model_profile_id: "core-groq".to_string(),
                role: "primary".to_string(),
                memory_scope: "workspace_shared".to_string(),
                task_categories: vec!["general".to_string()],
                tool_allow: vec![],
                tool_deny: vec![],
                enabled: true,
                metadata: serde_json::json!({}),
            },
        })?,
    )?;
    write_if_missing(
        &claws_dir(&root).join("orchestrator.yaml"),
        &serde_yaml::to_string(&ClawManifest {
            version: 1,
            claw: ClawSpec {
                id: "orchestrator".to_string(),
                name: Some("Orchestrator Claw".to_string()),
                agent_profile_id: "orchestrator".to_string(),
                model_profile_id: "control-openrouter".to_string(),
                role: "orchestrator".to_string(),
                memory_scope: "workspace_shared".to_string(),
                task_categories: vec!["routing".to_string(), "planning".to_string()],
                tool_allow: vec![],
                tool_deny: vec![],
                enabled: true,
                metadata: serde_json::json!({}),
            },
        })?,
    )?;

    if !runtime_path(&root).exists() {
        let runtime = RuntimeModeManifest {
            version: 1,
            runtime: RuntimeModeSpec {
                mode: "solo_claw".to_string(),
                default_claw_id: Some("main".to_string()),
                orchestrator_claw_id: None,
                task_assignments: BTreeMap::new(),
                category_assignments: BTreeMap::new(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                autonomy: AutonomyPolicy::default(),
                metadata: serde_json::json!({}),
            },
        };
        write_yaml(&runtime_path(&root), &runtime)?;
    }

    sync_runtime_artifact(&root)?;
    println!("Initialized control registry at {}", root.display());
    Ok(())
}

fn write_detected_delegated_model_profiles(root: &Path) -> Result<()> {
    let catalog = AgentBackendCatalogService::new().discover();
    let control = AgentBackendControlService::new();
    for contract in control
        .contracts_from_catalog(&catalog)
        .into_iter()
        .filter(|contract| contract.execution_eligible)
    {
        let profile_id = format!("delegated-{}", contract.backend_id);
        write_if_missing(
            &models_dir(root).join(format!("{profile_id}.yaml")),
            &serde_yaml::to_string(&ModelProfileManifest {
                version: 1,
                model: ModelProfileSpec {
                    id: profile_id,
                    provider: contract.backend_id.clone(),
                    model: "vendor-managed".to_string(),
                    context_window: None,
                    max_output_tokens: Some(4096),
                    supports_tools: false,
                    supports_vision: false,
                    role_tags: vec![
                        "delegated_backend".to_string(),
                        "vendor_managed".to_string(),
                    ],
                    artifact_preferences: vec!["AGENTS.md".to_string()],
                    fallback_order: vec![
                        "control-openrouter".to_string(),
                        "local-ollama".to_string(),
                    ],
                    latency_hint: Some("local_cli".to_string()),
                    cost_hint: Some("vendor_managed".to_string()),
                    reasoning_hint: Some("delegated".to_string()),
                    metadata: serde_json::json!({
                        "delegated_backend_id": contract.backend_id,
                        "detected_local_agent": true,
                        "vendor_managed_model_selection": true,
                        "display_name": contract.display_name,
                    }),
                },
            })?,
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct FabricRegistryReport {
    pub manifest_path: String,
    pub local_advertisement: AgentFabricAdvertisement,
    pub registry: AgentFabricRegistry,
    pub route_signals: Vec<FabricRouteSignal>,
}

pub fn workspace_root_from_control_root(root: &Path) -> PathBuf {
    root.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| root.to_path_buf()))
}

fn load_agent_fabric_registry(root: &Path) -> Result<AgentFabricRegistry> {
    let path = agent_fabric_path(root);
    if !path.exists() {
        return Ok(AgentFabricRegistry::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Failed to parse '{}'", path.display()))
}

fn save_agent_fabric_registry(root: &Path, registry: &AgentFabricRegistry) -> Result<()> {
    let path = agent_fabric_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(registry)?)
        .with_context(|| format!("Failed to write '{}'", path.display()))
}

pub fn fabric_export(root: Option<&str>) -> Result<AgentFabricAdvertisement> {
    let root = resolve_root(root)?;
    let workspace_root = workspace_root_from_control_root(&root);
    let host_label = workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("local")
        .to_string();
    let catalog = AgentBackendCatalogService::new().discover();
    let contracts = AgentBackendControlService::new().contracts_from_catalog(&catalog);
    Ok(
        AgentFabricRegistryService::new().export_local_advertisement(
            "local",
            &host_label,
            chrono::Utc::now().to_rfc3339(),
            &catalog,
            &contracts,
        ),
    )
}

pub fn fabric_enroll(
    root: Option<&str>,
    host_id: &str,
    host_label: &str,
    from: &Path,
    base_url: Option<&str>,
    notes: Option<&str>,
) -> Result<TrustedRemoteHostRecord> {
    let root = resolve_root(root)?;
    let raw =
        fs::read_to_string(from).with_context(|| format!("Failed to read '{}'", from.display()))?;
    let advertisement = serde_json::from_str::<AgentFabricAdvertisement>(&raw)
        .with_context(|| format!("Failed to parse '{}'", from.display()))?;
    let mut registry = load_agent_fabric_registry(&root)?;
    AgentFabricRegistryService::new().enroll_remote_host(
        &mut registry,
        EnrollRemoteHostRequest {
            host_id: host_id.to_string(),
            host_label: host_label.to_string(),
            base_url: base_url.map(ToString::to_string),
            notes: notes.map(ToString::to_string),
            enrolled_at: chrono::Utc::now().to_rfc3339(),
            advertisement,
        },
    );
    save_agent_fabric_registry(&root, &registry)?;
    registry
        .hosts
        .into_iter()
        .find(|host| host.host_id == host_id)
        .ok_or_else(|| anyhow::anyhow!("fabric host '{}' was not persisted", host_id))
}

pub fn fabric_refresh(
    root: Option<&str>,
    host_id: &str,
    from: &Path,
) -> Result<TrustedRemoteHostRecord> {
    let root = resolve_root(root)?;
    let existing = load_agent_fabric_registry(&root)?
        .hosts
        .into_iter()
        .find(|host| host.host_id == host_id)
        .ok_or_else(|| anyhow::anyhow!("fabric host '{}' is not enrolled", host_id))?;
    fabric_enroll(
        root.to_str(),
        host_id,
        &existing.host_label,
        from,
        existing.base_url.as_deref(),
        existing.notes.as_deref(),
    )
}

pub fn fabric_hosts(root: Option<&str>) -> Result<FabricRegistryReport> {
    let root = resolve_root(root)?;
    let manifest_path = agent_fabric_path(&root).display().to_string();
    let local_advertisement = fabric_export(root.to_str())?;
    let registry = load_agent_fabric_registry(&root)?;
    let route_signals =
        AgentFabricRegistryService::new().route_signals(&local_advertisement, &registry);
    Ok(FabricRegistryReport {
        manifest_path,
        local_advertisement,
        registry,
        route_signals,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDecisionHistoryReport {
    pub receipts_path: String,
    pub entries: Vec<DelegatedRouteDecision>,
}

fn append_route_receipt(root: &Path, decision: &DelegatedRouteDecision) -> Result<()> {
    let path = agent_route_receipts_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write as _;
    writeln!(file, "{}", serde_json::to_string(decision)?)?;
    Ok(())
}

pub fn resolve_route(
    root: Option<&str>,
    request: DelegatedRouteRequest,
) -> Result<DelegatedRouteDecision> {
    let root = resolve_root(root)?;
    let report = fabric_hosts(root.to_str())?;
    let decision = AgentRoutePolicyService::new().resolve_route(
        &request,
        &report.route_signals,
        chrono::Utc::now().to_rfc3339(),
    );
    append_route_receipt(&root, &decision)?;
    Ok(decision)
}

pub fn route_receipts(root: Option<&str>, limit: usize) -> Result<RouteDecisionHistoryReport> {
    let root = resolve_root(root)?;
    let path = agent_route_receipts_path(&root);
    if !path.exists() {
        return Ok(RouteDecisionHistoryReport {
            receipts_path: path.display().to_string(),
            entries: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut entries = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<DelegatedRouteDecision>(line).ok())
        .collect::<Vec<_>>();
    entries.reverse();
    entries.truncate(limit);
    Ok(RouteDecisionHistoryReport {
        receipts_path: path.display().to_string(),
        entries,
    })
}

pub fn remote_route_envelope(
    root: Option<&str>,
    receipt_id: &str,
) -> Result<Option<RemoteRouteExecutionEnvelope>> {
    let root = resolve_root(root)?;
    let receipt = route_receipts(root.to_str(), 200)?
        .entries
        .into_iter()
        .find(|entry| entry.receipt_id == receipt_id)
        .ok_or_else(|| anyhow::anyhow!("route receipt '{}' was not found", receipt_id))?;
    let registry = load_registry(root.clone())?;
    let runtime = registry.runtime.unwrap_or_else(|| RuntimeModeSpec {
        mode: "solo_claw".to_string(),
        default_claw_id: None,
        orchestrator_claw_id: None,
        task_assignments: BTreeMap::new(),
        category_assignments: BTreeMap::new(),
        allow_shared_context: false,
        isolation_mode: "strict".to_string(),
        autonomy: AutonomyPolicy::default(),
        metadata: serde_json::json!({}),
    });
    let config = runtime::load_effective_config(
        "config/default.toml",
        &workspace_root_from_control_root(&root),
    )
    .unwrap_or_else(|_| AppConfig::default());
    Ok(AgentRoutePolicyService::new().build_remote_envelope(
        receipt,
        &runtime.autonomy.approval_policy,
        runtime.autonomy.max_runtime_secs,
        runtime.autonomy.max_delegations,
        runtime.autonomy.max_iterations,
        &config.external_backends.command_env_allowlist,
        chrono::Utc::now().to_rfc3339(),
    ))
}

pub fn list(root: Option<&str>) -> Result<()> {
    let registry = load_registry(resolve_root(root)?)?;
    println!(
        "Runtime mode: {} (default_claw={}, orchestrator={})",
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.mode.as_str())
            .unwrap_or("unconfigured"),
        registry
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.default_claw_id.as_deref())
            .unwrap_or("-"),
        registry
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.orchestrator_claw_id.as_deref())
            .unwrap_or("-")
    );
    println!(
        "Autonomy: {} (yolo={}, steering={}, learning={}, critic={})",
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.autonomy.autonomy_level.as_str())
            .unwrap_or("managed"),
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.autonomy.yolo_mode)
            .unwrap_or(false),
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.autonomy.steering_enabled)
            .unwrap_or(true),
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.autonomy.decision_learning_enabled)
            .unwrap_or(true),
        registry
            .runtime
            .as_ref()
            .map(|runtime| runtime.autonomy.critic_enabled)
            .unwrap_or(true)
    );

    println!("Agent profiles:");
    for profile in registry.agent_profiles.values() {
        println!(
            "- {} model_profile={} extends={} memory_scope={} output_policy={}",
            profile.id,
            profile.model_profile_id.as_deref().unwrap_or("-"),
            if profile.extends.is_empty() {
                "-".to_string()
            } else {
                profile.extends.join(",")
            },
            profile.memory_scope,
            profile.output_policy
        );
    }

    println!("Model profiles:");
    for profile in registry.model_profiles.values() {
        println!(
            "- {} provider={} model={} roles={} fallbacks={}",
            profile.id,
            profile.provider,
            profile.model,
            if profile.role_tags.is_empty() {
                "-".to_string()
            } else {
                profile.role_tags.join(",")
            },
            if profile.fallback_order.is_empty() {
                "-".to_string()
            } else {
                profile.fallback_order.join(",")
            }
        );
    }

    println!("Claws:");
    for claw in registry.claws.values() {
        println!(
            "- {} role={} agent_profile={} model_profile={} categories={}",
            claw.id,
            claw.role,
            claw.agent_profile_id,
            claw.model_profile_id,
            if claw.task_categories.is_empty() {
                "-".to_string()
            } else {
                claw.task_categories.join(",")
            }
        );
    }

    println!("Decision lessons:");
    for lesson in registry.lessons.values() {
        println!(
            "- {} active={} signal={} scope(category={}, claw={}, model={}, autonomy={})",
            lesson.id,
            lesson.active,
            lesson.signal,
            lesson.scope.category.as_deref().unwrap_or("-"),
            lesson.scope.claw_id.as_deref().unwrap_or("-"),
            lesson.scope.model_profile_id.as_deref().unwrap_or("-"),
            lesson.scope.autonomy_level.as_deref().unwrap_or("-")
        );
    }

    Ok(())
}

pub fn show(root: Option<&str>, kind: &str, id: Option<&str>) -> Result<()> {
    let registry = load_registry(resolve_root(root)?)?;
    match kind {
        "runtime" => println!(
            "{}",
            serde_yaml::to_string(&RuntimeModeManifest {
                version: 1,
                runtime: registry.runtime.unwrap_or(RuntimeModeSpec {
                    mode: default_runtime_mode(),
                    default_claw_id: None,
                    orchestrator_claw_id: None,
                    task_assignments: BTreeMap::new(),
                    category_assignments: BTreeMap::new(),
                    allow_shared_context: false,
                    isolation_mode: default_isolation_mode(),
                    autonomy: AutonomyPolicy::default(),
                    metadata: serde_json::json!({}),
                }),
            })?
        ),
        "agent" => {
            let id = id.context("agent show requires --id")?;
            let profile = registry
                .agent_profiles
                .get(id)
                .with_context(|| format!("Unknown agent profile '{id}'"))?;
            println!(
                "{}",
                serde_yaml::to_string(&AgentProfileManifest {
                    version: 1,
                    profile: profile.clone(),
                })?
            );
        }
        "model" => {
            let id = id.context("model show requires --id")?;
            let profile = registry
                .model_profiles
                .get(id)
                .with_context(|| format!("Unknown model profile '{id}'"))?;
            println!(
                "{}",
                serde_yaml::to_string(&ModelProfileManifest {
                    version: 1,
                    model: profile.clone(),
                })?
            );
        }
        "claw" => {
            let id = id.context("claw show requires --id")?;
            let claw = registry
                .claws
                .get(id)
                .with_context(|| format!("Unknown claw '{id}'"))?;
            println!(
                "{}",
                serde_yaml::to_string(&ClawManifest {
                    version: 1,
                    claw: claw.clone(),
                })?
            );
        }
        "lesson" => {
            let id = id.context("lesson show requires --id")?;
            let lesson = registry
                .lessons
                .get(id)
                .with_context(|| format!("Unknown lesson '{id}'"))?;
            println!(
                "{}",
                serde_yaml::to_string(&DecisionLessonManifest {
                    version: 1,
                    lesson: lesson.clone(),
                })?
            );
        }
        _ => anyhow::bail!("kind must be one of: runtime, agent, model, claw, lesson"),
    }
    Ok(())
}

pub fn validate(root: Option<&str>) -> Result<()> {
    let root = resolve_root(root)?;
    let registry = load_registry(root.clone())?;
    validate_registry(&registry)?;
    println!(
        "Control registry is valid: {} agent profiles, {} model profiles, {} claws, {} lessons",
        registry.agent_profiles.len(),
        registry.model_profiles.len(),
        registry.claws.len(),
        registry.lessons.len()
    );
    Ok(())
}

pub fn describe(root: Option<&str>, json: bool) -> Result<()> {
    let description = describe_registry(resolve_root(root)?)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&description)?);
    } else {
        println!("Execution mode: {}", description["execution_mode"]);
        println!(
            "Default claw: {}",
            description["default_claw"].as_str().unwrap_or("-")
        );
        println!(
            "Available claws: {}",
            description["available_claws"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("id").and_then(|value| value.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default()
        );
        println!(
            "Isolation mode: {}",
            description["isolation_mode"].as_str().unwrap_or("-")
        );
        println!(
            "Autonomy level: {}",
            description["autonomy"]["autonomy_level"]
                .as_str()
                .unwrap_or("managed")
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn create_agent(
    root: Option<&str>,
    id: &str,
    name: Option<&str>,
    model_profile_id: Option<&str>,
    extends: &[String],
    timeout_secs: u64,
    memory_scope: Option<&str>,
    output_policy: Option<&str>,
    tool_allow: &[String],
    tool_deny: &[String],
) -> Result<()> {
    let root = resolve_root(root)?;
    let registry = load_registry(root.clone())?;
    for parent in extends {
        if !registry.agent_profiles.contains_key(parent) {
            anyhow::bail!("cannot extend missing agent profile '{}'", parent);
        }
    }
    if let Some(model_profile_id) = model_profile_id
        && !registry.model_profiles.contains_key(model_profile_id)
    {
        anyhow::bail!(
            "agent profile '{}' references missing model profile '{}'",
            id,
            model_profile_id
        );
    }
    fs::create_dir_all(agents_dir(&root))?;
    let manifest = AgentProfileManifest {
        version: 1,
        profile: AgentProfileSpec {
            id: id.to_string(),
            name: name.map(ToString::to_string),
            extends: extends.to_vec(),
            model_profile_id: model_profile_id.map(ToString::to_string),
            thinking_level: None,
            timeout_secs,
            tool_allow: tool_allow.to_vec(),
            tool_deny: tool_deny.to_vec(),
            memory_scope: memory_scope.unwrap_or("workspace_shared").to_string(),
            output_policy: output_policy.unwrap_or("standard").to_string(),
            metadata: serde_json::json!({}),
        },
    };
    write_yaml(
        &agents_dir(&root).join(format!("{}.yaml", slugify(id))),
        &manifest,
    )?;
    sync_runtime_artifact(&root)?;
    println!("Wrote agent profile {}", id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn create_model(
    root: Option<&str>,
    id: &str,
    provider: &str,
    model: &str,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
    latency_hint: Option<&str>,
    cost_hint: Option<&str>,
    reasoning_hint: Option<&str>,
    role_tags: &[String],
    artifact_preferences: &[String],
    fallback_order: &[String],
) -> Result<()> {
    let root = resolve_root(root)?;
    let registry = load_registry(root.clone())?;
    for fallback in fallback_order {
        if !registry.model_profiles.contains_key(fallback) && fallback != id {
            anyhow::bail!("model fallback '{}' does not exist yet", fallback);
        }
    }
    fs::create_dir_all(models_dir(&root))?;
    let manifest = ModelProfileManifest {
        version: 1,
        model: ModelProfileSpec {
            id: id.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            context_window,
            max_output_tokens,
            supports_tools: true,
            supports_vision: false,
            role_tags: role_tags.to_vec(),
            artifact_preferences: artifact_preferences.to_vec(),
            fallback_order: fallback_order.to_vec(),
            latency_hint: latency_hint.map(ToString::to_string),
            cost_hint: cost_hint.map(ToString::to_string),
            reasoning_hint: reasoning_hint.map(ToString::to_string),
            metadata: serde_json::json!({}),
        },
    };
    write_yaml(
        &models_dir(&root).join(format!("{}.yaml", slugify(id))),
        &manifest,
    )?;
    sync_runtime_artifact(&root)?;
    println!("Wrote model profile {}", id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn create_claw(
    root: Option<&str>,
    id: &str,
    name: Option<&str>,
    agent_profile_id: &str,
    model_profile_id: &str,
    role: Option<&str>,
    categories: &[String],
    memory_scope: Option<&str>,
) -> Result<()> {
    let root = resolve_root(root)?;
    let registry = load_registry(root.clone())?;
    if !registry.agent_profiles.contains_key(agent_profile_id) {
        anyhow::bail!("missing agent profile '{}'", agent_profile_id);
    }
    if !registry.model_profiles.contains_key(model_profile_id) {
        anyhow::bail!("missing model profile '{}'", model_profile_id);
    }
    fs::create_dir_all(claws_dir(&root))?;
    let manifest = ClawManifest {
        version: 1,
        claw: ClawSpec {
            id: id.to_string(),
            name: name.map(ToString::to_string),
            agent_profile_id: agent_profile_id.to_string(),
            model_profile_id: model_profile_id.to_string(),
            role: role.unwrap_or("worker").to_string(),
            memory_scope: memory_scope.unwrap_or("workspace_shared").to_string(),
            task_categories: categories.to_vec(),
            tool_allow: vec![],
            tool_deny: vec![],
            enabled: true,
            metadata: serde_json::json!({}),
        },
    };
    write_yaml(
        &claws_dir(&root).join(format!("{}.yaml", slugify(id))),
        &manifest,
    )?;
    sync_runtime_artifact(&root)?;
    println!("Wrote claw {}", id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn configure_mode(
    root: Option<&str>,
    mode: &str,
    default_claw_id: Option<&str>,
    orchestrator_claw_id: Option<&str>,
    allow_shared_context: bool,
    isolation_mode: Option<&str>,
    autonomy_level: Option<&str>,
    yolo_mode: Option<bool>,
    steering_enabled: Option<bool>,
    decision_learning_enabled: Option<bool>,
    critic_enabled: Option<bool>,
    max_delegations: Option<usize>,
    max_iterations: Option<usize>,
    max_runtime_secs: Option<u64>,
    max_lesson_hints: Option<usize>,
    approval_policy: Option<&str>,
) -> Result<()> {
    let root = resolve_root(root)?;
    let mut runtime = read_runtime(&root)?;
    runtime.runtime.mode = mode.to_string();
    runtime.runtime.default_claw_id = default_claw_id.map(ToString::to_string);
    runtime.runtime.orchestrator_claw_id = orchestrator_claw_id.map(ToString::to_string);
    runtime.runtime.allow_shared_context = allow_shared_context;
    if let Some(isolation_mode) = isolation_mode {
        runtime.runtime.isolation_mode = isolation_mode.to_string();
    }
    if let Some(autonomy_level) = autonomy_level {
        runtime.runtime.autonomy.autonomy_level = autonomy_level.to_string();
    }
    if let Some(yolo_mode) = yolo_mode {
        runtime.runtime.autonomy.yolo_mode = yolo_mode;
    }
    if let Some(steering_enabled) = steering_enabled {
        runtime.runtime.autonomy.steering_enabled = steering_enabled;
    }
    if let Some(decision_learning_enabled) = decision_learning_enabled {
        runtime.runtime.autonomy.decision_learning_enabled = decision_learning_enabled;
    }
    if let Some(critic_enabled) = critic_enabled {
        runtime.runtime.autonomy.critic_enabled = critic_enabled;
    }
    if let Some(max_delegations) = max_delegations {
        runtime.runtime.autonomy.max_delegations = max_delegations;
    }
    if let Some(max_iterations) = max_iterations {
        runtime.runtime.autonomy.max_iterations = max_iterations;
    }
    if let Some(max_runtime_secs) = max_runtime_secs {
        runtime.runtime.autonomy.max_runtime_secs = max_runtime_secs;
    }
    if let Some(max_lesson_hints) = max_lesson_hints {
        runtime.runtime.autonomy.max_lesson_hints = max_lesson_hints;
    }
    if let Some(approval_policy) = approval_policy {
        runtime.runtime.autonomy.approval_policy = approval_policy.to_string();
    }
    write_yaml(&runtime_path(&root), &runtime)?;
    sync_runtime_artifact(&root)?;
    println!("Configured runtime mode {}", mode);
    Ok(())
}

pub fn set_runtime_approval_policy(root: &Path, approval_policy: &str) -> Result<()> {
    let mut runtime = read_runtime(root)?;
    runtime.runtime.autonomy.approval_policy = approval_policy.to_string();
    write_yaml(&runtime_path(root), &runtime)?;
    sync_runtime_artifact(root)?;
    Ok(())
}

pub fn runtime_autonomy_policy(root: &Path) -> Result<AutonomyPolicy> {
    Ok(read_runtime(root)?.runtime.autonomy)
}

pub fn set_runtime_autonomy(root: &Path, autonomy: &AutonomyPolicy) -> Result<()> {
    let mut runtime = read_runtime(root)?;
    runtime.runtime.autonomy = autonomy.clone();
    write_yaml(&runtime_path(root), &runtime)?;
    sync_runtime_artifact(root)?;
    Ok(())
}

pub fn assign_task(root: Option<&str>, task_id: &str, claw_id: &str) -> Result<()> {
    let root = resolve_root(root)?;
    let mut runtime = read_runtime(&root)?;
    runtime
        .runtime
        .task_assignments
        .insert(task_id.to_string(), claw_id.to_string());
    write_yaml(&runtime_path(&root), &runtime)?;
    sync_runtime_artifact(&root)?;
    println!("Assigned task {} -> {}", task_id, claw_id);
    Ok(())
}

pub fn assign_category(root: Option<&str>, category: &str, claw_id: &str) -> Result<()> {
    let root = resolve_root(root)?;
    let mut runtime = read_runtime(&root)?;
    runtime
        .runtime
        .category_assignments
        .insert(category.to_string(), claw_id.to_string());
    write_yaml(&runtime_path(&root), &runtime)?;
    sync_runtime_artifact(&root)?;
    println!("Assigned category {} -> {}", category, claw_id);
    Ok(())
}

pub fn load_registry(root: PathBuf) -> Result<ControlRegistry> {
    let mut registry = ControlRegistry::default();
    if !root.exists() {
        return Ok(registry);
    }

    let agents_root = agents_dir(&root);
    if agents_root.exists() {
        for entry in WalkDir::new(&agents_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() || !is_yaml(entry.path()) {
                continue;
            }
            let manifest: AgentProfileManifest = read_yaml(entry.path())?;
            registry
                .agent_profiles
                .insert(manifest.profile.id.clone(), manifest.profile);
        }
    }

    let models_root = models_dir(&root);
    if models_root.exists() {
        for entry in WalkDir::new(&models_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() || !is_yaml(entry.path()) {
                continue;
            }
            let manifest: ModelProfileManifest = read_yaml(entry.path())?;
            registry
                .model_profiles
                .insert(manifest.model.id.clone(), manifest.model);
        }
    }

    let claws_root = claws_dir(&root);
    if claws_root.exists() {
        for entry in WalkDir::new(&claws_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() || !is_yaml(entry.path()) {
                continue;
            }
            let manifest: ClawManifest = read_yaml(entry.path())?;
            registry
                .claws
                .insert(manifest.claw.id.clone(), manifest.claw);
        }
    }

    let lessons_root = lessons_dir(&root);
    if lessons_root.exists() {
        for entry in WalkDir::new(&lessons_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() || !is_yaml(entry.path()) {
                continue;
            }
            let manifest: DecisionLessonManifest = read_yaml(entry.path())?;
            registry
                .lessons
                .insert(manifest.lesson.id.clone(), manifest.lesson);
        }
    }

    if runtime_path(&root).exists() {
        registry.runtime = Some(read_runtime(&root)?.runtime);
    }

    Ok(registry)
}

pub fn describe_registry(root: PathBuf) -> Result<Value> {
    let registry = load_registry(root)?;
    Ok(app_control_registry::ControlRegistryService::new()
        .describe_registry(&app_registry_snapshot(&registry))?)
}

pub fn sync_runtime_artifact(root: &Path) -> Result<()> {
    let description = describe_registry(root.to_path_buf())?;
    let runtime_md = render_runtime_markdown(&description);
    fs::write(runtime_artifact_path(root), runtime_md.as_bytes()).with_context(|| {
        format!(
            "Failed to write '{}'",
            runtime_artifact_path(root).display()
        )
    })?;
    Ok(())
}

pub fn validate_registry(registry: &ControlRegistry) -> Result<()> {
    Ok(app_control_registry::ControlRegistryService::new()
        .validate_registry(&app_registry_snapshot(registry))?)
}

fn render_runtime_markdown(description: &Value) -> String {
    let mut output = String::new();
    output.push_str("# OpenRustClaw Runtime Mode\n\n");
    output.push_str(&format!(
        "- Execution mode: `{}`\n",
        description["execution_mode"]
            .as_str()
            .unwrap_or("solo_claw")
    ));
    if let Some(default_claw) = description["default_claw"].as_str() {
        output.push_str(&format!("- Default Claw: `{default_claw}`\n"));
    }
    if let Some(orchestrator) = description["orchestrator_claw"].as_str() {
        output.push_str(&format!("- Orchestrator Claw: `{orchestrator}`\n"));
    }
    output.push_str(&format!(
        "- Shared context allowed: `{}`\n",
        description["allow_shared_context"]
            .as_bool()
            .unwrap_or(false)
    ));
    output.push_str(&format!(
        "- Isolation mode: `{}`\n\n",
        description["isolation_mode"].as_str().unwrap_or("strict")
    ));
    output.push_str("## Autonomy Policy\n\n");
    output.push_str(&format!(
        "- Autonomy level: `{}`\n",
        description["autonomy"]["autonomy_level"]
            .as_str()
            .unwrap_or("managed")
    ));
    output.push_str(&format!(
        "- Yolo mode: `{}`\n",
        description["autonomy"]["yolo_mode"]
            .as_bool()
            .unwrap_or(false)
    ));
    output.push_str(&format!(
        "- Steering enabled: `{}`\n",
        description["autonomy"]["steering_enabled"]
            .as_bool()
            .unwrap_or(true)
    ));
    output.push_str(&format!(
        "- Decision learning enabled: `{}`\n",
        description["autonomy"]["decision_learning_enabled"]
            .as_bool()
            .unwrap_or(true)
    ));
    output.push_str(&format!(
        "- Critic enabled: `{}`\n",
        description["autonomy"]["critic_enabled"]
            .as_bool()
            .unwrap_or(true)
    ));
    output.push_str(&format!(
        "- Approval policy: `{}`\n",
        description["autonomy"]["approval_policy"]
            .as_str()
            .unwrap_or("side_effects")
    ));
    output.push_str(&format!(
        "- Max delegations: `{}`\n",
        description["autonomy"]["max_delegations"]
            .as_u64()
            .unwrap_or(4)
    ));
    output.push_str(&format!(
        "- Max lesson hints: `{}`\n\n",
        description["autonomy"]["max_lesson_hints"]
            .as_u64()
            .unwrap_or(5)
    ));
    output.push_str("## Available Claws\n\n");
    if let Some(items) = description["available_claws"].as_array() {
        for item in items {
            output.push_str(&format!(
                "- `{}` role=`{}` model_profile=`{}` memory_scope=`{}` categories={}\n",
                item["id"].as_str().unwrap_or("unknown"),
                item["role"].as_str().unwrap_or("worker"),
                item["model_profile_id"].as_str().unwrap_or("-"),
                item["memory_scope"].as_str().unwrap_or("-"),
                item["task_categories"]
                    .as_array()
                    .map(|values| values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .collect::<Vec<_>>()
                        .join(","))
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "-".to_string())
            ));
        }
    }
    output.push_str("\n## Delegation Contract\n\n");
    output.push_str("- Worker Claws report back to the primary/orchestrator Claw.\n");
    output.push_str(
        "- Cross-task contamination is disallowed unless shared context is explicitly enabled.\n",
    );
    output.push_str("- Task/category assignments are authoritative when present.\n");
    output.push_str(
        "- Use delegation only when another Claw is available and better suited for the task.\n",
    );
    if let Some(lessons) = description["decision_lessons"].as_array() {
        output.push_str("\n## Active Decision Lessons\n\n");
        let mut any = false;
        for lesson in lessons
            .iter()
            .filter(|lesson| lesson["active"].as_bool().unwrap_or(false))
        {
            any = true;
            output.push_str(&format!(
                "- `{}` signal=`{}` recommendation=`{}` confidence={}\n",
                lesson["id"].as_str().unwrap_or("unknown"),
                lesson["signal"].as_str().unwrap_or("-"),
                lesson["recommendation"].as_str().unwrap_or("-"),
                lesson["confidence"].as_f64().unwrap_or(0.0)
            ));
        }
        if !any {
            output.push_str("- No active decision lessons yet.\n");
        }
    }
    output
}

fn read_runtime(root: &Path) -> Result<RuntimeModeManifest> {
    if !runtime_path(root).exists() {
        return Ok(RuntimeModeManifest {
            version: 1,
            runtime: RuntimeModeSpec {
                mode: default_runtime_mode(),
                default_claw_id: None,
                orchestrator_claw_id: None,
                task_assignments: BTreeMap::new(),
                category_assignments: BTreeMap::new(),
                allow_shared_context: false,
                isolation_mode: default_isolation_mode(),
                autonomy: AutonomyPolicy::default(),
                metadata: serde_json::json!({}),
            },
        });
    }
    read_yaml(&runtime_path(root))
}

pub fn list_lessons(root: Option<&str>, active_only: bool) -> Result<()> {
    let registry = load_registry(resolve_root(root)?)?;
    for lesson in registry.lessons.values() {
        if active_only && !lesson.active {
            continue;
        }
        println!(
            "{}  active={}  signal={}  recommendation={}  scope(category={}, claw={}, model={}, autonomy={})",
            lesson.id,
            lesson.active,
            lesson.signal,
            lesson.recommendation,
            lesson.scope.category.as_deref().unwrap_or("-"),
            lesson.scope.claw_id.as_deref().unwrap_or("-"),
            lesson.scope.model_profile_id.as_deref().unwrap_or("-"),
            lesson.scope.autonomy_level.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct NewLessonInput<'a> {
    pub id: &'a str,
    pub active: bool,
    pub signal: &'a str,
    pub recommendation: &'a str,
    pub rationale: Option<&'a str>,
    pub confidence: f32,
    pub source: Option<&'a str>,
    pub task_id: Option<&'a str>,
    pub category: Option<&'a str>,
    pub claw_id: Option<&'a str>,
    pub model_profile_id: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub autonomy_level: Option<&'a str>,
    pub execution_mode: Option<&'a str>,
}

pub fn create_lesson(root: Option<&str>, input: NewLessonInput<'_>) -> Result<()> {
    let root = resolve_root(root)?;
    let registry = load_registry(root.clone())?;
    if let Some(claw_id) = input.claw_id
        && !registry.claws.contains_key(claw_id)
    {
        anyhow::bail!("missing claw '{}'", claw_id);
    }
    if let Some(model_profile_id) = input.model_profile_id
        && !registry.model_profiles.contains_key(model_profile_id)
    {
        anyhow::bail!("missing model profile '{}'", model_profile_id);
    }
    let manifest = DecisionLessonManifest {
        version: 1,
        lesson: DecisionLessonSpec {
            id: input.id.to_string(),
            active: input.active,
            signal: input.signal.to_string(),
            recommendation: input.recommendation.to_string(),
            rationale: input.rationale.map(ToString::to_string),
            confidence: input.confidence,
            source: input
                .source
                .map(ToString::to_string)
                .unwrap_or_else(default_lesson_source),
            scope: DecisionLessonScope {
                task_id: input.task_id.map(ToString::to_string),
                category: input.category.map(ToString::to_string),
                claw_id: input.claw_id.map(ToString::to_string),
                model_profile_id: input.model_profile_id.map(ToString::to_string),
                provider: input.provider.map(ToString::to_string),
                autonomy_level: input.autonomy_level.map(ToString::to_string),
                execution_mode: input.execution_mode.map(ToString::to_string),
            },
            metadata: serde_json::json!({}),
        },
    };
    write_yaml(
        &lessons_dir(&root).join(format!("{}.yaml", slugify(input.id))),
        &manifest,
    )?;
    sync_runtime_artifact(&root)?;
    println!("Wrote decision lesson {}", input.id);
    Ok(())
}

pub fn deactivate_lesson(root: Option<&str>, id: &str) -> Result<()> {
    let root = resolve_root(root)?;
    let mut registry = load_registry(root.clone())?;
    let lesson = registry
        .lessons
        .remove(id)
        .with_context(|| format!("Unknown lesson '{}'", id))?;
    let manifest = DecisionLessonManifest {
        version: 1,
        lesson: DecisionLessonSpec {
            active: false,
            ..lesson
        },
    };
    write_yaml(
        &lessons_dir(&root).join(format!("{}.yaml", slugify(id))),
        &manifest,
    )?;
    sync_runtime_artifact(&root)?;
    println!("Deactivated decision lesson {}", id);
    Ok(())
}

#[derive(Clone)]
pub struct WorkspaceLearningReviewSource {
    workspace_root: PathBuf,
    learning_store: SqliteLearningStore,
}

impl WorkspaceLearningReviewSource {
    pub fn from_pool(workspace_root: PathBuf, pool: SqlitePool) -> Self {
        Self {
            workspace_root,
            learning_store: SqliteLearningStore::new(pool),
        }
    }
}

#[async_trait]
impl LearningReviewSource for WorkspaceLearningReviewSource {
    async fn list_learning_candidates(
        &self,
        namespace: Option<&str>,
        status: Option<LearningCandidateStatus>,
        limit: usize,
    ) -> CoreResult<Vec<LearningCandidate>> {
        self.learning_store
            .list_candidates(namespace, status, limit)
            .await
    }

    async fn get_learning_candidate(&self, id: &str) -> CoreResult<Option<LearningCandidate>> {
        self.learning_store.get_candidate(id).await
    }

    async fn create_learning_candidate(
        &self,
        request: &LearningCandidateCreateRequest,
    ) -> CoreResult<LearningCandidate> {
        self.learning_store.create_candidate(request).await
    }

    async fn review_learning_candidate(
        &self,
        id: &str,
        request: &LearningCandidateReviewRequest,
    ) -> CoreResult<LearningCandidate> {
        self.learning_store.review_candidate(id, request).await
    }

    async fn mark_learning_candidate_promoted(
        &self,
        id: &str,
        lesson_id: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> CoreResult<LearningCandidate> {
        self.learning_store
            .mark_promoted(id, lesson_id, actor, note)
            .await
    }

    async fn mark_learning_candidate_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> CoreResult<LearningCandidate> {
        self.learning_store
            .mark_rolled_back(id, actor, reason)
            .await
    }

    async fn mark_learning_candidate_quarantined(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> CoreResult<LearningCandidate> {
        self.learning_store
            .mark_quarantined(id, actor, reason)
            .await
    }

    async fn create_lesson(&self, request: &AutonomyLessonRequest) -> CoreResult<()> {
        create_lesson(
            Some(
                control_root_for(&self.workspace_root)
                    .to_string_lossy()
                    .as_ref(),
            ),
            NewLessonInput {
                id: &request.id,
                active: request.active,
                signal: &request.signal,
                recommendation: &request.recommendation,
                rationale: request.rationale.as_deref(),
                confidence: request.confidence.unwrap_or(default_confidence()),
                source: request.source.as_deref(),
                task_id: request.task_id.as_deref(),
                category: request.category.as_deref(),
                claw_id: request.claw_id.as_deref(),
                model_profile_id: request.model_profile_id.as_deref(),
                provider: request.provider.as_deref(),
                autonomy_level: request.autonomy_level.as_deref(),
                execution_mode: request.execution_mode.as_deref(),
            },
        )
        .map_err(anyhow_to_core)
    }

    async fn deactivate_lesson(&self, id: &str) -> CoreResult<()> {
        deactivate_lesson(
            Some(
                control_root_for(&self.workspace_root)
                    .to_string_lossy()
                    .as_ref(),
            ),
            id,
        )
        .map_err(anyhow_to_core)
    }
}

pub async fn learning_review_service_for_workspace(
    workspace_root: &Path,
) -> Result<LearningReviewService<WorkspaceLearningReviewSource>> {
    let source = learning_review_source_for_workspace(workspace_root).await?;
    Ok(LearningReviewService::new(source))
}

pub async fn learning_review_source_for_workspace(
    workspace_root: &Path,
) -> Result<WorkspaceLearningReviewSource> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)
        .unwrap_or_else(|_| AppConfig::default());
    let database_url = resolve_workspace_database_url(workspace_root, &config.database.url);
    ensure_workspace_database_parent(&database_url)?;
    let pool = init_pool(&database_url, 2)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    run_migrations(&pool)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(WorkspaceLearningReviewSource::from_pool(
        workspace_root.to_path_buf(),
        pool,
    ))
}

#[derive(Clone)]
pub struct WorkspaceSkillProposalSource {
    workspace_root: PathBuf,
    pool: SqlitePool,
    proposal_store: SqliteSkillProposalStore,
}

impl WorkspaceSkillProposalSource {
    pub fn from_pool(workspace_root: PathBuf, pool: SqlitePool) -> Self {
        Self {
            workspace_root,
            pool: pool.clone(),
            proposal_store: SqliteSkillProposalStore::new(pool),
        }
    }

    fn proposal_root(&self) -> PathBuf {
        skill_proposals_dir(&control_root_for(&self.workspace_root))
    }

    fn proposal_file_path(&self, proposal_id: &str) -> PathBuf {
        self.proposal_root().join(proposal_id).join("SKILL.md")
    }

    fn proposal_preview_root(&self, proposal_id: &str) -> PathBuf {
        self.proposal_root().join(proposal_id).join("preview")
    }

    fn active_skill_file_path(&self, skill_name: &str) -> PathBuf {
        self.workspace_root
            .join("skills")
            .join(skill_name)
            .join("SKILL.md")
    }

    fn active_compiled_skill_root(&self) -> PathBuf {
        self.workspace_root
            .join(".claw")
            .join("skills")
            .join("compiled")
    }
}

#[async_trait]
impl SkillProposalSource for WorkspaceSkillProposalSource {
    async fn list_skill_proposals(
        &self,
        namespace: Option<&str>,
        status: Option<SkillProposalStatus>,
        limit: usize,
    ) -> CoreResult<Vec<SkillProposal>> {
        self.proposal_store
            .list_proposals(namespace, status, limit)
            .await
    }

    async fn get_skill_proposal(&self, id: &str) -> CoreResult<Option<SkillProposal>> {
        self.proposal_store.get_proposal(id).await
    }

    async fn learning_candidate_god_mode_origin(&self, id: &str) -> CoreResult<bool> {
        Ok(SqliteLearningStore::new(self.pool.clone())
            .get_candidate(id)
            .await?
            .map(|candidate| candidate.god_mode_origin)
            .unwrap_or(false))
    }

    async fn create_skill_proposal(&self, proposal: &SkillProposal) -> CoreResult<SkillProposal> {
        self.proposal_store.create_proposal(proposal).await
    }

    async fn review_skill_proposal(
        &self,
        id: &str,
        request: &SkillProposalReviewRequest,
    ) -> CoreResult<SkillProposal> {
        self.proposal_store.review_proposal(id, request).await
    }

    async fn record_skill_proposal_verification(
        &self,
        id: &str,
        report: &SkillProposalVerificationReport,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> CoreResult<SkillProposal> {
        self.proposal_store
            .record_verification(id, report, actor, note)
            .await
    }

    async fn mark_skill_proposal_installed(
        &self,
        id: &str,
        installed_skill_name: &str,
        actor: Option<&str>,
        note: Option<&str>,
    ) -> CoreResult<SkillProposal> {
        self.proposal_store
            .mark_installed(id, installed_skill_name, actor, note)
            .await
    }

    async fn mark_skill_proposal_rolled_back(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> CoreResult<SkillProposal> {
        self.proposal_store
            .mark_rolled_back(id, actor, reason)
            .await
    }

    async fn mark_skill_proposal_quarantined(
        &self,
        id: &str,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> CoreResult<SkillProposal> {
        self.proposal_store
            .mark_quarantined(id, actor, reason)
            .await
    }

    async fn write_proposal_artifact(&self, proposal: &SkillProposal) -> CoreResult<String> {
        let path = self.proposal_file_path(&proposal.id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| CoreError::Internal(error.to_string()))?;
        }
        tokio::fs::write(&path, &proposal.body)
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        Ok(path.display().to_string())
    }

    async fn verify_proposal_artifact(
        &self,
        proposal: &SkillProposal,
    ) -> CoreResult<SkillProposalVerificationReport> {
        let proposal_path = Path::new(&proposal.artifact_path);
        let preview_root = self.proposal_preview_root(&proposal.id);
        if preview_root.exists() {
            let _ = fs::remove_dir_all(&preview_root);
        }
        fs::create_dir_all(&preview_root)
            .map_err(|error| CoreError::Internal(error.to_string()))?;

        match compile_skill_to_dir(proposal_path, &preview_root, SkillSource::Workspace, true) {
            Ok(artifact) => {
                let blocked = matches!(artifact.manifest.status, CompiledSkillStatus::Blocked);
                Ok(SkillProposalVerificationReport {
                    status: if blocked {
                        SkillProposalVerificationStatus::Blocked
                    } else {
                        SkillProposalVerificationStatus::Passed
                    },
                    summary: if blocked {
                        format!(
                            "compiled proposal '{}' but the artifact is blocked by skill safety checks",
                            artifact.manifest.name
                        )
                    } else {
                        format!(
                            "compiled proposal '{}' successfully",
                            artifact.manifest.name
                        )
                    },
                    compiled_skill_name: Some(artifact.manifest.name),
                    verification_artifact_path: Some(preview_root.display().to_string()),
                    blocked,
                })
            }
            Err(error) => Ok(SkillProposalVerificationReport {
                status: SkillProposalVerificationStatus::Failed,
                summary: format!(
                    "failed to compile proposal '{}': {error}",
                    proposal.skill_name
                ),
                compiled_skill_name: None,
                verification_artifact_path: Some(preview_root.display().to_string()),
                blocked: false,
            }),
        }
    }

    async fn install_skill_proposal(&self, proposal: &SkillProposal) -> CoreResult<String> {
        let active_skill_path = self.active_skill_file_path(&proposal.skill_name);
        if let Some(parent) = active_skill_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| CoreError::Internal(error.to_string()))?;
        }
        tokio::fs::write(&active_skill_path, &proposal.body)
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;

        let compiled_root = self.active_compiled_skill_root();
        fs::create_dir_all(&compiled_root)
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        let artifact = compile_skill_to_dir(
            &active_skill_path,
            &compiled_root,
            SkillSource::Workspace,
            true,
        )
        .map_err(|error| CoreError::Internal(error.to_string()))?;

        let capabilities_json = serde_json::to_string(&artifact.manifest.capabilities)
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, description, source, version, signature,
                verified, enabled, capabilities, schema, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, datetime('now'), datetime('now'))
            ON CONFLICT(name) DO UPDATE SET
                description = excluded.description,
                source = excluded.source,
                version = excluded.version,
                signature = excluded.signature,
                verified = excluded.verified,
                enabled = 1,
                capabilities = excluded.capabilities,
                schema = excluded.schema,
                updated_at = datetime('now')
            "#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&artifact.manifest.name)
        .bind(&artifact.manifest.description)
        .bind("workspace")
        .bind(&artifact.manifest.version)
        .bind::<Option<String>>(None)
        .bind(1_i64)
        .bind(&capabilities_json)
        .bind::<Option<String>>(None)
        .execute(&self.pool)
        .await
        .map_err(|error| CoreError::Internal(error.to_string()))?;

        Ok(artifact.manifest.name)
    }

    async fn rollback_installed_skill(&self, skill_name: &str) -> CoreResult<()> {
        sqlx::query("DELETE FROM skills WHERE name = ?")
            .bind(skill_name)
            .execute(&self.pool)
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        let _ = remove_compiled_artifact(&self.active_compiled_skill_root(), skill_name);
        let skill_dir = self.workspace_root.join("skills").join(skill_name);
        if skill_dir.exists() {
            let _ = fs::remove_dir_all(skill_dir);
        }
        Ok(())
    }
}

pub async fn skill_proposal_service_for_workspace(
    workspace_root: &Path,
) -> Result<SkillProposalService<WorkspaceSkillProposalSource>> {
    let source = skill_proposal_source_for_workspace(workspace_root).await?;
    Ok(SkillProposalService::new(source))
}

pub async fn skill_proposal_source_for_workspace(
    workspace_root: &Path,
) -> Result<WorkspaceSkillProposalSource> {
    let config = runtime::load_effective_config("config/default.toml", workspace_root)
        .unwrap_or_else(|_| AppConfig::default());
    let database_url = resolve_workspace_database_url(workspace_root, &config.database.url);
    ensure_workspace_database_parent(&database_url)?;
    let pool = init_pool(&database_url, 2)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    run_migrations(&pool)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(WorkspaceSkillProposalSource::from_pool(
        workspace_root.to_path_buf(),
        pool,
    ))
}

fn resolve_workspace_database_url(workspace_root: &Path, database_url: &str) -> String {
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return database_url.to_string();
        }
        return format!("sqlite://{}", workspace_root.join(candidate).display());
    }
    database_url.to_string()
}

fn ensure_workspace_database_parent(database_url: &str) -> Result<()> {
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        let db_path = Path::new(path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create database directory '{}'", parent.display())
            })?;
        }
    }
    Ok(())
}

pub async fn list_learning_candidates_cli(
    workspace_root: &Path,
    namespace: Option<&str>,
    status: Option<LearningCandidateStatus>,
    limit: usize,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let candidates = service
        .list(namespace, status, limit)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    if candidates.is_empty() {
        println!("No learning candidates found.");
        return Ok(());
    }
    for candidate in candidates {
        println!(
            "{} [{}] {} ({:.2})",
            candidate.id,
            learning_status_label(candidate.status),
            candidate.signal,
            candidate.confidence
        );
        println!("  kind: {}", candidate.kind);
        println!("  namespace: {}", candidate.namespace);
        println!("  impact: {}", learning_impact_label(candidate.impact));
        println!(
            "  lane: {}",
            if candidate.god_mode_origin {
                "God Mode"
            } else {
                "trust-first"
            }
        );
        println!(
            "  source: {}:{}",
            learning_source_kind_label(candidate.source.kind),
            candidate.source.source_id
        );
        if let Some(quarantined_at) = candidate.quarantined_at {
            println!(
                "  quarantine: {} by {}",
                quarantined_at.to_rfc3339(),
                candidate.quarantined_by.as_deref().unwrap_or("-")
            );
        }
        if let Some(lesson_id) = candidate.promoted_lesson_id.as_deref() {
            println!("  lesson: {lesson_id}");
        }
        if !candidate.evidence.is_empty() {
            println!("  evidence: {}", candidate.evidence.len());
        }
    }
    Ok(())
}

pub async fn queue_learning_candidate_cli(
    workspace_root: &Path,
    request: &LearningCandidateCreateRequest,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let candidate = service
        .queue(request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!("Queued learning candidate {}", candidate.id);
    Ok(())
}

pub async fn review_learning_candidate_cli(
    workspace_root: &Path,
    id: &str,
    request: &LearningCandidateReviewRequest,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let candidate = service
        .review(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Updated learning candidate {} to {}",
        candidate.id,
        learning_status_label(candidate.status)
    );
    Ok(())
}

pub async fn promote_learning_candidate_cli(
    workspace_root: &Path,
    id: &str,
    request: &LearningCandidatePromotionRequest,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let report = service
        .promote(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Promoted learning candidate {} to lesson {}",
        report.candidate.id, report.lesson_id
    );
    Ok(())
}

pub async fn rollback_learning_candidate_cli(
    workspace_root: &Path,
    id: &str,
    request: &LearningCandidateRollbackRequest,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let candidate = service
        .rollback(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Rolled back learning candidate {} to {}",
        candidate.id,
        learning_status_label(candidate.status)
    );
    Ok(())
}

pub async fn quarantine_learning_candidate_cli(
    workspace_root: &Path,
    id: &str,
    request: &LearningCandidateQuarantineRequest,
) -> Result<()> {
    let service = learning_review_service_for_workspace(workspace_root).await?;
    let candidate = service
        .quarantine(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Quarantined learning candidate {}{}",
        candidate.id,
        if candidate.god_mode_origin {
            " (God Mode)"
        } else {
            ""
        }
    );
    Ok(())
}

pub async fn list_skill_proposals_cli(
    workspace_root: &Path,
    namespace: Option<&str>,
    status: Option<SkillProposalStatus>,
    limit: usize,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposals = service
        .list(namespace, status, limit)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    if proposals.is_empty() {
        println!("No skill proposals found.");
        return Ok(());
    }
    for proposal in proposals {
        println!(
            "{} [{} / {}] {}",
            proposal.id,
            skill_proposal_status_label(proposal.status),
            skill_proposal_verification_label(proposal.verification_status),
            proposal.skill_name
        );
        println!("  namespace: {}", proposal.namespace);
        println!(
            "  lane: {}",
            if proposal.god_mode_origin {
                "God Mode"
            } else {
                "trust-first"
            }
        );
        println!(
            "  source: {}:{}",
            skill_proposal_source_kind_label(proposal.source.kind),
            proposal.source.source_id
        );
        println!("  artifact: {}", proposal.artifact_path);
        if let Some(quarantined_at) = proposal.quarantined_at {
            println!(
                "  quarantine: {} by {}",
                quarantined_at.to_rfc3339(),
                proposal.quarantined_by.as_deref().unwrap_or("-")
            );
        }
        if let Some(installed_skill_name) = proposal.installed_skill_name.as_deref() {
            println!("  installed: {installed_skill_name}");
        }
    }
    Ok(())
}

pub async fn queue_skill_proposal_cli(
    workspace_root: &Path,
    request: &SkillProposalCreateRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposal = service
        .queue(request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!("Queued skill proposal {}", proposal.id);
    Ok(())
}

pub async fn review_skill_proposal_cli(
    workspace_root: &Path,
    id: &str,
    request: &SkillProposalReviewRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposal = service
        .review(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Updated skill proposal {} to {}",
        proposal.id,
        skill_proposal_status_label(proposal.status)
    );
    Ok(())
}

pub async fn verify_skill_proposal_cli(
    workspace_root: &Path,
    id: &str,
    request: &SkillProposalVerifyRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposal = service
        .verify(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Verified skill proposal {} as {}",
        proposal.id,
        skill_proposal_verification_label(proposal.verification_status)
    );
    Ok(())
}

pub async fn install_skill_proposal_cli(
    workspace_root: &Path,
    id: &str,
    request: &SkillProposalInstallRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let report = service
        .install(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Installed skill proposal {} as {}",
        report.proposal.id, report.installed_skill_name
    );
    Ok(())
}

pub async fn rollback_skill_proposal_cli(
    workspace_root: &Path,
    id: &str,
    request: &SkillProposalRollbackRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposal = service
        .rollback(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Rolled back skill proposal {} to {}",
        proposal.id,
        skill_proposal_status_label(proposal.status)
    );
    Ok(())
}

pub async fn quarantine_skill_proposal_cli(
    workspace_root: &Path,
    id: &str,
    request: &SkillProposalQuarantineRequest,
) -> Result<()> {
    let service = skill_proposal_service_for_workspace(workspace_root).await?;
    let proposal = service
        .quarantine(id, request)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!(
        "Quarantined skill proposal {}{}",
        proposal.id,
        if proposal.god_mode_origin {
            " (God Mode)"
        } else {
            ""
        }
    );
    Ok(())
}

fn anyhow_to_core(error: anyhow::Error) -> CoreError {
    CoreError::Internal(error.to_string())
}

pub fn parse_learning_status(value: &str) -> Result<LearningCandidateStatus> {
    match value {
        "pending_review" | "pending" => Ok(LearningCandidateStatus::PendingReview),
        "approved" | "approve" => Ok(LearningCandidateStatus::Approved),
        "rejected" | "reject" => Ok(LearningCandidateStatus::Rejected),
        "superseded" | "supersede" => Ok(LearningCandidateStatus::Superseded),
        "promoted" | "promote" => Ok(LearningCandidateStatus::Promoted),
        "rolled_back" | "rollback" => Ok(LearningCandidateStatus::RolledBack),
        other => anyhow::bail!("unknown learning candidate status '{other}'"),
    }
}

pub fn parse_skill_proposal_status(value: &str) -> Result<SkillProposalStatus> {
    match value {
        "pending_review" | "pending" => Ok(SkillProposalStatus::PendingReview),
        "approved" | "approve" => Ok(SkillProposalStatus::Approved),
        "rejected" | "reject" => Ok(SkillProposalStatus::Rejected),
        "superseded" | "supersede" => Ok(SkillProposalStatus::Superseded),
        "installed" | "install" => Ok(SkillProposalStatus::Installed),
        "rolled_back" | "rollback" => Ok(SkillProposalStatus::RolledBack),
        other => anyhow::bail!("unknown skill proposal status '{other}'"),
    }
}

pub fn parse_learning_review_action(value: &str) -> Result<LearningCandidateReviewAction> {
    match value {
        "approved" | "approve" => Ok(LearningCandidateReviewAction::Approve),
        "rejected" | "reject" => Ok(LearningCandidateReviewAction::Reject),
        "superseded" | "supersede" => Ok(LearningCandidateReviewAction::Supersede),
        other => anyhow::bail!("unknown learning review action '{other}'"),
    }
}

pub fn parse_skill_proposal_review_action(value: &str) -> Result<SkillProposalReviewAction> {
    match value {
        "approved" | "approve" => Ok(SkillProposalReviewAction::Approve),
        "rejected" | "reject" => Ok(SkillProposalReviewAction::Reject),
        "superseded" | "supersede" => Ok(SkillProposalReviewAction::Supersede),
        other => anyhow::bail!("unknown skill proposal review action '{other}'"),
    }
}

pub fn parse_learning_impact(value: &str) -> Result<LearningCandidateImpact> {
    match value {
        "standard" => Ok(LearningCandidateImpact::Standard),
        "high" => Ok(LearningCandidateImpact::High),
        other => anyhow::bail!("unknown learning candidate impact '{other}'"),
    }
}

pub fn parse_learning_source_kind(value: &str) -> Result<LearningCandidateSourceKind> {
    match value {
        "reflection_candidate" | "reflection" => {
            Ok(LearningCandidateSourceKind::ReflectionCandidate)
        }
        "audit_record" | "audit" => Ok(LearningCandidateSourceKind::AuditRecord),
        "runtime_event" | "event" => Ok(LearningCandidateSourceKind::RuntimeEvent),
        "model_artifact" | "artifact" => Ok(LearningCandidateSourceKind::ModelArtifact),
        "manual" => Ok(LearningCandidateSourceKind::Manual),
        other => anyhow::bail!("unknown learning candidate source kind '{other}'"),
    }
}

pub fn parse_skill_proposal_source_kind(value: &str) -> Result<SkillProposalSourceKind> {
    match value {
        "learning_candidate" | "candidate" => Ok(SkillProposalSourceKind::LearningCandidate),
        "lesson" => Ok(SkillProposalSourceKind::Lesson),
        "manual" => Ok(SkillProposalSourceKind::Manual),
        other => anyhow::bail!("unknown skill proposal source kind '{other}'"),
    }
}

fn learning_status_label(value: LearningCandidateStatus) -> &'static str {
    match value {
        LearningCandidateStatus::PendingReview => "pending_review",
        LearningCandidateStatus::Approved => "approved",
        LearningCandidateStatus::Rejected => "rejected",
        LearningCandidateStatus::Superseded => "superseded",
        LearningCandidateStatus::Promoted => "promoted",
        LearningCandidateStatus::RolledBack => "rolled_back",
    }
}

fn skill_proposal_status_label(value: SkillProposalStatus) -> &'static str {
    match value {
        SkillProposalStatus::PendingReview => "pending_review",
        SkillProposalStatus::Approved => "approved",
        SkillProposalStatus::Rejected => "rejected",
        SkillProposalStatus::Superseded => "superseded",
        SkillProposalStatus::Installed => "installed",
        SkillProposalStatus::RolledBack => "rolled_back",
    }
}

fn skill_proposal_verification_label(value: SkillProposalVerificationStatus) -> &'static str {
    match value {
        SkillProposalVerificationStatus::Pending => "pending",
        SkillProposalVerificationStatus::Passed => "passed",
        SkillProposalVerificationStatus::Failed => "failed",
        SkillProposalVerificationStatus::Blocked => "blocked",
    }
}

fn learning_impact_label(value: LearningCandidateImpact) -> &'static str {
    match value {
        LearningCandidateImpact::Standard => "standard",
        LearningCandidateImpact::High => "high",
    }
}

fn learning_source_kind_label(value: LearningCandidateSourceKind) -> &'static str {
    match value {
        LearningCandidateSourceKind::ReflectionCandidate => "reflection_candidate",
        LearningCandidateSourceKind::AuditRecord => "audit_record",
        LearningCandidateSourceKind::RuntimeEvent => "runtime_event",
        LearningCandidateSourceKind::ModelArtifact => "model_artifact",
        LearningCandidateSourceKind::Manual => "manual",
    }
}

fn skill_proposal_source_kind_label(value: SkillProposalSourceKind) -> &'static str {
    match value {
        SkillProposalSourceKind::LearningCandidate => "learning_candidate",
        SkillProposalSourceKind::Lesson => "lesson",
        SkillProposalSourceKind::Manual => "manual",
    }
}

fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content.as_bytes())
            .with_context(|| format!("Failed to write '{}'", path.display()))?;
    }
    Ok(())
}

fn write_yaml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml::to_string(value)?.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn read_yaml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("Failed to read '{}'", path.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("Failed to parse '{}'", path.display()))
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

fn slugify(input: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_valid_registry() {
        let dir = tempdir().unwrap();
        init(Some(dir.path().to_str().unwrap())).unwrap();
        let registry = load_registry(dir.path().to_path_buf()).unwrap();
        validate_registry(&registry).unwrap();
        assert!(registry.agent_profiles.contains_key("default"));
        assert!(registry.model_profiles.contains_key("core-groq"));
        assert!(registry.claws.contains_key("main"));
        assert!(runtime_artifact_path(dir.path()).exists());
    }

    #[test]
    fn assign_task_updates_runtime() {
        let dir = tempdir().unwrap();
        init(Some(dir.path().to_str().unwrap())).unwrap();
        assign_task(Some(dir.path().to_str().unwrap()), "daily-review", "main").unwrap();
        let registry = load_registry(dir.path().to_path_buf()).unwrap();
        assert_eq!(
            registry
                .runtime
                .unwrap()
                .task_assignments
                .get("daily-review")
                .cloned(),
            Some("main".to_string())
        );
    }

    #[test]
    fn create_lesson_persists_in_registry() {
        let dir = tempdir().unwrap();
        init(Some(dir.path().to_str().unwrap())).unwrap();
        create_lesson(
            Some(dir.path().to_str().unwrap()),
            NewLessonInput {
                id: "prefer-local-fallback",
                active: true,
                signal: "provider timeout",
                recommendation: "prefer local fallback on repeated timeouts",
                rationale: None,
                confidence: 0.9,
                source: None,
                task_id: None,
                category: Some("code"),
                claw_id: Some("main"),
                model_profile_id: Some("core-groq"),
                provider: Some("groq"),
                autonomy_level: Some("managed"),
                execution_mode: Some("solo_claw"),
            },
        )
        .unwrap();
        let registry = load_registry(dir.path().to_path_buf()).unwrap();
        let lesson = registry.lessons.get("prefer-local-fallback").unwrap();
        assert!(lesson.active);
        assert_eq!(lesson.scope.category.as_deref(), Some("code"));
        assert_eq!(lesson.scope.claw_id.as_deref(), Some("main"));
    }
}
