use anyhow::Result;
use chrono::Utc;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::types::{MemoryEntry, Message};
use openrustclaw_db::models::MemoryArchiveRow;
use openrustclaw_db::{
    PersistedSession, SessionStatus, SqliteMemoryStore, SqlitePool, SqliteSessionStore,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::{
    browser, control, enterprise_access, enterprise_autonomy, enterprise_policy, mobile,
    orchestrate, skills, talk, voice_runtime,
};

#[derive(Debug, Clone, Serialize)]
pub struct AssistantContinuitySummary {
    pub assistant_managed: bool,
    pub assistant_identity: Option<String>,
    pub assistant_surface: Option<String>,
    pub assistant_session_model: Option<String>,
    pub route_key: Option<String>,
    pub workspace_root: Option<String>,
    pub route_bound: bool,
    pub history_messages: usize,
    pub likely_resumed: bool,
    pub status_label: String,
    pub detail: String,
}

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
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryArchiveReport {
    pub namespace: Option<String>,
    pub entries: Vec<MemoryArchiveRow>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    pub id: String,
    pub tool_name: String,
    pub source: String,
    pub status: String,
    pub status_detail: String,
    pub duration_ms: u64,
    pub created_at: String,
    pub error: Option<String>,
    pub artifact_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub result_preview: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolExecutionHistoryReport {
    pub limit: usize,
    pub entries: Vec<ToolExecutionRecord>,
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
    let assistant_identity = metadata_string(&session.session.metadata, "assistant_identity");
    let assistant_surface = metadata_string(&session.session.metadata, "assistant_surface");
    let assistant_session_model =
        metadata_string(&session.session.metadata, "assistant_session_model");
    let route_key = session
        .route_key
        .clone()
        .or_else(|| metadata_string(&session.session.metadata, "route_key"));
    let workspace_root = metadata_string(&session.session.metadata, "workspace_root");
    let route_bound = route_key.is_some();
    let assistant_managed = assistant_identity.is_some()
        || assistant_surface.is_some()
        || assistant_session_model.is_some();
    let likely_resumed = assistant_managed && route_bound && history_messages > 0;
    let status_label = if !assistant_managed {
        "generic"
    } else if likely_resumed {
        "resumed"
    } else if route_bound {
        "route-bound"
    } else if assistant_session_model.as_deref() == Some("persisted") {
        "persisted"
    } else {
        "assistant"
    }
    .to_string();
    let detail = continuity_detail(
        assistant_managed,
        assistant_surface.as_deref(),
        route_bound,
        history_messages,
    );

    AssistantContinuitySummary {
        assistant_managed,
        assistant_identity,
        assistant_surface,
        assistant_session_model,
        route_key,
        workspace_root,
        route_bound,
        history_messages,
        likely_resumed,
        status_label,
        detail,
    }
}

async fn count_history_messages(pool: &SqlitePool, session_id: &str) -> Result<usize> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await?;
    Ok(count.max(0) as usize)
}

fn metadata_string(metadata: &serde_json::Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn continuity_detail(
    assistant_managed: bool,
    assistant_surface: Option<&str>,
    route_bound: bool,
    history_messages: usize,
) -> String {
    let history_label = if history_messages == 1 {
        "1 persisted message".to_string()
    } else {
        format!("{history_messages} persisted messages")
    };

    if !assistant_managed {
        return format!("Generic session with {history_label}.");
    }

    let surface = assistant_surface.unwrap_or("assistant");
    if route_bound && history_messages > 0 {
        return format!(
            "Primary {surface} assistant session matched by route key with {history_label} restored."
        );
    }
    if route_bound {
        return format!(
            "Primary {surface} assistant session is route-bound and ready to accumulate persisted history."
        );
    }
    format!("Primary {surface} assistant session with {history_label}.")
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
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryTimelineReport> {
    let namespace_value = namespace.map(|value| value.to_string());
    let entries = store.list_recent(namespace, limit.max(1)).await?;
    Ok(MemoryTimelineReport {
        namespace: namespace_value,
        entries,
    })
}

pub fn tool_execution_log_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".claw")
        .join("control")
        .join("tool-executions.jsonl")
}

pub fn append_tool_execution_record(
    workspace_root: &Path,
    record: &ToolExecutionRecord,
) -> Result<()> {
    let path = tool_execution_log_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    serde_json::to_writer(&mut file, record)?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn tool_execution_history(
    workspace_root: &Path,
    limit: usize,
    source: Option<&str>,
    status: Option<&str>,
    tool_name: Option<&str>,
) -> Result<ToolExecutionHistoryReport> {
    let path = tool_execution_log_path(workspace_root);
    if !path.exists() {
        return Ok(ToolExecutionHistoryReport {
            limit: limit.max(1),
            entries: Vec::new(),
        });
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
        if source.is_some_and(|value| record.source != value) {
            continue;
        }
        if status.is_some_and(|value| record.status != value) {
            continue;
        }
        if tool_name.is_some_and(|value| record.tool_name != value) {
            continue;
        }
        entries.push(record);
    }

    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    entries.truncate(limit.max(1));

    Ok(ToolExecutionHistoryReport {
        limit: limit.max(1),
        entries,
    })
}

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
    ToolExecutionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        tool_name: tool_name.into(),
        source: source.into(),
        status: status.into(),
        status_detail: status_detail.into(),
        duration_ms,
        created_at: Utc::now().to_rfc3339(),
        error,
        artifact_path,
        args,
        result_preview,
    }
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
            let coverage_status = if !rule.active {
                "disabled".to_string()
            } else if rule.approval_mode == "dual"
                && (eligible_requester_count == 0
                    || eligible_approver_count == 0
                    || active_operator_roles.len() < 2)
            {
                "coverage_gap".to_string()
            } else if eligible_requester_count == 0 {
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
    let attention_required_count = active_runs
        .iter()
        .filter(|run| {
            run.lifecycle.intervention_required
                || run.pause_requested
                || run.kill_requested
                || run.last_error.is_some()
                || run.lifecycle.state != "active"
        })
        .count();
    let escalated_count = active_runs
        .iter()
        .filter(|run| run.lifecycle.escalation_requested || run.lifecycle.state == "escalated")
        .count();
    let rollback_requested_count = active_runs
        .iter()
        .filter(|run| {
            run.lifecycle.rollback_requested || run.lifecycle.state == "rollback_requested"
        })
        .count();

    let supervision = EnterpriseAdminSupervisionSummary {
        active_run_count: active_runs.len(),
        attention_required_count,
        escalated_count,
        rollback_requested_count,
        route_hint: "/control/ui".to_string(),
        detail: if active_runs.is_empty() {
            "No active supervised orchestration runs currently need enterprise operator attention."
                .to_string()
        } else {
            format!(
                "{} active supervised run(s), {} needing attention, {} escalated, {} waiting on rollback handling.",
                active_runs.len(),
                attention_required_count,
                escalated_count,
                rollback_requested_count
            )
        },
    };

    let requires_operator_headers = access.explicit_identity_required;
    let detail = if requires_operator_headers {
        format!(
            "Enterprise admin is live for organization `{}` with {} operator(s). Use `/control/ui` with the scoped operator headers to manage policy, identity, governance, audit export, supervised-runtime controls, and the explicit full-autonomy lane from one shipped surface.",
            access
                .organization
                .as_ref()
                .map(|value| value.id.as_str())
                .unwrap_or("-"),
            access.operators.len()
        )
    } else {
        "Enterprise admin is not bootstrapped yet. Bootstrap enterprise access first, then use the same shipped surface to manage policy, governance, audit export, and supervised-runtime controls under scoped operator identity.".to_string()
    };

    Ok(EnterpriseAdminReport {
        status: if requires_operator_headers {
            "ok".to_string()
        } else {
            "bootstrap_required".to_string()
        },
        detail,
        requires_operator_headers,
        access,
        policy,
        autonomy,
        supervision,
    })
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
    .map(|row| row_to_job_summary(row));

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
        new_tool_execution_record, tool_execution_log_path,
    };
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use openrustclaw_mobile::protocol::{DeviceCommandKind, MobileCommandRecord};
    use std::fs;
    use tempfile::tempdir;

    use crate::commands::browser::{ExternalBackendAuditEntry, backend_policy};
    use crate::commands::enterprise_access::{
        APPROVER_ID_HEADER, EnterpriseAccessBootstrapRequest, EnterpriseAccessOperatorRequest,
        bootstrap_manifest, upsert_operator,
    };

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
        assert_eq!(report.autonomy.governance_scope, "enterprise.full_autonomy.manage");
        Ok(())
    }

    fn parse_time(value: &str) -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(value)
            .expect("valid rfc3339")
            .with_timezone(&Utc)
    }
}
