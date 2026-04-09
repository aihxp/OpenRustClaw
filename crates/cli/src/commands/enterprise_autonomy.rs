use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::{control, enterprise_access, orchestrate};

const ENTERPRISE_AUTONOMY_VERSION: u32 = 1;
fn default_version() -> u32 {
    ENTERPRISE_AUTONOMY_VERSION
}

fn default_mode_label() -> String {
    "God Mode".to_string()
}

fn default_scope() -> String {
    "workspace".to_string()
}

fn default_override_policy() -> control::AutonomyPolicy {
    control::AutonomyPolicy {
        autonomy_level: "yolo".to_string(),
        yolo_mode: true,
        steering_enabled: true,
        decision_learning_enabled: true,
        critic_enabled: true,
        max_delegations: 8,
        max_iterations: 16,
        max_runtime_secs: 1800,
        max_lesson_hints: 8,
        approval_policy: "none".to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAutonomyManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default = "default_mode_label")]
    pub mode_label: String,
    #[serde(default = "default_scope")]
    pub scope: String,
    #[serde(default)]
    pub enabled_at: Option<String>,
    #[serde(default)]
    pub enabled_by: Option<String>,
    #[serde(default)]
    pub disabled_at: Option<String>,
    #[serde(default)]
    pub disabled_by: Option<String>,
    #[serde(default)]
    pub disable_reason: Option<String>,
    #[serde(default)]
    pub kill_switch_triggered: bool,
    #[serde(default)]
    pub kill_switch_at: Option<String>,
    #[serde(default)]
    pub kill_switch_by: Option<String>,
    #[serde(default)]
    pub kill_switch_reason: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub expired_at: Option<String>,
    #[serde(default)]
    pub expired_reason: Option<String>,
    #[serde(default = "default_override_policy")]
    pub override_policy: control::AutonomyPolicy,
    #[serde(default)]
    pub baseline_policy: control::AutonomyPolicy,
}

impl Default for EnterpriseAutonomyManifest {
    fn default() -> Self {
        Self {
            version: ENTERPRISE_AUTONOMY_VERSION,
            enabled: false,
            updated_at: None,
            note: None,
            mode_label: default_mode_label(),
            scope: default_scope(),
            enabled_at: None,
            enabled_by: None,
            disabled_at: None,
            disabled_by: None,
            disable_reason: None,
            kill_switch_triggered: false,
            kill_switch_at: None,
            kill_switch_by: None,
            kill_switch_reason: None,
            expires_at: None,
            expired_at: None,
            expired_reason: None,
            override_policy: default_override_policy(),
            baseline_policy: control::AutonomyPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAutonomyEnableRequest {
    pub operator_id: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub max_delegations: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub max_lesson_hints: Option<usize>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAutonomyDisableRequest {
    pub operator_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAutonomyKillSwitchRequest {
    pub operator_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAutonomyEvent {
    pub created_at: String,
    pub kind: String,
    pub mode_label: String,
    pub scope: String,
    pub operator_id: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    pub enabled: bool,
    pub kill_switch_triggered: bool,
    pub override_policy: control::AutonomyPolicy,
    pub baseline_policy: control::AutonomyPolicy,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub affected_run_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAutonomyRunSummary {
    pub run_id: String,
    pub mode_label: String,
    pub created_at: String,
    pub status: String,
    pub approval_policy: String,
    pub autonomy_level: String,
    pub worker_count: usize,
    pub failed_count: usize,
    pub receipt_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAutonomyExecutionSummary {
    pub active_run_count: usize,
    pub recent_run_count: usize,
    pub killed_run_count: usize,
    pub detail: String,
    pub active_runs: Vec<EnterpriseAutonomyRunSummary>,
    pub recent_runs: Vec<EnterpriseAutonomyRunSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseAutonomyReport {
    pub status: String,
    pub detail: String,
    pub access_boundary_active: bool,
    pub mode_label: String,
    pub scope: String,
    pub manifest_path: String,
    pub events_path: String,
    pub enabled: bool,
    pub updated_at: Option<String>,
    pub note: Option<String>,
    pub enabled_at: Option<String>,
    pub enabled_by: Option<String>,
    pub disabled_at: Option<String>,
    pub disabled_by: Option<String>,
    pub disable_reason: Option<String>,
    pub kill_switch_triggered: bool,
    pub kill_switch_at: Option<String>,
    pub kill_switch_by: Option<String>,
    pub kill_switch_reason: Option<String>,
    pub expires_at: Option<String>,
    pub expired_at: Option<String>,
    pub expired_reason: Option<String>,
    pub governance_scope: String,
    pub override_policy: control::AutonomyPolicy,
    pub baseline_policy: control::AutonomyPolicy,
    pub execution: EnterpriseAutonomyExecutionSummary,
    pub recent_events: Vec<EnterpriseAutonomyEvent>,
}

pub fn enterprise_autonomy_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("enterprise")
        .join("full-autonomy.json")
}

pub fn enterprise_autonomy_events_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("enterprise")
        .join("full-autonomy-events.jsonl")
}

pub fn load_manifest(workspace_root: &Path) -> Result<Option<EnterpriseAutonomyManifest>> {
    let path = enterprise_autonomy_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

pub fn recent_events(workspace_root: &Path, limit: usize) -> Result<Vec<EnterpriseAutonomyEvent>> {
    let path = enterprise_autonomy_events_path(workspace_root);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file =
        fs::File::open(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        events.push(
            serde_json::from_str::<EnterpriseAutonomyEvent>(trimmed)
                .with_context(|| format!("failed to parse {}", path.display()))?,
        );
    }
    events.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    events.truncate(limit.max(1));
    Ok(events)
}

pub fn enable(
    workspace_root: &Path,
    request: EnterpriseAutonomyEnableRequest,
) -> Result<EnterpriseAutonomyManifest> {
    require_bootstrapped_access(workspace_root)?;
    validate_operator_id(&request.operator_id)?;

    let mut manifest = refresh_manifest_state(workspace_root)?;
    let current_runtime = control::runtime_autonomy_policy(workspace_root)?;
    if manifest.enabled {
        anyhow::bail!("God Mode is already enabled");
    }

    let now = Utc::now().to_rfc3339();
    let override_policy = build_override_policy(&request)?;
    control::set_runtime_autonomy(workspace_root, &override_policy)?;

    manifest.enabled = true;
    manifest.updated_at = Some(now.clone());
    manifest.note = trim_optional(request.note);
    manifest.mode_label = default_mode_label();
    manifest.scope = default_scope();
    manifest.enabled_at = Some(now.clone());
    manifest.enabled_by = Some(request.operator_id.clone());
    manifest.disabled_at = None;
    manifest.disabled_by = None;
    manifest.disable_reason = None;
    manifest.kill_switch_triggered = false;
    manifest.kill_switch_at = None;
    manifest.kill_switch_by = None;
    manifest.kill_switch_reason = None;
    manifest.expires_at = expiry_from_ttl(request.ttl_secs);
    manifest.expired_at = None;
    manifest.expired_reason = None;
    manifest.override_policy = override_policy.clone();
    manifest.baseline_policy = current_runtime.clone();

    save_manifest(workspace_root, &manifest)?;
    append_event(
        workspace_root,
        EnterpriseAutonomyEvent {
            created_at: now,
            kind: "enabled".to_string(),
            mode_label: manifest.mode_label.clone(),
            scope: manifest.scope.clone(),
            operator_id: request.operator_id,
            note: manifest.note.clone(),
            reason: None,
            enabled: true,
            kill_switch_triggered: false,
            override_policy,
            baseline_policy: current_runtime,
            expires_at: manifest.expires_at.clone(),
            affected_run_ids: Vec::new(),
        },
    )?;
    Ok(manifest)
}

pub fn disable(
    workspace_root: &Path,
    request: EnterpriseAutonomyDisableRequest,
) -> Result<EnterpriseAutonomyManifest> {
    require_bootstrapped_access(workspace_root)?;
    validate_operator_id(&request.operator_id)?;

    let mut manifest = refresh_manifest_state(workspace_root)?;
    if !manifest.enabled {
        anyhow::bail!("God Mode is not enabled");
    }

    control::set_runtime_autonomy(workspace_root, &manifest.baseline_policy)?;
    let now = Utc::now().to_rfc3339();
    manifest.enabled = false;
    manifest.updated_at = Some(now.clone());
    manifest.disabled_at = Some(now.clone());
    manifest.disabled_by = Some(request.operator_id.clone());
    manifest.disable_reason = trim_optional(request.reason);

    save_manifest(workspace_root, &manifest)?;
    append_event(
        workspace_root,
        EnterpriseAutonomyEvent {
            created_at: now,
            kind: "disabled".to_string(),
            mode_label: manifest.mode_label.clone(),
            scope: manifest.scope.clone(),
            operator_id: request.operator_id,
            note: manifest.note.clone(),
            reason: manifest.disable_reason.clone(),
            enabled: false,
            kill_switch_triggered: false,
            override_policy: manifest.override_policy.clone(),
            baseline_policy: manifest.baseline_policy.clone(),
            expires_at: manifest.expires_at.clone(),
            affected_run_ids: Vec::new(),
        },
    )?;
    Ok(manifest)
}

pub fn kill_switch(
    workspace_root: &Path,
    request: EnterpriseAutonomyKillSwitchRequest,
) -> Result<EnterpriseAutonomyManifest> {
    require_bootstrapped_access(workspace_root)?;
    validate_operator_id(&request.operator_id)?;

    let mut manifest = refresh_manifest_state(workspace_root)?;
    if !manifest.enabled {
        anyhow::bail!("God Mode is not enabled");
    }

    let mut affected_run_ids = Vec::new();
    for run in orchestrate::list_active_runs(workspace_root, true, usize::MAX)?
        .into_iter()
        .filter(is_full_autonomy_active_run)
    {
        orchestrate::kill_active_run(
            workspace_root,
            &run.run_id,
            orchestrate::ActiveRunInterventionRequest {
                requested_by: Some(request.operator_id.clone()),
                reason: trim_optional(request.reason.clone()),
                rollback_reference: None,
            },
        )?;
        affected_run_ids.push(run.run_id);
    }

    control::set_runtime_autonomy(workspace_root, &manifest.baseline_policy)?;
    let now = Utc::now().to_rfc3339();
    let reason = trim_optional(request.reason)
        .or_else(|| Some("kill switch triggered by enterprise operator".to_string()));
    manifest.enabled = false;
    manifest.updated_at = Some(now.clone());
    manifest.disabled_at = Some(now.clone());
    manifest.disabled_by = Some(request.operator_id.clone());
    manifest.disable_reason = reason.clone();
    manifest.kill_switch_triggered = true;
    manifest.kill_switch_at = Some(now.clone());
    manifest.kill_switch_by = Some(request.operator_id.clone());
    manifest.kill_switch_reason = reason.clone();

    save_manifest(workspace_root, &manifest)?;
    append_event(
        workspace_root,
        EnterpriseAutonomyEvent {
            created_at: now,
            kind: "kill_switch".to_string(),
            mode_label: manifest.mode_label.clone(),
            scope: manifest.scope.clone(),
            operator_id: request.operator_id,
            note: manifest.note.clone(),
            reason,
            enabled: false,
            kill_switch_triggered: true,
            override_policy: manifest.override_policy.clone(),
            baseline_policy: manifest.baseline_policy.clone(),
            expires_at: manifest.expires_at.clone(),
            affected_run_ids,
        },
    )?;
    Ok(manifest)
}

pub fn summary(workspace_root: &Path, limit: usize) -> Result<EnterpriseAutonomyReport> {
    let manifest = refresh_manifest_state(workspace_root)?;
    let access_boundary_active = enterprise_access::access_is_configured(workspace_root)?;
    let recent_events = recent_events(workspace_root, limit)?;
    let active_runs = summarize_active_runs(workspace_root, limit)?;
    let recent_runs = summarize_recent_runs(workspace_root, limit)?;
    let killed_run_count = recent_events
        .iter()
        .filter(|event| event.kind == "kill_switch")
        .map(|event| event.affected_run_ids.len())
        .sum();

    let execution = EnterpriseAutonomyExecutionSummary {
        active_run_count: active_runs.len(),
        recent_run_count: recent_runs.len(),
        killed_run_count,
        detail: if active_runs.is_empty() && recent_runs.is_empty() {
            "No full-autonomy execution evidence has been recorded yet.".to_string()
        } else {
            format!(
                "{} active full-autonomy run(s), {} recent full-autonomy receipt(s), {} run(s) affected by kill-switch events.",
                active_runs.len(),
                recent_runs.len(),
                killed_run_count
            )
        },
        active_runs,
        recent_runs,
    };

    let status = if !access_boundary_active {
        "bootstrap_required"
    } else if manifest.enabled {
        "active"
    } else if manifest.kill_switch_triggered {
        "stopped"
    } else if manifest.expired_at.is_some() {
        "expired"
    } else {
        "ready"
    }
    .to_string();

    let detail = if !access_boundary_active {
        "Enterprise access is not bootstrapped yet. Full autonomy remains unavailable until the enterprise operator boundary is configured.".to_string()
    } else if manifest.enabled {
        let expiry_detail = manifest
            .expires_at
            .as_deref()
            .map(|value| {
                format!(" It is scheduled to expire at `{value}` unless disabled earlier.")
            })
            .unwrap_or_default();
        format!(
            "{} is enabled by `{}` with autonomy `{}` and approval policy `{}`. The stronger lane stays reversible through explicit disable, expiry, and kill-switch actions.{}",
            manifest.mode_label,
            manifest.enabled_by.as_deref().unwrap_or("-"),
            manifest.override_policy.autonomy_level,
            manifest.override_policy.approval_policy,
            expiry_detail
        )
    } else if manifest.kill_switch_triggered {
        format!(
            "{} is currently stopped after a kill-switch action by `{}`. The runtime has been restored to the baseline autonomy policy.",
            manifest.mode_label,
            manifest.kill_switch_by.as_deref().unwrap_or("-")
        )
    } else if manifest.expired_at.is_some() {
        format!(
            "{} expired at `{}` and the runtime has already been restored to the baseline autonomy policy.",
            manifest.mode_label,
            manifest.expired_at.as_deref().unwrap_or("-")
        )
    } else {
        format!(
            "{} is currently disabled. Operators can enable it explicitly as a separate enterprise override without changing the default trust-first runtime.",
            manifest.mode_label
        )
    };

    Ok(EnterpriseAutonomyReport {
        status,
        detail,
        access_boundary_active,
        mode_label: manifest.mode_label.clone(),
        scope: manifest.scope.clone(),
        manifest_path: enterprise_autonomy_path(workspace_root)
            .display()
            .to_string(),
        events_path: enterprise_autonomy_events_path(workspace_root)
            .display()
            .to_string(),
        enabled: manifest.enabled,
        updated_at: manifest.updated_at,
        note: manifest.note,
        enabled_at: manifest.enabled_at,
        enabled_by: manifest.enabled_by,
        disabled_at: manifest.disabled_at,
        disabled_by: manifest.disabled_by,
        disable_reason: manifest.disable_reason,
        kill_switch_triggered: manifest.kill_switch_triggered,
        kill_switch_at: manifest.kill_switch_at,
        kill_switch_by: manifest.kill_switch_by,
        kill_switch_reason: manifest.kill_switch_reason,
        expires_at: manifest.expires_at,
        expired_at: manifest.expired_at,
        expired_reason: manifest.expired_reason,
        governance_scope: "enterprise.full_autonomy.manage".to_string(),
        override_policy: manifest.override_policy,
        baseline_policy: manifest.baseline_policy,
        execution,
        recent_events,
    })
}

fn summarize_recent_runs(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<EnterpriseAutonomyRunSummary>> {
    let mut runs = Vec::new();
    for summary in orchestrate::list_runs(workspace_root, limit.max(1) * 5)? {
        let record = orchestrate::read_run(workspace_root, &summary.receipt_id)?;
        if !is_full_autonomy_policy(&record.routing.autonomy) {
            continue;
        }
        runs.push(EnterpriseAutonomyRunSummary {
            run_id: summary.run_id,
            mode_label: default_mode_label(),
            created_at: summary.created_at,
            status: summary.lifecycle_state,
            approval_policy: record.routing.autonomy.approval_policy,
            autonomy_level: record.routing.autonomy.autonomy_level,
            worker_count: summary.worker_count,
            failed_count: summary.failed_count,
            receipt_path: summary.receipt_path,
        });
        if runs.len() >= limit.max(1) {
            break;
        }
    }
    Ok(runs)
}

fn summarize_active_runs(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<EnterpriseAutonomyRunSummary>> {
    let mut runs = Vec::new();
    for run in orchestrate::list_active_runs(workspace_root, true, limit.max(1) * 5)? {
        if !is_full_autonomy_active_run(&run) {
            continue;
        }
        let autonomy = run
            .routing
            .as_ref()
            .map(|routing| routing.autonomy.clone())
            .unwrap_or_default();
        runs.push(EnterpriseAutonomyRunSummary {
            run_id: run.run_id,
            mode_label: default_mode_label(),
            created_at: run.created_at,
            status: run.lifecycle.state,
            approval_policy: autonomy.approval_policy,
            autonomy_level: autonomy.autonomy_level,
            worker_count: run.worker_count,
            failed_count: usize::from(run.last_error.is_some()),
            receipt_path: run.receipt_path.unwrap_or_default(),
        });
        if runs.len() >= limit.max(1) {
            break;
        }
    }
    Ok(runs)
}

fn is_full_autonomy_active_run(run: &orchestrate::ActiveOrchestrationRun) -> bool {
    run.routing
        .as_ref()
        .map(|routing| is_full_autonomy_policy(&routing.autonomy))
        .unwrap_or(false)
}

fn is_full_autonomy_policy(policy: &control::AutonomyPolicy) -> bool {
    policy.autonomy_level == "yolo" && policy.approval_policy == "none"
}

fn build_override_policy(
    request: &EnterpriseAutonomyEnableRequest,
) -> Result<control::AutonomyPolicy> {
    let mut policy = default_override_policy();
    if let Some(max_delegations) = request.max_delegations {
        if max_delegations == 0 {
            anyhow::bail!("max_delegations must be at least 1");
        }
        policy.max_delegations = max_delegations;
    }
    if let Some(max_iterations) = request.max_iterations {
        if max_iterations == 0 {
            anyhow::bail!("max_iterations must be at least 1");
        }
        policy.max_iterations = max_iterations;
    }
    if let Some(max_runtime_secs) = request.max_runtime_secs {
        if max_runtime_secs == 0 {
            anyhow::bail!("max_runtime_secs must be at least 1");
        }
        policy.max_runtime_secs = max_runtime_secs;
    }
    if let Some(max_lesson_hints) = request.max_lesson_hints {
        if max_lesson_hints == 0 {
            anyhow::bail!("max_lesson_hints must be at least 1");
        }
        policy.max_lesson_hints = max_lesson_hints;
    }
    Ok(policy)
}

fn require_bootstrapped_access(workspace_root: &Path) -> Result<()> {
    if !enterprise_access::access_is_configured(workspace_root)? {
        anyhow::bail!("enterprise access is not bootstrapped yet");
    }
    Ok(())
}

fn validate_operator_id(operator_id: &str) -> Result<()> {
    if operator_id.trim().is_empty() {
        anyhow::bail!("operator_id is required");
    }
    Ok(())
}

fn expiry_from_ttl(ttl_secs: Option<u64>) -> Option<String> {
    ttl_secs.and_then(|ttl_secs| {
        if ttl_secs == 0 {
            None
        } else {
            Some(
                (Utc::now() + Duration::seconds(ttl_secs.min(i64::MAX as u64) as i64)).to_rfc3339(),
            )
        }
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| anyhow::anyhow!("invalid RFC3339 timestamp '{value}': {error}"))
}

fn refresh_manifest_state(workspace_root: &Path) -> Result<EnterpriseAutonomyManifest> {
    let Some(mut manifest) = load_manifest(workspace_root)? else {
        return Ok(EnterpriseAutonomyManifest::default());
    };

    let Some(expires_at) = manifest.expires_at.as_deref() else {
        return Ok(manifest);
    };
    if !manifest.enabled || parse_timestamp(expires_at)? > Utc::now() {
        return Ok(manifest);
    }

    control::set_runtime_autonomy(workspace_root, &manifest.baseline_policy)?;
    let now = Utc::now().to_rfc3339();
    let reason = Some("God Mode expired after its TTL boundary.".to_string());
    manifest.enabled = false;
    manifest.updated_at = Some(now.clone());
    manifest.disabled_at = Some(now.clone());
    manifest.disabled_by = Some("system".to_string());
    manifest.disable_reason = reason.clone();
    manifest.expired_at = Some(now.clone());
    manifest.expired_reason = reason.clone();

    save_manifest(workspace_root, &manifest)?;
    append_event(
        workspace_root,
        EnterpriseAutonomyEvent {
            created_at: now,
            kind: "expired".to_string(),
            mode_label: manifest.mode_label.clone(),
            scope: manifest.scope.clone(),
            operator_id: "system".to_string(),
            note: manifest.note.clone(),
            reason,
            enabled: false,
            kill_switch_triggered: false,
            override_policy: manifest.override_policy.clone(),
            baseline_policy: manifest.baseline_policy.clone(),
            expires_at: manifest.expires_at.clone(),
            affected_run_ids: Vec::new(),
        },
    )?;
    Ok(manifest)
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn save_manifest(workspace_root: &Path, manifest: &EnterpriseAutonomyManifest) -> Result<()> {
    let path = enterprise_autonomy_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(manifest)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn append_event(workspace_root: &Path, event: EnterpriseAutonomyEvent) -> Result<()> {
    let path = enterprise_autonomy_events_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to write {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(&event)?)
        .with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        EnterpriseAutonomyDisableRequest, EnterpriseAutonomyEnableRequest,
        EnterpriseAutonomyKillSwitchRequest, EnterpriseAutonomyManifest, default_override_policy,
        disable, enable, kill_switch, load_manifest, save_manifest, summary,
    };
    use crate::commands::{enterprise_access, orchestrate};
    use anyhow::Result;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;

    fn bootstrap(root: &std::path::Path) -> Result<()> {
        enterprise_access::bootstrap_manifest(
            root,
            enterprise_access::EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )?;
        Ok(())
    }

    fn yolo_routing() -> orchestrate::RoutingDecision {
        orchestrate::RoutingDecision {
            execution_mode: "multi_claw".to_string(),
            route_source: "test".to_string(),
            task_id: None,
            category: None,
            selected_claw_id: "orchestrator".to_string(),
            selected_claw_role: "orchestrator".to_string(),
            selected_agent_profile_id: "agent".to_string(),
            selected_model_profile_id: "model".to_string(),
            selected_model: orchestrate::ResolvedModelDecision {
                requested_profile_id: "model".to_string(),
                selected_profile_id: "model".to_string(),
                provider: "openai".to_string(),
                model: "gpt".to_string(),
                fallback_path: Vec::new(),
                warnings: Vec::new(),
            },
            available_workers: vec!["worker-1".to_string()],
            autonomy: super::default_override_policy(),
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            applied_lessons: Vec::new(),
            steering_notes: Vec::new(),
            warnings: Vec::new(),
            request_overrides: orchestrate::OrchestrationRequestOverrides::default(),
        }
    }

    #[test]
    fn enable_and_disable_round_trip_runtime_policy() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap(root.path())?;

        let manifest = enable(
            root.path(),
            EnterpriseAutonomyEnableRequest {
                operator_id: "owner-1".to_string(),
                note: Some("full send".to_string()),
                max_delegations: Some(12),
                max_iterations: Some(20),
                max_runtime_secs: Some(2400),
                max_lesson_hints: Some(9),
                ttl_secs: None,
            },
        )?;
        assert!(manifest.enabled);
        assert_eq!(
            crate::commands::control::runtime_autonomy_policy(root.path())?.approval_policy,
            "none"
        );

        let disabled = disable(
            root.path(),
            EnterpriseAutonomyDisableRequest {
                operator_id: "owner-1".to_string(),
                reason: Some("done".to_string()),
            },
        )?;
        assert!(!disabled.enabled);
        assert_eq!(
            crate::commands::control::runtime_autonomy_policy(root.path())?.approval_policy,
            crate::commands::control::AutonomyPolicy::default().approval_policy
        );
        Ok(())
    }

    #[test]
    fn kill_switch_restores_baseline_and_marks_matching_runs() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap(root.path())?;
        enable(
            root.path(),
            EnterpriseAutonomyEnableRequest {
                operator_id: "owner-1".to_string(),
                note: None,
                max_delegations: None,
                max_iterations: None,
                max_runtime_secs: None,
                max_lesson_hints: None,
                ttl_secs: None,
            },
        )?;

        orchestrate::write_active_run(
            root.path(),
            &orchestrate::ActiveOrchestrationRun {
                run_id: "run-yolo".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                started_at: None,
                finished_at: None,
                status: "running".to_string(),
                request: orchestrate::OrchestrationRequest::default(),
                routing: Some(yolo_routing()),
                current_stage: None,
                current_actor_type: None,
                current_actor_id: None,
                current_note: None,
                lifecycle: orchestrate::SupervisionLifecycleSummary::default(),
                pause_requested: false,
                kill_requested: false,
                checkpoint_count: 0,
                trace_count: 0,
                worker_count: 1,
                relationship_count: 0,
                resource_totals: None,
                receipt_id: None,
                receipt_path: None,
                last_error: None,
            },
        )?;

        let manifest = kill_switch(
            root.path(),
            EnterpriseAutonomyKillSwitchRequest {
                operator_id: "owner-1".to_string(),
                reason: Some("too risky".to_string()),
            },
        )?;
        assert!(manifest.kill_switch_triggered);
        assert!(!manifest.enabled);
        assert!(
            orchestrate::read_active_run(root.path(), "run-yolo")?.kill_requested,
            "kill switch should mark matching active run"
        );
        Ok(())
    }

    #[test]
    fn summary_reports_execution_evidence_and_events() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap(root.path())?;
        enable(
            root.path(),
            EnterpriseAutonomyEnableRequest {
                operator_id: "owner-1".to_string(),
                note: Some("operator override".to_string()),
                max_delegations: None,
                max_iterations: None,
                max_runtime_secs: None,
                max_lesson_hints: None,
                ttl_secs: None,
            },
        )?;

        let report = summary(root.path(), 5)?;
        assert!(report.enabled);
        assert_eq!(report.status, "active");
        assert_eq!(report.mode_label, "God Mode");
        assert_eq!(report.governance_scope, "enterprise.full_autonomy.manage");
        assert_eq!(report.recent_events.len(), 1);
        assert!(load_manifest(root.path())?.is_some());
        Ok(())
    }

    #[test]
    fn summary_expires_god_mode_and_restores_baseline() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap(root.path())?;
        let mut manifest = EnterpriseAutonomyManifest::default();
        manifest.enabled = true;
        manifest.updated_at = Some(Utc::now().to_rfc3339());
        manifest.enabled_at = manifest.updated_at.clone();
        manifest.enabled_by = Some("owner-1".to_string());
        manifest.override_policy = default_override_policy();
        manifest.baseline_policy = crate::commands::control::AutonomyPolicy::default();
        manifest.expires_at = Some((Utc::now() - Duration::seconds(1)).to_rfc3339());
        save_manifest(root.path(), &manifest)?;
        crate::commands::control::set_runtime_autonomy(root.path(), &manifest.override_policy)?;

        let report = summary(root.path(), 5)?;
        assert_eq!(report.status, "expired");
        assert!(!report.enabled);
        assert!(report.expired_at.is_some());
        assert_eq!(
            crate::commands::control::runtime_autonomy_policy(root.path())?.approval_policy,
            crate::commands::control::AutonomyPolicy::default().approval_policy
        );
        Ok(())
    }

    #[test]
    fn reenable_after_expiry_captures_restored_baseline() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap(root.path())?;

        let baseline = crate::commands::control::AutonomyPolicy {
            autonomy_level: "managed".to_string(),
            yolo_mode: false,
            steering_enabled: true,
            decision_learning_enabled: true,
            critic_enabled: true,
            max_delegations: 2,
            max_iterations: 3,
            max_runtime_secs: 120,
            max_lesson_hints: 2,
            approval_policy: "side_effects".to_string(),
        };
        crate::commands::control::set_runtime_autonomy(root.path(), &baseline)?;

        let mut manifest = EnterpriseAutonomyManifest::default();
        manifest.enabled = true;
        manifest.updated_at = Some(Utc::now().to_rfc3339());
        manifest.enabled_at = manifest.updated_at.clone();
        manifest.enabled_by = Some("owner-1".to_string());
        manifest.override_policy = default_override_policy();
        manifest.baseline_policy = baseline.clone();
        manifest.expires_at = Some((Utc::now() - Duration::seconds(1)).to_rfc3339());
        save_manifest(root.path(), &manifest)?;
        crate::commands::control::set_runtime_autonomy(root.path(), &manifest.override_policy)?;

        let enabled = enable(
            root.path(),
            EnterpriseAutonomyEnableRequest {
                operator_id: "owner-1".to_string(),
                note: Some("re-enable after expiry".to_string()),
                max_delegations: None,
                max_iterations: None,
                max_runtime_secs: None,
                max_lesson_hints: None,
                ttl_secs: None,
            },
        )?;

        assert_eq!(
            enabled.baseline_policy.approval_policy,
            baseline.approval_policy
        );
        assert_eq!(
            enabled.baseline_policy.max_delegations,
            baseline.max_delegations
        );
        assert_eq!(
            crate::commands::control::runtime_autonomy_policy(root.path())?.approval_policy,
            "none"
        );

        let disabled = disable(
            root.path(),
            EnterpriseAutonomyDisableRequest {
                operator_id: "owner-1".to_string(),
                reason: Some("done".to_string()),
            },
        )?;
        assert_eq!(
            disabled.baseline_policy.approval_policy,
            baseline.approval_policy
        );
        assert_eq!(
            crate::commands::control::runtime_autonomy_policy(root.path())?.approval_policy,
            baseline.approval_policy
        );
        Ok(())
    }
}
