use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use openrustclaw_core::config::AppConfig;
use openrustclaw_mobile::protocol::DeviceCommandKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{control, enterprise_access, inspect, runtime};

const ENTERPRISE_POLICY_VERSION: u32 = 1;
const DEFAULT_EXPORT_ROOT: &str = ".claw/control/enterprise/exports";
const DEFAULT_RECENT_EVENT_LIMIT: usize = 25;
const DEFAULT_TOOL_HISTORY_LIMIT: usize = 50;
const DEFAULT_RETENTION_DAYS: usize = 30;
const DEFAULT_EXPORT_HISTORY_LIMIT: usize = 12;

fn default_version() -> u32 {
    ENTERPRISE_POLICY_VERSION
}

fn default_export_root() -> String {
    DEFAULT_EXPORT_ROOT.to_string()
}

fn default_recent_event_limit() -> usize {
    DEFAULT_RECENT_EVENT_LIMIT
}

fn default_tool_history_limit() -> usize {
    DEFAULT_TOOL_HISTORY_LIMIT
}

fn default_retention_days() -> usize {
    DEFAULT_RETENTION_DAYS
}

fn default_export_history_limit() -> usize {
    DEFAULT_EXPORT_HISTORY_LIMIT
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterprisePolicyManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub mobile: EnterpriseMobileApprovalPolicy,
    #[serde(default)]
    pub audit_export: EnterpriseAuditExportPolicy,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Default for EnterprisePolicyManifest {
    fn default() -> Self {
        Self {
            version: ENTERPRISE_POLICY_VERSION,
            mobile: EnterpriseMobileApprovalPolicy::default(),
            audit_export: EnterpriseAuditExportPolicy::default(),
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseMobileApprovalPolicy {
    #[serde(default = "default_true")]
    pub send_message_requires_approval: bool,
    #[serde(default = "default_true")]
    pub push_notification_requires_approval: bool,
    #[serde(default)]
    pub sync_now_requires_approval: bool,
}

impl Default for EnterpriseMobileApprovalPolicy {
    fn default() -> Self {
        Self {
            send_message_requires_approval: true,
            push_notification_requires_approval: true,
            sync_now_requires_approval: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAuditExportPolicy {
    #[serde(default = "default_export_root")]
    pub export_root: String,
    #[serde(default = "default_recent_event_limit")]
    pub recent_event_limit: usize,
    #[serde(default = "default_tool_history_limit")]
    pub tool_history_limit: usize,
    #[serde(default = "default_retention_days")]
    pub retention_days: usize,
    #[serde(default = "default_export_history_limit")]
    pub export_history_limit: usize,
}

impl Default for EnterpriseAuditExportPolicy {
    fn default() -> Self {
        Self {
            export_root: default_export_root(),
            recent_event_limit: default_recent_event_limit(),
            tool_history_limit: default_tool_history_limit(),
            retention_days: default_retention_days(),
            export_history_limit: default_export_history_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseBrowserPolicySummary {
    pub allowed_backends: Vec<String>,
    pub allow_local_cli_wrappers: bool,
    pub allow_cloud_agent_execution: bool,
    pub audit_log_path: String,
    pub command_env_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterprisePolicyReport {
    pub status: String,
    pub detail: String,
    pub access_boundary_active: bool,
    pub access_registry_path: String,
    pub policy_path: String,
    pub updated_at: Option<String>,
    pub autonomy_level: String,
    pub approval_policy: String,
    pub browser: EnterpriseBrowserPolicySummary,
    pub mobile: EnterpriseMobileApprovalPolicy,
    pub audit_export: EnterpriseAuditExportPolicy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnterprisePolicyUpdateRequest {
    #[serde(default)]
    pub approval_policy: Option<String>,
    #[serde(default)]
    pub browser: Option<EnterpriseBrowserPolicyUpdate>,
    #[serde(default)]
    pub mobile: Option<EnterpriseMobileApprovalPolicyUpdate>,
    #[serde(default)]
    pub audit_export: Option<EnterpriseAuditExportPolicyUpdate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnterpriseBrowserPolicyUpdate {
    #[serde(default)]
    pub allowed_backends: Option<Vec<String>>,
    #[serde(default)]
    pub allow_local_cli_wrappers: Option<bool>,
    #[serde(default)]
    pub allow_cloud_agent_execution: Option<bool>,
    #[serde(default)]
    pub command_env_allowlist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnterpriseMobileApprovalPolicyUpdate {
    #[serde(default)]
    pub send_message_requires_approval: Option<bool>,
    #[serde(default)]
    pub push_notification_requires_approval: Option<bool>,
    #[serde(default)]
    pub sync_now_requires_approval: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnterpriseAuditExportPolicyUpdate {
    #[serde(default)]
    pub export_root: Option<String>,
    #[serde(default)]
    pub recent_event_limit: Option<usize>,
    #[serde(default)]
    pub tool_history_limit: Option<usize>,
    #[serde(default)]
    pub retention_days: Option<usize>,
    #[serde(default)]
    pub export_history_limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnterpriseAuditExportRequest {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAuditExportReceipt {
    pub status: String,
    pub detail: String,
    pub exported_at: String,
    pub export_path: String,
    pub recent_event_count: usize,
    pub tool_history_count: usize,
    pub operator_event_count: usize,
    pub pruned_export_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAuditExportSummary {
    pub exported_at: String,
    pub export_path: String,
    pub note: Option<String>,
    pub recent_event_count: usize,
    pub tool_history_count: usize,
    pub operator_event_count: usize,
    pub governance_rule_count: usize,
    pub attention_required_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAuditReviewReport {
    pub status: String,
    pub detail: String,
    pub export_root: String,
    pub retention_days: usize,
    pub export_history_limit: usize,
    pub recent_event_limit: usize,
    pub tool_history_limit: usize,
    pub recent_exports: Vec<EnterpriseAuditExportSummary>,
    pub recent_operator_history: inspect::ToolExecutionHistoryReport,
    pub governance: inspect::EnterpriseGovernanceReport,
    pub supervision: inspect::EnterpriseAdminSupervisionSummary,
}

#[derive(Debug, Clone, Serialize)]
struct EnterpriseAuditExportBundle {
    exported_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    policy: EnterprisePolicyReport,
    enterprise_foundations: inspect::EnterpriseFoundationsReport,
    governance: inspect::EnterpriseGovernanceReport,
    supervision: inspect::EnterpriseAdminSupervisionSummary,
    operator_history: inspect::ToolExecutionHistoryReport,
    tool_history: inspect::ToolExecutionHistoryReport,
}

fn default_true() -> bool {
    true
}

pub fn enterprise_policy_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("enterprise")
        .join("policy.json")
}

pub fn load_manifest(workspace_root: &Path) -> Result<Option<EnterprisePolicyManifest>> {
    let path = enterprise_policy_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read enterprise policy manifest {}",
            path.display()
        )
    })?;
    let manifest = serde_json::from_str::<EnterprisePolicyManifest>(&raw).with_context(|| {
        format!(
            "failed to parse enterprise policy manifest {}",
            path.display()
        )
    })?;
    Ok(Some(manifest))
}

pub fn approval_required_for_command(workspace_root: &Path, command: &DeviceCommandKind) -> bool {
    let manifest = load_manifest(workspace_root)
        .ok()
        .flatten()
        .unwrap_or_default();
    match command {
        DeviceCommandKind::SendMessage => manifest.mobile.send_message_requires_approval,
        DeviceCommandKind::PushNotification => manifest.mobile.push_notification_requires_approval,
        DeviceCommandKind::SyncNow => manifest.mobile.sync_now_requires_approval,
    }
}

pub fn summary(workspace_root: &Path, config_path: &str) -> Result<EnterprisePolicyReport> {
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

    let config = runtime::load_effective_config(config_path, workspace_root)
        .unwrap_or_else(|_| AppConfig::default());
    let browser = browser_summary_from_config(&config);
    let manifest = load_manifest(workspace_root)?.unwrap_or_default();
    let access_boundary_active = enterprise_access::access_is_configured(workspace_root)?;
    let access_registry_path = enterprise_access::enterprise_access_path(workspace_root)
        .display()
        .to_string();
    let policy_path = enterprise_policy_path(workspace_root).display().to_string();
    let detail = format!(
        "Enterprise policy centralizes approval policy `{}` with {} allowed browser backend(s), explicit mobile approval defaults, and a durable audit export target.",
        approval_policy,
        browser.allowed_backends.len()
    );

    Ok(EnterprisePolicyReport {
        status: "ok".to_string(),
        detail,
        access_boundary_active,
        access_registry_path,
        policy_path,
        updated_at: manifest.updated_at,
        autonomy_level,
        approval_policy,
        browser,
        mobile: manifest.mobile,
        audit_export: manifest.audit_export,
    })
}

pub fn update_policy(
    workspace_root: &Path,
    config_path: &str,
    request: EnterprisePolicyUpdateRequest,
) -> Result<EnterprisePolicyReport> {
    if let Some(approval_policy) = request.approval_policy.as_deref() {
        validate_approval_policy(approval_policy)?;
        control::set_runtime_approval_policy(workspace_root, approval_policy)?;
    }

    if let Some(browser_update) = request.browser {
        let mut config = runtime::load_effective_config(config_path, workspace_root)?;
        if let Some(allowed_backends) = browser_update.allowed_backends {
            config.external_backends.allowed_backends =
                normalize_browser_backends(&allowed_backends)?;
        }
        if let Some(value) = browser_update.allow_local_cli_wrappers {
            config.external_backends.allow_local_cli_wrappers = value;
        }
        if let Some(value) = browser_update.allow_cloud_agent_execution {
            config.external_backends.allow_cloud_agent_execution = value;
        }
        if let Some(values) = browser_update.command_env_allowlist {
            config.external_backends.command_env_allowlist = normalize_env_allowlist(&values);
        }
        runtime::write_config_with_backup(config_path, &config)?;
    }

    if request.mobile.is_some() || request.audit_export.is_some() {
        let mut manifest = load_manifest(workspace_root)?.unwrap_or_default();
        if let Some(mobile_update) = request.mobile {
            if let Some(value) = mobile_update.send_message_requires_approval {
                manifest.mobile.send_message_requires_approval = value;
            }
            if let Some(value) = mobile_update.push_notification_requires_approval {
                manifest.mobile.push_notification_requires_approval = value;
            }
            if let Some(value) = mobile_update.sync_now_requires_approval {
                manifest.mobile.sync_now_requires_approval = value;
            }
        }
        if let Some(audit_update) = request.audit_export {
            if let Some(export_root) = audit_update.export_root {
                let trimmed = export_root.trim();
                if trimmed.is_empty() {
                    anyhow::bail!("audit export root cannot be empty");
                }
                manifest.audit_export.export_root = trimmed.to_string();
            }
            if let Some(limit) = audit_update.recent_event_limit {
                manifest.audit_export.recent_event_limit = limit.max(1);
            }
            if let Some(limit) = audit_update.tool_history_limit {
                manifest.audit_export.tool_history_limit = limit.max(1);
            }
            if let Some(days) = audit_update.retention_days {
                manifest.audit_export.retention_days = days.max(1);
            }
            if let Some(limit) = audit_update.export_history_limit {
                manifest.audit_export.export_history_limit = limit.max(1);
            }
        }
        manifest.updated_at = Some(Utc::now().to_rfc3339());
        save_manifest(workspace_root, &manifest)?;
    }

    summary(workspace_root, config_path)
}

pub fn review_summary(
    workspace_root: &Path,
    config_path: &str,
) -> Result<EnterpriseAuditReviewReport> {
    let policy = summary(workspace_root, config_path)?;
    let access = inspect::enterprise_access_summary(workspace_root)?;
    let admin = inspect::enterprise_admin_summary(workspace_root, config_path)?;
    let recent_operator_history = review_operator_history(
        workspace_root,
        policy.audit_export.tool_history_limit,
        policy.audit_export.recent_event_limit,
    )?;
    let recent_exports = list_recent_exports(workspace_root, &policy.audit_export)?;

    Ok(EnterpriseAuditReviewReport {
        status: "ok".to_string(),
        detail: format!(
            "{} retained export(s) under `{}` with {} recent enterprise or autonomy operator event(s).",
            recent_exports.len(),
            policy.audit_export.export_root,
            recent_operator_history.entries.len()
        ),
        export_root: policy.audit_export.export_root.clone(),
        retention_days: policy.audit_export.retention_days,
        export_history_limit: policy.audit_export.export_history_limit,
        recent_event_limit: policy.audit_export.recent_event_limit,
        tool_history_limit: policy.audit_export.tool_history_limit,
        recent_exports,
        recent_operator_history,
        governance: access.governance,
        supervision: admin.supervision,
    })
}

pub fn export_audit_bundle(
    workspace_root: &Path,
    config_path: &str,
    request: EnterpriseAuditExportRequest,
) -> Result<EnterpriseAuditExportReceipt> {
    let policy = summary(workspace_root, config_path)?;
    let foundations = inspect::enterprise_foundations_summary(
        workspace_root,
        policy.audit_export.recent_event_limit,
    )?;
    let access = inspect::enterprise_access_summary(workspace_root)?;
    let admin = inspect::enterprise_admin_summary(workspace_root, config_path)?;
    let operator_history = review_operator_history(
        workspace_root,
        policy.audit_export.tool_history_limit,
        policy.audit_export.recent_event_limit,
    )?;
    let tool_history = inspect::tool_execution_history(
        workspace_root,
        policy.audit_export.tool_history_limit,
        None,
        None,
        None,
    )?;
    let exported_at = Utc::now().to_rfc3339();
    let export_root = resolve_workspace_path(workspace_root, &policy.audit_export.export_root);
    fs::create_dir_all(&export_root)
        .with_context(|| format!("failed to create {}", export_root.display()))?;
    let pruned_export_count =
        prune_expired_exports(&export_root, policy.audit_export.retention_days)?;
    let export_path = export_root.join(format!(
        "enterprise-audit-{}.json",
        Utc::now().format("%Y%m%dT%H%M%SZ")
    ));
    let bundle = EnterpriseAuditExportBundle {
        exported_at: exported_at.clone(),
        note: request.note.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }),
        policy,
        enterprise_foundations: foundations,
        governance: access.governance,
        supervision: admin.supervision,
        operator_history: operator_history.clone(),
        tool_history,
    };
    fs::write(&export_path, serde_json::to_vec_pretty(&bundle)?)
        .with_context(|| format!("failed to write {}", export_path.display()))?;

    Ok(EnterpriseAuditExportReceipt {
        status: "ok".to_string(),
        detail: format!(
            "Enterprise audit bundle exported with {} recent event(s), {} review event(s), and {} tool execution record(s).",
            bundle.enterprise_foundations.recent_events.len(),
            bundle.operator_history.entries.len(),
            bundle.tool_history.entries.len()
        ),
        exported_at,
        export_path: export_path.display().to_string(),
        recent_event_count: bundle.enterprise_foundations.recent_events.len(),
        tool_history_count: bundle.tool_history.entries.len(),
        operator_event_count: bundle.operator_history.entries.len(),
        pruned_export_count,
    })
}

fn review_operator_history(
    workspace_root: &Path,
    tool_history_limit: usize,
    review_event_limit: usize,
) -> Result<inspect::ToolExecutionHistoryReport> {
    let history = inspect::tool_execution_history(
        workspace_root,
        tool_history_limit.max(review_event_limit).max(1) * 3,
        Some("runtime_tool"),
        None,
        None,
    )?;
    let mut entries = history
        .entries
        .into_iter()
        .filter(|entry| {
            entry.tool_name.starts_with("enterprise.")
                || entry.tool_name.starts_with("orchestration.active.")
                || entry.tool_name.starts_with("mobile.command.")
        })
        .collect::<Vec<_>>();
    entries.truncate(review_event_limit.max(1));
    Ok(inspect::ToolExecutionHistoryReport {
        limit: review_event_limit.max(1),
        entries,
    })
}

fn list_recent_exports(
    workspace_root: &Path,
    policy: &EnterpriseAuditExportPolicy,
) -> Result<Vec<EnterpriseAuditExportSummary>> {
    let export_root = resolve_workspace_path(workspace_root, &policy.export_root);
    if !export_root.exists() {
        return Ok(Vec::new());
    }

    let mut exports = Vec::new();
    for path in list_export_paths(&export_root)? {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let Some(summary) = export_summary_from_value(&path, &value) else {
            continue;
        };
        exports.push(summary);
    }

    exports.sort_by(|left, right| right.exported_at.cmp(&left.exported_at));
    exports.truncate(policy.export_history_limit.max(1));
    Ok(exports)
}

fn list_export_paths(export_root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(export_root)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn prune_expired_exports(export_root: &Path, retention_days: usize) -> Result<usize> {
    if !export_root.exists() {
        return Ok(0);
    }

    let cutoff = Utc::now() - Duration::days(retention_days.max(1) as i64);
    let mut pruned = 0;
    for path in list_export_paths(export_root)? {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let Some(exported_at_raw) = value.get("exported_at").and_then(|field| field.as_str())
        else {
            continue;
        };
        let Ok(exported_at) = chrono::DateTime::parse_from_rfc3339(exported_at_raw) else {
            continue;
        };
        if exported_at.with_timezone(&Utc) < cutoff {
            fs::remove_file(&path).with_context(|| {
                format!("failed to remove expired audit bundle {}", path.display())
            })?;
            pruned += 1;
        }
    }
    Ok(pruned)
}

fn export_summary_from_value(
    path: &Path,
    value: &serde_json::Value,
) -> Option<EnterpriseAuditExportSummary> {
    let exported_at = value.get("exported_at")?.as_str()?.to_string();
    let note = value
        .get("note")
        .and_then(|field| field.as_str())
        .map(ToString::to_string);
    let recent_event_count = value
        .get("enterprise_foundations")
        .and_then(|field| field.get("recent_events"))
        .and_then(|field| field.as_array())
        .map(|events| events.len())
        .unwrap_or(0);
    let tool_history_count = value
        .get("tool_history")
        .and_then(|field| field.get("entries"))
        .and_then(|field| field.as_array())
        .map(|entries| entries.len())
        .unwrap_or(0);
    let operator_event_count = value
        .get("operator_history")
        .and_then(|field| field.get("entries"))
        .and_then(|field| field.as_array())
        .map(|entries| entries.len())
        .unwrap_or(0);
    let governance_rule_count = value
        .get("governance")
        .and_then(|field| field.get("rules"))
        .and_then(|field| field.as_array())
        .map(|rules| {
            rules
                .iter()
                .filter(|rule| {
                    rule.get("active")
                        .and_then(|field| field.as_bool())
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);
    let attention_required_count = value
        .get("supervision")
        .and_then(|field| field.get("attention_required_count"))
        .and_then(|field| field.as_u64())
        .map(|value| value as usize)
        .unwrap_or(0);

    Some(EnterpriseAuditExportSummary {
        exported_at,
        export_path: path.display().to_string(),
        note,
        recent_event_count,
        tool_history_count,
        operator_event_count,
        governance_rule_count,
        attention_required_count,
    })
}

fn browser_summary_from_config(config: &AppConfig) -> EnterpriseBrowserPolicySummary {
    let mut allowed_backends: Vec<String> = config
        .external_backends
        .allowed_backends
        .iter()
        .map(|value| normalize_backend_name(value))
        .collect();
    allowed_backends.sort();
    allowed_backends.dedup();
    EnterpriseBrowserPolicySummary {
        allowed_backends,
        allow_local_cli_wrappers: config.external_backends.allow_local_cli_wrappers,
        allow_cloud_agent_execution: config.external_backends.allow_cloud_agent_execution,
        audit_log_path: config.external_backends.audit_log_path.clone(),
        command_env_allowlist: config.external_backends.command_env_allowlist.clone(),
    }
}

fn save_manifest(workspace_root: &Path, manifest: &EnterprisePolicyManifest) -> Result<()> {
    let path = enterprise_policy_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(manifest)?).with_context(|| {
        format!(
            "failed to write enterprise policy manifest {}",
            path.display()
        )
    })?;
    Ok(())
}

fn normalize_browser_backends(values: &[String]) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for value in values {
        let candidate = normalize_backend_name(value);
        match candidate.as_str() {
            "native" | "native_cdp" | "native_browser" | "rust" => {
                normalized.insert("native_cdp".to_string());
            }
            "agent_browser" | "agent_browser_cli" | "agentbrowser" => {
                normalized.insert("agent_browser_cli".to_string());
            }
            other => anyhow::bail!(
                "unsupported browser backend '{}'; expected native_cdp or agent_browser_cli",
                other
            ),
        }
    }
    Ok(normalized.into_iter().collect())
}

fn normalize_env_allowlist(values: &[String]) -> Vec<String> {
    let mut normalized = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            normalized.insert(trimmed.to_string());
        }
    }
    normalized.into_iter().collect()
}

fn normalize_backend_name(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace('-', "_")
}

fn resolve_workspace_path(workspace_root: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace_root.join(candidate)
    }
}

fn validate_approval_policy(value: &str) -> Result<()> {
    match value.trim() {
        "none" | "side_effects" | "always" => Ok(()),
        other => Err(anyhow!(
            "invalid approval policy '{}'; expected none, side_effects, or always",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EnterpriseAuditExportPolicyUpdate, EnterpriseAuditExportRequest,
        EnterpriseBrowserPolicyUpdate, EnterpriseMobileApprovalPolicyUpdate,
        EnterprisePolicyUpdateRequest, approval_required_for_command, export_audit_bundle,
        review_summary, summary, update_policy,
    };
    use anyhow::Result;
    use openrustclaw_core::config::AppConfig;
    use openrustclaw_mobile::protocol::DeviceCommandKind;
    use std::fs;
    use tempfile::tempdir;

    use crate::commands::enterprise_access::{
        EnterpriseAccessBootstrapRequest, bootstrap_manifest,
    };
    use crate::commands::inspect::{append_tool_execution_record, new_tool_execution_record};
    use crate::commands::runtime;

    #[test]
    fn summary_reports_default_enterprise_policy_surface() -> Result<()> {
        let root = tempdir().expect("tempdir");

        let report = summary(
            root.path(),
            &root
                .path()
                .join("config/default.toml")
                .display()
                .to_string(),
        )?;
        assert_eq!(report.approval_policy, "side_effects");
        assert!(report.mobile.send_message_requires_approval);
        assert!(report.mobile.push_notification_requires_approval);
        assert!(!report.mobile.sync_now_requires_approval);
        assert_eq!(report.audit_export.recent_event_limit, 25);
        assert_eq!(report.audit_export.retention_days, 30);
        assert_eq!(report.audit_export.export_history_limit, 12);
        Ok(())
    }

    #[test]
    fn update_policy_writes_runtime_config_and_mobile_overrides() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let config_path = root.path().join("config/default.toml");
        runtime::write_config_with_backup(
            &config_path.display().to_string(),
            &AppConfig::default(),
        )?;

        let report = update_policy(
            root.path(),
            &config_path.display().to_string(),
            EnterprisePolicyUpdateRequest {
                approval_policy: Some("always".to_string()),
                browser: Some(EnterpriseBrowserPolicyUpdate {
                    allowed_backends: Some(vec![
                        "native".to_string(),
                        "agent_browser_cli".to_string(),
                    ]),
                    allow_local_cli_wrappers: Some(false),
                    allow_cloud_agent_execution: Some(true),
                    command_env_allowlist: Some(vec![
                        "PATH".to_string(),
                        "HOME".to_string(),
                        "PATH".to_string(),
                    ]),
                }),
                mobile: Some(EnterpriseMobileApprovalPolicyUpdate {
                    send_message_requires_approval: Some(false),
                    push_notification_requires_approval: None,
                    sync_now_requires_approval: Some(true),
                }),
                audit_export: Some(EnterpriseAuditExportPolicyUpdate {
                    export_root: Some(".claw/control/enterprise/custom-exports".to_string()),
                    recent_event_limit: Some(7),
                    tool_history_limit: Some(9),
                    retention_days: Some(14),
                    export_history_limit: Some(4),
                }),
            },
        )?;

        assert_eq!(report.approval_policy, "always");
        assert_eq!(
            report.browser.allowed_backends,
            vec!["agent_browser_cli".to_string(), "native_cdp".to_string()]
        );
        assert!(!report.browser.allow_local_cli_wrappers);
        assert!(report.browser.allow_cloud_agent_execution);
        assert_eq!(
            report.browser.command_env_allowlist,
            vec!["HOME".to_string(), "PATH".to_string()]
        );
        assert!(!report.mobile.send_message_requires_approval);
        assert!(report.mobile.sync_now_requires_approval);
        assert_eq!(report.audit_export.recent_event_limit, 7);
        assert_eq!(report.audit_export.tool_history_limit, 9);
        assert_eq!(report.audit_export.retention_days, 14);
        assert_eq!(report.audit_export.export_history_limit, 4);

        assert!(!approval_required_for_command(
            root.path(),
            &DeviceCommandKind::SendMessage
        ));
        assert!(approval_required_for_command(
            root.path(),
            &DeviceCommandKind::SyncNow
        ));

        Ok(())
    }

    #[test]
    fn export_audit_bundle_writes_durable_json_bundle() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let config_path = root.path().join("config/default.toml");
        runtime::write_config_with_backup(
            &config_path.display().to_string(),
            &AppConfig::default(),
        )?;
        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )?;

        update_policy(
            root.path(),
            &config_path.display().to_string(),
            EnterprisePolicyUpdateRequest {
                approval_policy: None,
                browser: None,
                mobile: None,
                audit_export: Some(EnterpriseAuditExportPolicyUpdate {
                    export_root: Some(".claw/control/enterprise/exports".to_string()),
                    recent_event_limit: Some(5),
                    tool_history_limit: Some(5),
                    retention_days: Some(30),
                    export_history_limit: Some(5),
                }),
            },
        )?;

        let mut record = new_tool_execution_record(
            "enterprise.access.bootstrap",
            "runtime_tool",
            "success",
            "success",
            12,
            None,
            None,
            None,
            None,
        );
        record.created_at = "2026-03-27T10:00:00Z".to_string();
        append_tool_execution_record(root.path(), &record)?;

        let receipt = export_audit_bundle(
            root.path(),
            &config_path.display().to_string(),
            EnterpriseAuditExportRequest {
                note: Some("phase 17 verification".to_string()),
            },
        )?;

        assert_eq!(receipt.status, "ok");
        assert!(receipt.export_path.ends_with(".json"));
        assert_eq!(receipt.pruned_export_count, 0);
        let payload = fs::read_to_string(&receipt.export_path)?;
        assert!(payload.contains("\"enterprise_foundations\""));
        assert!(payload.contains("\"governance\""));
        assert!(payload.contains("\"operator_history\""));
        assert!(payload.contains("\"tool_history\""));
        assert!(payload.contains("phase 17 verification"));
        Ok(())
    }

    #[test]
    fn review_summary_reports_recent_exports_and_retention_contract() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let config_path = root.path().join("config/default.toml");
        runtime::write_config_with_backup(
            &config_path.display().to_string(),
            &AppConfig::default(),
        )?;
        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )?;
        update_policy(
            root.path(),
            &config_path.display().to_string(),
            EnterprisePolicyUpdateRequest {
                approval_policy: None,
                browser: None,
                mobile: None,
                audit_export: Some(EnterpriseAuditExportPolicyUpdate {
                    export_root: Some(".claw/control/enterprise/review-exports".to_string()),
                    recent_event_limit: Some(4),
                    tool_history_limit: Some(6),
                    retention_days: Some(10),
                    export_history_limit: Some(3),
                }),
            },
        )?;

        let export = export_audit_bundle(
            root.path(),
            &config_path.display().to_string(),
            EnterpriseAuditExportRequest {
                note: Some("review package".to_string()),
            },
        )?;

        let review = review_summary(root.path(), &config_path.display().to_string())?;
        assert_eq!(review.status, "ok");
        assert_eq!(review.retention_days, 10);
        assert_eq!(review.export_history_limit, 3);
        assert_eq!(review.recent_exports.len(), 1);
        assert_eq!(review.recent_exports[0].export_path, export.export_path);
        assert!(review.governance.dual_approval_rule_count >= 1);
        Ok(())
    }
}
