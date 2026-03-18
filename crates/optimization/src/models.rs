use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Skill,
    RagPolicy,
    ContextAssembly,
    PromptPolicy,
    ToolPolicy,
    MemoryPolicy,
    SchedulerPolicy,
    RoutingPolicy,
    WorkflowDefinition,
    BoundedCode,
    ResearchProgram,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTier {
    RustNative,
    CompatSidecar,
    ExperimentalLanggraph,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    SafeConfig,
    BoundedWorkflow,
    BoundedCode,
    HumanReviewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShipStatus {
    Shipped,
    Gated,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Draft,
    Approved,
    Running,
    Passed,
    Failed,
    Rejected,
    PromotedExperimental,
    PromotedCompat,
    QueuedRustMerge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromotionDecision {
    Reject,
    KeepCandidate,
    Approve,
    PromoteExperimental,
    PromoteCompat,
    QueueRustMerge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationPolicy {
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub allowed_fields: Vec<String>,
    pub max_changed_files: usize,
    pub max_total_bytes: usize,
    pub max_diff_lines: usize,
    pub mandatory_evals: Vec<String>,
    pub required_tests: Vec<String>,
}

impl Default for MutationPolicy {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            forbidden_paths: vec![".git".to_string(), "target".to_string(), ".env".to_string()],
            allowed_fields: Vec::new(),
            max_changed_files: 8,
            max_total_bytes: 32_768,
            max_diff_lines: 400,
            mandatory_evals: Vec::new(),
            required_tests: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationSpec {
    pub name: String,
    pub command: EvaluationCommand,
    pub working_directory: Option<String>,
    pub timeout_secs: Option<u64>,
    pub success_metric: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromotionPolicy {
    pub allow_auto_promote_experimental: bool,
    pub allow_auto_promote_compat: bool,
    pub require_human_review_for_rust_merge: bool,
    pub minimum_success_ratio: f64,
    pub max_regressions: u32,
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            allow_auto_promote_experimental: false,
            allow_auto_promote_compat: false,
            require_human_review_for_rust_merge: true,
            minimum_success_ratio: 1.0,
            max_regressions: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetRegistration {
    pub name: String,
    pub description: Option<String>,
    pub target_kind: TargetKind,
    pub execution_tier: ExecutionTier,
    pub risk_class: RiskClass,
    pub ship_status: ShipStatus,
    pub workspace_root: String,
    pub mutation_policy: MutationPolicy,
    pub eval_suite: Vec<EvaluationSpec>,
    pub promotion_policy: PromotionPolicy,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetUpdate {
    pub description: Option<String>,
    pub execution_tier: Option<ExecutionTier>,
    pub risk_class: Option<RiskClass>,
    pub ship_status: Option<ShipStatus>,
    pub workspace_root: Option<String>,
    pub mutation_policy: Option<MutationPolicy>,
    pub eval_suite: Option<Vec<EvaluationSpec>>,
    pub promotion_policy: Option<PromotionPolicy>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationTarget {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_kind: TargetKind,
    pub execution_tier: ExecutionTier,
    pub risk_class: RiskClass,
    pub ship_status: ShipStatus,
    pub workspace_root: String,
    pub mutation_policy: MutationPolicy,
    pub eval_suite: Vec<EvaluationSpec>,
    pub promotion_policy: PromotionPolicy,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateChange {
    pub path: String,
    pub new_content: String,
    pub summary: Option<String>,
    pub field_path: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationCandidate {
    pub id: String,
    pub target_id: String,
    pub hypothesis: String,
    pub proposed_by: String,
    pub changes: Vec<CandidateChange>,
    pub status: CandidateStatus,
    pub diff_summary: Option<serde_json::Value>,
    pub result_summary: Option<serde_json::Value>,
    pub artifact_manifest: serde_json::Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateEvaluationRecord {
    pub id: String,
    pub candidate_id: String,
    pub eval_name: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub duration_ms: i64,
    pub stdout: String,
    pub stderr: String,
    pub metrics: serde_json::Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromotionEvent {
    pub id: String,
    pub candidate_id: String,
    pub decision: PromotionDecision,
    pub decided_by: String,
    pub notes: Option<String>,
    pub rollback_reference: Option<String>,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentMetrics {
    pub success_count: usize,
    pub failure_count: usize,
    pub required_eval_count: usize,
    pub required_success_count: usize,
    pub total_duration_ms: i64,
    pub status: CandidateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateRunSummary {
    pub candidate: OptimizationCandidate,
    pub evaluations: Vec<CandidateEvaluationRecord>,
    pub metrics: ExperimentMetrics,
}
