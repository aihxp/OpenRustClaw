use anyhow::Result;
use chrono::Utc;
use openrustclaw_app::assistant_continuity::AssistantContinuityService;
use openrustclaw_app::enterprise_admin::{
    EnterpriseAdminAccessState as AppEnterpriseAdminAccessState,
    EnterpriseAdminAutonomyState as AppEnterpriseAdminAutonomyState,
    EnterpriseAdminPolicyState as AppEnterpriseAdminPolicyState,
    EnterpriseAdminRunState as AppEnterpriseAdminRunState, EnterpriseAdminService,
    EnterpriseAdminState as AppEnterpriseAdminState,
    EnterpriseAdminSupervisionSummary as AppEnterpriseAdminSupervisionSummary,
};
use openrustclaw_app::greenfield_progress::{GreenfieldProgressReport, GreenfieldProgressService};
use openrustclaw_app::self_hosted_product::{
    SelfHostedProductModeControlService, SelfHostedProductModeReport, SelfHostedProductModeService,
    SelfHostedProductModeSource, SelfHostedProductModeState,
    SelfHostedProductModeTransitionExecutor, SelfHostedProductModeTransitionRequest,
    SelfHostedProductTransitionEvent as AppSelfHostedProductTransitionEvent,
};
use openrustclaw_app::setup_handoff::{
    RemoteConnectivityProfile as AppRemoteConnectivityProfile,
    SetupBootstrapOutcome as AppSetupBootstrapOutcome, SetupHandoffReport, SetupHandoffService,
    SetupHandoffState, SetupHandoffStateSource, SetupStatus,
};
use openrustclaw_app::tool_execution_audit::ToolExecutionAuditService;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::Error as CoreError;
use openrustclaw_core::types::{
    MemoryEntry, Message, ModelArtifact, ModelArtifactProjection, RecallPack,
};
use openrustclaw_db::models::MemoryArchiveRow;
use openrustclaw_db::{
    PersistedSession, SessionStatus, SqliteCoreMemoryStore, SqliteMemoryStore, SqlitePool,
    SqliteSessionStore,
};
use openrustclaw_memory::ModelArtifactService;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::{
    browser, control, enterprise_access, enterprise_autonomy, enterprise_policy, mobile, onboard,
    orchestrate, self_hosted, skills, talk, voice_runtime,
};

pub use openrustclaw_app::assistant_continuity::AssistantContinuitySummary;
pub use openrustclaw_app::tool_execution_audit::{ToolExecutionHistoryReport, ToolExecutionRecord};

#[derive(Debug, Clone, Serialize)]
pub struct SessionListEntry {
    pub session: PersistedSession,
    pub continuity: AssistantContinuitySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDetailReport {
    pub session: Option<PersistedSession>,
    pub history: Vec<Message>,
    pub continuity: Option<AssistantContinuitySummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryTimelineReport {
    pub namespace: Option<String>,
    pub entries: Vec<MemoryEntry>,
    #[serde(default)]
    pub recent_searches: Vec<MemorySearchInspectionReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryArchiveReport {
    pub namespace: Option<String>,
    pub entries: Vec<MemoryArchiveRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelArtifactInspectionReport {
    pub namespace: Option<String>,
    pub artifacts: Vec<ModelArtifact>,
    #[serde(default)]
    pub projected_core_entries: Vec<ModelArtifactProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchInspectionReport {
    pub query: String,
    pub namespace: Option<String>,
    pub result_count: usize,
    pub degraded: bool,
    pub recall_pack: RecallPack,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub state: String,
    pub timezone: Option<String>,
    pub max_retries: i64,
    pub priority: i64,
    pub source_kind: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub disabled_until: Option<String>,
    pub manifest_path: Option<String>,
    pub task_notes_path: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub run_count: i64,
    pub consecutive_failures: i64,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobRunSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub retry_count: i64,
    pub langsmith_trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeadLetterSummary {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub last_error: String,
    pub failed_at: String,
    pub retry_count: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobDetailReport {
    pub job: Option<JobSummary>,
    pub recent_runs: Vec<JobRunSummary>,
    pub dead_letters: Vec<DeadLetterSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterprisePolicyBoundary {
    pub autonomy_level: String,
    pub approval_policy: String,
    pub approval_scope_detail: String,
    pub operator_review_required_for_side_effects: bool,
    pub mobile_command_detail: String,
    pub browser_backend_detail: String,
    pub allowed_browser_backends: Vec<String>,
    pub allow_local_cli_wrappers: bool,
    pub allow_cloud_agent_execution: bool,
    pub browser_audit_log_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAuditEntry {
    pub observed_at: String,
    pub source: String,
    pub kind: String,
    pub status: String,
    pub summary: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseFoundationsReport {
    pub status: String,
    pub detail: String,
    pub explicit_boundary: bool,
    pub policy: EnterprisePolicyBoundary,
    pub mobile_metrics: mobile::MobileCommandMetricsSummary,
    pub recent_events: Vec<EnterpriseAuditEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAccessOrganizationSummary {
    pub id: String,
    pub name: String,
    pub slug: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAccessOperatorSummary {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: String,
    pub active: bool,
    pub scope_count: usize,
    pub scopes: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseRoleGrantSummary {
    pub role: String,
    pub default_scopes: Vec<String>,
    pub operator_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseGovernanceRuleSummary {
    pub scope: String,
    pub approval_mode: String,
    pub requester_roles: Vec<String>,
    pub approver_roles: Vec<String>,
    pub forbid_self_approval: bool,
    pub active: bool,
    pub eligible_requester_count: usize,
    pub eligible_approver_count: usize,
    pub coverage_status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseGovernanceReport {
    pub status: String,
    pub detail: String,
    pub updated_at: String,
    pub approver_id_header: String,
    pub approver_token_header: String,
    pub active_rule_count: usize,
    pub dual_approval_rule_count: usize,
    pub rules: Vec<EnterpriseGovernanceRuleSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAccessReport {
    pub status: String,
    pub detail: String,
    pub explicit_identity_required: bool,
    pub registry_path: String,
    pub operator_id_header: String,
    pub operator_token_header: String,
    pub organization: Option<EnterpriseAccessOrganizationSummary>,
    pub operators: Vec<EnterpriseAccessOperatorSummary>,
    pub role_grants: Vec<EnterpriseRoleGrantSummary>,
    pub governance: EnterpriseGovernanceReport,
    pub protected_routes: Vec<enterprise_access::EnterpriseProtectedRoute>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAdminSupervisionSummary {
    pub active_run_count: usize,
    pub attention_required_count: usize,
    pub escalated_count: usize,
    pub rollback_requested_count: usize,
    pub route_hint: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAdminReport {
    pub status: String,
    pub detail: String,
    pub requires_operator_headers: bool,
    pub access: EnterpriseAccessReport,
    pub policy: enterprise_policy::EnterprisePolicyReport,
    pub autonomy: enterprise_autonomy::EnterpriseAutonomyReport,
    pub supervision: EnterpriseAdminSupervisionSummary,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoiceOperatorReportRequest {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub stale_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceProviderCoverage {
    pub stt_total: usize,
    pub stt_ready: usize,
    pub tts_total: usize,
    pub tts_ready: usize,
    pub catalog_voice_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOperatorLaneSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub stale_sessions: usize,
    pub paused_sessions: usize,
    pub attention_needed: usize,
    pub total_turns: usize,
    pub total_artifacts: usize,
    pub avg_turns_per_session: f64,
    #[serde(default)]
    pub avg_session_duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TalkOperatorLaneSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub ended_sessions: usize,
    pub error_sessions: usize,
    pub total_turns: usize,
    pub total_events: usize,
    pub avg_turns_per_session: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceCallOperatorLaneSummary {
    pub total_calls: usize,
    pub active_calls: usize,
    pub stale_calls: usize,
    pub ended_calls: usize,
    pub reaped_calls: usize,
    pub reconnects: usize,
    pub with_greeting_audio: usize,
    #[serde(default)]
    pub oldest_active_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOperatorAttentionSignal {
    pub kind: String,
    pub severity: String,
    pub summary: String,
    pub detail: String,
    pub route_hint: String,
    #[serde(default)]
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOperatorRecentActivity {
    pub kind: String,
    pub observed_at: String,
    pub status: String,
    pub label: String,
    pub detail: String,
    #[serde(default)]
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOperatorReport {
    pub status: String,
    pub detail: String,
    pub voice_status: voice_runtime::VoiceStatus,
    pub provider_coverage: VoiceProviderCoverage,
    pub voice_lane: VoiceOperatorLaneSummary,
    pub talk_lane: TalkOperatorLaneSummary,
    pub bounded_call_lane: VoiceCallOperatorLaneSummary,
    pub attention_signals: Vec<VoiceOperatorAttentionSignal>,
    pub recent_activity: Vec<VoiceOperatorRecentActivity>,
}

pub async fn list_sessions(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<SessionListEntry>> {
    let store = SqliteSessionStore::new(pool.clone());
    let parsed = status.and_then(parse_session_status);
    let sessions = store.list_sessions(parsed, limit.max(1)).await?;
    let mut entries = Vec::with_capacity(sessions.len());
    for session in sessions {
        let history_messages =
            count_history_messages(pool, &session.session.id.to_string()).await?;
        entries.push(SessionListEntry {
            continuity: assistant_continuity_summary(&session, history_messages),
            session,
        });
    }
    Ok(entries)
}

pub async fn inspect_session(
    pool: &SqlitePool,
    id: &str,
    history_limit: usize,
) -> Result<SessionDetailReport> {
    let store = SqliteSessionStore::new(pool.clone());
    let session = store.get_session(id).await?;
    let history = if session.is_some() {
        store.list_history(id, history_limit.max(1)).await?
    } else {
        Vec::new()
    };
    let continuity = match session.as_ref() {
        Some(session) => Some(assistant_continuity_summary(
            session,
            count_history_messages(pool, id).await?,
        )),
        None => None,
    };
    Ok(SessionDetailReport {
        session,
        history,
        continuity,
    })
}

pub fn assistant_continuity_summary(
    session: &PersistedSession,
    history_messages: usize,
) -> AssistantContinuitySummary {
    AssistantContinuityService::summarize(
        &session.session.metadata,
        session.route_key.as_deref(),
        history_messages,
    )
}

async fn count_history_messages(pool: &SqlitePool, session_id: &str) -> Result<usize> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await?;
    Ok(count.max(0) as usize)
}

fn approval_scope_detail(approval_policy: &str) -> String {
    match approval_policy {
        "always" => {
            "Autonomous runtime actions require explicit approval before execution.".to_string()
        }
        "none" => {
            "Autonomous runtime actions can execute without an approval stop. The policy is explicit, but it is intentionally permissive.".to_string()
        }
        _ => {
            "Autonomous runtime actions may continue until a side effect is requested, then operator approval is required.".to_string()
        }
    }
}

fn browser_backend_policy_detail(policy: &browser::ExternalBackendPolicy) -> String {
    let backends = if policy.allowed_backends.is_empty() {
        "no external backends".to_string()
    } else {
        policy.allowed_backends.join(", ")
    };
    let local = if policy.allow_local_cli_wrappers {
        "local wrappers allowed"
    } else {
        "local wrappers blocked"
    };
    let cloud = if policy.allow_cloud_agent_execution {
        "cloud execution allowed"
    } else {
        "cloud execution blocked"
    };
    format!("Allowed backends: {backends}; {local}; {cloud}.")
}

pub async fn memory_timeline(
    store: &SqliteMemoryStore,
    pool: &SqlitePool,
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryTimelineReport> {
    let namespace_value = namespace.map(|value| value.to_string());
    let entries = store.list_recent(namespace, limit.max(1)).await?;
    let search_rows = sqlx::query(
        r#"
        SELECT payload, created_at
        FROM runtime_events
        WHERE event_name = 'memory.searched'
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit.max(1) as i64)
    .fetch_all(pool)
    .await?;

    let mut recent_searches = Vec::new();
    for row in search_rows {
        let payload = row.get::<String, _>("payload");
        let payload: serde_json::Value =
            serde_json::from_str(&payload).unwrap_or_else(|_| serde_json::json!({}));
        let payload_namespace = payload
            .get("namespace")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        if namespace_value.as_ref().is_some()
            && payload_namespace.as_deref() != namespace_value.as_deref()
        {
            continue;
        }

        let recall_pack: RecallPack = payload
            .get("recall_pack")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        recent_searches.push(MemorySearchInspectionReport {
            query: payload
                .get("query")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            namespace: payload_namespace,
            result_count: payload
                .get("result_count")
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as usize,
            degraded: recall_pack.degraded,
            recall_pack,
            created_at: row.get("created_at"),
        });
    }

    Ok(MemoryTimelineReport {
        namespace: namespace_value,
        entries,
        recent_searches,
    })
}

pub async fn memory_model_artifacts(
    store: &SqliteMemoryStore,
    core_store: &SqliteCoreMemoryStore,
    namespace: Option<&str>,
    include_inactive: bool,
    limit: usize,
) -> Result<ModelArtifactInspectionReport> {
    let service = ModelArtifactService::new(store.clone(), core_store.clone());
    let artifacts = service
        .list(namespace, include_inactive, limit.max(1))
        .await?;
    let projected_core_entries = if let Some(namespace) = namespace {
        service.projected_entries(namespace).await?
    } else {
        Vec::new()
    };

    Ok(ModelArtifactInspectionReport {
        namespace: namespace.map(str::to_string),
        artifacts,
        projected_core_entries,
    })
}

struct ToolExecutionAuditFileStore {
    workspace_root: PathBuf,
}

impl ToolExecutionAuditFileStore {
    fn new(workspace_root: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    fn path(&self) -> PathBuf {
        self.workspace_root
            .join(".claw")
            .join("control")
            .join("tool-executions.jsonl")
    }

    fn append(&self, record: &ToolExecutionRecord) -> Result<()> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn read_all(&self) -> Result<Vec<ToolExecutionRecord>> {
        let path = self.path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = OpenOptions::new().read(true).open(&path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(record) = serde_json::from_str::<ToolExecutionRecord>(&line) else {
                continue;
            };
            entries.push(record);
        }
        Ok(entries)
    }
}

#[cfg(test)]
pub fn tool_execution_log_path(workspace_root: &Path) -> PathBuf {
    ToolExecutionAuditFileStore::new(workspace_root).path()
}

pub fn append_tool_execution_record(
    workspace_root: &Path,
    record: &ToolExecutionRecord,
) -> Result<()> {
    ToolExecutionAuditFileStore::new(workspace_root).append(record)
}

pub fn tool_execution_history(
    workspace_root: &Path,
    limit: usize,
    source: Option<&str>,
    status: Option<&str>,
    tool_name: Option<&str>,
) -> Result<ToolExecutionHistoryReport> {
    let records = ToolExecutionAuditFileStore::new(workspace_root).read_all()?;
    Ok(ToolExecutionAuditService::history(
        &records, limit, source, status, tool_name,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn new_tool_execution_record(
    tool_name: impl Into<String>,
    source: impl Into<String>,
    status: impl Into<String>,
    status_detail: impl Into<String>,
    duration_ms: u64,
    error: Option<String>,
    artifact_path: Option<String>,
    args: Option<serde_json::Value>,
    result_preview: Option<serde_json::Value>,
) -> ToolExecutionRecord {
    let tool_name = tool_name.into();
    let source = source.into();
    let status = status.into();
    let status_detail = status_detail.into();
    ToolExecutionAuditService::new_record(
        &tool_name,
        &source,
        &status,
        &status_detail,
        duration_ms,
        error.as_deref(),
        artifact_path.as_deref(),
        args,
        result_preview,
    )
}

fn collect_mobile_command_audit_entries(
    workspace_root: &Path,
) -> Result<Vec<EnterpriseAuditEntry>> {
    let commands = mobile::list_command_data(workspace_root, None, Some(24))?;
    Ok(commands
        .into_iter()
        .map(|command| {
            let observed_at = command
                .executed_at
                .as_ref()
                .or(command.approved_at.as_ref())
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| command.created_at.to_rfc3339());
            let summary = match command.status.as_str() {
                "pending_approval" => format!(
                    "{} for node {} is waiting for operator approval.",
                    command.command, command.node_id
                ),
                "rejected" => format!(
                    "{} for node {} was rejected by {}.",
                    command.command,
                    command.node_id,
                    command.approved_by.as_deref().unwrap_or("operator")
                ),
                "approved" => format!(
                    "{} for node {} was approved by {}.",
                    command.command,
                    command.node_id,
                    command.approved_by.as_deref().unwrap_or("operator")
                ),
                "executed" if command.approval_required => format!(
                    "{} for node {} executed after approval by {}.",
                    command.command,
                    command.node_id,
                    command.approved_by.as_deref().unwrap_or("operator")
                ),
                "executed" => format!(
                    "{} for node {} executed under the current mobile command policy.",
                    command.command, command.node_id
                ),
                other => format!(
                    "{} for node {} is recorded with status {}.",
                    command.command, command.node_id, other
                ),
            };
            let detail = format!(
                "approval_required={} required_capability={} decided_reason={}",
                command.approval_required,
                command.required_capability,
                command.decided_reason.as_deref().unwrap_or("-")
            );
            EnterpriseAuditEntry {
                observed_at,
                source: "mobile_command".to_string(),
                kind: command.command.to_string(),
                status: command.status,
                summary,
                detail,
            }
        })
        .collect())
}

fn collect_browser_backend_audit_entries(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<EnterpriseAuditEntry>> {
    Ok(browser::list_backend_audit(workspace_root, limit.max(1))?
        .into_iter()
        .map(|entry| {
            let status = if !entry.allowed {
                "blocked".to_string()
            } else if entry.success {
                "success".to_string()
            } else {
                "failed".to_string()
            };
            let summary = if !entry.allowed {
                format!(
                    "{} via {} was blocked by browser backend policy.",
                    entry.action, entry.backend
                )
            } else if entry.success {
                format!(
                    "{} via {} succeeded under the current browser backend policy.",
                    entry.action, entry.backend
                )
            } else {
                format!(
                    "{} via {} was allowed but failed.",
                    entry.action, entry.backend
                )
            };
            let detail = entry
                .detail
                .clone()
                .or_else(|| {
                    entry
                        .session_id
                        .clone()
                        .map(|value| format!("session={value}"))
                })
                .unwrap_or_else(|| "No extra detail recorded.".to_string());
            EnterpriseAuditEntry {
                observed_at: entry.timestamp,
                source: "browser_backend".to_string(),
                kind: entry.action,
                status,
                summary,
                detail,
            }
        })
        .collect())
}

fn collect_sensitive_tool_audit_entries(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<EnterpriseAuditEntry>> {
    const SENSITIVE_TOOLS: &[&str] = &[
        "mobile.command.dispatch",
        "mobile.command.approve",
        "mobile.command.reject",
        "mobile.capability.execute",
    ];

    Ok(
        tool_execution_history(workspace_root, limit.max(1) * 3, None, None, None)?
            .entries
            .into_iter()
            .filter(|entry| SENSITIVE_TOOLS.contains(&entry.tool_name.as_str()))
            .take(limit.max(1))
            .map(|entry| {
                let detail = entry
                    .error
                    .clone()
                    .or_else(|| entry.artifact_path.clone())
                    .unwrap_or_else(|| entry.status_detail.clone());
                EnterpriseAuditEntry {
                    observed_at: entry.created_at,
                    source: entry.source,
                    kind: entry.tool_name.clone(),
                    status: entry.status.clone(),
                    summary: format!("{} recorded as {}.", entry.tool_name, entry.status_detail),
                    detail,
                }
            })
            .collect(),
    )
}

pub fn enterprise_foundations_summary(
    workspace_root: &Path,
    limit: usize,
) -> Result<EnterpriseFoundationsReport> {
    let default_autonomy = control::AutonomyPolicy::default();
    let control_description = control::describe_registry(workspace_root.to_path_buf()).ok();
    let autonomy_level = control_description
        .as_ref()
        .and_then(|value| value.get("autonomy"))
        .and_then(|value| value.get("autonomy_level"))
        .and_then(|value| value.as_str())
        .unwrap_or(&default_autonomy.autonomy_level)
        .to_string();
    let approval_policy = control_description
        .as_ref()
        .and_then(|value| value.get("autonomy"))
        .and_then(|value| value.get("approval_policy"))
        .and_then(|value| value.as_str())
        .unwrap_or(&default_autonomy.approval_policy)
        .to_string();

    let browser_policy = browser::backend_policy(workspace_root)?;
    let mobile_metrics = mobile::command_metrics_data(workspace_root, None, None)?.metrics;
    let mut recent_events = collect_mobile_command_audit_entries(workspace_root)?;
    recent_events.extend(collect_browser_backend_audit_entries(
        workspace_root,
        limit,
    )?);
    recent_events.extend(collect_sensitive_tool_audit_entries(workspace_root, limit)?);
    recent_events.sort_by(|left, right| right.observed_at.cmp(&left.observed_at));
    recent_events.truncate(limit.max(1));

    let policy = EnterprisePolicyBoundary {
        autonomy_level: autonomy_level.clone(),
        approval_policy: approval_policy.clone(),
        approval_scope_detail: approval_scope_detail(&approval_policy),
        operator_review_required_for_side_effects: approval_policy != "none",
        mobile_command_detail: if mobile_metrics.pending_approval_commands > 0 {
            format!(
                "Mobile command execution preserves an explicit operator gate: {} command(s) are currently waiting in pending approval.",
                mobile_metrics.pending_approval_commands
            )
        } else {
            "Mobile command execution preserves an explicit operator gate through pending-approval, approve, and reject states.".to_string()
        },
        browser_backend_detail: browser_backend_policy_detail(&browser_policy),
        allowed_browser_backends: browser_policy.allowed_backends.clone(),
        allow_local_cli_wrappers: browser_policy.allow_local_cli_wrappers,
        allow_cloud_agent_execution: browser_policy.allow_cloud_agent_execution,
        browser_audit_log_path: browser_policy.audit_log_path.clone(),
    };

    let detail = format!(
        "Runtime autonomy is `{}` with approval policy `{}`. Mobile command side effects stay reviewable through explicit approval states, and external browser backends are governed by an allowlisted policy with durable audit logging.",
        autonomy_level, approval_policy
    );

    Ok(EnterpriseFoundationsReport {
        status: "ok".to_string(),
        detail,
        explicit_boundary: true,
        policy,
        mobile_metrics,
        recent_events,
    })
}

struct WorkspaceSelfHostedProductModeSource<'a> {
    workspace_root: &'a Path,
    manifest_path: String,
    events_path: String,
}

impl<'a> WorkspaceSelfHostedProductModeSource<'a> {
    fn new(workspace_root: &'a Path) -> Self {
        Self {
            workspace_root,
            manifest_path: self_hosted::self_hosted_product_path(workspace_root)
                .display()
                .to_string(),
            events_path: self_hosted::self_hosted_product_events_path(workspace_root)
                .display()
                .to_string(),
        }
    }
}

impl SelfHostedProductModeSource for WorkspaceSelfHostedProductModeSource<'_> {
    fn load_self_hosted_product_mode_state(
        &self,
    ) -> openrustclaw_core::error::Result<SelfHostedProductModeState> {
        let manifest = self_hosted::load_manifest(self.workspace_root).map_err(|error| {
            CoreError::Internal(format!(
                "failed to load self-hosted product mode manifest: {error}"
            ))
        })?;

        let (explicit_mode_selected, profile) = if let Some(value) = manifest {
            (true, value.profile)
        } else {
            let descriptor = self_hosted::default_descriptor();
            (
                false,
                self_hosted::SelfHostedProductProfile {
                    mode: descriptor.mode.to_string(),
                    onboarding_path: descriptor.onboarding_path.to_string(),
                    self_hosted: true,
                    open_source: true,
                    note: None,
                    updated_at: String::new(),
                },
            )
        };
        let current_warnings = self_hosted::current_warnings(self.workspace_root, &profile.mode)
            .map_err(|error| {
                CoreError::Internal(format!(
                    "failed to collect self-hosted product mode warnings: {error}"
                ))
            })?;
        let recent_transitions = self_hosted::recent_transition_events(self.workspace_root, 8)
            .map_err(|error| {
                CoreError::Internal(format!(
                    "failed to load self-hosted product mode transitions: {error}"
                ))
            })?
            .into_iter()
            .map(map_self_hosted_transition_event)
            .collect();

        Ok(SelfHostedProductModeState {
            manifest_path: self.manifest_path.clone(),
            events_path: self.events_path.clone(),
            explicit_mode_selected,
            self_hosted: profile.self_hosted,
            open_source: profile.open_source,
            mode: profile.mode,
            onboarding_path: profile.onboarding_path,
            current_warnings,
            recent_transitions,
        })
    }
}

impl SelfHostedProductModeTransitionExecutor for WorkspaceSelfHostedProductModeSource<'_> {
    fn transition_self_hosted_product_mode(
        &self,
        request: SelfHostedProductModeTransitionRequest,
    ) -> openrustclaw_core::error::Result<()> {
        self_hosted::transition_mode(
            self.workspace_root,
            self_hosted::SelfHostedProductTransitionRequest {
                target_mode: request.target_mode,
                actor: request.actor,
                reason: request.reason,
                via: Some(request.via),
            },
        )
        .map(|_| ())
        .map_err(|error| {
            CoreError::Internal(format!(
                "failed to transition self-hosted product mode: {error}"
            ))
        })
    }
}

fn map_self_hosted_transition_event(
    event: self_hosted::SelfHostedProductTransitionEvent,
) -> AppSelfHostedProductTransitionEvent {
    AppSelfHostedProductTransitionEvent {
        created_at: event.created_at,
        from_mode: event.from_mode,
        to_mode: event.to_mode,
        direction: event.direction,
        actor: event.actor,
        reason: event.reason,
        via: event.via,
        warnings: event.warnings,
    }
}

pub fn self_hosted_product_mode_summary(
    workspace_root: &Path,
) -> Result<SelfHostedProductModeReport> {
    let service = SelfHostedProductModeService::new(WorkspaceSelfHostedProductModeSource::new(
        workspace_root,
    ));
    service.report().map_err(Into::into)
}

pub fn transition_self_hosted_product_mode_summary(
    workspace_root: &Path,
    target_mode: String,
    actor: String,
    reason: Option<String>,
) -> Result<SelfHostedProductModeReport> {
    let service = SelfHostedProductModeControlService::new(
        WorkspaceSelfHostedProductModeSource::new(workspace_root),
    );
    service
        .transition_and_report(SelfHostedProductModeTransitionRequest {
            target_mode,
            actor,
            reason,
            via: "control_api".to_string(),
        })
        .map_err(Into::into)
}

pub fn greenfield_progress_summary() -> GreenfieldProgressReport {
    GreenfieldProgressService::new().report(Utc::now().to_rfc3339())
}

struct WorkspaceSetupHandoffSource<'a> {
    workspace_root: &'a Path,
    manifest_path: String,
}

impl<'a> WorkspaceSetupHandoffSource<'a> {
    fn new(workspace_root: &'a Path) -> Self {
        Self {
            workspace_root,
            manifest_path: onboard::setup_state_path(workspace_root)
                .display()
                .to_string(),
        }
    }
}

impl SetupHandoffStateSource for WorkspaceSetupHandoffSource<'_> {
    fn load_setup_handoff_state(
        &self,
    ) -> openrustclaw_core::error::Result<Option<SetupHandoffState>> {
        let setup_state = onboard::load_setup_state(self.workspace_root).map_err(|error| {
            CoreError::Internal(format!("failed to load durable setup state: {error}"))
        })?;
        Ok(setup_state.map(|manifest| map_setup_handoff_state(manifest, self.manifest_path())))
    }

    fn manifest_path(&self) -> String {
        self.manifest_path.clone()
    }
}

fn map_setup_status(setup: &onboard::SetupState) -> SetupStatus {
    match onboard::setup_handoff_status(setup) {
        "ready" => SetupStatus::Ready,
        "degraded" => SetupStatus::Degraded,
        "pending" => SetupStatus::Pending,
        _ => SetupStatus::NotStarted,
    }
}

fn map_remote_connectivity_profile(
    profile: onboard::RemoteConnectivityProfile,
) -> AppRemoteConnectivityProfile {
    AppRemoteConnectivityProfile {
        mode: profile.mode,
        primary_path: profile.primary_path,
        fallback_paths: profile.fallback_paths,
        detail: profile.detail,
    }
}

fn map_bootstrap_outcome(outcome: onboard::SetupBootstrapOutcome) -> AppSetupBootstrapOutcome {
    AppSetupBootstrapOutcome {
        category: outcome.category,
        target: outcome.target,
        status: outcome.status,
        detail: outcome.detail,
        issue_kind: outcome.issue_kind,
        verification_stage: outcome.verification_stage,
        suggested_action: outcome.suggested_action,
        updated_at: outcome.updated_at,
    }
}

fn map_setup_handoff_state(
    manifest: onboard::SetupStateManifest,
    manifest_path: String,
) -> SetupHandoffState {
    let setup = manifest.setup;
    let status = map_setup_status(&setup);
    let detail = onboard::setup_handoff_detail(&setup);
    let pending_steps = onboard::pending_step_names(&setup);

    SetupHandoffState {
        manifest_path,
        status,
        detail,
        explicit_setup_state: true,
        selected_provider: setup.selected_provider,
        selected_access_mode: setup.selected_access_mode,
        selected_primary_model: setup.selected_primary_model,
        selected_primary_model_source: setup.selected_primary_model_source,
        deployment_mode: setup.deployment_mode,
        deployment_path: setup.deployment_path,
        remote_connectivity_profile: setup
            .remote_connectivity_profile
            .map(map_remote_connectivity_profile),
        setup_path: setup.setup_path,
        workspace_action: Some(setup.workspace_action),
        current_step: setup.current_step,
        next_action: setup.next_action,
        selected_step_count: setup.selected_steps.len(),
        completed_step_count: setup.completed_steps.len(),
        pending_steps,
        blockers: setup.blockers,
        bootstrap_outcomes: setup
            .bootstrap_outcomes
            .into_iter()
            .map(map_bootstrap_outcome)
            .collect(),
    }
}

pub fn setup_handoff_summary(workspace_root: &Path) -> Result<SetupHandoffReport> {
    let service = SetupHandoffService::new(WorkspaceSetupHandoffSource::new(workspace_root));
    service.report().map_err(Into::into)
}

pub fn enterprise_access_summary(workspace_root: &Path) -> Result<EnterpriseAccessReport> {
    let registry_path = enterprise_access::enterprise_access_path(workspace_root)
        .display()
        .to_string();
    let protected_routes = enterprise_access::protected_routes();
    let role_defaults = enterprise_access::role_defaults();
    let manifest = enterprise_access::load_manifest(workspace_root)?;
    let explicit_identity_required = manifest.is_some();

    let detail = if let Some(manifest) = manifest.as_ref() {
        format!(
            "Enterprise access is bootstrapped for organization `{}` with {} operator(s). Sensitive control routes now require `{}` and `{}` headers when their scope boundary is active, and governed scopes can additionally require `{}` and `{}` for a second approver.",
            manifest.organization.id,
            manifest.operators.len(),
            enterprise_access::OPERATOR_ID_HEADER,
            enterprise_access::OPERATOR_TOKEN_HEADER,
            enterprise_access::APPROVER_ID_HEADER,
            enterprise_access::APPROVER_TOKEN_HEADER
        )
    } else {
        format!(
            "Enterprise access is not bootstrapped yet. Bootstrap the file-backed organization registry to require scoped operator identity on sensitive control routes via `{}` and `{}`, then layer governed dual-approval headers where the policy demands them.",
            enterprise_access::OPERATOR_ID_HEADER,
            enterprise_access::OPERATOR_TOKEN_HEADER
        )
    };

    let role_grants = role_defaults
        .into_iter()
        .map(|(role, default_scopes)| {
            let operator_count = manifest
                .as_ref()
                .map(|value| {
                    value
                        .operators
                        .iter()
                        .filter(|operator| operator.role == role)
                        .count()
                })
                .unwrap_or(0);
            EnterpriseRoleGrantSummary {
                role,
                default_scopes,
                operator_count,
            }
        })
        .collect::<Vec<_>>();

    let organization = manifest
        .as_ref()
        .map(|value| EnterpriseAccessOrganizationSummary {
            id: value.organization.id.clone(),
            name: value.organization.name.clone(),
            slug: value.organization.slug.clone(),
            created_at: value.organization.created_at.clone(),
        });

    let operators = manifest
        .as_ref()
        .map(|value| {
            value
                .operators
                .iter()
                .map(|operator| EnterpriseAccessOperatorSummary {
                    id: operator.id.clone(),
                    name: operator.name.clone(),
                    email: operator.email.clone(),
                    role: operator.role.clone(),
                    active: operator.active,
                    scope_count: operator.scopes.len(),
                    scopes: operator.scopes.clone(),
                    updated_at: operator.updated_at.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let governance_rules = manifest
        .as_ref()
        .map(|value| value.governance.rules.clone())
        .unwrap_or_else(enterprise_access::governance_defaults);
    let active_operator_roles = manifest
        .as_ref()
        .map(|value| {
            value
                .operators
                .iter()
                .filter(|entry| entry.active)
                .map(|entry| entry.role.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let governance_rule_summaries = governance_rules
        .iter()
        .map(|rule| {
            let eligible_requester_count = active_operator_roles
                .iter()
                .filter(|role| {
                    rule.requester_roles.is_empty()
                        || rule.requester_roles.iter().any(|allowed| allowed == *role)
                })
                .count();
            let eligible_approver_count = active_operator_roles
                .iter()
                .filter(|role| {
                    rule.approver_roles.is_empty()
                        || rule.approver_roles.iter().any(|allowed| allowed == *role)
                })
                .count();
            let dual_coverage_gap = rule.approval_mode == "dual"
                && (eligible_requester_count == 0
                    || eligible_approver_count == 0
                    || active_operator_roles.len() < 2);
            let coverage_status = if !rule.active {
                "disabled".to_string()
            } else if eligible_requester_count == 0 || dual_coverage_gap {
                "coverage_gap".to_string()
            } else {
                "ready".to_string()
            };

            EnterpriseGovernanceRuleSummary {
                scope: rule.scope.clone(),
                approval_mode: rule.approval_mode.clone(),
                requester_roles: rule.requester_roles.clone(),
                approver_roles: rule.approver_roles.clone(),
                forbid_self_approval: rule.forbid_self_approval,
                active: rule.active,
                eligible_requester_count,
                eligible_approver_count,
                coverage_status,
                detail: rule.detail.clone(),
            }
        })
        .collect::<Vec<_>>();
    let dual_approval_rule_count = governance_rule_summaries
        .iter()
        .filter(|rule| rule.active && rule.approval_mode == "dual")
        .count();
    let active_rule_count = governance_rule_summaries
        .iter()
        .filter(|rule| rule.active)
        .count();
    let governance = EnterpriseGovernanceReport {
        status: if explicit_identity_required {
            "ok".to_string()
        } else {
            "bootstrap_required".to_string()
        },
        detail: if explicit_identity_required {
            format!(
                "{} governed enterprise scope(s) are active, with {} requiring explicit dual approval through `{}` and `{}`.",
                active_rule_count,
                dual_approval_rule_count,
                enterprise_access::APPROVER_ID_HEADER,
                enterprise_access::APPROVER_TOKEN_HEADER
            )
        } else {
            format!(
                "Governance defaults are defined but inactive until enterprise access is bootstrapped. Dual-approval scopes will require `{}` and `{}` once the registry is live.",
                enterprise_access::APPROVER_ID_HEADER,
                enterprise_access::APPROVER_TOKEN_HEADER
            )
        },
        updated_at: manifest
            .as_ref()
            .map(|value| value.governance.updated_at.clone())
            .unwrap_or_default(),
        approver_id_header: enterprise_access::APPROVER_ID_HEADER.to_string(),
        approver_token_header: enterprise_access::APPROVER_TOKEN_HEADER.to_string(),
        active_rule_count,
        dual_approval_rule_count,
        rules: governance_rule_summaries,
    };

    Ok(EnterpriseAccessReport {
        status: if explicit_identity_required {
            "ok".to_string()
        } else {
            "bootstrap_required".to_string()
        },
        detail,
        explicit_identity_required,
        registry_path,
        operator_id_header: enterprise_access::OPERATOR_ID_HEADER.to_string(),
        operator_token_header: enterprise_access::OPERATOR_TOKEN_HEADER.to_string(),
        organization,
        operators,
        role_grants,
        governance,
        protected_routes,
    })
}

pub fn enterprise_admin_summary(
    workspace_root: &Path,
    config_path: &str,
) -> Result<EnterpriseAdminReport> {
    let access = enterprise_access_summary(workspace_root)?;
    let policy = enterprise_policy::summary(workspace_root, config_path)?;
    let autonomy = enterprise_autonomy::summary(workspace_root, 10)?;
    let active_runs = orchestrate::list_active_runs(workspace_root, true, 20)?;
    let presentation = EnterpriseAdminService::new(AppEnterpriseAdminState {
        access: AppEnterpriseAdminAccessState {
            organization_id: access.organization.as_ref().map(|value| value.id.clone()),
            operator_count: access.operators.len(),
            explicit_identity_required: access.explicit_identity_required,
        },
        policy: AppEnterpriseAdminPolicyState {
            approval_policy: policy.approval_policy.clone(),
        },
        autonomy: AppEnterpriseAdminAutonomyState {
            status: autonomy.status.clone(),
            governance_scope: autonomy.governance_scope.clone(),
        },
        active_runs: active_runs
            .iter()
            .map(|run| AppEnterpriseAdminRunState {
                intervention_required: run.lifecycle.intervention_required,
                pause_requested: run.pause_requested,
                kill_requested: run.kill_requested,
                has_last_error: run.last_error.is_some(),
                lifecycle_state: run.lifecycle.state.clone(),
                escalation_requested: run.lifecycle.escalation_requested,
                rollback_requested: run.lifecycle.rollback_requested,
            })
            .collect(),
    })
    .report()?;
    let supervision = map_enterprise_admin_supervision(presentation.supervision);

    Ok(EnterpriseAdminReport {
        status: presentation.status,
        detail: presentation.detail,
        requires_operator_headers: presentation.requires_operator_headers,
        access,
        policy,
        autonomy,
        supervision,
    })
}

fn map_enterprise_admin_supervision(
    summary: AppEnterpriseAdminSupervisionSummary,
) -> EnterpriseAdminSupervisionSummary {
    EnterpriseAdminSupervisionSummary {
        active_run_count: summary.active_run_count,
        attention_required_count: summary.attention_required_count,
        escalated_count: summary.escalated_count,
        rollback_requested_count: summary.rollback_requested_count,
        route_hint: summary.route_hint,
        detail: summary.detail,
    }
}

pub async fn voice_operator_report_summary(
    config: &AppConfig,
    workspace_root: &Path,
    limit: usize,
    stale_after_secs: Option<u64>,
) -> Result<VoiceOperatorReport> {
    let limit = limit.max(1);
    let voice_status = voice_runtime::voice_status(config, workspace_root);
    let provider_catalog = voice_runtime::voice_provider_catalog(config);
    let catalog_voice_count = voice_runtime::list_voices_with_config(config, workspace_root)
        .map(|result| result.voices.len())
        .unwrap_or(0);
    let voice_metrics = voice_runtime::voice_metrics(workspace_root).await?;
    let voice_outcomes =
        voice_runtime::voice_session_outcomes(workspace_root, stale_after_secs, Some(limit))
            .await?;
    let voice_health = voice_runtime::voice_session_health(
        workspace_root,
        voice_runtime::VoiceSessionHealthRequest { stale_after_secs },
    )
    .await?;
    let talk_status = talk::runtime_status(workspace_root, limit).await?;
    let talk_metrics = talk::runtime_metrics_data(workspace_root).await?;
    let voice_calls = skills::voice_calls_data_for(workspace_root).await?;
    let voice_call_health = skills::voice_call_health_data_for(workspace_root).await?;
    let voice_call_metrics = skills::voice_call_metrics_data_for(workspace_root).await?;

    let provider_coverage = VoiceProviderCoverage {
        stt_total: provider_catalog.stt.len(),
        stt_ready: provider_catalog
            .stt
            .iter()
            .filter(|provider| provider.api_key_present)
            .count(),
        tts_total: provider_catalog.tts.len(),
        tts_ready: provider_catalog
            .tts
            .iter()
            .filter(|provider| provider.api_key_present)
            .count(),
        catalog_voice_count,
    };

    let voice_lane = VoiceOperatorLaneSummary {
        total_sessions: voice_metrics.total_sessions,
        active_sessions: voice_metrics.active_sessions,
        ended_sessions: voice_metrics.ended_sessions,
        stale_sessions: voice_health.stale_sessions,
        paused_sessions: voice_outcomes
            .outcomes
            .iter()
            .filter(|entry| entry.outcome_label == "paused")
            .count(),
        attention_needed: voice_outcomes.attention_needed,
        total_turns: voice_metrics.total_turns,
        total_artifacts: voice_metrics.total_artifacts,
        avg_turns_per_session: voice_metrics.avg_turns_per_session,
        avg_session_duration_secs: voice_metrics.avg_session_duration_secs,
    };

    let talk_lane = TalkOperatorLaneSummary {
        total_sessions: talk_metrics.total_sessions,
        active_sessions: talk_metrics.active_sessions,
        ended_sessions: talk_metrics.ended_sessions,
        error_sessions: talk_metrics.error_sessions,
        total_turns: talk_metrics.total_turns,
        total_events: talk_metrics.total_events,
        avg_turns_per_session: talk_metrics.avg_turns_per_session,
    };

    let bounded_call_lane = VoiceCallOperatorLaneSummary {
        total_calls: voice_call_metrics.metrics.total,
        active_calls: voice_call_health.health.active,
        stale_calls: voice_call_health.health.stale,
        ended_calls: voice_call_health.health.ended,
        reaped_calls: voice_call_health.health.reaped,
        reconnects: voice_call_metrics.metrics.reconnects,
        with_greeting_audio: voice_call_metrics.metrics.with_greeting_audio,
        oldest_active_call_id: voice_call_health.health.oldest_active_call_id.clone(),
    };

    let attention_signals = build_voice_operator_attention_signals(
        &voice_status,
        &provider_coverage,
        &voice_outcomes,
        &talk_metrics,
        &voice_calls.calls,
    );
    let recent_activity = build_voice_operator_recent_activity(
        &voice_outcomes,
        &talk_status,
        &voice_calls.calls,
        limit,
    );

    let status = if !voice_status.enabled {
        "degraded"
    } else if attention_signals.is_empty() {
        "ok"
    } else {
        "attention"
    }
    .to_string();

    let detail = format!(
        "Voice runtime is {} with {} session(s) needing attention, {} talk error receipt(s), and {} stale bounded voice call(s).",
        if voice_status.enabled {
            "enabled"
        } else {
            "disabled"
        },
        voice_outcomes.attention_needed,
        talk_metrics.error_sessions,
        voice_call_health.health.stale
    );

    Ok(VoiceOperatorReport {
        status,
        detail,
        voice_status,
        provider_coverage,
        voice_lane,
        talk_lane,
        bounded_call_lane,
        attention_signals,
        recent_activity,
    })
}

pub async fn memory_namespaces(store: &SqliteMemoryStore) -> Result<Vec<String>> {
    Ok(store.list_namespaces().await?)
}

pub async fn memory_archive(
    store: &SqliteMemoryStore,
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryArchiveReport> {
    let namespace_value = namespace.map(|value| value.to_string());
    let entries = store.list_archive_entries(namespace, limit.max(1)).await?;
    Ok(MemoryArchiveReport {
        namespace: namespace_value,
        entries,
    })
}

pub async fn list_jobs(
    pool: &SqlitePool,
    state: Option<&str>,
    limit: usize,
) -> Result<Vec<JobSummary>> {
    let mut sql = String::from(
        r#"
        SELECT id, name, description, workflow_id, trigger_type, trigger_config,
               state, timezone, max_retries, priority, source_kind, owner,
               tags, disabled_until, manifest_path, task_notes_path,
               next_run_at, last_run_at, run_count, consecutive_failures,
               metadata, created_at
        FROM scheduled_jobs
        "#,
    );

    let state = state.unwrap_or_default().trim().to_string();
    if state.is_empty() {
        sql.push_str(" ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?");
    } else {
        sql.push_str(
            " WHERE state = ? ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?",
        );
    }

    let mut query = sqlx::query(&sql);
    if !state.is_empty() {
        query = query.bind(state);
    }
    query = query.bind(limit.max(1) as i64);

    let rows = query.fetch_all(pool).await?;
    Ok(rows.into_iter().map(row_to_job_summary).collect())
}

pub async fn inspect_job(
    pool: &SqlitePool,
    id_or_name: &str,
    run_limit: usize,
    dead_letter_limit: usize,
) -> Result<JobDetailReport> {
    let job = sqlx::query(
        r#"
        SELECT id, name, description, workflow_id, trigger_type, trigger_config,
               state, timezone, max_retries, priority, source_kind, owner,
               tags, disabled_until, manifest_path, task_notes_path,
               next_run_at, last_run_at, run_count, consecutive_failures,
               metadata, created_at
        FROM scheduled_jobs
        WHERE id = ? OR name = ?
        LIMIT 1
        "#,
    )
    .bind(id_or_name)
    .bind(id_or_name)
    .fetch_optional(pool)
    .await?
    .map(row_to_job_summary);

    let Some(job) = job else {
        return Ok(JobDetailReport {
            job: None,
            recent_runs: Vec::new(),
            dead_letters: Vec::new(),
        });
    };

    let recent_runs = sqlx::query(
        r#"
        SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
               r.retry_count, r.langsmith_trace_id
        FROM job_runs r
        JOIN scheduled_jobs j ON j.id = r.job_id
        WHERE r.job_id = ? OR j.name = ?
        ORDER BY r.started_at DESC
        LIMIT ?
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(run_limit.max(1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| JobRunSummary {
        id: row.get("id"),
        job_id: row.get("job_id"),
        name: row.get("name"),
        status: row.get("status"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        retry_count: row.get::<Option<i64>, _>("retry_count").unwrap_or(0),
        langsmith_trace_id: row.get("langsmith_trace_id"),
    })
    .collect();

    let dead_letters = sqlx::query(
        r#"
        SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count, d.resolved
        FROM dead_letter_queue d
        JOIN scheduled_jobs j ON j.id = d.job_id
        WHERE d.job_id = ? OR j.name = ?
        ORDER BY d.failed_at DESC
        LIMIT ?
        "#,
    )
    .bind(&job.id)
    .bind(&job.name)
    .bind(dead_letter_limit.max(1) as i64)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| DeadLetterSummary {
        id: row.get("id"),
        job_id: row.get("job_id"),
        name: row.get("name"),
        last_error: row.get("last_error"),
        failed_at: row.get("failed_at"),
        retry_count: row.get("retry_count"),
        resolved: row.get::<Option<i64>, _>("resolved").unwrap_or(0) == 1,
    })
    .collect();

    Ok(JobDetailReport {
        job: Some(job),
        recent_runs,
        dead_letters,
    })
}

fn parse_session_status(value: &str) -> Option<SessionStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "active" => Some(SessionStatus::Active),
        "archived" => Some(SessionStatus::Archived),
        "closed" => Some(SessionStatus::Closed),
        _ => None,
    }
}

fn row_to_job_summary(row: sqlx::sqlite::SqliteRow) -> JobSummary {
    JobSummary {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        workflow_id: row.get("workflow_id"),
        trigger_type: row.get("trigger_type"),
        trigger_config: parse_json_value(row.get::<String, _>("trigger_config")),
        state: row
            .get::<Option<String>, _>("state")
            .unwrap_or_else(|| "active".to_string()),
        timezone: row.get("timezone"),
        max_retries: row.get::<Option<i64>, _>("max_retries").unwrap_or(0),
        priority: row.get::<Option<i64>, _>("priority").unwrap_or(0),
        source_kind: row.get("source_kind"),
        owner: row.get("owner"),
        tags: parse_string_array(row.get::<Option<String>, _>("tags")),
        disabled_until: row.get("disabled_until"),
        manifest_path: row.get("manifest_path"),
        task_notes_path: row.get("task_notes_path"),
        next_run_at: row.get("next_run_at"),
        last_run_at: row.get("last_run_at"),
        run_count: row.get::<Option<i64>, _>("run_count").unwrap_or(0),
        consecutive_failures: row
            .get::<Option<i64>, _>("consecutive_failures")
            .unwrap_or(0),
        metadata: parse_optional_json_value(row.get::<Option<String>, _>("metadata")),
        created_at: row.get("created_at"),
    }
}

fn parse_string_array(value: Option<String>) -> Vec<String> {
    value
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

fn parse_json_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or_else(|_| serde_json::json!({ "raw": value }))
}

fn parse_optional_json_value(value: Option<String>) -> serde_json::Value {
    value
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn build_voice_operator_attention_signals(
    voice_status: &voice_runtime::VoiceStatus,
    provider_coverage: &VoiceProviderCoverage,
    voice_outcomes: &voice_runtime::VoiceOutcomesSummary,
    talk_metrics: &talk::TalkRuntimeMetrics,
    voice_calls: &[skills::SkillVoiceCallRecord],
) -> Vec<VoiceOperatorAttentionSignal> {
    let mut signals = Vec::new();

    if !voice_status.enabled {
        signals.push(VoiceOperatorAttentionSignal {
            kind: "voice_disabled".to_string(),
            severity: "critical".to_string(),
            summary: "Voice runtime is disabled".to_string(),
            detail: "The shipped voice surface is configured off, so operators cannot rely on voice parity routes as an active lane.".to_string(),
            route_hint: "/control/voice/status".to_string(),
            target_id: None,
        });
    }

    if voice_status.enabled
        && (!voice_status.api_key_present
            || !voice_status.tts_api_key_present
            || provider_coverage.stt_ready == 0
            || provider_coverage.tts_ready == 0)
    {
        signals.push(VoiceOperatorAttentionSignal {
            kind: "provider_readiness".to_string(),
            severity: "critical".to_string(),
            summary: "Voice provider readiness is incomplete".to_string(),
            detail: format!(
                "STT ready: {}/{}. TTS ready: {}/{}. The configured voice lane is enabled but provider credentials or readiness are still missing.",
                provider_coverage.stt_ready,
                provider_coverage.stt_total,
                provider_coverage.tts_ready,
                provider_coverage.tts_total
            ),
            route_hint: "/control/voice/providers".to_string(),
            target_id: None,
        });
    }

    signals.extend(
        voice_outcomes
            .outcomes
            .iter()
            .filter(|entry| entry.attention_needed)
            .take(3)
            .map(|entry| VoiceOperatorAttentionSignal {
                kind: format!("voice_session_{}", entry.outcome_label),
                severity: if entry.stale { "critical" } else { "warning" }.to_string(),
                summary: format!(
                    "Voice session {} is {}",
                    entry.session_id, entry.outcome_label
                ),
                detail: format!(
                    "{}. Idle for {}s with {} turn(s) and {} artifact(s).",
                    entry.detail, entry.idle_secs, entry.turn_count, entry.artifact_count
                ),
                route_hint: format!("/control/voice/sessions/{}", entry.session_id),
                target_id: Some(entry.session_id.clone()),
            }),
    );

    signals.extend(
        talk_metrics
            .sessions
            .iter()
            .filter(|entry| entry.status == "error")
            .take(3)
            .map(|entry| VoiceOperatorAttentionSignal {
                kind: "talk_error".to_string(),
                severity: "warning".to_string(),
                summary: format!("Talk receipt {} ended in error", entry.session_id),
                detail: format!(
                    "Talk session recorded {} event(s) across {} turn(s). Final state: {}.",
                    entry.event_count,
                    entry.turn_count,
                    entry.final_state.as_deref().unwrap_or("unknown")
                ),
                route_hint: format!("/control/talk/sessions/{}", entry.session_id),
                target_id: Some(entry.session_id.clone()),
            }),
    );

    signals.extend(
        voice_calls
            .iter()
            .filter(|call| call.health == "stale")
            .take(3)
            .map(|call| VoiceOperatorAttentionSignal {
                kind: "voice_call_stale".to_string(),
                severity: "warning".to_string(),
                summary: format!("Bounded voice call {} is stale", call.call_id),
                detail: format!(
                    "Plugin {} for skill {} has gone stale after {}s without a fresh receipt.",
                    call.plugin_id, call.skill_name, call.stale_after_secs
                ),
                route_hint: format!("/control/skills/voice-calls/{}/events", call.call_id),
                target_id: Some(call.call_id.clone()),
            }),
    );

    signals
}

fn build_voice_operator_recent_activity(
    voice_outcomes: &voice_runtime::VoiceOutcomesSummary,
    talk_status: &talk::TalkRuntimeStatus,
    voice_calls: &[skills::SkillVoiceCallRecord],
    limit: usize,
) -> Vec<VoiceOperatorRecentActivity> {
    let mut entries = Vec::new();

    entries.extend(
        voice_outcomes
            .outcomes
            .iter()
            .map(|entry| VoiceOperatorRecentActivity {
                kind: "voice_session".to_string(),
                observed_at: entry.last_activity_at.clone(),
                status: entry.outcome_label.clone(),
                label: entry.session_id.clone(),
                detail: format!(
                    "{} turn(s), {} artifact(s). {}",
                    entry.turn_count, entry.artifact_count, entry.detail
                ),
                target_id: Some(entry.session_id.clone()),
            }),
    );

    entries.extend(
        talk_status
            .sessions
            .iter()
            .map(|entry| VoiceOperatorRecentActivity {
                kind: "talk_session".to_string(),
                observed_at: entry.last_activity_at.clone(),
                status: entry.status.clone(),
                label: entry.id.clone(),
                detail: format!(
                    "Wake word '{}', {} turn(s), final state {}.",
                    entry.wake_word,
                    entry.turn_count,
                    entry.final_state.as_deref().unwrap_or("unknown")
                ),
                target_id: Some(entry.id.clone()),
            }),
    );

    entries.extend(voice_calls.iter().map(|call| {
        VoiceOperatorRecentActivity {
            kind: "voice_call".to_string(),
            observed_at: call
                .last_seen_at
                .clone()
                .or_else(|| call.ended_at.clone())
                .unwrap_or_else(|| call.started_at.clone()),
            status: call.health.clone(),
            label: call.call_id.clone(),
            detail: format!(
                "Plugin {} for skill {}. Remote: {}.",
                call.plugin_id,
                call.skill_name,
                call.remote.as_deref().unwrap_or("n/a")
            ),
            target_id: Some(call.call_id.clone()),
        }
    }));

    entries.sort_by(|left, right| {
        right
            .observed_at
            .cmp(&left.observed_at)
            .then_with(|| left.label.cmp(&right.label))
    });
    entries.truncate(limit.max(1));
    entries
}

#[cfg(test)]
mod tests {
    use super::{
        enterprise_access_summary, enterprise_admin_summary, enterprise_foundations_summary,
        greenfield_progress_summary, memory_model_artifacts, memory_timeline,
        new_tool_execution_record, self_hosted_product_mode_summary, setup_handoff_summary,
        tool_execution_log_path, transition_self_hosted_product_mode_summary,
    };
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use openrustclaw_core::traits::MemoryStore;
    use openrustclaw_core::types::{
        MemoryEntry, MemoryType, RetrievalArtifactKind, RetrievalExplanation, SourceType,
    };
    use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore, init_pool, run_migrations};
    use openrustclaw_memory::ModelArtifactService;
    use openrustclaw_mobile::protocol::{DeviceCommandKind, MobileCommandRecord};
    use sqlx::query;
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    use crate::commands::browser::{ExternalBackendAuditEntry, backend_policy};
    use crate::commands::enterprise_access::{
        APPROVER_ID_HEADER, EnterpriseAccessBootstrapRequest, EnterpriseAccessOperatorRequest,
        bootstrap_manifest, upsert_operator,
    };
    use crate::commands::{onboard, self_hosted};

    #[test]
    fn enterprise_foundations_summary_reports_policy_and_recent_audit_evidence() -> Result<()> {
        let root = tempdir().expect("tempdir");

        let commands_dir = root.path().join(".claw/mobile/commands");
        fs::create_dir_all(&commands_dir)?;
        let command = MobileCommandRecord {
            id: "cmd-1".to_string(),
            node_id: "ios-1".to_string(),
            command: DeviceCommandKind::SendMessage,
            required_capability: "mobile".to_string(),
            approval_required: true,
            status: "executed".to_string(),
            payload: serde_json::json!({"target": "+15551234567"}),
            result: serde_json::json!({"ok": true}),
            created_at: parse_time("2026-03-26T18:00:00Z"),
            approved_by: Some("operator-1".to_string()),
            decided_reason: Some("approved from control ui".to_string()),
            approved_at: Some(parse_time("2026-03-26T18:01:00Z")),
            executed_at: Some(parse_time("2026-03-26T18:02:00Z")),
        };
        fs::write(
            commands_dir.join("cmd-1.json"),
            serde_json::to_vec_pretty(&command)?,
        )?;

        let policy = backend_policy(root.path())?;
        let audit_path = root.path().join(&policy.audit_log_path);
        if let Some(parent) = audit_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let browser_audit = ExternalBackendAuditEntry {
            timestamp: "2026-03-26T18:03:00Z".to_string(),
            backend: "agent_browser_cli".to_string(),
            transport: "cli".to_string(),
            action: "inspect".to_string(),
            session_id: Some("browser-1".to_string()),
            allowed: true,
            success: true,
            detail: Some("captured checkout page".to_string()),
        };
        fs::write(
            &audit_path,
            format!("{}\n", serde_json::to_string(&browser_audit)?),
        )?;

        let tool_log = tool_execution_log_path(root.path());
        if let Some(parent) = tool_log.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut tool_record = new_tool_execution_record(
            "mobile.command.approve",
            "runtime_tool",
            "success",
            "success",
            25,
            None,
            None,
            None,
            None,
        );
        tool_record.created_at = "2026-03-26T18:04:00Z".to_string();
        fs::write(
            &tool_log,
            format!("{}\n", serde_json::to_string(&tool_record)?),
        )?;

        let report = enterprise_foundations_summary(root.path(), 12)?;
        assert!(report.explicit_boundary);
        assert_eq!(report.policy.approval_policy, "side_effects");
        assert!(
            report
                .policy
                .approval_scope_detail
                .contains("side effect is requested")
        );
        assert_eq!(report.mobile_metrics.executed_commands, 1);
        assert!(report.recent_events.iter().any(|entry| {
            entry.source == "mobile_command"
                && entry
                    .summary
                    .contains("executed after approval by operator-1")
        }));
        assert!(report.recent_events.iter().any(|entry| {
            entry.source == "browser_backend" && entry.detail.contains("captured checkout page")
        }));
        assert!(report.recent_events.iter().any(|entry| {
            entry.kind == "mobile.command.approve" && entry.source == "runtime_tool"
        }));

        Ok(())
    }

    #[test]
    fn enterprise_access_summary_reports_bootstrap_state_and_operator_counts() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let report = enterprise_access_summary(root.path())?;
        assert_eq!(report.status, "bootstrap_required");
        assert!(!report.explicit_identity_required);
        assert!(report.organization.is_none());
        assert!(!report.protected_routes.is_empty());
        assert_eq!(report.governance.status, "bootstrap_required");
        assert!(report.governance.dual_approval_rule_count >= 1);

        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: Some("Owner".to_string()),
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )?;
        upsert_operator(
            root.path(),
            EnterpriseAccessOperatorRequest {
                id: "admin-1".to_string(),
                name: Some("Admin".to_string()),
                email: None,
                role: "admin".to_string(),
                token: "admin-secret-123".to_string(),
                scopes: vec![],
                active: true,
            },
        )?;

        let report = enterprise_access_summary(root.path())?;
        assert_eq!(report.status, "ok");
        assert!(report.explicit_identity_required);
        assert_eq!(
            report.organization.as_ref().map(|value| value.id.as_str()),
            Some("acme")
        );
        assert_eq!(report.operators.len(), 2);
        assert_eq!(report.governance.status, "ok");
        assert!(report.governance.dual_approval_rule_count >= 1);
        assert_eq!(report.governance.approver_id_header, APPROVER_ID_HEADER);
        assert!(report.governance.rules.iter().any(|rule| {
            rule.scope == "enterprise.config.write"
                && rule.approval_mode == "dual"
                && rule.coverage_status == "ready"
        }));
        assert!(report.role_grants.iter().any(|entry| {
            entry.role == "admin" && entry.operator_count == 1 && !entry.default_scopes.is_empty()
        }));
        Ok(())
    }

    #[test]
    fn self_hosted_product_mode_summary_defaults_to_solo_self_hosted() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let report = self_hosted_product_mode_summary(root.path())?;
        assert_eq!(report.status, "implicit_default");
        assert!(!report.explicit_mode_selected);
        assert_eq!(report.mode, self_hosted::MODE_SOLO);
        assert!(report.self_hosted);
        assert!(report.open_source);
        assert_eq!(report.recommended_runtime_mode, "solo_claw");
        assert!(
            report
                .transition_targets
                .iter()
                .any(|value| value == "team")
        );
        Ok(())
    }

    #[test]
    fn self_hosted_product_mode_summary_reports_explicit_company_mode() -> Result<()> {
        let root = tempdir().expect("tempdir");
        self_hosted::configure_mode(
            root.path(),
            self_hosted::MODE_COMPANY,
            Some("company_ops_setup"),
            Some("phase 24 test"),
        )?;

        let report = self_hosted_product_mode_summary(root.path())?;
        assert_eq!(report.status, "ok");
        assert!(report.explicit_mode_selected);
        assert_eq!(report.mode_label, "Company");
        assert_eq!(report.onboarding_path, "company_ops_setup");
        assert!(report.enterprise_controls_expected);
        assert!(report.multi_user);
        Ok(())
    }

    #[test]
    fn self_hosted_product_mode_summary_reports_recent_transition_receipts() -> Result<()> {
        let root = tempdir().expect("tempdir");
        self_hosted::configure_mode(root.path(), self_hosted::MODE_TEAM, None, None)?;
        self_hosted::transition_mode(
            root.path(),
            self_hosted::SelfHostedProductTransitionRequest {
                target_mode: self_hosted::MODE_COMPANY.to_string(),
                actor: "operator-1".to_string(),
                reason: Some("team grew".to_string()),
                via: Some("test".to_string()),
            },
        )?;

        let report = self_hosted_product_mode_summary(root.path())?;
        assert_eq!(report.mode, self_hosted::MODE_COMPANY);
        assert_eq!(report.recent_transitions.len(), 1);
        assert_eq!(report.recent_transitions[0].direction, "upgrade");
        Ok(())
    }

    #[test]
    fn transition_self_hosted_product_mode_summary_returns_updated_report() -> Result<()> {
        let root = tempdir().expect("tempdir");
        self_hosted::configure_mode(root.path(), self_hosted::MODE_TEAM, None, None)?;

        let report = transition_self_hosted_product_mode_summary(
            root.path(),
            self_hosted::MODE_COMPANY.to_string(),
            "operator-1".to_string(),
            Some("team grew".to_string()),
        )?;

        assert_eq!(report.mode, self_hosted::MODE_COMPANY);
        assert_eq!(report.status, "ok");
        assert_eq!(report.recent_transitions.len(), 1);
        assert_eq!(report.recent_transitions[0].actor, "operator-1");
        Ok(())
    }

    #[test]
    fn greenfield_progress_summary_reports_current_inventory_score() {
        let report = greenfield_progress_summary();

        assert_eq!(report.total_seams, 18);
        assert_eq!(report.completed_seams, 18);
        assert_eq!(report.remaining_seams, 0);
        assert_eq!(report.completion_percent, 100);
        assert_eq!(report.ledger_status, "complete");
        assert_eq!(report.queue_decision, "retire_current_ranked_inventory");
    }

    #[test]
    fn setup_handoff_summary_defaults_to_not_started() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let report = setup_handoff_summary(root.path())?;
        assert_eq!(report.status, "not_started");
        assert!(!report.explicit_setup_state);
        assert!(!report.ready_for_first_start);
        assert!(report.detail.contains("Run `openrustclaw onboard`"));
        Ok(())
    }

    #[test]
    fn setup_handoff_summary_reports_degraded_setup_state() -> Result<()> {
        let root = tempdir().expect("tempdir");
        onboard::save_setup_state(
            root.path(),
            &onboard::SetupStateManifest {
                version: 1,
                setup: onboard::SetupState {
                    started_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                    completed_at: Some(Utc::now().to_rfc3339()),
                    status: "ready".to_string(),
                    workspace_action: "repair_existing".to_string(),
                    deployment_mode: Some(self_hosted::MODE_TEAM.to_string()),
                    deployment_path: Some("shared_team_setup".to_string()),
                    remote_connectivity_profile: Some(onboard::RemoteConnectivityProfile {
                        mode: "remote_access".to_string(),
                        primary_path: "node_first".to_string(),
                        fallback_paths: vec!["ssh_tunnel".to_string()],
                        detail: "Remote access profile saved.".to_string(),
                    }),
                    setup_path: Some("Advanced".to_string()),
                    selected_provider: Some("openrouter".to_string()),
                    selected_access_mode: Some("api_key".to_string()),
                    selected_primary_model: Some("openai/gpt-4o".to_string()),
                    selected_primary_model_source: Some("live_discovery".to_string()),
                    selected_steps: vec!["gateway".to_string(), "model".to_string()],
                    completed_steps: vec!["gateway".to_string(), "model".to_string()],
                    blockers: Vec::new(),
                    next_action: Some("Start the gateway.".to_string()),
                    current_step: None,
                    bootstrap_outcomes: vec![onboard::SetupBootstrapOutcome {
                        category: "channel".to_string(),
                        target: "slack".to_string(),
                        status: "warning".to_string(),
                        detail: "Slack auth probe failed".to_string(),
                        issue_kind: Some("auth".to_string()),
                        verification_stage: None,
                        suggested_action: Some(
                            "Review the Slack credential and rerun onboarding.".to_string(),
                        ),
                        updated_at: Utc::now().to_rfc3339(),
                    }],
                },
            },
        )?;

        let report = setup_handoff_summary(root.path())?;
        assert_eq!(report.status, "degraded");
        assert!(report.explicit_setup_state);
        assert!(!report.ready_for_first_start);
        assert_eq!(report.selected_provider.as_deref(), Some("openrouter"));
        assert_eq!(report.selected_access_mode.as_deref(), Some("api_key"));
        assert_eq!(
            report.selected_primary_model.as_deref(),
            Some("openai/gpt-4o")
        );
        assert_eq!(report.completed_step_count, 2);
        assert_eq!(report.bootstrap_outcomes.len(), 1);
        assert_eq!(
            report.bootstrap_outcomes[0].issue_kind.as_deref(),
            Some("auth")
        );
        assert_eq!(
            report
                .remote_connectivity_profile
                .as_ref()
                .map(|profile| profile.primary_path.as_str()),
            Some("node_first")
        );
        assert!(report.detail.contains("needs review"));
        Ok(())
    }

    #[test]
    fn enterprise_admin_summary_combines_access_policy_and_supervision() -> Result<()> {
        let root = tempdir().expect("tempdir");

        let report = enterprise_admin_summary(root.path(), "config/default.toml")?;
        assert_eq!(report.status, "bootstrap_required");
        assert!(!report.requires_operator_headers);
        assert_eq!(report.policy.approval_policy, "side_effects");
        assert_eq!(report.autonomy.status, "bootstrap_required");
        assert_eq!(report.supervision.active_run_count, 0);
        assert_eq!(report.supervision.attention_required_count, 0);

        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: Some("Owner".to_string()),
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )?;

        let report = enterprise_admin_summary(root.path(), "config/default.toml")?;
        assert_eq!(report.status, "ok");
        assert!(report.requires_operator_headers);
        assert_eq!(
            report
                .access
                .organization
                .as_ref()
                .map(|value| value.id.as_str()),
            Some("acme")
        );
        assert!(report.detail.contains(
            "manage policy, identity, governance, audit export, supervised-runtime controls, and the explicit full-autonomy lane"
        ));
        assert!(report.access.governance.dual_approval_rule_count >= 1);
        assert_eq!(
            report.autonomy.governance_scope,
            "enterprise.full_autonomy.manage"
        );
        Ok(())
    }

    #[tokio::test]
    async fn memory_timeline_reports_recent_search_explanations() -> Result<()> {
        let workspace = tempdir().expect("tempdir");
        let db_path = workspace.path().join("inspect-memory.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await?;
        run_migrations(&pool).await?;

        let store = SqliteMemoryStore::new(pool.clone());
        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: "Rust ownership preference".to_string(),
            content_hash: "inspect-hash".to_string(),
            source: Some("test".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: None,
            user_id: Some("user-1".to_string()),
            namespace: "user-1".to_string(),
            importance: 0.8,
            confidence: 0.9,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({}),
        };
        store.store(entry.clone()).await?;

        let explanation = RetrievalExplanation::empty(
            RetrievalArtifactKind::ConversationMemory,
            entry.id.to_string(),
            "user-1".to_string(),
        );
        let payload = serde_json::json!({
            "query": "ownership",
            "namespace": "user-1",
            "result_count": 1,
            "recall_pack": {
                "degraded": true,
                "items": [{
                    "id": entry.id.to_string(),
                    "memory_type": "semantic",
                    "namespace": "user-1",
                    "content": "Rust ownership preference",
                    "score": 0.93,
                    "importance": 0.8,
                    "confidence": 0.9,
                    "explanation": explanation
                }]
            }
        });
        query(
            r#"
            INSERT INTO runtime_events (id, event_name, event_type, session_id, payload, status, created_at)
            VALUES (?, 'memory.searched', 'memory_searched', NULL, ?, 'processed', datetime('now'))
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(payload.to_string())
        .execute(&pool)
        .await?;

        let report = memory_timeline(&store, &pool, Some("user-1"), 10).await?;
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.recent_searches.len(), 1);
        assert_eq!(report.recent_searches[0].query, "ownership");
        assert!(report.recent_searches[0].degraded);
        assert_eq!(report.recent_searches[0].recall_pack.items.len(), 1);
        assert_eq!(
            report.recent_searches[0].recall_pack.items[0]
                .explanation
                .primary_artifact
                .artifact_kind,
            RetrievalArtifactKind::ConversationMemory
        );
        Ok(())
    }

    #[tokio::test]
    async fn memory_model_artifacts_report_includes_projection() -> Result<()> {
        let workspace = tempdir().expect("tempdir");
        let db_path = workspace.path().join("inspect-model-artifacts.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await?;
        run_migrations(&pool).await?;

        let memory_store = SqliteMemoryStore::new(pool.clone());
        let core_store = SqliteCoreMemoryStore::new(pool.clone());
        let service = ModelArtifactService::new(memory_store.clone(), core_store.clone());

        service
            .promote(&openrustclaw_core::types::ModelArtifactPromotionRequest {
                namespace: "user-1".to_string(),
                kind: openrustclaw_core::types::ModelArtifactKind::UserModel,
                summary: "User prefers concise Rust answers.".to_string(),
                importance: 0.9,
                confidence: 0.95,
                source_lineage: vec![openrustclaw_core::types::ModelArtifactSourceRef {
                    kind: openrustclaw_core::types::ModelArtifactSourceKind::MemoryEntry,
                    source_id: Uuid::new_v4().to_string(),
                    detail: None,
                }],
                promoted_by: Some("test".to_string()),
                correction_note: None,
            })
            .await?;

        let report = memory_model_artifacts(&memory_store, &core_store, Some("user-1"), true, 10)
            .await?;
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.projected_core_entries.len(), 1);
        assert_eq!(report.projected_core_entries[0].key, "model.user");
        Ok(())
    }

    fn parse_time(value: &str) -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(value)
            .expect("valid rfc3339")
            .with_timezone(&Utc)
    }
}
