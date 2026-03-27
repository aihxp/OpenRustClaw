use anyhow::{Context, Result, anyhow};
use axum::http::{HeaderMap, Method};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::control;

pub const OPERATOR_ID_HEADER: &str = "x-openrustclaw-operator-id";
pub const OPERATOR_TOKEN_HEADER: &str = "x-openrustclaw-operator-token";
pub const APPROVER_ID_HEADER: &str = "x-openrustclaw-approver-id";
pub const APPROVER_TOKEN_HEADER: &str = "x-openrustclaw-approver-token";

const ENTERPRISE_ACCESS_VERSION: u32 = 1;
const ENTERPRISE_SCOPE_IDENTITY_WRITE: &str = "enterprise.identity.write";
const ENTERPRISE_SCOPE_CONFIG_WRITE: &str = "enterprise.config.write";
const ENTERPRISE_SCOPE_AUDIT_EXPORT: &str = "enterprise.audit.export";
const ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE: &str = "enterprise.full_autonomy.manage";
const ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE: &str = "enterprise.mobile.command.manage";
const ENTERPRISE_SCOPE_RUNTIME_CONTROL: &str = "enterprise.runtime.control";
const ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE: &str = "enterprise.skills.auth.manage";
const APPROVAL_MODE_SINGLE: &str = "single";
const APPROVAL_MODE_DUAL: &str = "dual";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAccessManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub organization: EnterpriseOrganization,
    #[serde(default)]
    pub operators: Vec<EnterpriseOperatorRecord>,
    #[serde(default = "default_governance_policy")]
    pub governance: EnterpriseGovernancePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseOrganization {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseOperatorRecord {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub role: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_true")]
    pub active: bool,
    pub token_sha256: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseGovernancePolicy {
    #[serde(default = "default_governance_rules")]
    pub rules: Vec<EnterpriseGovernanceRule>,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseGovernanceRule {
    pub scope: String,
    pub approval_mode: String,
    #[serde(default)]
    pub requester_roles: Vec<String>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default = "default_true")]
    pub forbid_self_approval: bool,
    #[serde(default = "default_true")]
    pub active: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseGovernanceRuleRequest {
    pub scope: String,
    pub approval_mode: String,
    #[serde(default)]
    pub requester_roles: Vec<String>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default = "default_true")]
    pub forbid_self_approval: bool,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAccessBootstrapRequest {
    pub organization_id: String,
    pub organization_name: String,
    pub owner_id: String,
    #[serde(default)]
    pub owner_name: Option<String>,
    #[serde(default)]
    pub owner_email: Option<String>,
    pub owner_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAccessOperatorRequest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub role: String,
    pub token: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_true")]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAuthenticatedOperator {
    pub id: String,
    pub role: String,
    pub scopes: Vec<String>,
    pub approval_mode: String,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub governance_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterpriseProtectedRoute {
    pub method: String,
    pub path: String,
    pub scope: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy)]
struct EnterpriseProtectedRouteSpec {
    method: &'static str,
    prefix: &'static str,
    suffix: &'static str,
    scope: &'static str,
    detail: &'static str,
}

const ENTERPRISE_PROTECTED_ROUTE_SPECS: &[EnterpriseProtectedRouteSpec] = &[
    EnterpriseProtectedRouteSpec {
        method: "PUT",
        prefix: "/control/enterprise/policy",
        suffix: "",
        scope: ENTERPRISE_SCOPE_CONFIG_WRITE,
        detail: "Enterprise policy writes change approval, browser, mobile, and export controls.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/audit/export",
        suffix: "",
        scope: ENTERPRISE_SCOPE_AUDIT_EXPORT,
        detail: "Enterprise audit export writes durable evidence bundles for operator review.",
    },
    EnterpriseProtectedRouteSpec {
        method: "PUT",
        prefix: "/control/config",
        suffix: "",
        scope: ENTERPRISE_SCOPE_CONFIG_WRITE,
        detail: "Control config writes can change runtime behavior and provider posture.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/access/operators",
        suffix: "",
        scope: ENTERPRISE_SCOPE_IDENTITY_WRITE,
        detail: "Operator provisioning changes the enterprise identity boundary.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/governance/rules",
        suffix: "",
        scope: ENTERPRISE_SCOPE_CONFIG_WRITE,
        detail: "Enterprise governance rule updates change approval-chain and separation-of-duties behavior.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/autonomy/enable",
        suffix: "",
        scope: ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE,
        detail: "Enabling full autonomy activates the stronger operator-gated execution lane.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/autonomy/disable",
        suffix: "",
        scope: ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE,
        detail: "Disabling full autonomy restores the baseline runtime autonomy contract.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/enterprise/autonomy/kill-switch",
        suffix: "",
        scope: ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE,
        detail: "The full-autonomy kill switch disables the stronger lane and stops matching active runs.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/mobile/commands/dispatch",
        suffix: "",
        scope: ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE,
        detail: "Dispatching mobile commands can trigger approval-sensitive device side effects.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/mobile/commands/",
        suffix: "/approve",
        scope: ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE,
        detail: "Approving a mobile command releases a pending device side effect.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/mobile/commands/",
        suffix: "/reject",
        scope: ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE,
        detail: "Rejecting a mobile command resolves an approval-sensitive operator decision.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/mobile/capabilities/execute",
        suffix: "",
        scope: ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE,
        detail: "Executing mobile capabilities can trigger remote device side effects.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/runtime/reload",
        suffix: "",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Runtime reload changes the active operator environment.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/orchestration/active/",
        suffix: "/pause",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Pausing an orchestration run changes supervised runtime behavior.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/orchestration/active/",
        suffix: "/resume",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Resuming an orchestration run changes supervised runtime behavior.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/orchestration/active/",
        suffix: "/kill",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Killing an orchestration run stops a supervised runtime workflow.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/orchestration/active/",
        suffix: "/escalate",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Escalating an orchestration run requires explicit operator review.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/orchestration/active/",
        suffix: "/rollback",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Rolling back an orchestration run records an explicit supervised recovery action.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/runtime/switch-provider",
        suffix: "",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Provider switching changes the active production model lane.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/runtime/switch-model",
        suffix: "",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Model switching changes the active production model contract.",
    },
    EnterpriseProtectedRouteSpec {
        method: "PUT",
        prefix: "/control/runtime/vault/",
        suffix: "",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Vault writes alter runtime secret material.",
    },
    EnterpriseProtectedRouteSpec {
        method: "DELETE",
        prefix: "/control/runtime/vault/",
        suffix: "",
        scope: ENTERPRISE_SCOPE_RUNTIME_CONTROL,
        detail: "Vault deletes remove runtime secret material.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/skills/auth-plugins/bind",
        suffix: "",
        scope: ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE,
        detail: "Binding an auth plugin changes enterprise-connected identity integrations.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/skills/auth-plugins/",
        suffix: "/authorize",
        scope: ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE,
        detail: "Auth-plugin authorize flows create enterprise-facing authorization grants.",
    },
    EnterpriseProtectedRouteSpec {
        method: "POST",
        prefix: "/control/skills/auth-plugins/",
        suffix: "/exchange",
        scope: ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE,
        detail: "Auth-plugin token exchange resolves enterprise-facing authorization state.",
    },
];

fn default_version() -> u32 {
    ENTERPRISE_ACCESS_VERSION
}

fn default_governance_policy() -> EnterpriseGovernancePolicy {
    EnterpriseGovernancePolicy {
        rules: default_governance_rules(),
        updated_at: String::new(),
    }
}

fn default_governance_rules() -> Vec<EnterpriseGovernanceRule> {
    all_scope_names()
        .into_iter()
        .map(|scope| default_governance_rule(&scope))
        .collect()
}

fn default_true() -> bool {
    true
}

pub fn enterprise_access_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("enterprise")
        .join("access.json")
}

pub fn load_manifest(workspace_root: &Path) -> Result<Option<EnterpriseAccessManifest>> {
    let path = enterprise_access_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read enterprise access manifest {}",
            path.display()
        )
    })?;
    let manifest: EnterpriseAccessManifest = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse enterprise access manifest {}",
            path.display()
        )
    })?;
    Ok(Some(manifest))
}

pub fn bootstrap_manifest(
    workspace_root: &Path,
    request: EnterpriseAccessBootstrapRequest,
) -> Result<EnterpriseAccessManifest> {
    if let Some(existing) = load_manifest(workspace_root)?
        && !existing.operators.is_empty()
    {
        anyhow::bail!("enterprise access is already bootstrapped");
    }

    validate_operator_id(&request.organization_id, "organization id")?;
    validate_presence(&request.organization_name, "organization name")?;
    validate_operator_id(&request.owner_id, "owner id")?;
    validate_secret(&request.owner_token, "owner token")?;

    let now = Utc::now().to_rfc3339();
    let manifest = EnterpriseAccessManifest {
        version: ENTERPRISE_ACCESS_VERSION,
        organization: EnterpriseOrganization {
            id: request.organization_id.trim().to_string(),
            name: request.organization_name.trim().to_string(),
            slug: Some(slugify(&request.organization_name)),
            created_at: now.clone(),
        },
        operators: vec![EnterpriseOperatorRecord {
            id: request.owner_id.trim().to_string(),
            name: request.owner_name.map(|value| value.trim().to_string()),
            email: request.owner_email.map(|value| value.trim().to_string()),
            role: "owner".to_string(),
            scopes: default_scopes_for_role("owner"),
            active: true,
            token_sha256: hash_secret(&request.owner_token),
            created_at: now.clone(),
            updated_at: now,
        }],
        governance: EnterpriseGovernancePolicy {
            rules: default_governance_rules(),
            updated_at: Utc::now().to_rfc3339(),
        },
    };
    save_manifest(workspace_root, &manifest)?;
    Ok(manifest)
}

pub fn upsert_operator(
    workspace_root: &Path,
    request: EnterpriseAccessOperatorRequest,
) -> Result<EnterpriseOperatorRecord> {
    let mut manifest = load_manifest(workspace_root)?
        .ok_or_else(|| anyhow!("enterprise access is not bootstrapped yet"))?;
    validate_operator_id(&request.id, "operator id")?;
    validate_secret(&request.token, "operator token")?;
    validate_role(&request.role)?;

    let now = Utc::now().to_rfc3339();
    let effective_scopes = merge_scopes(&default_scopes_for_role(&request.role), &request.scopes);
    let record = EnterpriseOperatorRecord {
        id: request.id.trim().to_string(),
        name: request.name.map(|value| value.trim().to_string()),
        email: request.email.map(|value| value.trim().to_string()),
        role: request.role.trim().to_string(),
        scopes: effective_scopes,
        active: request.active,
        token_sha256: hash_secret(&request.token),
        created_at: manifest
            .operators
            .iter()
            .find(|entry| entry.id == request.id.trim())
            .map(|entry| entry.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };

    if let Some(existing) = manifest
        .operators
        .iter_mut()
        .find(|entry| entry.id == record.id)
    {
        *existing = record.clone();
    } else {
        manifest.operators.push(record.clone());
    }

    manifest
        .operators
        .sort_by(|left, right| left.id.cmp(&right.id));
    save_manifest(workspace_root, &manifest)?;
    Ok(record)
}

pub fn upsert_governance_rule(
    workspace_root: &Path,
    request: EnterpriseGovernanceRuleRequest,
) -> Result<EnterpriseGovernanceRule> {
    let mut manifest = load_manifest(workspace_root)?
        .ok_or_else(|| anyhow!("enterprise access is not bootstrapped yet"))?;
    let rule = normalize_governance_rule_request(
        &request,
        manifest
            .governance
            .rules
            .iter()
            .find(|entry| entry.scope == request.scope),
    )?;

    if let Some(existing) = manifest
        .governance
        .rules
        .iter_mut()
        .find(|entry| entry.scope == rule.scope)
    {
        *existing = rule.clone();
    } else {
        manifest.governance.rules.push(rule.clone());
    }

    manifest.governance.rules = sort_governance_rules(manifest.governance.rules);
    manifest.governance.updated_at = Utc::now().to_rfc3339();
    save_manifest(workspace_root, &manifest)?;
    Ok(rule)
}

pub fn authenticate_request(
    workspace_root: &Path,
    headers: &HeaderMap,
    required_scope: &str,
) -> Result<EnterpriseAuthenticatedOperator> {
    let manifest = load_manifest(workspace_root)?
        .ok_or_else(|| anyhow!("enterprise access is not configured"))?;
    let operator = authenticate_named_operator(
        &manifest,
        headers,
        OPERATOR_ID_HEADER,
        OPERATOR_TOKEN_HEADER,
        "operator",
    )?;
    if !operator.scopes.iter().any(|scope| scope == required_scope) {
        anyhow::bail!(
            "enterprise operator '{}' lacks required scope '{}'",
            operator.id,
            required_scope
        );
    }

    let governance_rule = governance_rule_for_scope(&manifest, required_scope);
    if governance_rule.active && !role_allowed(&governance_rule.requester_roles, &operator.role) {
        anyhow::bail!(
            "enterprise operator '{}' with role '{}' is not allowed to request scope '{}' under enterprise governance",
            operator.id,
            operator.role,
            required_scope
        );
    }

    let approved_by = if governance_rule.active
        && governance_rule.approval_mode == APPROVAL_MODE_DUAL
    {
        let approver = authenticate_named_operator(
            &manifest,
            headers,
            APPROVER_ID_HEADER,
            APPROVER_TOKEN_HEADER,
            "approver",
        )?;
        if !approver.scopes.iter().any(|scope| scope == required_scope) {
            anyhow::bail!(
                "enterprise approver '{}' lacks required scope '{}'",
                approver.id,
                required_scope
            );
        }
        if !role_allowed(&governance_rule.approver_roles, &approver.role) {
            anyhow::bail!(
                "enterprise approver '{}' with role '{}' is not allowed to approve scope '{}' under enterprise governance",
                approver.id,
                approver.role,
                required_scope
            );
        }
        if governance_rule.forbid_self_approval && approver.id == operator.id {
            anyhow::bail!(
                "enterprise governance for scope '{}' requires a second operator; self-approval is not allowed",
                required_scope
            );
        }
        Some(approver.id)
    } else {
        None
    };

    Ok(EnterpriseAuthenticatedOperator {
        id: operator.id.clone(),
        role: operator.role.clone(),
        scopes: operator.scopes,
        approval_mode: governance_rule.approval_mode,
        approved_by,
        governance_scope: Some(required_scope.to_string()),
    })
}

pub fn protected_scope_for_request(method: &Method, path: &str) -> Option<&'static str> {
    ENTERPRISE_PROTECTED_ROUTE_SPECS
        .iter()
        .find(|spec| route_matches(spec, method, path))
        .map(|spec| spec.scope)
}

pub fn protected_routes() -> Vec<EnterpriseProtectedRoute> {
    ENTERPRISE_PROTECTED_ROUTE_SPECS
        .iter()
        .map(|spec| EnterpriseProtectedRoute {
            method: spec.method.to_string(),
            path: route_display_path(spec),
            scope: spec.scope.to_string(),
            detail: spec.detail.to_string(),
        })
        .collect()
}

pub fn role_defaults() -> Vec<(String, Vec<String>)> {
    ["owner", "admin", "operator", "auditor"]
        .into_iter()
        .map(|role| (role.to_string(), default_scopes_for_role(role)))
        .collect()
}

pub fn governance_defaults() -> Vec<EnterpriseGovernanceRule> {
    default_governance_rules()
}

pub fn access_is_configured(workspace_root: &Path) -> Result<bool> {
    Ok(load_manifest(workspace_root)?.is_some())
}

fn save_manifest(workspace_root: &Path, manifest: &EnterpriseAccessManifest) -> Result<()> {
    let path = enterprise_access_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(manifest)?).with_context(|| {
        format!(
            "failed to write enterprise access manifest {}",
            path.display()
        )
    })?;
    Ok(())
}

fn route_matches(spec: &EnterpriseProtectedRouteSpec, method: &Method, path: &str) -> bool {
    if method.as_str() != spec.method {
        return false;
    }
    if !path.starts_with(spec.prefix) {
        return false;
    }
    if spec.suffix.is_empty() {
        return path == spec.prefix
            || path.starts_with(&format!("{}/", spec.prefix.trim_end_matches('/')));
    }
    path.ends_with(spec.suffix)
}

fn route_display_path(spec: &EnterpriseProtectedRouteSpec) -> String {
    if spec.suffix.is_empty() {
        spec.prefix.to_string()
    } else {
        format!("{}{{id}}{}", spec.prefix, spec.suffix)
    }
}

fn default_governance_rule(scope: &str) -> EnterpriseGovernanceRule {
    match scope {
        ENTERPRISE_SCOPE_IDENTITY_WRITE => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_SINGLE.to_string(),
            requester_roles: vec!["owner".to_string(), "admin".to_string()],
            approver_roles: Vec::new(),
            forbid_self_approval: true,
            active: true,
            detail: "Enterprise identity writes stay restricted to owner or admin roles.".to_string(),
        },
        ENTERPRISE_SCOPE_CONFIG_WRITE => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_DUAL.to_string(),
            requester_roles: vec!["owner".to_string(), "admin".to_string()],
            approver_roles: vec!["owner".to_string(), "admin".to_string()],
            forbid_self_approval: true,
            active: true,
            detail: "Enterprise configuration and policy writes require a second owner or admin approver.".to_string(),
        },
        ENTERPRISE_SCOPE_AUDIT_EXPORT => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_SINGLE.to_string(),
            requester_roles: vec![
                "owner".to_string(),
                "admin".to_string(),
                "auditor".to_string(),
            ],
            approver_roles: Vec::new(),
            forbid_self_approval: true,
            active: true,
            detail: "Audit export stays available to enterprise reviewers without secondary approval.".to_string(),
        },
        ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_DUAL.to_string(),
            requester_roles: vec!["owner".to_string(), "admin".to_string()],
            approver_roles: vec!["owner".to_string(), "admin".to_string()],
            forbid_self_approval: true,
            active: true,
            detail: "Full-autonomy enablement and shutdown require dual owner or admin approval.".to_string(),
        },
        ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_SINGLE.to_string(),
            requester_roles: vec![
                "owner".to_string(),
                "admin".to_string(),
                "operator".to_string(),
            ],
            approver_roles: Vec::new(),
            forbid_self_approval: true,
            active: true,
            detail: "Mobile command actions stay scoped to operators, admins, or owners.".to_string(),
        },
        ENTERPRISE_SCOPE_RUNTIME_CONTROL => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_SINGLE.to_string(),
            requester_roles: vec![
                "owner".to_string(),
                "admin".to_string(),
                "operator".to_string(),
            ],
            approver_roles: Vec::new(),
            forbid_self_approval: true,
            active: true,
            detail: "Runtime supervision actions stay limited to actively scoped operators.".to_string(),
        },
        ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_DUAL.to_string(),
            requester_roles: vec!["owner".to_string(), "admin".to_string()],
            approver_roles: vec!["owner".to_string(), "admin".to_string()],
            forbid_self_approval: true,
            active: true,
            detail: "Enterprise-facing auth-plugin changes require a second owner or admin approver.".to_string(),
        },
        _ => EnterpriseGovernanceRule {
            scope: scope.to_string(),
            approval_mode: APPROVAL_MODE_SINGLE.to_string(),
            requester_roles: vec!["owner".to_string()],
            approver_roles: Vec::new(),
            forbid_self_approval: true,
            active: true,
            detail: "Enterprise governance defaults to a single scoped owner request.".to_string(),
        },
    }
}

fn default_scopes_for_role(role: &str) -> Vec<String> {
    match role {
        "owner" => all_scope_names(),
        "admin" => vec![
            ENTERPRISE_SCOPE_IDENTITY_WRITE.to_string(),
            ENTERPRISE_SCOPE_CONFIG_WRITE.to_string(),
            ENTERPRISE_SCOPE_AUDIT_EXPORT.to_string(),
            ENTERPRISE_SCOPE_FULL_AUTONOMY_MANAGE.to_string(),
            ENTERPRISE_SCOPE_RUNTIME_CONTROL.to_string(),
            ENTERPRISE_SCOPE_SKILLS_AUTH_MANAGE.to_string(),
        ],
        "operator" => vec![
            ENTERPRISE_SCOPE_MOBILE_COMMAND_MANAGE.to_string(),
            ENTERPRISE_SCOPE_RUNTIME_CONTROL.to_string(),
        ],
        "auditor" => vec![ENTERPRISE_SCOPE_AUDIT_EXPORT.to_string()],
        _ => Vec::new(),
    }
}

fn all_scope_names() -> Vec<String> {
    let mut scopes = BTreeSet::new();
    for spec in ENTERPRISE_PROTECTED_ROUTE_SPECS {
        scopes.insert(spec.scope.to_string());
    }
    scopes.into_iter().collect()
}

fn merge_scopes(defaults: &[String], explicit: &[String]) -> Vec<String> {
    let mut scopes = BTreeSet::new();
    for value in defaults.iter().chain(explicit.iter()) {
        if !value.trim().is_empty() {
            scopes.insert(value.trim().to_string());
        }
    }
    scopes.into_iter().collect()
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn governance_rule_for_scope(
    manifest: &EnterpriseAccessManifest,
    required_scope: &str,
) -> EnterpriseGovernanceRule {
    manifest
        .governance
        .rules
        .iter()
        .find(|entry| entry.scope == required_scope)
        .cloned()
        .unwrap_or_else(|| default_governance_rule(required_scope))
}

fn role_allowed(allowed_roles: &[String], actual_role: &str) -> bool {
    allowed_roles.is_empty() || allowed_roles.iter().any(|role| role == actual_role)
}

fn sort_governance_rules(rules: Vec<EnterpriseGovernanceRule>) -> Vec<EnterpriseGovernanceRule> {
    let mut normalized = rules;
    let scope_order = all_scope_names();
    normalized.sort_by(|left, right| {
        let left_index = scope_order
            .iter()
            .position(|scope| scope == &left.scope)
            .unwrap_or(scope_order.len());
        let right_index = scope_order
            .iter()
            .position(|scope| scope == &right.scope)
            .unwrap_or(scope_order.len());
        left_index
            .cmp(&right_index)
            .then_with(|| left.scope.cmp(&right.scope))
    });
    normalized
}

fn normalize_governance_rule_request(
    request: &EnterpriseGovernanceRuleRequest,
    existing: Option<&EnterpriseGovernanceRule>,
) -> Result<EnterpriseGovernanceRule> {
    validate_scope(&request.scope)?;
    validate_approval_mode(&request.approval_mode)?;
    validate_role_list(&request.requester_roles, "requester roles")?;
    validate_role_list(&request.approver_roles, "approver roles")?;
    if request.approval_mode.trim() == APPROVAL_MODE_DUAL && request.approver_roles.is_empty() {
        anyhow::bail!("dual approval governance requires at least one approver role");
    }

    let detail = request
        .detail
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| existing.map(|value| value.detail.clone()))
        .unwrap_or_else(|| default_governance_rule(request.scope.trim()).detail);

    Ok(EnterpriseGovernanceRule {
        scope: request.scope.trim().to_string(),
        approval_mode: request.approval_mode.trim().to_string(),
        requester_roles: sort_roles(request.requester_roles.clone()),
        approver_roles: sort_roles(request.approver_roles.clone()),
        forbid_self_approval: request.forbid_self_approval,
        active: request.active,
        detail,
    })
}

fn sort_roles(roles: Vec<String>) -> Vec<String> {
    let mut values = BTreeSet::new();
    for role in roles {
        if !role.trim().is_empty() {
            values.insert(role.trim().to_string());
        }
    }
    values.into_iter().collect()
}

fn authenticate_named_operator(
    manifest: &EnterpriseAccessManifest,
    headers: &HeaderMap,
    id_header: &str,
    token_header: &str,
    label: &str,
) -> Result<EnterpriseAuthenticatedOperator> {
    let operator_id = header_value(headers, id_header)
        .ok_or_else(|| anyhow!("missing required {} header `{}`", label, id_header))?;
    let operator_token = header_value(headers, token_header)
        .ok_or_else(|| anyhow!("missing required {} header `{}`", label, token_header))?;
    let operator = manifest
        .operators
        .iter()
        .find(|entry| entry.id == operator_id)
        .ok_or_else(|| anyhow!("unknown enterprise {} '{}'", label, operator_id))?;
    if !operator.active {
        anyhow::bail!("enterprise {} '{}' is inactive", label, operator.id);
    }
    if operator.token_sha256 != hash_secret(&operator_token) {
        anyhow::bail!("invalid enterprise {} token", label);
    }

    Ok(EnterpriseAuthenticatedOperator {
        id: operator.id.clone(),
        role: operator.role.clone(),
        scopes: merge_scopes(&default_scopes_for_role(&operator.role), &operator.scopes),
        approval_mode: APPROVAL_MODE_SINGLE.to_string(),
        approved_by: None,
        governance_scope: None,
    })
}

fn validate_scope(scope: &str) -> Result<()> {
    let normalized = scope.trim();
    if normalized.is_empty() {
        anyhow::bail!("governance scope is required");
    }
    if !all_scope_names().iter().any(|value| value == normalized) {
        anyhow::bail!("unsupported enterprise governance scope '{}'", normalized);
    }
    Ok(())
}

fn validate_approval_mode(mode: &str) -> Result<()> {
    match mode.trim() {
        APPROVAL_MODE_SINGLE | APPROVAL_MODE_DUAL => Ok(()),
        other => Err(anyhow!(
            "unsupported enterprise approval mode '{}'; expected single or dual",
            other
        )),
    }
}

fn validate_role_list(roles: &[String], label: &str) -> Result<()> {
    for role in roles {
        validate_role(role).with_context(|| format!("invalid {}", label))?;
    }
    Ok(())
}

fn validate_role(role: &str) -> Result<()> {
    match role.trim() {
        "owner" | "admin" | "operator" | "auditor" => Ok(()),
        other => Err(anyhow!(
            "unsupported enterprise operator role '{}'; expected owner, admin, operator, or auditor",
            other
        )),
    }
}

fn validate_operator_id(value: &str, label: &str) -> Result<()> {
    validate_presence(value, label)?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        anyhow::bail!(
            "{} must only contain ASCII letters, digits, '-' or '_'",
            label
        );
    }
    Ok(())
}

fn validate_presence(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{} is required", label);
    }
    Ok(())
}

fn validate_secret(value: &str, label: &str) -> Result<()> {
    if value.trim().len() < 8 {
        anyhow::bail!("{} must be at least 8 characters", label);
    }
    Ok(())
}

fn hash_secret(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn slugify(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::{
        APPROVER_ID_HEADER, APPROVER_TOKEN_HEADER, EnterpriseAccessBootstrapRequest,
        EnterpriseAccessOperatorRequest, EnterpriseGovernanceRuleRequest, OPERATOR_ID_HEADER,
        OPERATOR_TOKEN_HEADER, authenticate_request, bootstrap_manifest, governance_defaults,
        load_manifest, protected_scope_for_request, role_defaults, upsert_governance_rule,
        upsert_operator,
    };
    use anyhow::Result;
    use axum::http::{HeaderMap, HeaderValue, Method};
    use tempfile::tempdir;

    #[test]
    fn bootstrap_manifest_hashes_owner_token() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let manifest = bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: Some("Owner".to_string()),
                owner_email: None,
                owner_token: "super-secret-owner".to_string(),
            },
        )?;

        assert_eq!(manifest.organization.id, "acme");
        assert_eq!(manifest.operators.len(), 1);
        assert_ne!(manifest.operators[0].token_sha256, "super-secret-owner");
        let loaded = load_manifest(root.path())?.expect("manifest");
        assert_eq!(loaded.operators[0].role, "owner");
        Ok(())
    }

    #[test]
    fn authenticate_request_requires_scope_membership() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "super-secret-owner".to_string(),
            },
        )?;
        upsert_operator(
            root.path(),
            EnterpriseAccessOperatorRequest {
                id: "operator-1".to_string(),
                name: None,
                email: None,
                role: "operator".to_string(),
                token: "operator-secret".to_string(),
                scopes: Vec::new(),
                active: true,
            },
        )?;

        let mut headers = HeaderMap::new();
        headers.insert(OPERATOR_ID_HEADER, HeaderValue::from_static("operator-1"));
        headers.insert(
            OPERATOR_TOKEN_HEADER,
            HeaderValue::from_static("operator-secret"),
        );

        let auth = authenticate_request(root.path(), &headers, "enterprise.runtime.control")?;
        assert_eq!(auth.id, "operator-1");
        assert!(authenticate_request(root.path(), &headers, "enterprise.identity.write").is_err());
        Ok(())
    }

    #[test]
    fn governance_defaults_require_dual_approval_for_config_writes() {
        let defaults = governance_defaults();
        let config_rule = defaults
            .iter()
            .find(|rule| rule.scope == "enterprise.config.write")
            .expect("config write governance rule");
        assert_eq!(config_rule.approval_mode, "dual");
        assert!(config_rule.forbid_self_approval);
        let autonomy_rule = defaults
            .iter()
            .find(|rule| rule.scope == "enterprise.full_autonomy.manage")
            .expect("full autonomy governance rule");
        assert_eq!(autonomy_rule.approval_mode, "dual");
        assert!(autonomy_rule.forbid_self_approval);
    }

    #[test]
    fn authenticate_request_requires_secondary_approver_for_governed_scope() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "super-secret-owner".to_string(),
            },
        )?;
        upsert_operator(
            root.path(),
            EnterpriseAccessOperatorRequest {
                id: "admin-1".to_string(),
                name: None,
                email: None,
                role: "admin".to_string(),
                token: "admin-secret".to_string(),
                scopes: Vec::new(),
                active: true,
            },
        )?;

        let mut operator_only = HeaderMap::new();
        operator_only.insert(OPERATOR_ID_HEADER, HeaderValue::from_static("owner-1"));
        operator_only.insert(
            OPERATOR_TOKEN_HEADER,
            HeaderValue::from_static("super-secret-owner"),
        );
        assert!(
            authenticate_request(root.path(), &operator_only, "enterprise.config.write").is_err()
        );

        let mut self_approved = operator_only.clone();
        self_approved.insert(APPROVER_ID_HEADER, HeaderValue::from_static("owner-1"));
        self_approved.insert(
            APPROVER_TOKEN_HEADER,
            HeaderValue::from_static("super-secret-owner"),
        );
        assert!(
            authenticate_request(root.path(), &self_approved, "enterprise.config.write").is_err()
        );

        let mut dual_headers = operator_only;
        dual_headers.insert(APPROVER_ID_HEADER, HeaderValue::from_static("admin-1"));
        dual_headers.insert(
            APPROVER_TOKEN_HEADER,
            HeaderValue::from_static("admin-secret"),
        );
        let auth = authenticate_request(root.path(), &dual_headers, "enterprise.config.write")?;
        assert_eq!(auth.approval_mode, "dual");
        assert_eq!(auth.approved_by.as_deref(), Some("admin-1"));
        Ok(())
    }

    #[test]
    fn upsert_governance_rule_persists_role_and_mode_overrides() -> Result<()> {
        let root = tempdir().expect("tempdir");
        bootstrap_manifest(
            root.path(),
            EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "super-secret-owner".to_string(),
            },
        )?;

        let rule = upsert_governance_rule(
            root.path(),
            EnterpriseGovernanceRuleRequest {
                scope: "enterprise.runtime.control".to_string(),
                approval_mode: "dual".to_string(),
                requester_roles: vec!["admin".to_string(), "owner".to_string()],
                approver_roles: vec!["owner".to_string()],
                forbid_self_approval: true,
                active: true,
                detail: Some("runtime changes require owner approval".to_string()),
            },
        )?;

        assert_eq!(rule.approval_mode, "dual");
        let manifest = load_manifest(root.path())?.expect("manifest");
        let runtime_rule = manifest
            .governance
            .rules
            .iter()
            .find(|entry| entry.scope == "enterprise.runtime.control")
            .expect("runtime governance rule");
        assert_eq!(runtime_rule.approval_mode, "dual");
        assert_eq!(runtime_rule.approver_roles, vec!["owner".to_string()]);
        Ok(())
    }

    #[test]
    fn protected_scope_classifies_sensitive_routes() {
        assert_eq!(
            protected_scope_for_request(&Method::PUT, "/control/enterprise/policy"),
            Some("enterprise.config.write")
        );
        assert_eq!(
            protected_scope_for_request(&Method::POST, "/control/enterprise/audit/export"),
            Some("enterprise.audit.export")
        );
        assert_eq!(
            protected_scope_for_request(
                &Method::POST,
                "/control/orchestration/active/run-1/escalate"
            ),
            Some("enterprise.runtime.control")
        );
        assert_eq!(
            protected_scope_for_request(
                &Method::POST,
                "/control/orchestration/active/run-1/rollback"
            ),
            Some("enterprise.runtime.control")
        );
        assert_eq!(
            protected_scope_for_request(&Method::PUT, "/control/config"),
            Some("enterprise.config.write")
        );
        assert_eq!(
            protected_scope_for_request(&Method::POST, "/control/mobile/commands/abc/approve"),
            Some("enterprise.mobile.command.manage")
        );
        assert_eq!(
            protected_scope_for_request(&Method::POST, "/control/enterprise/governance/rules"),
            Some("enterprise.config.write")
        );
        assert_eq!(
            protected_scope_for_request(&Method::POST, "/control/enterprise/autonomy/enable"),
            Some("enterprise.full_autonomy.manage")
        );
        assert_eq!(
            protected_scope_for_request(
                &Method::POST,
                "/control/skills/auth-plugins/provider-1/authorize"
            ),
            Some("enterprise.skills.auth.manage")
        );
        assert_eq!(
            protected_scope_for_request(&Method::GET, "/control/enterprise/access"),
            None
        );
    }

    #[test]
    fn role_defaults_cover_owner_and_auditor() {
        let defaults = role_defaults();
        let owner = defaults
            .iter()
            .find(|(role, _)| role == "owner")
            .expect("owner");
        let auditor = defaults
            .iter()
            .find(|(role, _)| role == "auditor")
            .expect("auditor");
        assert!(!owner.1.is_empty());
        assert!(
            auditor
                .1
                .iter()
                .any(|scope| scope == "enterprise.audit.export")
        );
    }
}
