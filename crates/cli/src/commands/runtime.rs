//! Runtime configuration, vault, and provider/model switching helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use chrono::Utc;
use openrustclaw_app::runtime_maintenance_planning::{
    RuntimeMaintenancePlanningService, RuntimeRollbackPlanningRequest,
    RuntimeSelfUpdatePlanningRequest, RuntimeUpgradePlanningRequest,
};
use openrustclaw_app::runtime_provider_switch::{
    RuntimeConfigMutationSource, RuntimeProviderSwitchRequest, RuntimeProviderSwitchService,
};
use openrustclaw_app::runtime_reload_planning::{
    RuntimeAppliedSnapshot, RuntimeReloadPlan, RuntimeReloadPlanningService,
};
use openrustclaw_app::runtime_vault::{RuntimeVaultService, RuntimeVaultState};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::Error as CoreError;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_memory::{WorkspaceArtifactRegistry, artifacts::ArtifactClass};
use openrustclaw_providers::{
    AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider,
    openrouter::RouteStrategy,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

pub const DEFAULT_VAULT_PATH: &str = ".claw/control/runtime-vault.json";
pub const DEFAULT_RUNTIME_HEALTH_PATH: &str = ".claw/control/runtime-health.json";
pub const DEFAULT_RUNTIME_RELOAD_STATE_PATH: &str = ".claw/control/runtime-reload-state.json";
pub const DEFAULT_RUNTIME_BEACON_PATH: &str = ".claw/control/runtime-beacon.json";
pub const DEFAULT_RUNTIME_BACKUP_ROOT: &str = ".claw/runtime-backups";
pub const DEFAULT_RUNTIME_RELEASE_ROOT: &str = ".claw/runtime-releases";
pub const DEFAULT_RUNTIME_LOCK_PATH: &str = ".claw/control/runtime-lock.json";
pub const DEFAULT_SYSTEMD_SERVICE_NAME: &str = "openrustclaw.service";
pub const DEFAULT_LAUNCHD_LABEL: &str = "dev.openrustclaw.openrustclaw";
const DEFAULT_PASSPHRASE_ENV: &str = "OPENRUSTCLAW_VAULT_PASSPHRASE";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeVault {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedVaultFile {
    version: u32,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub config_path: String,
    pub network_mode: String,
    pub gateway_host: String,
    pub gateway_port: u16,
    pub allowed_origins: Vec<String>,
    pub trusted_proxy_enabled: bool,
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_plane_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_plane_fallback_chain: Vec<String>,
    pub control_plane_action_provider: String,
    pub control_plane_action_model: String,
    pub anthropic_model: String,
    pub openai_model: String,
    pub openrouter_model: String,
    pub gemini_model: String,
    pub ollama_model: String,
    pub vault_path: String,
    pub vault_present: bool,
    pub vault_unlocked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfigMigrationIssue {
    pub legacy_key: String,
    pub canonical_key: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfigMigrationReport {
    pub config_path: String,
    pub applied: bool,
    pub changed: bool,
    pub legacy_issues: Vec<RuntimeConfigMigrationIssue>,
    pub inferred_defaults: Vec<String>,
    pub backup_created: bool,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeUpgradePlan {
    pub generated_at: String,
    pub config_path: String,
    pub runtime_status: RuntimeStatus,
    pub runtime_health: RuntimeHealthReport,
    pub reload_plan: RuntimeReloadPlan,
    pub service_install_status: RuntimeServiceInstallStatus,
    pub lock_status: RuntimeLockStatus,
    pub control_plane_action_provider: String,
    pub control_plane_action_model: String,
    pub backup_command: String,
    pub log_rotation_command: String,
    pub migrate_config_command: String,
    pub ready_for_upgrade: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSelfUpdatePlan {
    pub generated_at: String,
    pub config_path: String,
    pub current_executable: String,
    pub artifact_path: String,
    pub artifact_exists: bool,
    pub artifact_executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_executable_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_size_bytes: Option<u64>,
    pub recommended_rollback_path: String,
    pub service_install_status: RuntimeServiceInstallStatus,
    pub lock_status: RuntimeLockStatus,
    pub control_plane_action_provider: String,
    pub control_plane_action_model: String,
    pub backup_command: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeRollbackPlan {
    pub generated_at: String,
    pub config_path: String,
    pub current_executable: String,
    pub rollback_artifact_path: String,
    pub rollback_artifact_exists: bool,
    pub rollback_artifact_executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_artifact_size_bytes: Option<u64>,
    pub service_install_status: RuntimeServiceInstallStatus,
    pub lock_status: RuntimeLockStatus,
    pub control_plane_action_provider: String,
    pub control_plane_action_model: String,
    pub backup_command: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealthProviderEntry {
    pub provider: String,
    pub role: String,
    pub model: String,
    pub configured: bool,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeArtifactHealth {
    pub artifact_count: usize,
    pub persona_artifact_count: usize,
    pub registry_path: String,
    pub model_family: String,
    pub included_for_default_model: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealthReport {
    pub generated_at: String,
    pub config_path: String,
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_plane_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_plane_fallback_chain: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_control_plane_provider: Option<String>,
    pub startup_fallback_valid: bool,
    pub degraded_control_plane_mode: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failover_recommendations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator_warnings: Vec<String>,
    pub providers: Vec<RuntimeHealthProviderEntry>,
    pub artifacts: RuntimeArtifactHealth,
}

#[derive(Debug, Clone)]
struct ProviderHealthProbe {
    healthy: bool,
    issue_kind: Option<String>,
    issue: Option<String>,
    model_available: Option<bool>,
    limit_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBeacon {
    pub generated_at: String,
    pub process_id: u32,
    pub gateway_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<i64>,
    pub sidecar_running: bool,
    pub default_provider: String,
    pub degraded_control_plane_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_control_plane_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeBackupManifest {
    version: u32,
    created_at: String,
    entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBackupSummary {
    pub created_at: String,
    pub backup_path: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeRestoreSummary {
    pub restored_at: String,
    pub backup_path: String,
    pub pre_restore_backup_path: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeServiceInstallStatus {
    pub service_manager: String,
    pub service_label: String,
    pub supported: bool,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_path: Option<String>,
    pub executable_path: String,
    pub workspace_root: String,
    pub config_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon_reload_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeLockRecord {
    acquired_at: String,
    process_id: u32,
    gateway_addr: String,
    config_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeLockStatus {
    pub path: String,
    pub present: bool,
    pub active: bool,
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_alive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquired_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeOperatorOpsSummary {
    pub generated_at: String,
    pub config_path: String,
    pub ready_for_managed_restart: bool,
    pub backup_root: String,
    pub release_root: String,
    pub service_install_status: RuntimeServiceInstallStatus,
    pub lock_status: RuntimeLockStatus,
    pub runtime_health: RuntimeHealthReport,
    pub beacon: RuntimeBeacon,
    pub reload_plan: RuntimeReloadPlan,
    pub recommended_actions: Vec<String>,
}

pub struct RuntimeLockGuard {
    path: PathBuf,
    process_id: u32,
}

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn vault_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_VAULT_PATH)
}

pub fn runtime_health_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_HEALTH_PATH)
}

pub fn runtime_reload_state_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root
        .as_ref()
        .join(DEFAULT_RUNTIME_RELOAD_STATE_PATH)
}

pub fn runtime_beacon_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_BEACON_PATH)
}

pub fn runtime_backup_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_BACKUP_ROOT)
}

pub fn runtime_release_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_RELEASE_ROOT)
}

pub fn runtime_lock_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_LOCK_PATH)
}

pub fn runtime_service_install_status(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeServiceInstallStatus> {
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    Ok(match preferred_service_manager() {
        RuntimeServiceManager::SystemdUser => {
            let service_path = user_systemd_service_path();
            let supported = systemd_user_service_supported();
            let installed = service_path
                .as_ref()
                .map(|path| path.exists())
                .unwrap_or(false);
            RuntimeServiceInstallStatus {
                service_manager: "systemd-user".to_string(),
                service_label: DEFAULT_SYSTEMD_SERVICE_NAME.to_string(),
                supported,
                installed,
                service_path: service_path.as_ref().map(|path| path.display().to_string()),
                executable_path: current_exe.display().to_string(),
                workspace_root: workspace_root.display().to_string(),
                config_path: resolved_config_path.display().to_string(),
                daemon_reload_command: supported
                    .then(|| "systemctl --user daemon-reload".to_string()),
                enable_command: supported
                    .then(|| format!("systemctl --user enable {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
                start_command: supported
                    .then(|| format!("systemctl --user start {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
                stop_command: supported
                    .then(|| format!("systemctl --user stop {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
                restart_command: supported
                    .then(|| format!("systemctl --user restart {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
            }
        }
        RuntimeServiceManager::LaunchdUser => {
            let service_path = user_launchd_service_path();
            let supported = launchd_user_service_supported();
            let installed = service_path
                .as_ref()
                .map(|path| path.exists())
                .unwrap_or(false);
            let domain_target = launchd_domain_target();
            let service_path_string = service_path.as_ref().map(|path| path.display().to_string());
            RuntimeServiceInstallStatus {
                service_manager: "launchd-user".to_string(),
                service_label: DEFAULT_LAUNCHD_LABEL.to_string(),
                supported,
                installed,
                service_path: service_path_string.clone(),
                executable_path: current_exe.display().to_string(),
                workspace_root: workspace_root.display().to_string(),
                config_path: resolved_config_path.display().to_string(),
                daemon_reload_command: supported.then(|| {
                    format!(
                        "launchctl bootout {domain}/{label} >/dev/null 2>&1 || true",
                        domain = domain_target,
                        label = DEFAULT_LAUNCHD_LABEL
                    )
                }),
                enable_command: supported.then(|| {
                    format!(
                        "launchctl enable {domain}/{label}",
                        domain = domain_target,
                        label = DEFAULT_LAUNCHD_LABEL
                    )
                }),
                start_command: supported.then(|| {
                    let path = service_path_string
                        .clone()
                        .unwrap_or_else(|| "~/Library/LaunchAgents".to_string());
                    format!(
                        "launchctl bootstrap {domain} \"{path}\"",
                        domain = domain_target,
                        path = path
                    )
                }),
                stop_command: supported.then(|| {
                    format!(
                        "launchctl bootout {domain}/{label}",
                        domain = domain_target,
                        label = DEFAULT_LAUNCHD_LABEL
                    )
                }),
                restart_command: supported.then(|| {
                    format!(
                        "launchctl kickstart -k {domain}/{label}",
                        domain = domain_target,
                        label = DEFAULT_LAUNCHD_LABEL
                    )
                }),
            }
        }
        RuntimeServiceManager::Unsupported => RuntimeServiceInstallStatus {
            service_manager: "unsupported".to_string(),
            service_label: DEFAULT_SYSTEMD_SERVICE_NAME.to_string(),
            supported: false,
            installed: false,
            service_path: None,
            executable_path: current_exe.display().to_string(),
            workspace_root: workspace_root.display().to_string(),
            config_path: resolved_config_path.display().to_string(),
            daemon_reload_command: None,
            enable_command: None,
            start_command: None,
            stop_command: None,
            restart_command: None,
        },
    })
}

pub fn install_runtime_user_service(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeServiceInstallStatus> {
    match preferred_service_manager() {
        RuntimeServiceManager::SystemdUser => {
            install_runtime_systemd_service(config_path, workspace_root)
        }
        RuntimeServiceManager::LaunchdUser => {
            install_runtime_launchd_service(config_path, workspace_root)
        }
        RuntimeServiceManager::Unsupported => {
            anyhow::bail!("No supported user service manager was detected on this host")
        }
    }
}

pub fn migrate_config(
    config_path: &str,
    workspace_root: &Path,
    apply: bool,
) -> Result<RuntimeConfigMigrationReport> {
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    let raw = fs::read_to_string(&resolved_config_path)
        .with_context(|| format!("Failed to read '{}'", resolved_config_path.display()))?;
    let mut value = raw
        .parse::<toml::Value>()
        .context("Failed to parse runtime config TOML")?;
    let mut legacy_issues = Vec::new();
    let mut inferred_defaults = Vec::new();

    migrate_legacy_string_key(
        &mut value,
        &["providers", "primary"],
        &["providers", "default_provider"],
        &mut legacy_issues,
    );
    migrate_legacy_string_key(
        &mut value,
        &["providers", "default"],
        &["providers", "default_provider"],
        &mut legacy_issues,
    );
    migrate_legacy_array_key(
        &mut value,
        &["providers", "fallbacks"],
        &["providers", "fallback_chain"],
        &mut legacy_issues,
    );
    migrate_legacy_string_key(
        &mut value,
        &["security", "require_authentication"],
        &["security", "require_auth"],
        &mut legacy_issues,
    );
    migrate_legacy_string_key(
        &mut value,
        &["gateway", "allowed_origin"],
        &["gateway", "allowed_origins"],
        &mut legacy_issues,
    );
    migrate_legacy_array_key(
        &mut value,
        &["gateway", "origin_whitelist"],
        &["gateway", "allowed_origins"],
        &mut legacy_issues,
    );
    migrate_legacy_bind_key(&mut value, &mut legacy_issues);
    infer_gateway_network_mode(&mut value, &mut inferred_defaults)?;

    let canonical = toml::from_str::<AppConfig>(&toml::to_string(&value)?)
        .context("Canonicalized config failed validation")?;
    let changed = !legacy_issues.is_empty() || !inferred_defaults.is_empty();
    if apply && changed {
        write_config_with_backup(config_path, &canonical)?;
    }

    let mut next_steps = Vec::new();
    if changed && !apply {
        next_steps.push(format!(
            "Run `openrustclaw runtime migrate-config --config {} --apply` to rewrite the canonical config with a timestamped backup.",
            config_path
        ));
    }
    next_steps.push(format!(
        "Run `openrustclaw runtime upgrade-plan --config {}` before the next production restart.",
        config_path
    ));

    Ok(RuntimeConfigMigrationReport {
        config_path: resolved_config_path.display().to_string(),
        applied: apply && changed,
        changed,
        legacy_issues,
        inferred_defaults,
        backup_created: apply && changed && resolved_config_path.exists(),
        next_steps,
    })
}

pub async fn runtime_upgrade_plan(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeUpgradePlan> {
    let runtime_status = runtime_status(config_path, workspace_root)?;
    let control_plane_action_provider = runtime_status.control_plane_action_provider.clone();
    let control_plane_action_model = runtime_status.control_plane_action_model.clone();
    let runtime_health = runtime_health_status(config_path, workspace_root, false).await?;
    let reload_plan = runtime_reload_plan(config_path, workspace_root)?;
    let service_install_status = runtime_service_install_status(config_path, workspace_root)?;
    let lock_status = runtime_lock_status(workspace_root)?;
    let default_or_fallback_healthy = runtime_health.startup_fallback_valid
        || runtime_health
            .providers
            .iter()
            .any(|entry| entry.provider == runtime_health.default_provider && entry.healthy);
    let plan = RuntimeMaintenancePlanningService::new().build_upgrade_plan(
        RuntimeUpgradePlanningRequest {
            generated_at: Utc::now().to_rfc3339(),
            config_path: config_path.to_string(),
            default_or_fallback_healthy,
            service_manager_supported: service_install_status.supported,
            service_installed: service_install_status.installed,
            restart_command: service_install_status.restart_command.clone(),
            lock_active: lock_status.active,
            reload_restart_required: reload_plan.restart_required,
        },
    );

    Ok(RuntimeUpgradePlan {
        generated_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        runtime_status,
        runtime_health,
        reload_plan,
        service_install_status,
        lock_status,
        control_plane_action_provider,
        control_plane_action_model,
        backup_command: plan.backup_command,
        log_rotation_command: plan.log_rotation_command,
        migrate_config_command: plan.migrate_config_command,
        ready_for_upgrade: plan.ready,
        blockers: plan.blockers,
        steps: plan.steps,
    })
}

pub async fn runtime_self_update_plan(
    config_path: &str,
    workspace_root: &Path,
    artifact_path: &str,
) -> Result<RuntimeSelfUpdatePlan> {
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let artifact = resolve_runtime_artifact_path(workspace_root, artifact_path);
    let artifact_exists = artifact.exists();
    let artifact_executable = if artifact_exists {
        path_is_executable(&artifact)?
    } else {
        false
    };
    let current_executable_size_bytes = file_size_bytes(&current_exe);
    let artifact_size_bytes = file_size_bytes(&artifact);
    let rollback_path = recommended_runtime_rollback_path(workspace_root, &current_exe);
    let service_install_status = runtime_service_install_status(config_path, workspace_root)?;
    let lock_status = runtime_lock_status(workspace_root)?;
    let upgrade_plan = runtime_upgrade_plan(config_path, workspace_root).await?;
    let artifact_is_file = artifact.is_file();
    let plan = RuntimeMaintenancePlanningService::new().build_self_update_plan(
        RuntimeSelfUpdatePlanningRequest {
            generated_at: Utc::now().to_rfc3339(),
            config_path: config_path.to_string(),
            current_executable: current_exe.display().to_string(),
            artifact_path: artifact.display().to_string(),
            recommended_rollback_path: rollback_path.display().to_string(),
            artifact_exists,
            artifact_is_file,
            artifact_executable,
            current_executable_matches_artifact: artifact == current_exe,
            service_installed: service_install_status.installed,
            restart_command: service_install_status.restart_command.clone(),
            lock_active: lock_status.active,
            inherited_upgrade_blockers: upgrade_plan.blockers.clone(),
        },
    );

    Ok(RuntimeSelfUpdatePlan {
        generated_at: Utc::now().to_rfc3339(),
        config_path: resolved_config_path.display().to_string(),
        current_executable: current_exe.display().to_string(),
        artifact_path: artifact.display().to_string(),
        artifact_exists,
        artifact_executable,
        current_executable_size_bytes,
        artifact_size_bytes,
        recommended_rollback_path: rollback_path.display().to_string(),
        service_install_status,
        lock_status,
        control_plane_action_provider: upgrade_plan.control_plane_action_provider,
        control_plane_action_model: upgrade_plan.control_plane_action_model,
        backup_command: plan.backup_command,
        ready: plan.ready,
        blockers: plan.blockers,
        steps: plan.steps,
    })
}

pub async fn runtime_rollback_plan(
    config_path: &str,
    workspace_root: &Path,
    artifact_path: &str,
) -> Result<RuntimeRollbackPlan> {
    let runtime_status = runtime_status(config_path, workspace_root)?;
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let artifact = resolve_runtime_artifact_path(workspace_root, artifact_path);
    let rollback_artifact_exists = artifact.exists();
    let rollback_artifact_executable = if rollback_artifact_exists {
        path_is_executable(&artifact)?
    } else {
        false
    };
    let rollback_artifact_size_bytes = file_size_bytes(&artifact);
    let service_install_status = runtime_service_install_status(config_path, workspace_root)?;
    let lock_status = runtime_lock_status(workspace_root)?;
    let plan = RuntimeMaintenancePlanningService::new().build_rollback_plan(
        RuntimeRollbackPlanningRequest {
            generated_at: Utc::now().to_rfc3339(),
            config_path: config_path.to_string(),
            current_executable: current_exe.display().to_string(),
            artifact_path: artifact.display().to_string(),
            artifact_exists: rollback_artifact_exists,
            artifact_is_file: artifact.is_file(),
            artifact_executable: rollback_artifact_executable,
            current_executable_matches_artifact: artifact == current_exe,
            service_installed: service_install_status.installed,
            restart_command: service_install_status.restart_command.clone(),
            lock_active: lock_status.active,
        },
    );

    Ok(RuntimeRollbackPlan {
        generated_at: Utc::now().to_rfc3339(),
        config_path: resolved_config_path.display().to_string(),
        current_executable: current_exe.display().to_string(),
        rollback_artifact_path: artifact.display().to_string(),
        rollback_artifact_exists,
        rollback_artifact_executable,
        rollback_artifact_size_bytes,
        service_install_status,
        lock_status,
        control_plane_action_provider: runtime_status.control_plane_action_provider,
        control_plane_action_model: runtime_status.control_plane_action_model,
        backup_command: plan.backup_command,
        ready: plan.ready,
        blockers: plan.blockers,
        steps: plan.steps,
    })
}

pub fn runtime_lock_status(workspace_root: &Path) -> Result<RuntimeLockStatus> {
    let path = runtime_lock_path_for(workspace_root);
    let Some(record) = load_runtime_lock(workspace_root)? else {
        return Ok(RuntimeLockStatus {
            path: path.display().to_string(),
            present: false,
            active: false,
            stale: false,
            process_id: None,
            owner_alive: None,
            acquired_at: None,
            gateway_addr: None,
            config_path: None,
        });
    };

    let owner_alive = process_is_alive(record.process_id);
    Ok(RuntimeLockStatus {
        path: path.display().to_string(),
        present: true,
        active: owner_alive,
        stale: !owner_alive,
        process_id: Some(record.process_id),
        owner_alive: Some(owner_alive),
        acquired_at: Some(record.acquired_at),
        gateway_addr: Some(record.gateway_addr),
        config_path: Some(record.config_path),
    })
}

pub fn acquire_runtime_lock(
    config_path: &str,
    workspace_root: &Path,
    gateway_addr: &str,
) -> Result<RuntimeLockGuard> {
    let path = runtime_lock_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    if let Some(existing) = load_runtime_lock(workspace_root)? {
        let alive = process_is_alive(existing.process_id);
        if alive && existing.process_id != std::process::id() {
            anyhow::bail!(
                "Runtime lock is already held by pid {} at {}; remove '{}' only if that process is gone",
                existing.process_id,
                existing.gateway_addr,
                path.display()
            );
        }
    }

    let record = RuntimeLockRecord {
        acquired_at: Utc::now().to_rfc3339(),
        process_id: std::process::id(),
        gateway_addr: gateway_addr.to_string(),
        config_path: config_path.to_string(),
    };
    save_runtime_lock(workspace_root, &record)?;

    Ok(RuntimeLockGuard {
        path,
        process_id: record.process_id,
    })
}

pub fn load_effective_config(config_path: &str, workspace_root: &Path) -> Result<AppConfig> {
    apply_runtime_secret_sources(workspace_root)?;
    AppConfig::load_from(config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))
}

pub fn runtime_status(config_path: &str, workspace_root: &Path) -> Result<RuntimeStatus> {
    let config = load_effective_config(config_path, workspace_root)?;
    let vault_path = vault_path_for(workspace_root);
    let vault_present = vault_path.exists();
    let vault_unlocked = if vault_present {
        load_vault(workspace_root, None).is_ok()
    } else {
        false
    };

    let control_plane_action_provider = effective_control_plane_action_provider(&config);
    let control_plane_action_model =
        provider_model_for(&config, &control_plane_action_provider).to_string();

    Ok(RuntimeStatus {
        config_path: config_path.to_string(),
        network_mode: config.gateway.network_mode.clone(),
        gateway_host: config.gateway.host.clone(),
        gateway_port: config.gateway.port,
        allowed_origins: config.gateway.allowed_origins.clone(),
        trusted_proxy_enabled: config.security.trusted_proxy_token_env.is_some(),
        default_provider: config.providers.default_provider.clone(),
        fallback_chain: config.providers.fallback_chain.clone(),
        control_plane_provider: config.providers.control_plane_provider.clone(),
        control_plane_fallback_chain: config.providers.control_plane_fallback_chain.clone(),
        control_plane_action_provider,
        control_plane_action_model,
        anthropic_model: config.providers.anthropic.model.clone(),
        openai_model: config.providers.openai.model.clone(),
        openrouter_model: config.providers.openrouter.model.clone(),
        gemini_model: config.providers.gemini.model.clone(),
        ollama_model: config.providers.ollama.model.clone(),
        vault_path: vault_path.display().to_string(),
        vault_present,
        vault_unlocked,
    })
}

pub async fn runtime_health_status(
    config_path: &str,
    workspace_root: &Path,
    refresh: bool,
) -> Result<RuntimeHealthReport> {
    if !refresh && let Some(report) = load_cached_runtime_health(workspace_root)? {
        return Ok(report);
    }
    scan_runtime_health(config_path, workspace_root).await
}

pub fn load_cached_runtime_health(workspace_root: &Path) -> Result<Option<RuntimeHealthReport>> {
    let path = runtime_health_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let report: RuntimeHealthReport =
        serde_json::from_str(&raw).context("Failed to parse cached runtime health report")?;
    Ok(Some(report))
}

pub fn load_applied_runtime_snapshot(
    workspace_root: &Path,
) -> Result<Option<RuntimeAppliedSnapshot>> {
    let path = runtime_reload_state_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let snapshot: RuntimeAppliedSnapshot =
        serde_json::from_str(&raw).context("Failed to parse runtime reload state")?;
    Ok(Some(snapshot))
}

pub fn load_cached_runtime_beacon(workspace_root: &Path) -> Result<Option<RuntimeBeacon>> {
    let path = runtime_beacon_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let beacon: RuntimeBeacon =
        serde_json::from_str(&raw).context("Failed to parse runtime beacon")?;
    Ok(Some(beacon))
}

fn load_runtime_lock(workspace_root: &Path) -> Result<Option<RuntimeLockRecord>> {
    let path = runtime_lock_path_for(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let record: RuntimeLockRecord =
        serde_json::from_str(&raw).context("Failed to parse runtime lock")?;
    Ok(Some(record))
}

pub async fn scan_runtime_health(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeHealthReport> {
    let config = load_effective_config(config_path, workspace_root)?;
    let previous_report = load_cached_runtime_health(workspace_root)?;

    let ordered = health_scan_provider_order(&config);

    let mut entries = Vec::new();
    for provider in &ordered {
        entries.push(scan_provider_health(provider, &config).await);
    }

    let control_plane_chain = control_plane_provider_order(&config);
    let control_plane_provider = config
        .providers
        .control_plane_provider
        .clone()
        .or_else(|| Some(config.providers.default_provider.clone()));
    let startup_fallback_valid = entries.iter().any(|entry| {
        control_plane_chain
            .iter()
            .skip(1)
            .any(|provider| provider == &entry.provider)
            && entry.healthy
    });
    let default_healthy = entries
        .iter()
        .find(|entry| Some(entry.provider.clone()) == control_plane_provider)
        .map(|entry| entry.healthy)
        .unwrap_or(false);
    let recommended_control_plane_provider = entries
        .iter()
        .find(|entry| {
            control_plane_chain
                .iter()
                .any(|provider| provider == &entry.provider)
                && entry.healthy
        })
        .map(|entry| entry.provider.clone())
        .or_else(|| {
            default_healthy
                .then(|| control_plane_provider.clone())
                .flatten()
        });

    let failover_recommendations = build_failover_recommendations(
        &config,
        &control_plane_chain,
        &entries,
        recommended_control_plane_provider.as_deref(),
    );
    let operator_warnings = build_runtime_operator_warnings(
        &config,
        &control_plane_chain,
        &entries,
        recommended_control_plane_provider.as_deref(),
        previous_report.as_ref(),
    );

    let artifacts = summarize_runtime_artifacts(workspace_root, &config)?;
    let report = RuntimeHealthReport {
        generated_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        default_provider: config.providers.default_provider.clone(),
        fallback_chain: config.providers.fallback_chain.clone(),
        control_plane_provider,
        control_plane_fallback_chain: config.providers.control_plane_fallback_chain.clone(),
        recommended_control_plane_provider,
        startup_fallback_valid,
        degraded_control_plane_mode: !default_healthy && startup_fallback_valid,
        failover_recommendations,
        operator_warnings,
        providers: entries,
        artifacts,
    };

    save_runtime_health(workspace_root, &report)?;
    Ok(report)
}

pub fn runtime_reload_plan(config_path: &str, workspace_root: &Path) -> Result<RuntimeReloadPlan> {
    let current = capture_runtime_snapshot(config_path, workspace_root)?;
    let applied = load_applied_runtime_snapshot(workspace_root)?;
    Ok(RuntimeReloadPlanningService::new().build_plan(current, applied, Utc::now().to_rfc3339()))
}

pub fn mark_runtime_applied(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeAppliedSnapshot> {
    let snapshot = capture_runtime_snapshot(config_path, workspace_root)?;
    save_runtime_snapshot(workspace_root, &snapshot)?;
    Ok(snapshot)
}

pub async fn runtime_beacon_status(
    config_path: &str,
    workspace_root: &Path,
    refresh: bool,
    gateway_addr: &str,
    started_at: Option<chrono::DateTime<Utc>>,
    sidecar_running: bool,
) -> Result<RuntimeBeacon> {
    if !refresh && let Some(beacon) = load_cached_runtime_beacon(workspace_root)? {
        return Ok(beacon);
    }
    refresh_runtime_beacon(
        config_path,
        workspace_root,
        gateway_addr,
        started_at,
        sidecar_running,
    )
    .await
}

pub async fn runtime_operator_ops_summary(
    config_path: &str,
    workspace_root: &Path,
    gateway_addr: &str,
    started_at: Option<chrono::DateTime<Utc>>,
    sidecar_running: bool,
) -> Result<RuntimeOperatorOpsSummary> {
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    let runtime_health = runtime_health_status(config_path, workspace_root, false).await?;
    let beacon = runtime_beacon_status(
        config_path,
        workspace_root,
        false,
        gateway_addr,
        started_at,
        sidecar_running,
    )
    .await?;
    let reload_plan = runtime_reload_plan(config_path, workspace_root)?;
    let service_install_status = runtime_service_install_status(config_path, workspace_root)?;
    let lock_status = runtime_lock_status(workspace_root)?;
    let backup_root = runtime_backup_root_for(workspace_root);
    let release_root = runtime_release_root_for(workspace_root);

    let mut recommended_actions = Vec::new();
    if service_install_status.installed {
        if let Some(restart_command) = service_install_status.restart_command.as_ref() {
            recommended_actions.push(format!(
                "Managed restart is available with `{}`.",
                restart_command
            ));
        }
    } else if service_install_status.supported {
        recommended_actions.push(format!(
            "Install the workspace runtime service with `openrustclaw runtime services install --config {}` before relying on unattended restarts.",
            config_path
        ));
    } else {
        recommended_actions.push(
            "No supported host user service manager was detected; document a manual restart path for this runtime target.".to_string(),
        );
    }

    if lock_status.active {
        recommended_actions.push(format!(
            "Runtime lock is active{}; stop or drain the running process before upgrade or rollback.",
            lock_status
                .process_id
                .map(|pid| format!(" for PID {}", pid))
                .unwrap_or_default()
        ));
    } else if lock_status.stale {
        recommended_actions.push(
            "Runtime lock is stale; confirm the old process is gone and refresh the runtime through the managed restart path.".to_string(),
        );
    } else {
        recommended_actions.push(
            "Runtime lock is clear; maintenance operations can proceed without an active owner record."
                .to_string(),
        );
    }

    if reload_plan.restart_required {
        recommended_actions.push(format!(
            "Current config delta requires restart: {}.",
            reload_plan.restart_required_reasons.join(" | ")
        ));
    } else if reload_plan.live_reload_ready {
        recommended_actions.push(format!(
            "Current config delta is live-reload safe via `openrustclaw runtime reload --config {}` or the Control UI reload action.",
            config_path
        ));
    } else {
        recommended_actions.push(
            "Runtime config matches the last applied snapshot; no reload work is pending."
                .to_string(),
        );
    }

    if runtime_health.degraded_control_plane_mode {
        recommended_actions.push(format!(
            "Control plane is degraded; promote healthy provider `{}` before risky runtime changes.",
            runtime_health
                .recommended_control_plane_provider
                .as_deref()
                .unwrap_or(runtime_health.default_provider.as_str())
        ));
    } else if !runtime_health.startup_fallback_valid {
        recommended_actions.push(
            "No healthy fallback control-plane provider is currently available; avoid risky restarts until provider health is restored."
                .to_string(),
        );
    }

    if !runtime_health.operator_warnings.is_empty() {
        recommended_actions.push(format!(
            "Review runtime warnings: {}",
            runtime_health.operator_warnings.join(" | ")
        ));
    }

    recommended_actions.push(format!(
        "Take a workspace snapshot with `openrustclaw runtime backup` before config or binary changes. Backups are stored under `{}`.",
        backup_root.display()
    ));
    recommended_actions.push(format!(
        "Use `openrustclaw runtime upgrade-plan --config {}` for planned maintenance and `openrustclaw runtime rollback-plan --config {} --artifact <path>` for recovery.",
        config_path, config_path
    ));

    Ok(RuntimeOperatorOpsSummary {
        generated_at: Utc::now().to_rfc3339(),
        config_path: resolved_config_path.display().to_string(),
        ready_for_managed_restart: service_install_status.installed && !lock_status.active,
        backup_root: backup_root.display().to_string(),
        release_root: release_root.display().to_string(),
        service_install_status,
        lock_status,
        runtime_health,
        beacon,
        reload_plan,
        recommended_actions,
    })
}

pub async fn refresh_runtime_beacon(
    config_path: &str,
    workspace_root: &Path,
    gateway_addr: &str,
    started_at: Option<chrono::DateTime<Utc>>,
    sidecar_running: bool,
) -> Result<RuntimeBeacon> {
    let health = runtime_health_status(config_path, workspace_root, false).await?;
    let beacon = RuntimeBeacon {
        generated_at: Utc::now().to_rfc3339(),
        process_id: std::process::id(),
        gateway_addr: gateway_addr.to_string(),
        started_at: started_at.map(|value| value.to_rfc3339()),
        uptime_seconds: started_at.map(|value| (Utc::now() - value).num_seconds().max(0)),
        sidecar_running,
        default_provider: health.default_provider,
        degraded_control_plane_mode: health.degraded_control_plane_mode,
        recommended_control_plane_provider: health.recommended_control_plane_provider,
    };
    save_runtime_beacon(workspace_root, &beacon)?;
    Ok(beacon)
}

pub fn validate_runtime_reload(config_path: &str, workspace_root: &Path) -> Result<RuntimeStatus> {
    let config = load_effective_config(config_path, workspace_root)?;
    validate_runtime_provider(&config, &config.providers.default_provider)?;
    for provider in &config.providers.fallback_chain {
        validate_runtime_provider(&config, provider)?;
    }
    runtime_status(config_path, workspace_root)
}

fn save_runtime_health(workspace_root: &Path, report: &RuntimeHealthReport) -> Result<()> {
    let path = runtime_health_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(report)
        .context("Failed to serialize runtime health report")?;
    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn save_runtime_snapshot(workspace_root: &Path, snapshot: &RuntimeAppliedSnapshot) -> Result<()> {
    let path = runtime_reload_state_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(snapshot)
        .context("Failed to serialize runtime reload state")?;
    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn save_runtime_beacon(workspace_root: &Path, beacon: &RuntimeBeacon) -> Result<()> {
    let path = runtime_beacon_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let rendered =
        serde_json::to_string_pretty(beacon).context("Failed to serialize runtime beacon")?;
    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn save_runtime_lock(workspace_root: &Path, record: &RuntimeLockRecord) -> Result<()> {
    let path = runtime_lock_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let rendered =
        serde_json::to_string_pretty(record).context("Failed to serialize runtime lock")?;
    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }

    #[cfg(not(target_os = "linux"))]
    {
        pid == std::process::id()
    }
}

fn capture_runtime_snapshot(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeAppliedSnapshot> {
    let config = load_effective_config(config_path, workspace_root)?;
    let artifacts = WorkspaceArtifactRegistry::scan(workspace_root)?;
    let artifact_paths = artifacts
        .iter()
        .map(|artifact| artifact.path.display().to_string())
        .collect::<Vec<_>>();
    let persona_paths = artifacts
        .iter()
        .filter(|artifact| artifact.class == ArtifactClass::Persona)
        .map(|artifact| artifact.path.display().to_string())
        .collect::<Vec<_>>();
    let artifact_fingerprint = artifact_bundle_fingerprint(workspace_root, &artifact_paths)?;

    Ok(RuntimeAppliedSnapshot {
        captured_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        default_provider: config.providers.default_provider.clone(),
        enabled_channels: enabled_channels(&config),
        provider_fingerprint: fingerprint_json(&serde_json::json!({
            "default_provider": config.providers.default_provider,
            "fallback_chain": config.providers.fallback_chain,
            "anthropic": config.providers.anthropic,
            "openai": config.providers.openai,
            "openrouter": config.providers.openrouter,
            "gemini": config.providers.gemini,
            "ollama": config.providers.ollama,
        }))?,
        gateway_fingerprint: fingerprint_json(&config.gateway)?,
        security_fingerprint: fingerprint_json(&config.security)?,
        scheduler_fingerprint: fingerprint_json(&config.scheduler)?,
        sidecar_fingerprint: fingerprint_json(&config.sidecar)?,
        channel_fingerprint: fingerprint_json(&config.channels)?,
        channel_route_fingerprint: fingerprint_json(&serde_json::json!({
            "telegram": {
                "enabled": config.channels.telegram.enabled,
                "mode": config.channels.telegram.mode,
                "webhook_url": config.channels.telegram.webhook_url,
                "webhook_port": config.channels.telegram.webhook_port,
            },
            "slack": {
                "enabled": config.channels.slack.enabled,
                "mode": config.channels.slack.mode,
                "socket_mode": config.channels.slack.socket_mode,
            },
            "mattermost": {
                "enabled": config.channels.mattermost.enabled,
                "webhook_path": config.channels.mattermost.webhook_path,
            },
            "teams": {
                "enabled": config.channels.teams.enabled,
                "webhook_path": config.channels.teams.webhook_path,
            },
            "google_meet": {
                "enabled": config.channels.google_meet.enabled,
                "webhook_path": config.channels.google_meet.webhook_path,
            },
            "gmail_pubsub": {
                "enabled": config.channels.gmail_pubsub.enabled,
            },
            "google_chat": {
                "enabled": config.channels.google_chat.enabled,
            }
        }))?,
        artifact_fingerprint,
        artifact_paths,
        persona_paths,
    })
}

fn artifact_bundle_fingerprint(workspace_root: &Path, artifact_paths: &[String]) -> Result<String> {
    let mut hasher = Sha256::new();
    for path in artifact_paths {
        hasher.update(path.as_bytes());
        let bytes = fs::read(workspace_root.join(path))
            .with_context(|| format!("Failed to read artifact '{}'", path))?;
        hasher.update(&bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn fingerprint_json<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value).context("Failed to serialize runtime fingerprint")?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

async fn scan_provider_health(provider: &str, config: &AppConfig) -> RuntimeHealthProviderEntry {
    let role = provider_runtime_role(provider, config);

    let model = match provider {
        "anthropic" => config.providers.anthropic.model.clone(),
        "openai" => config.providers.openai.model.clone(),
        "openrouter" => config.providers.openrouter.model.clone(),
        "gemini" => config.providers.gemini.model.clone(),
        "ollama" => config.providers.ollama.model.clone(),
        _ => String::new(),
    };

    let result = match provider {
        "ollama" => probe_ollama_provider(config).await,
        _ => probe_remote_provider_health(provider, config, &model).await,
    };

    match result {
        Ok(probe) => RuntimeHealthProviderEntry {
            provider: provider.to_string(),
            role,
            model,
            configured: true,
            healthy: probe.healthy,
            issue_kind: probe.issue_kind,
            recommendation: provider_recommendation(provider, config),
            issue: probe.issue,
            model_available: probe.model_available,
            limit_snapshot: probe.limit_snapshot,
        },
        Err(error) => RuntimeHealthProviderEntry {
            provider: provider.to_string(),
            role,
            model,
            configured: !error.to_string().contains("environment variable not set"),
            healthy: false,
            issue_kind: Some(classify_provider_error(&error.to_string()).to_string()),
            recommendation: provider_recommendation(provider, config),
            issue: Some(error.to_string()),
            model_available: None,
            limit_snapshot: None,
        },
    }
}

fn provider_runtime_role(provider: &str, config: &AppConfig) -> String {
    if provider == config.providers.default_provider {
        return "primary".to_string();
    }
    if config.providers.control_plane_provider.as_deref() == Some(provider) {
        return "control_plane_primary".to_string();
    }
    if let Some(index) = config
        .providers
        .control_plane_fallback_chain
        .iter()
        .position(|value| value == provider)
    {
        return format!("control_plane_fallback_{}", index + 1);
    }
    let index = config
        .providers
        .fallback_chain
        .iter()
        .position(|value| value == provider)
        .map(|value| value + 1)
        .unwrap_or(0);
    format!("fallback_{index}")
}

fn provider_recommendation(provider: &str, config: &AppConfig) -> Option<String> {
    if config.providers.control_plane_provider.as_deref() == Some(provider) {
        return Some("configured control-plane primary".to_string());
    }
    if config
        .providers
        .control_plane_fallback_chain
        .iter()
        .any(|value| value == provider)
    {
        return Some("configured control-plane fallback".to_string());
    }
    if provider == config.providers.default_provider {
        return Some("configured task/runtime primary".to_string());
    }
    None
}

enum RemoteProviderAuth {
    Bearer,
    Header(&'static str),
}

fn gemini_api_key(config: &AppConfig) -> Result<String> {
    let primary_env = config
        .providers
        .gemini
        .api_key_env
        .as_deref()
        .unwrap_or("GEMINI_API_KEY");
    std::env::var(primary_env)
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .with_context(|| {
            format!(
                "{} or GOOGLE_API_KEY environment variable not set",
                primary_env
            )
        })
}

fn health_scan_provider_order(config: &AppConfig) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();
    for provider in std::iter::once(config.providers.default_provider.clone())
        .chain(config.providers.fallback_chain.clone().into_iter())
        .chain(control_plane_provider_order(config).into_iter())
    {
        if seen.insert(provider.clone()) {
            ordered.push(provider);
        }
    }
    ordered
}

fn control_plane_provider_order(config: &AppConfig) -> Vec<String> {
    let primary = config
        .providers
        .control_plane_provider
        .clone()
        .unwrap_or_else(|| config.providers.default_provider.clone());
    std::iter::once(primary)
        .chain(config.providers.control_plane_fallback_chain.clone())
        .collect()
}

fn build_failover_recommendations(
    config: &AppConfig,
    control_plane_chain: &[String],
    entries: &[RuntimeHealthProviderEntry],
    recommended_control_plane_provider: Option<&str>,
) -> Vec<String> {
    let mut recommendations = Vec::new();
    if let Some(provider) = recommended_control_plane_provider {
        recommendations.push(format!(
            "Use `{}` as the current control-plane provider for onboarding, config edits, and model-switch operations.",
            provider
        ));
    }
    if control_plane_chain.len() < 2 {
        recommendations.push(
            "Configure `providers.control_plane_fallback_chain` so control-plane actions still work when the control-plane primary is unavailable."
                .to_string(),
        );
    }
    if !config
        .providers
        .control_plane_fallback_chain
        .iter()
        .any(|provider| provider == "ollama")
    {
        recommendations.push(
            "Consider adding `ollama` to `providers.control_plane_fallback_chain` for a local/offline control-plane fallback."
                .to_string(),
        );
    }
    if !entries.iter().any(|entry| {
        config
            .providers
            .control_plane_fallback_chain
            .iter()
            .any(|provider| provider == &entry.provider)
            && entry.healthy
    }) {
        recommendations.push(
            "No healthy control-plane fallback is currently available; configure or repair at least one fallback provider before the next model swap."
                .to_string(),
        );
    }
    recommendations
}

fn build_runtime_operator_warnings(
    config: &AppConfig,
    control_plane_chain: &[String],
    entries: &[RuntimeHealthProviderEntry],
    recommended_control_plane_provider: Option<&str>,
    previous_report: Option<&RuntimeHealthReport>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let control_plane_primary = config
        .providers
        .control_plane_provider
        .clone()
        .unwrap_or_else(|| config.providers.default_provider.clone());
    let primary_health = entries
        .iter()
        .find(|entry| entry.provider == control_plane_primary)
        .map(|entry| entry.healthy)
        .unwrap_or(false);

    if !primary_health {
        warnings.push(format!(
            "Configured control-plane provider `{}` is not currently healthy.",
            control_plane_primary
        ));
    }
    if recommended_control_plane_provider.is_none() {
        warnings.push(
            "No healthy provider is currently available for the control-plane lane; onboarding and model-switch operations may fail until one recovers."
                .to_string(),
        );
    }
    if control_plane_chain.iter().collect::<BTreeSet<_>>().len() != control_plane_chain.len() {
        warnings.push(
            "Control-plane provider order contains duplicates; simplify `providers.control_plane_fallback_chain` to keep failover intent clear."
                .to_string(),
        );
    }
    for entry in entries {
        match entry.issue_kind.as_deref() {
            Some("model_unavailable") => warnings.push(format!(
                "Configured model `{}` is no longer listed by provider `{}`; update that lane before the next runtime cutover.",
                entry.model, entry.provider
            )),
            Some("auth") => warnings.push(format!(
                "Provider `{}` reported an authentication or access error during the latest health scan.",
                entry.provider
            )),
            Some("billing") => warnings.push(format!(
                "Provider `{}` reported a billing or quota error during the latest health scan.",
                entry.provider
            )),
            Some("rate_limited") => warnings.push(format!(
                "Provider `{}` is currently rate-limited; expect degraded failover behavior until the limit window resets.",
                entry.provider
            )),
            _ => {}
        }
    }
    if let Some(previous_report) = previous_report {
        for entry in entries {
            let previous = previous_report
                .providers
                .iter()
                .find(|candidate| candidate.provider == entry.provider);
            if let Some(previous) = previous {
                if previous.model_available == Some(true) && entry.model_available == Some(false) {
                    warnings.push(format!(
                        "Provider `{}` previously listed configured model `{}` but the latest scan does not; treat this as a removed or disabled model regression.",
                        entry.provider, entry.model
                    ));
                }
                if previous.issue_kind.is_none()
                    && matches!(entry.issue_kind.as_deref(), Some("auth") | Some("billing"))
                {
                    warnings.push(format!(
                        "Provider `{}` regressed from healthy to `{}` since the last persisted runtime-health scan.",
                        entry.provider,
                        entry.issue_kind.clone().unwrap_or_default()
                    ));
                }
                if let (Some(old_limits), Some(new_limits)) = (
                    previous.limit_snapshot.as_deref(),
                    entry.limit_snapshot.as_deref(),
                ) && old_limits != new_limits
                {
                    warnings.push(format!(
                        "Provider `{}` exposed different rate-limit headers since the last scan: `{}` -> `{}`.",
                        entry.provider, old_limits, new_limits
                    ));
                }
            }
        }
    }
    warnings
}

async fn probe_ollama_provider(config: &AppConfig) -> Result<ProviderHealthProbe> {
    let base_url = config.providers.ollama.base_url.trim_end_matches('/');
    let response = reqwest::Client::new()
        .get(format!("{base_url}/api/tags"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .context("Failed to reach Ollama")?;
    let limit_snapshot = summarize_limit_headers(response.headers());
    let response = response
        .error_for_status()
        .context("Ollama health probe returned an error status")?;
    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse Ollama model catalog")?;
    let available = body["models"]
        .as_array()
        .map(|models| {
            models.iter().any(|model| {
                model["name"]
                    .as_str()
                    .map(|name| name == config.providers.ollama.model)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    Ok(ProviderHealthProbe {
        healthy: available,
        issue_kind: (!available).then(|| "model_unavailable".to_string()),
        issue: (!available).then(|| {
            format!(
                "Configured model '{}' is not listed by the local Ollama catalog",
                config.providers.ollama.model
            )
        }),
        model_available: Some(available),
        limit_snapshot,
    })
}

async fn probe_remote_provider_health(
    provider: &str,
    config: &AppConfig,
    model: &str,
) -> Result<ProviderHealthProbe> {
    let (url, api_key, auth, extra_headers) = match provider {
        "anthropic" => (
            "https://api.anthropic.com/v1/models".to_string(),
            std::env::var(
                config
                    .providers
                    .anthropic
                    .api_key_env
                    .as_deref()
                    .unwrap_or("ANTHROPIC_API_KEY"),
            )
            .with_context(|| {
                format!(
                    "{} environment variable not set",
                    config
                        .providers
                        .anthropic
                        .api_key_env
                        .as_deref()
                        .unwrap_or("ANTHROPIC_API_KEY")
                )
            })?,
            RemoteProviderAuth::Bearer,
            vec![(
                "anthropic-version".to_string(),
                config.providers.anthropic.api_version.clone(),
            )],
        ),
        "openai" => (
            "https://api.openai.com/v1/models".to_string(),
            std::env::var(
                config
                    .providers
                    .openai
                    .api_key_env
                    .as_deref()
                    .unwrap_or("OPENAI_API_KEY"),
            )
            .with_context(|| {
                format!(
                    "{} environment variable not set",
                    config
                        .providers
                        .openai
                        .api_key_env
                        .as_deref()
                        .unwrap_or("OPENAI_API_KEY")
                )
            })?,
            RemoteProviderAuth::Bearer,
            Vec::new(),
        ),
        "openrouter" => (
            "https://openrouter.ai/api/v1/models".to_string(),
            std::env::var(
                config
                    .providers
                    .openrouter
                    .api_key_env
                    .as_deref()
                    .unwrap_or("OPENROUTER_API_KEY"),
            )
            .with_context(|| {
                format!(
                    "{} environment variable not set",
                    config
                        .providers
                        .openrouter
                        .api_key_env
                        .as_deref()
                        .unwrap_or("OPENROUTER_API_KEY")
                )
            })?,
            RemoteProviderAuth::Bearer,
            Vec::new(),
        ),
        "gemini" => (
            format!(
                "{}/models",
                config
                    .providers
                    .gemini
                    .base_url
                    .as_deref()
                    .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
                    .trim_end_matches('/')
            ),
            gemini_api_key(config)?,
            RemoteProviderAuth::Header("x-goog-api-key"),
            Vec::new(),
        ),
        _ => {
            validate_runtime_provider(config, provider)?;
            return Ok(ProviderHealthProbe {
                healthy: true,
                issue_kind: None,
                issue: None,
                model_available: None,
                limit_snapshot: None,
            });
        }
    };

    let client = reqwest::Client::new();
    let mut request = client.get(url).timeout(Duration::from_secs(5));
    request = match auth {
        RemoteProviderAuth::Bearer => request.bearer_auth(api_key),
        RemoteProviderAuth::Header(name) => request.header(name, api_key),
    };
    for (name, value) in extra_headers {
        request = request.header(name, value);
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to reach provider `{provider}`"))?;
    let status = response.status();
    let limit_snapshot = summarize_limit_headers(response.headers());
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let issue_kind = classify_provider_status(status);
        return Ok(ProviderHealthProbe {
            healthy: false,
            issue_kind: Some(issue_kind.to_string()),
            issue: Some(format!(
                "Provider `{provider}` health probe returned {}: {}",
                status,
                body.trim()
            )),
            model_available: None,
            limit_snapshot,
        });
    }

    let body: serde_json::Value = response
        .json()
        .await
        .with_context(|| format!("Failed to parse provider `{provider}` model catalog"))?;
    let available_models = extract_provider_model_ids(provider, &body);
    let model_available = available_models.iter().any(|candidate| candidate == model);
    Ok(ProviderHealthProbe {
        healthy: model_available,
        issue_kind: (!model_available).then(|| "model_unavailable".to_string()),
        issue: (!model_available).then(|| {
            format!(
                "Configured model '{}' is not listed by provider `{}`",
                model, provider
            )
        }),
        model_available: Some(model_available),
        limit_snapshot,
    })
}

fn extract_provider_model_ids(provider: &str, body: &serde_json::Value) -> Vec<String> {
    body["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .chain(body["models"].as_array().cloned().unwrap_or_default())
        .filter_map(|entry| match provider {
            "anthropic" | "openai" | "openrouter" => entry
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            "gemini" => entry
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.trim_start_matches("models/").to_string()),
            _ => None,
        })
        .collect()
}

fn summarize_limit_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let mut values = headers
        .iter()
        .filter_map(|(name, value)| {
            let key = name.as_str().to_ascii_lowercase();
            if !(key.contains("ratelimit") || key.contains("rate-limit") || key.contains("quota")) {
                return None;
            }
            value
                .to_str()
                .ok()
                .map(|parsed| format!("{}={}", key, parsed))
        })
        .collect::<Vec<_>>();
    values.sort();
    (!values.is_empty()).then(|| values.join(", "))
}

fn classify_provider_status(status: reqwest::StatusCode) -> &'static str {
    match status.as_u16() {
        401 | 403 => "auth",
        402 => "billing",
        404 => "model_unavailable",
        429 => "rate_limited",
        _ if status.is_server_error() => "provider_unavailable",
        _ => "probe_failed",
    }
}

fn classify_provider_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("environment variable not set") {
        "not_configured"
    } else if lower.contains("auth")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
    {
        "auth"
    } else if lower.contains("billing") || lower.contains("quota") || lower.contains("credit") {
        "billing"
    } else if lower.contains("rate") && lower.contains("limit") {
        "rate_limited"
    } else if lower.contains("not listed") || lower.contains("unknown model") {
        "model_unavailable"
    } else {
        "probe_failed"
    }
}

fn summarize_runtime_artifacts(
    workspace_root: &Path,
    config: &AppConfig,
) -> Result<RuntimeArtifactHealth> {
    let artifacts = WorkspaceArtifactRegistry::scan(workspace_root)?;
    let resolved =
        WorkspaceArtifactRegistry::resolve(workspace_root, default_model_for_provider(config))?;
    let registry_path = workspace_root.join(".claw/artifacts/registry.json");
    Ok(RuntimeArtifactHealth {
        artifact_count: artifacts.len(),
        persona_artifact_count: artifacts
            .iter()
            .filter(|artifact| artifact.class == ArtifactClass::Persona)
            .count(),
        registry_path: registry_path.display().to_string(),
        model_family: resolved.model_family,
        included_for_default_model: resolved.included.len(),
    })
}

fn default_model_for_provider(config: &AppConfig) -> &str {
    match config.providers.default_provider.as_str() {
        "anthropic" => &config.providers.anthropic.model,
        "openai" => &config.providers.openai.model,
        "openrouter" => &config.providers.openrouter.model,
        "gemini" => &config.providers.gemini.model,
        "ollama" => &config.providers.ollama.model,
        _ => &config.providers.anthropic.model,
    }
}

fn enabled_channels(config: &AppConfig) -> Vec<String> {
    let mut channels = Vec::new();
    if config.channels.telegram.enabled {
        channels.push("telegram".to_string());
    }
    if config.channels.discord.enabled {
        channels.push("discord".to_string());
    }
    if config.channels.slack.enabled {
        channels.push("slack".to_string());
    }
    if config.channels.whatsapp.enabled {
        channels.push("whatsapp".to_string());
    }
    if config.channels.teams.enabled {
        channels.push("teams".to_string());
    }
    if config.channels.mattermost.enabled {
        channels.push("mattermost".to_string());
    }
    if config.channels.google_chat.enabled {
        channels.push("google_chat".to_string());
    }
    if config.channels.google_meet.enabled {
        channels.push("google_meet".to_string());
    }
    if config.channels.gmail_pubsub.enabled {
        channels.push("gmail_pubsub".to_string());
    }
    if config.channels.signal.enabled {
        channels.push("signal".to_string());
    }
    if config.channels.matrix.enabled {
        channels.push("matrix".to_string());
    }
    if config.channels.imessage.enabled {
        channels.push("imessage".to_string());
    }
    channels
}

pub fn apply_runtime_secret_sources(workspace_root: &Path) -> Result<()> {
    let _guard = env_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock runtime environment"))?;

    for (key, value) in load_dotenv_entries(workspace_root.join(".env"))? {
        // SAFETY: Runtime environment updates are serialized through a process-wide mutex.
        unsafe { std::env::set_var(key, value) };
    }

    if let Ok(vault) = load_vault(workspace_root, None) {
        for (key, value) in vault.entries {
            // SAFETY: Runtime environment updates are serialized through a process-wide mutex.
            unsafe { std::env::set_var(key, value) };
        }
    }

    Ok(())
}

pub fn list_vault_keys(workspace_root: &Path) -> Result<Vec<String>> {
    let vault = load_vault(workspace_root, None)?;
    Ok(vault.entries.keys().cloned().collect())
}

pub fn get_vault_secret(workspace_root: &Path, key: &str) -> Result<String> {
    let vault = load_vault(workspace_root, None)?;
    vault
        .entries
        .get(key)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Vault key '{}' not found", key))
}

pub fn set_vault_secret(workspace_root: &Path, key: &str, value: &str) -> Result<()> {
    let vault = load_vault(workspace_root, None).unwrap_or_default();
    let (vault, _) = RuntimeVaultService::new().set_secret(
        RuntimeVaultState {
            version: vault.version,
            entries: vault.entries,
            updated_at: vault.updated_at,
        },
        key,
        value,
        Utc::now().to_rfc3339(),
    );
    let vault = RuntimeVault {
        version: vault.version,
        entries: vault.entries,
        updated_at: vault.updated_at,
    };
    save_vault(workspace_root, &vault)
}

pub fn delete_vault_secret(workspace_root: &Path, key: &str) -> Result<()> {
    let vault = load_vault(workspace_root, None)?;
    let (vault, _) = RuntimeVaultService::new().delete_secret(
        RuntimeVaultState {
            version: vault.version,
            entries: vault.entries,
            updated_at: vault.updated_at,
        },
        key,
        Utc::now().to_rfc3339(),
    );
    let vault = RuntimeVault {
        version: vault.version,
        entries: vault.entries,
        updated_at: vault.updated_at,
    };
    save_vault(workspace_root, &vault)
}

pub fn switch_provider(
    config_path: &str,
    workspace_root: &Path,
    provider: &str,
    model: Option<&str>,
    api_key_env: Option<&str>,
    fallback_chain: Option<Vec<String>>,
) -> Result<AppConfig> {
    RuntimeProviderSwitchService::new(WorkspaceRuntimeConfigMutationSource::new(
        config_path,
        workspace_root,
    ))
    .switch_provider(RuntimeProviderSwitchRequest {
        provider: provider.to_string(),
        model: model.map(str::to_string),
        api_key_env: api_key_env.map(str::to_string),
        fallback_chain,
    })
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub fn switch_model(
    config_path: &str,
    workspace_root: &Path,
    provider: &str,
    model: &str,
) -> Result<AppConfig> {
    RuntimeProviderSwitchService::new(WorkspaceRuntimeConfigMutationSource::new(
        config_path,
        workspace_root,
    ))
    .switch_model(provider, model)
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}

struct WorkspaceRuntimeConfigMutationSource<'a> {
    config_path: &'a str,
    workspace_root: &'a Path,
}

impl<'a> WorkspaceRuntimeConfigMutationSource<'a> {
    fn new(config_path: &'a str, workspace_root: &'a Path) -> Self {
        Self {
            config_path,
            workspace_root,
        }
    }
}

impl RuntimeConfigMutationSource for WorkspaceRuntimeConfigMutationSource<'_> {
    fn load_effective_runtime_config(&self) -> openrustclaw_core::error::Result<AppConfig> {
        load_effective_config(self.config_path, self.workspace_root)
            .map_err(|error| CoreError::Internal(format!("failed to load runtime config: {error}")))
    }

    fn validate_runtime_provider(
        &self,
        config: &AppConfig,
        provider: &str,
    ) -> openrustclaw_core::error::Result<()> {
        validate_runtime_provider(config, provider).map_err(|error| {
            CoreError::Internal(format!("runtime provider validation failed: {error}"))
        })
    }

    fn write_runtime_config_with_backup(
        &self,
        config: &AppConfig,
    ) -> openrustclaw_core::error::Result<()> {
        write_config_with_backup(self.config_path, config).map_err(|error| {
            CoreError::Internal(format!(
                "failed to write runtime config with backup: {error}"
            ))
        })
    }
}

fn effective_control_plane_action_provider(config: &AppConfig) -> String {
    if let Some(provider) = config
        .providers
        .control_plane_provider
        .as_deref()
        .filter(|provider| !provider.trim().is_empty())
    {
        return provider.to_string();
    }
    config.providers.default_provider.clone()
}

fn provider_model_for<'a>(config: &'a AppConfig, provider: &str) -> &'a str {
    match provider {
        "anthropic" => &config.providers.anthropic.model,
        "openai" => &config.providers.openai.model,
        "openrouter" => &config.providers.openrouter.model,
        "ollama" => &config.providers.ollama.model,
        "gemini" => &config.providers.gemini.model,
        _ => &config.providers.anthropic.model,
    }
}

pub fn write_config_with_backup(config_path: &str, config: &AppConfig) -> Result<()> {
    let rendered = toml::to_string_pretty(config).context("Failed to render config TOML")?;
    let path = PathBuf::from(config_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    if path.exists() {
        let backup_path = path.with_extension(format!("{}.bak", Utc::now().format("%Y%m%d%H%M%S")));
        fs::copy(&path, &backup_path).with_context(|| {
            format!(
                "Failed to create runtime config backup '{}' -> '{}'",
                path.display(),
                backup_path.display()
            )
        })?;
    }

    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

pub fn backup_runtime_state(
    workspace_root: &Path,
    output_path: Option<&str>,
) -> Result<RuntimeBackupSummary> {
    backup_runtime_state_inner(workspace_root, output_path.map(PathBuf::from), None)
}

pub fn restore_runtime_state(
    workspace_root: &Path,
    backup_path: &str,
) -> Result<RuntimeRestoreSummary> {
    let backup_root = PathBuf::from(backup_path);
    let manifest = load_runtime_backup_manifest(&backup_root)?;
    let pre_restore_backup = backup_runtime_state_inner(workspace_root, None, Some("pre-restore"))?;
    let workspace_backup_root = backup_root.join("workspace");
    let mut restored_entries = Vec::new();

    for entry in &manifest.entries {
        let source = workspace_backup_root.join(entry);
        if !source.exists() {
            anyhow::bail!(
                "Backup '{}' is missing expected entry '{}'",
                backup_root.display(),
                source.display()
            );
        }
        let target = workspace_root.join(entry);
        remove_path_if_exists(&target)?;
        copy_path_recursive(&source, &target)?;
        restored_entries.push(entry.clone());
    }

    Ok(RuntimeRestoreSummary {
        restored_at: Utc::now().to_rfc3339(),
        backup_path: backup_root.display().to_string(),
        pre_restore_backup_path: pre_restore_backup.backup_path,
        entries: restored_entries,
    })
}

pub fn validate_runtime_provider(config: &AppConfig, provider: &str) -> Result<()> {
    let _provider = create_provider_from_config(provider, config)?;
    Ok(())
}

pub fn create_provider_from_config(
    provider_name: &str,
    config: &AppConfig,
) -> Result<Arc<dyn LlmProvider>> {
    match provider_name.to_lowercase().as_str() {
        "anthropic" => {
            let api_key_env = config
                .providers
                .anthropic
                .api_key_env
                .as_deref()
                .unwrap_or("ANTHROPIC_API_KEY");
            let api_key = std::env::var(api_key_env)
                .with_context(|| format!("{} environment variable not set", api_key_env))?;
            Ok(Arc::new(AnthropicProvider::new(
                api_key,
                config.providers.anthropic.model.clone(),
            )))
        }
        "openai" => {
            let api_key_env = config
                .providers
                .openai
                .api_key_env
                .as_deref()
                .unwrap_or("OPENAI_API_KEY");
            let api_key = std::env::var(api_key_env)
                .with_context(|| format!("{} environment variable not set", api_key_env))?;
            Ok(Arc::new(OpenAiProvider::new(
                api_key,
                config.providers.openai.model.clone(),
            )))
        }
        "openrouter" => {
            let api_key_env = config
                .providers
                .openrouter
                .api_key_env
                .as_deref()
                .unwrap_or("OPENROUTER_API_KEY");
            let api_key = std::env::var(api_key_env)
                .with_context(|| format!("{} environment variable not set", api_key_env))?;
            let strategy = match config.providers.openrouter.route_strategy.as_str() {
                "price" => RouteStrategy::Price,
                "throughput" => RouteStrategy::Throughput,
                "web_search" | "online" => RouteStrategy::WebSearch,
                _ => RouteStrategy::Quality,
            };
            Ok(Arc::new(OpenRouterProvider::with_strategy(
                api_key,
                config.providers.openrouter.model.clone(),
                strategy,
            )))
        }
        "gemini" => {
            let api_key = gemini_api_key(config)?;
            if let Some(base_url) = config.providers.gemini.base_url.clone() {
                Ok(Arc::new(GeminiProvider::with_base_url(
                    api_key,
                    config.providers.gemini.model.clone(),
                    base_url,
                )))
            } else {
                Ok(Arc::new(GeminiProvider::new(
                    api_key,
                    config.providers.gemini.model.clone(),
                )))
            }
        }
        "ollama" => Ok(Arc::new(OllamaProvider::with_base_url(
            config.providers.ollama.model.clone(),
            config.providers.ollama.base_url.clone(),
        ))),
        _ => anyhow::bail!(
            "Unknown provider '{}'. Available: anthropic, openai, openrouter, gemini, ollama",
            provider_name
        ),
    }
}

fn load_vault(workspace_root: &Path, passphrase: Option<&str>) -> Result<RuntimeVault> {
    let path = vault_path_for(workspace_root);
    if !path.exists() {
        return Ok(RuntimeVault::default());
    }

    let passphrase = resolve_vault_passphrase(passphrase)?;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let encrypted: EncryptedVaultFile =
        serde_json::from_str(&raw).context("Failed to parse encrypted vault file")?;
    decrypt_vault(&encrypted, &passphrase)
}

fn save_vault(workspace_root: &Path, vault: &RuntimeVault) -> Result<()> {
    let path = vault_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let passphrase = resolve_vault_passphrase(None)?;
    let encrypted = encrypt_vault(vault, &passphrase)?;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&encrypted).context("Failed to render encrypted vault")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn resolve_vault_passphrase(passphrase: Option<&str>) -> Result<String> {
    if let Some(passphrase) = passphrase {
        return Ok(passphrase.to_string());
    }
    std::env::var(DEFAULT_PASSPHRASE_ENV).with_context(|| {
        format!(
            "{} environment variable not set for runtime vault access",
            DEFAULT_PASSPHRASE_ENV
        )
    })
}

fn encrypt_vault(vault: &RuntimeVault, passphrase: &str) -> Result<EncryptedVaultFile> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let key = derive_key(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = serde_json::to_vec(vault).context("Failed to serialize vault")?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to encrypt runtime vault"))?;

    Ok(EncryptedVaultFile {
        version: 1,
        salt_b64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt),
        nonce_b64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, nonce),
        ciphertext_b64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            ciphertext,
        ),
        updated_at: Utc::now().to_rfc3339(),
    })
}

fn decrypt_vault(encrypted: &EncryptedVaultFile, passphrase: &str) -> Result<RuntimeVault> {
    let salt = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted.salt_b64.as_bytes(),
    )
    .context("Failed to decode vault salt")?;
    let nonce = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted.nonce_b64.as_bytes(),
    )
    .context("Failed to decode vault nonce")?;
    let ciphertext = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted.ciphertext_b64.as_bytes(),
    )
    .context("Failed to decode vault ciphertext")?;

    let key = derive_key(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to decrypt runtime vault"))?;
    let mut vault: RuntimeVault =
        serde_json::from_slice(&plaintext).context("Failed to parse decrypted vault")?;
    if vault.version == 0 {
        vault.version = 1;
    }
    Ok(vault)
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|err| anyhow::anyhow!("Failed to derive runtime vault key: {err}"))?;
    Ok(key)
}

fn load_dotenv_entries(path: PathBuf) -> Result<Vec<(String, String)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let mut entries = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let mut value = value.trim().to_string();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = value[1..value.len() - 1].to_string();
        }
        entries.push((key.trim().to_string(), value));
    }
    Ok(entries)
}

fn systemd_user_service_supported() -> bool {
    Path::new("/run/systemd/system").exists() || Path::new("/sbin/systemctl").exists()
}

fn launchd_user_service_supported() -> bool {
    cfg!(target_os = "macos") && dirs::home_dir().is_some()
}

fn user_systemd_service_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("systemd/user").join(DEFAULT_SYSTEMD_SERVICE_NAME))
}

fn user_launchd_service_path() -> Option<PathBuf> {
    dirs::home_dir().map(|dir| {
        dir.join("Library")
            .join("LaunchAgents")
            .join(format!("{DEFAULT_LAUNCHD_LABEL}.plist"))
    })
}

fn resolve_runtime_config_path(workspace_root: &Path, config_path: &str) -> PathBuf {
    let config_path = PathBuf::from(config_path);
    if config_path.is_absolute() {
        config_path
    } else {
        workspace_root.join(config_path)
    }
}

fn resolve_runtime_artifact_path(workspace_root: &Path, artifact_path: &str) -> PathBuf {
    let artifact = PathBuf::from(artifact_path);
    if artifact.is_absolute() {
        artifact
    } else {
        workspace_root.join(artifact)
    }
}

fn install_runtime_systemd_service(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeServiceInstallStatus> {
    if !systemd_user_service_supported() {
        anyhow::bail!("User-level systemd services are not available on this host");
    }

    let service_path = user_systemd_service_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine the user systemd service directory"))?;
    if let Some(parent) = service_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let unit = render_user_systemd_service_unit(&current_exe, workspace_root, config_path);
    fs::write(&service_path, unit.as_bytes())
        .with_context(|| format!("Failed to write '{}'", service_path.display()))?;

    runtime_service_install_status(config_path, workspace_root)
}

fn install_runtime_launchd_service(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeServiceInstallStatus> {
    if !launchd_user_service_supported() {
        anyhow::bail!("User-level launchd agents are not available on this host");
    }

    let service_path = user_launchd_service_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine the user launchd agent directory"))?;
    if let Some(parent) = service_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }
    let log_path = super::logs::runtime_log_path_for(workspace_root);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create '{}'", parent.display()))?;
    }

    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let plist = render_user_launchd_service_plist(&current_exe, workspace_root, config_path);
    fs::write(&service_path, plist.as_bytes())
        .with_context(|| format!("Failed to write '{}'", service_path.display()))?;

    runtime_service_install_status(config_path, workspace_root)
}

fn render_user_systemd_service_unit(
    current_exe: &Path,
    workspace_root: &Path,
    config_path: &str,
) -> String {
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    format!(
        r#"[Unit]
Description=OpenRustClaw Gateway
After=network.target

[Service]
Type=simple
ExecStart="{exe_path}" start --config "{config_path}"
Restart=on-failure
RestartSec=5
WorkingDirectory={working_directory}

[Install]
WantedBy=default.target
"#,
        exe_path = current_exe.display(),
        config_path = resolved_config_path.display(),
        working_directory = workspace_root.display(),
    )
}

fn render_user_launchd_service_plist(
    current_exe: &Path,
    workspace_root: &Path,
    config_path: &str,
) -> String {
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);
    let log_path = super::logs::runtime_log_path_for(workspace_root);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe_path}</string>
    <string>start</string>
    <string>--config</string>
    <string>{config_path}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>{working_directory}</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>StandardOutPath</key>
  <string>{log_path}</string>
  <key>StandardErrorPath</key>
  <string>{log_path}</string>
</dict>
</plist>
"#,
        label = DEFAULT_LAUNCHD_LABEL,
        exe_path = current_exe.display(),
        config_path = resolved_config_path.display(),
        working_directory = workspace_root.display(),
        log_path = log_path.display(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeServiceManager {
    Unsupported,
    SystemdUser,
    LaunchdUser,
}

fn preferred_service_manager() -> RuntimeServiceManager {
    if cfg!(target_os = "macos") {
        if launchd_user_service_supported() {
            RuntimeServiceManager::LaunchdUser
        } else {
            RuntimeServiceManager::Unsupported
        }
    } else if systemd_user_service_supported() {
        RuntimeServiceManager::SystemdUser
    } else {
        RuntimeServiceManager::Unsupported
    }
}

fn launchd_domain_target() -> String {
    "gui/$(id -u)".to_string()
}

fn file_size_bytes(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn path_is_executable(path: &Path) -> Result<bool> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to stat runtime artifact '{}'", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(metadata.is_file())
    }
}

fn recommended_runtime_rollback_path(workspace_root: &Path, current_exe: &Path) -> PathBuf {
    let file_name = current_exe
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openrustclaw"));
    runtime_release_root_for(workspace_root)
        .join(format!("rollback-{}", Utc::now().format("%Y%m%d%H%M%S")))
        .join(file_name)
}

fn backup_runtime_state_inner(
    workspace_root: &Path,
    output_path: Option<PathBuf>,
    reason: Option<&str>,
) -> Result<RuntimeBackupSummary> {
    let created_at = Utc::now();
    let backup_root = match output_path {
        Some(path) => path,
        None => {
            let mut name = created_at.format("%Y%m%d%H%M%S").to_string();
            if let Some(reason) = reason {
                name.push('-');
                name.push_str(reason);
            }
            runtime_backup_root_for(workspace_root).join(name)
        }
    };

    if backup_root.exists() {
        anyhow::bail!(
            "Backup destination '{}' already exists",
            backup_root.display()
        );
    }

    let entries = collect_runtime_backup_entries(workspace_root)?;
    let workspace_backup_root = backup_root.join("workspace");
    fs::create_dir_all(&workspace_backup_root)
        .with_context(|| format!("Failed to create '{}'", workspace_backup_root.display()))?;

    for entry in &entries {
        let source = workspace_root.join(entry);
        let dest = workspace_backup_root.join(entry);
        copy_path_recursive(&source, &dest)?;
    }

    let manifest = RuntimeBackupManifest {
        version: 1,
        created_at: created_at.to_rfc3339(),
        entries: entries.clone(),
    };
    fs::write(
        backup_root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).context("Failed to render runtime backup manifest")?,
    )
    .with_context(|| {
        format!(
            "Failed to write '{}'",
            backup_root.join("manifest.json").display()
        )
    })?;

    Ok(RuntimeBackupSummary {
        created_at: manifest.created_at,
        backup_path: backup_root.display().to_string(),
        entries,
    })
}

fn load_runtime_backup_manifest(backup_root: &Path) -> Result<RuntimeBackupManifest> {
    let manifest_path = backup_root.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read '{}'", manifest_path.display()))?;
    serde_json::from_str(&raw).context("Failed to parse runtime backup manifest")
}

fn collect_runtime_backup_entries(workspace_root: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();

    for entry in [".env", "config"] {
        if workspace_root.join(entry).exists() {
            entries.push(entry.to_string());
        }
    }

    let claw_root = workspace_root.join(".claw");
    if claw_root.exists() {
        let mut claw_entries = Vec::new();
        for entry in fs::read_dir(&claw_root)
            .with_context(|| format!("Failed to read '{}'", claw_root.display()))?
        {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), "runtime-backups" | "onboard-backups") {
                continue;
            }
            claw_entries.push(format!(".claw/{name}"));
        }
        claw_entries.sort();
        entries.extend(claw_entries);
    }

    Ok(entries)
}

fn copy_path_recursive(source: &Path, dest: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(dest)
            .with_context(|| format!("Failed to create '{}'", dest.display()))?;
        for entry in fs::read_dir(source)
            .with_context(|| format!("Failed to read '{}'", source.display()))?
        {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_dest = dest.join(entry.file_name());
            copy_path_recursive(&entry_path, &entry_dest)?;
        }
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create '{}'", parent.display()))?;
        }
        fs::copy(source, dest).with_context(|| {
            format!(
                "Failed to copy '{}' -> '{}'",
                source.display(),
                dest.display()
            )
        })?;
    }
    Ok(())
}

fn toml_section_mut<'a>(
    value: &'a mut toml::Value,
    section: &str,
) -> Option<&'a mut toml::value::Table> {
    value.get_mut(section)?.as_table_mut()
}

fn migrate_legacy_string_key(
    value: &mut toml::Value,
    legacy_path: &[&str],
    canonical_path: &[&str],
    issues: &mut Vec<RuntimeConfigMigrationIssue>,
) {
    migrate_legacy_key(value, legacy_path, canonical_path, issues, |v| {
        matches!(v, toml::Value::String(_) | toml::Value::Boolean(_))
    });
}

fn migrate_legacy_array_key(
    value: &mut toml::Value,
    legacy_path: &[&str],
    canonical_path: &[&str],
    issues: &mut Vec<RuntimeConfigMigrationIssue>,
) {
    migrate_legacy_key(value, legacy_path, canonical_path, issues, |v| {
        matches!(v, toml::Value::Array(_) | toml::Value::String(_))
    });
}

fn migrate_legacy_key(
    value: &mut toml::Value,
    legacy_path: &[&str],
    canonical_path: &[&str],
    issues: &mut Vec<RuntimeConfigMigrationIssue>,
    predicate: impl Fn(&toml::Value) -> bool,
) {
    if legacy_path.len() != 2 || canonical_path.len() != 2 {
        return;
    }
    let Some(section) = toml_section_mut(value, legacy_path[0]) else {
        return;
    };
    let Some(legacy_value) = section.get(legacy_path[1]).cloned() else {
        return;
    };
    if !predicate(&legacy_value) {
        return;
    }
    let canonical_present = section.contains_key(canonical_path[1]);
    if !canonical_present {
        let rewritten = match legacy_value {
            toml::Value::String(value) if canonical_path[1] == "allowed_origins" => {
                toml::Value::Array(vec![toml::Value::String(value)])
            }
            toml::Value::String(value) if canonical_path[1] == "fallback_chain" => {
                toml::Value::Array(vec![toml::Value::String(value)])
            }
            other => other,
        };
        section.insert(canonical_path[1].to_string(), rewritten);
    }
    issues.push(RuntimeConfigMigrationIssue {
        legacy_key: legacy_path.join("."),
        canonical_key: canonical_path.join("."),
        action: if canonical_present {
            "legacy key detected; canonical key already present".to_string()
        } else {
            "canonical key populated from legacy value".to_string()
        },
    });
}

fn migrate_legacy_bind_key(value: &mut toml::Value, issues: &mut Vec<RuntimeConfigMigrationIssue>) {
    let Some(section) = toml_section_mut(value, "gateway") else {
        return;
    };
    let Some(bind_value) = section
        .get("bind")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return;
    };

    if let Some((host, port)) = bind_value.rsplit_once(':') {
        if !section.contains_key("host") {
            section.insert("host".to_string(), toml::Value::String(host.to_string()));
        }
        if !section.contains_key("port")
            && let Ok(port) = port.parse::<i64>()
        {
            section.insert("port".to_string(), toml::Value::Integer(port));
        }
        issues.push(RuntimeConfigMigrationIssue {
            legacy_key: "gateway.bind".to_string(),
            canonical_key: "gateway.host + gateway.port".to_string(),
            action: "canonical bind fields populated from legacy gateway.bind".to_string(),
        });
    }
}

fn infer_gateway_network_mode(
    value: &mut toml::Value,
    inferred_defaults: &mut Vec<String>,
) -> Result<()> {
    let Some(section) = toml_section_mut(value, "gateway") else {
        return Ok(());
    };
    if section.contains_key("network_mode") {
        return Ok(());
    }

    let host = section
        .get("host")
        .and_then(|value| value.as_str())
        .unwrap_or("127.0.0.1")
        .to_string();
    let inferred = match host.as_str() {
        "127.0.0.1" | "localhost" => "loopback",
        "0.0.0.0" => "lan",
        _ => "remote",
    };
    section.insert(
        "network_mode".to_string(),
        toml::Value::String(inferred.to_string()),
    );
    inferred_defaults.push(format!(
        "gateway.network_mode inferred as `{}` from gateway.host = `{}`",
        inferred, host
    ));
    Ok(())
}

fn remove_path_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory '{}'", path.display()))?;
    } else {
        fs::remove_file(path)
            .with_context(|| format!("Failed to remove file '{}'", path.display()))?;
    }
    Ok(())
}

impl Drop for RuntimeLockGuard {
    fn drop(&mut self) {
        let Ok(raw) = fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(record) = serde_json::from_str::<RuntimeLockRecord>(&raw) else {
            return;
        };
        if record.process_id == self.process_id {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::logs;
    use serial_test::serial;
    use tempfile::tempdir;

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.as_deref() {
                unsafe { std::env::set_var(self.key, previous) };
            } else {
                unsafe { std::env::remove_var(self.key) };
            }
        }
    }

    #[test]
    fn runtime_backup_and_restore_round_trip_workspace_state() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();

        fs::write(workspace_root.join(".env"), "OPENRUSTCLAW_TEST=1\n")?;
        fs::create_dir_all(workspace_root.join("config"))?;
        fs::write(
            workspace_root.join("config/default.toml"),
            "model = \"test\"\n",
        )?;
        fs::create_dir_all(workspace_root.join(".claw/control"))?;
        fs::write(
            workspace_root.join(".claw/control/runtime.json"),
            "{\"ok\":true}\n",
        )?;
        fs::create_dir_all(workspace_root.join(".claw/runtime-backups/existing"))?;
        fs::write(
            workspace_root.join(".claw/runtime-backups/existing/skip.txt"),
            "skip me\n",
        )?;

        let backup = backup_runtime_state(workspace_root, None)?;
        let backup_root = PathBuf::from(&backup.backup_path);
        assert!(backup_root.exists());
        assert!(backup.entries.iter().any(|entry| entry == ".env"));
        assert!(backup.entries.iter().any(|entry| entry == "config"));
        assert!(backup.entries.iter().any(|entry| entry == ".claw/control"));
        assert!(!backup_root.join("workspace/.claw/runtime-backups").exists());

        fs::write(workspace_root.join(".env"), "OPENRUSTCLAW_TEST=mutated\n")?;
        fs::remove_dir_all(workspace_root.join(".claw/control"))?;
        fs::create_dir_all(workspace_root.join(".claw/control"))?;
        fs::write(
            workspace_root.join(".claw/control/runtime.json"),
            "{\"ok\":false}\n",
        )?;

        let restored = restore_runtime_state(workspace_root, &backup.backup_path)?;
        assert!(PathBuf::from(&restored.pre_restore_backup_path).exists());
        assert_eq!(
            fs::read_to_string(workspace_root.join(".env"))?,
            "OPENRUSTCLAW_TEST=1\n"
        );
        assert_eq!(
            fs::read_to_string(workspace_root.join(".claw/control/runtime.json"))?,
            "{\"ok\":true}\n"
        );
        assert!(
            workspace_root
                .join(".claw/runtime-backups/existing/skip.txt")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn render_systemd_unit_uses_explicit_config_path() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        let unit = render_user_systemd_service_unit(
            Path::new("/tmp/openrustclaw"),
            workspace_root,
            "config/default.toml",
        );

        assert!(unit.contains("ExecStart=\"/tmp/openrustclaw\" start --config"));
        assert!(
            unit.contains(
                &workspace_root
                    .join("config/default.toml")
                    .display()
                    .to_string()
            )
        );
        assert!(unit.contains(&format!("WorkingDirectory={}", workspace_root.display())));
        Ok(())
    }

    #[test]
    fn render_launchd_plist_uses_explicit_config_path() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        let plist = render_user_launchd_service_plist(
            Path::new("/tmp/openrustclaw"),
            workspace_root,
            "config/default.toml",
        );

        assert!(plist.contains("<string>/tmp/openrustclaw</string>"));
        assert!(plist.contains("<string>start</string>"));
        assert!(plist.contains("<string>--config</string>"));
        assert!(
            plist.contains(
                &workspace_root
                    .join("config/default.toml")
                    .display()
                    .to_string()
            )
        );
        assert!(
            plist.contains(
                &logs::runtime_log_path_for(workspace_root)
                    .display()
                    .to_string()
            )
        );
        Ok(())
    }

    #[test]
    fn runtime_status_includes_gateway_network_metadata() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/runtime.toml");
        let mut config = AppConfig::default();
        config.gateway.network_mode = "lan".to_string();
        config.gateway.host = "0.0.0.0".to_string();
        config.gateway.port = 19999;
        config.gateway.allowed_origins = vec!["https://console.example.com".to_string()];
        config.security.trusted_proxy_token_env =
            Some("OPENRUSTCLAW_TRUSTED_PROXY_TOKEN".to_string());
        fs::write(&config_path, toml::to_string_pretty(&config)?)?;

        let status = runtime_status(config_path.to_str().unwrap(), workspace_root)?;
        assert_eq!(status.network_mode, "lan");
        assert_eq!(status.gateway_host, "0.0.0.0");
        assert_eq!(status.gateway_port, 19999);
        assert_eq!(status.allowed_origins, vec!["https://console.example.com"]);
        assert!(status.trusted_proxy_enabled);
        assert_eq!(status.control_plane_provider.as_deref(), Some("openrouter"));
        assert_eq!(
            status.control_plane_fallback_chain,
            vec!["ollama".to_string(), "anthropic".to_string()]
        );
        Ok(())
    }

    #[test]
    fn runtime_lock_status_reports_active_and_stale_records() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        let active_record = RuntimeLockRecord {
            acquired_at: Utc::now().to_rfc3339(),
            process_id: std::process::id(),
            gateway_addr: "127.0.0.1:8080".to_string(),
            config_path: "config/default.toml".to_string(),
        };
        save_runtime_lock(workspace_root, &active_record)?;

        let active = runtime_lock_status(workspace_root)?;
        assert!(active.present);
        assert!(active.active);
        assert!(!active.stale);

        let stale_record = RuntimeLockRecord {
            acquired_at: Utc::now().to_rfc3339(),
            process_id: 999_999,
            gateway_addr: "127.0.0.1:8080".to_string(),
            config_path: "config/default.toml".to_string(),
        };
        save_runtime_lock(workspace_root, &stale_record)?;

        let stale = runtime_lock_status(workspace_root)?;
        assert!(stale.present);
        assert!(!stale.active);
        assert!(stale.stale);
        Ok(())
    }

    #[test]
    fn runtime_lock_guard_cleans_up_owned_lock() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        let lock_path = runtime_lock_path_for(workspace_root);
        {
            let _guard =
                acquire_runtime_lock("config/default.toml", workspace_root, "127.0.0.1:1")?;
            assert!(lock_path.exists());
        }
        assert!(!lock_path.exists());
        Ok(())
    }

    #[test]
    fn migrate_config_detects_and_applies_legacy_keys() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/default.toml");
        let mut value = toml::to_string_pretty(&AppConfig::default())?.parse::<toml::Value>()?;
        let gateway = value
            .get_mut("gateway")
            .and_then(toml::Value::as_table_mut)
            .unwrap();
        gateway.remove("host");
        gateway.remove("port");
        gateway.remove("allowed_origins");
        gateway.remove("network_mode");
        gateway.insert(
            "bind".to_string(),
            toml::Value::String("0.0.0.0:19999".to_string()),
        );
        gateway.insert(
            "origin_whitelist".to_string(),
            toml::Value::Array(vec![toml::Value::String(
                "https://console.example.com".to_string(),
            )]),
        );

        let providers = value
            .get_mut("providers")
            .and_then(toml::Value::as_table_mut)
            .unwrap();
        providers.remove("default_provider");
        providers.remove("fallback_chain");
        providers.insert(
            "primary".to_string(),
            toml::Value::String("anthropic".to_string()),
        );
        providers.insert(
            "fallbacks".to_string(),
            toml::Value::Array(vec![toml::Value::String("openai".to_string())]),
        );

        let security = value
            .get_mut("security")
            .and_then(toml::Value::as_table_mut)
            .unwrap();
        security.remove("require_auth");
        security.insert(
            "require_authentication".to_string(),
            toml::Value::Boolean(true),
        );

        fs::write(&config_path, toml::to_string_pretty(&value)?)?;

        let dry_run = migrate_config(config_path.to_str().unwrap(), workspace_root, false)?;
        assert!(dry_run.changed);
        assert!(!dry_run.applied);
        assert!(
            dry_run
                .legacy_issues
                .iter()
                .any(|issue| issue.legacy_key == "providers.primary")
        );

        let applied = migrate_config(config_path.to_str().unwrap(), workspace_root, true)?;
        assert!(applied.applied);
        let rendered = fs::read_to_string(&config_path)?;
        assert!(rendered.contains("default_provider = \"anthropic\""));
        assert!(rendered.contains("fallback_chain = [\"openai\"]"));
        assert!(rendered.contains("network_mode = \"lan\""));
        assert!(rendered.contains("allowed_origins = [\"https://console.example.com\"]"));
        assert!(rendered.contains("require_auth = true"));
        Ok(())
    }

    #[test]
    fn runtime_upgrade_plan_uses_cached_health() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&AppConfig::default())?)?;
        fs::create_dir_all(workspace_root.join(".claw/control"))?;
        fs::write(
            runtime_health_path_for(workspace_root),
            serde_json::to_vec_pretty(&RuntimeHealthReport {
                generated_at: Utc::now().to_rfc3339(),
                config_path: "config/default.toml".to_string(),
                default_provider: "anthropic".to_string(),
                fallback_chain: vec!["openai".to_string()],
                control_plane_provider: Some("openrouter".to_string()),
                control_plane_fallback_chain: vec!["ollama".to_string()],
                recommended_control_plane_provider: Some("anthropic".to_string()),
                startup_fallback_valid: true,
                degraded_control_plane_mode: false,
                failover_recommendations: vec![
                    "Use `anthropic` as the current control-plane provider.".to_string(),
                ],
                operator_warnings: Vec::new(),
                providers: vec![RuntimeHealthProviderEntry {
                    provider: "anthropic".to_string(),
                    role: "primary".to_string(),
                    model: "claude-sonnet-4-20250514".to_string(),
                    configured: true,
                    healthy: true,
                    issue_kind: None,
                    recommendation: Some("configured task/runtime primary".to_string()),
                    issue: None,
                    model_available: Some(true),
                    limit_snapshot: None,
                }],
                artifacts: RuntimeArtifactHealth {
                    artifact_count: 0,
                    persona_artifact_count: 0,
                    registry_path: workspace_root
                        .join(".claw/artifacts/registry.json")
                        .display()
                        .to_string(),
                    model_family: "anthropic".to_string(),
                    included_for_default_model: 0,
                },
            })?,
        )?;

        let runtime = tokio::runtime::Runtime::new()?;
        let plan = runtime.block_on(runtime_upgrade_plan(
            config_path.to_str().unwrap(),
            workspace_root,
        ))?;
        assert_eq!(plan.config_path, config_path.to_str().unwrap());
        assert_eq!(plan.backup_command, "openrustclaw runtime backup");
        assert!(
            plan.steps
                .iter()
                .any(|step| step.contains("runtime migrate-config"))
        );
        Ok(())
    }

    #[test]
    fn runtime_update_and_rollback_plans_report_artifact_metadata() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&AppConfig::default())?)?;

        let artifact_path = workspace_root.join("openrustclaw-next");
        fs::write(&artifact_path, "#!/bin/sh\nexit 0\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&artifact_path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&artifact_path, permissions)?;
        }

        let runtime = tokio::runtime::Runtime::new()?;
        let update_plan = runtime.block_on(runtime_self_update_plan(
            config_path.to_str().unwrap(),
            workspace_root,
            artifact_path.to_str().unwrap(),
        ))?;
        assert!(update_plan.artifact_exists);
        assert!(update_plan.artifact_executable);
        assert!(
            update_plan
                .recommended_rollback_path
                .contains(".claw/runtime-releases")
        );

        let rollback_plan = runtime.block_on(runtime_rollback_plan(
            config_path.to_str().unwrap(),
            workspace_root,
            artifact_path.to_str().unwrap(),
        ))?;
        assert!(rollback_plan.rollback_artifact_exists);
        assert!(rollback_plan.rollback_artifact_executable);
        Ok(())
    }

    #[test]
    fn runtime_maintenance_plans_use_service_lane() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&AppConfig::default())?)?;

        fs::create_dir_all(workspace_root.join(".claw/control"))?;
        fs::write(
            runtime_health_path_for(workspace_root),
            serde_json::to_string_pretty(&RuntimeHealthReport {
                generated_at: "2026-03-28T00:00:00Z".to_string(),
                config_path: config_path.display().to_string(),
                default_provider: "anthropic".to_string(),
                fallback_chain: vec![],
                control_plane_provider: None,
                control_plane_fallback_chain: vec![],
                recommended_control_plane_provider: None,
                startup_fallback_valid: true,
                degraded_control_plane_mode: false,
                failover_recommendations: vec![],
                operator_warnings: vec![],
                providers: vec![RuntimeHealthProviderEntry {
                    provider: "anthropic".to_string(),
                    role: "default".to_string(),
                    model: "claude-sonnet-4-20250514".to_string(),
                    configured: true,
                    healthy: true,
                    issue_kind: None,
                    recommendation: None,
                    issue: None,
                    model_available: Some(true),
                    limit_snapshot: None,
                }],
                artifacts: RuntimeArtifactHealth {
                    artifact_count: 0,
                    persona_artifact_count: 0,
                    registry_path: workspace_root.join(".claw/artifacts").display().to_string(),
                    model_family: "anthropic".to_string(),
                    included_for_default_model: 0,
                },
            })?,
        )?;

        let artifact_path = workspace_root.join("openrustclaw-next");
        fs::write(&artifact_path, "#!/bin/sh\nexit 0\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&artifact_path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&artifact_path, permissions)?;
        }

        let runtime = tokio::runtime::Runtime::new()?;
        let upgrade_plan = runtime.block_on(runtime_upgrade_plan(
            config_path.to_str().unwrap(),
            workspace_root,
        ))?;
        assert!(
            upgrade_plan
                .steps
                .iter()
                .any(|step| step.contains("runtime migrate-config"))
        );

        let self_update_plan = runtime.block_on(runtime_self_update_plan(
            config_path.to_str().unwrap(),
            workspace_root,
            artifact_path.to_str().unwrap(),
        ))?;
        assert!(
            self_update_plan
                .steps
                .iter()
                .any(|step| step.contains("rollback-plan"))
        );

        let rollback_plan = runtime.block_on(runtime_rollback_plan(
            config_path.to_str().unwrap(),
            workspace_root,
            artifact_path.to_str().unwrap(),
        ))?;
        assert!(
            rollback_plan
                .steps
                .iter()
                .any(|step| step.contains("runtime health"))
        );

        Ok(())
    }

    #[test]
    fn runtime_operator_ops_summary_reports_recovery_guidance() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&AppConfig::default())?)?;
        fs::create_dir_all(workspace_root.join(".claw/control"))?;
        fs::write(
            runtime_health_path_for(workspace_root),
            serde_json::to_vec_pretty(&RuntimeHealthReport {
                generated_at: Utc::now().to_rfc3339(),
                config_path: "config/default.toml".to_string(),
                default_provider: "anthropic".to_string(),
                fallback_chain: vec!["openai".to_string()],
                control_plane_provider: Some("anthropic".to_string()),
                control_plane_fallback_chain: vec!["openai".to_string()],
                recommended_control_plane_provider: Some("anthropic".to_string()),
                startup_fallback_valid: true,
                degraded_control_plane_mode: false,
                failover_recommendations: Vec::new(),
                operator_warnings: Vec::new(),
                providers: vec![RuntimeHealthProviderEntry {
                    provider: "anthropic".to_string(),
                    role: "primary".to_string(),
                    model: "claude-sonnet-4-20250514".to_string(),
                    configured: true,
                    healthy: true,
                    issue_kind: None,
                    recommendation: Some("configured task/runtime primary".to_string()),
                    issue: None,
                    model_available: Some(true),
                    limit_snapshot: None,
                }],
                artifacts: RuntimeArtifactHealth {
                    artifact_count: 0,
                    persona_artifact_count: 0,
                    registry_path: workspace_root
                        .join(".claw/artifacts/registry.json")
                        .display()
                        .to_string(),
                    model_family: "anthropic".to_string(),
                    included_for_default_model: 0,
                },
            })?,
        )?;

        let runtime = tokio::runtime::Runtime::new()?;
        let summary = runtime.block_on(runtime_operator_ops_summary(
            config_path.to_str().unwrap(),
            workspace_root,
            "127.0.0.1:18789",
            None,
            false,
        ))?;
        assert!(summary.backup_root.contains(".claw/runtime-backups"));
        assert!(summary.release_root.contains(".claw/runtime-releases"));
        assert!(
            summary
                .recommended_actions
                .iter()
                .any(|entry| entry.contains("openrustclaw runtime backup"))
        );
        assert!(
            summary
                .recommended_actions
                .iter()
                .any(|entry| entry.contains("openrustclaw runtime upgrade-plan"))
        );
        Ok(())
    }

    #[test]
    fn switch_provider_updates_runtime_config_through_service_lane() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;
        let _openai_env = EnvVarGuard::set("OPENAI_API_KEY", "test-key");

        let mut config = AppConfig::default();
        config.providers.default_provider = "anthropic".to_string();
        config.providers.control_plane_provider = None;
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&config)?)?;

        let updated = switch_provider(
            config_path.to_str().unwrap(),
            workspace_root,
            "openai",
            Some("gpt-4.1-mini"),
            Some("OPENAI_API_KEY"),
            Some(vec!["anthropic".to_string()]),
        )?;

        assert_eq!(updated.providers.default_provider, "openai");
        assert_eq!(updated.providers.openai.model, "gpt-4.1-mini");
        assert_eq!(
            updated.providers.openai.api_key_env.as_deref(),
            Some("OPENAI_API_KEY")
        );
        assert_eq!(
            updated.providers.fallback_chain,
            vec!["anthropic".to_string()]
        );
        assert!(updated.providers.control_plane_provider.is_some());

        let persisted = AppConfig::load_from(config_path.to_str().unwrap())?;
        assert_eq!(persisted.providers.default_provider, "openai");
        assert_eq!(persisted.providers.openai.model, "gpt-4.1-mini");
        Ok(())
    }

    #[test]
    #[serial(vault_env)]
    fn set_and_delete_vault_secret_use_service_lane() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        let _passphrase = EnvVarGuard::set("OPENRUSTCLAW_VAULT_PASSPHRASE", "test-passphrase");

        set_vault_secret(workspace_root, "OPENAI_API_KEY", "secret-1")?;
        set_vault_secret(workspace_root, "ANTHROPIC_API_KEY", "secret-2")?;

        let keys = list_vault_keys(workspace_root)?;
        assert_eq!(
            keys,
            vec![
                "ANTHROPIC_API_KEY".to_string(),
                "OPENAI_API_KEY".to_string()
            ]
        );
        assert_eq!(
            get_vault_secret(workspace_root, "OPENAI_API_KEY")?,
            "secret-1"
        );

        delete_vault_secret(workspace_root, "OPENAI_API_KEY")?;

        let keys = list_vault_keys(workspace_root)?;
        assert_eq!(keys, vec!["ANTHROPIC_API_KEY".to_string()]);
        assert!(get_vault_secret(workspace_root, "OPENAI_API_KEY").is_err());
        Ok(())
    }

    #[test]
    fn runtime_reload_plan_uses_service_lane() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("config"))?;

        let mut config = AppConfig::default();
        config.providers.default_provider = "anthropic".to_string();
        let config_path = workspace_root.join("config/default.toml");
        fs::write(&config_path, toml::to_string_pretty(&config)?)?;

        let applied = capture_runtime_snapshot(config_path.to_str().unwrap(), workspace_root)?;
        save_runtime_snapshot(workspace_root, &applied)?;

        config.providers.default_provider = "openai".to_string();
        config.providers.openai.api_key_env = Some("OPENAI_API_KEY".to_string());
        fs::write(&config_path, toml::to_string_pretty(&config)?)?;

        let plan = runtime_reload_plan(config_path.to_str().unwrap(), workspace_root)?;
        assert_eq!(plan.status, "live_reload_ready");
        assert!(plan.live_reload_ready);
        assert!(!plan.restart_required);
        assert_eq!(plan.provider_changes.len(), 1);
        Ok(())
    }
}
