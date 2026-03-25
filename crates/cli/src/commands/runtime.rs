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
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_memory::{WorkspaceArtifactRegistry, artifacts::ArtifactClass};
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider,
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
pub const DEFAULT_RUNTIME_LOCK_PATH: &str = ".claw/control/runtime-lock.json";
pub const DEFAULT_SYSTEMD_SERVICE_NAME: &str = "openrustclaw.service";
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
    pub anthropic_model: String,
    pub openai_model: String,
    pub openrouter_model: String,
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
    pub backup_command: String,
    pub log_rotation_command: String,
    pub migrate_config_command: String,
    pub ready_for_upgrade: bool,
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
    pub recommendation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeAppliedSnapshot {
    pub captured_at: String,
    pub config_path: String,
    pub default_provider: String,
    pub enabled_channels: Vec<String>,
    pub provider_fingerprint: String,
    pub gateway_fingerprint: String,
    pub security_fingerprint: String,
    pub scheduler_fingerprint: String,
    pub sidecar_fingerprint: String,
    pub channel_fingerprint: String,
    pub channel_route_fingerprint: String,
    pub artifact_fingerprint: String,
    pub artifact_paths: Vec<String>,
    pub persona_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeReloadPlan {
    pub generated_at: String,
    pub config_path: String,
    pub applied_snapshot_at: Option<String>,
    pub status: String,
    pub live_reload_ready: bool,
    pub restart_required: bool,
    pub live_reload_changes: Vec<String>,
    pub provider_changes: Vec<String>,
    pub artifact_changes: Vec<String>,
    pub restart_required_reasons: Vec<String>,
    pub changed_artifacts: Vec<String>,
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

pub fn runtime_lock_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_RUNTIME_LOCK_PATH)
}

pub fn runtime_service_install_status(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeServiceInstallStatus> {
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let service_path = user_systemd_service_path();
    let supported = systemd_user_service_supported();
    let installed = service_path
        .as_ref()
        .map(|path| path.exists())
        .unwrap_or(false);
    let resolved_config_path = resolve_runtime_config_path(workspace_root, config_path);

    Ok(RuntimeServiceInstallStatus {
        service_manager: "systemd-user".to_string(),
        supported,
        installed,
        service_path: service_path.as_ref().map(|path| path.display().to_string()),
        executable_path: current_exe.display().to_string(),
        workspace_root: workspace_root.display().to_string(),
        config_path: resolved_config_path.display().to_string(),
        daemon_reload_command: supported.then(|| "systemctl --user daemon-reload".to_string()),
        enable_command: supported
            .then(|| format!("systemctl --user enable {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
        start_command: supported
            .then(|| format!("systemctl --user start {}", DEFAULT_SYSTEMD_SERVICE_NAME)),
    })
}

pub fn install_runtime_user_service(
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
    let runtime_health = runtime_health_status(config_path, workspace_root, false).await?;
    let reload_plan = runtime_reload_plan(config_path, workspace_root)?;
    let service_install_status = runtime_service_install_status(config_path, workspace_root)?;
    let lock_status = runtime_lock_status(workspace_root)?;

    let mut blockers = Vec::new();
    if lock_status.active {
        blockers.push("runtime lock is currently active; schedule the upgrade around the running process or stop it first".to_string());
    }
    if !runtime_health.startup_fallback_valid
        && !runtime_health
            .providers
            .iter()
            .any(|entry| entry.provider == runtime_health.default_provider && entry.healthy)
    {
        blockers.push(
            "no healthy default or fallback control-plane provider is currently available"
                .to_string(),
        );
    }
    if !service_install_status.supported {
        blockers.push("user-level systemd integration is unavailable on this host".to_string());
    }

    let mut steps = vec![
        "Take a workspace snapshot with `openrustclaw runtime backup` before changing config or binaries.".to_string(),
        "Rotate and archive the runtime log with `openrustclaw runtime services rotate-logs --keep 7 --max-bytes 10485760` before restart windows.".to_string(),
        format!(
            "Run `openrustclaw runtime migrate-config --config {}{}` to normalize any remaining legacy config keys.",
            config_path,
            if blockers.is_empty() { "" } else { "" }
        ),
    ];
    if reload_plan.restart_required {
        steps.push("The current reload plan reports restart-required changes; prefer a controlled restart rather than live rebind.".to_string());
    } else {
        steps.push("The current reload plan is live-safe; apply non-disruptive config changes before the maintenance restart if needed.".to_string());
    }
    if service_install_status.installed {
        steps.push(
            "Use the installed user service to restart cleanly after the upgrade.".to_string(),
        );
    } else {
        steps.push("Consider `openrustclaw runtime services install --config ...` if this workspace should restart under user-level systemd.".to_string());
    }

    Ok(RuntimeUpgradePlan {
        generated_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        runtime_status,
        runtime_health,
        reload_plan,
        service_install_status,
        lock_status,
        backup_command: "openrustclaw runtime backup".to_string(),
        log_rotation_command:
            "openrustclaw runtime services rotate-logs --keep 7 --max-bytes 10485760".to_string(),
        migrate_config_command: format!(
            "openrustclaw runtime migrate-config --config {} --apply",
            config_path
        ),
        ready_for_upgrade: blockers.is_empty(),
        blockers,
        steps,
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
        anthropic_model: config.providers.anthropic.model.clone(),
        openai_model: config.providers.openai.model.clone(),
        openrouter_model: config.providers.openrouter.model.clone(),
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

    let mut live_reload_changes = Vec::new();
    let mut provider_changes = Vec::new();
    let mut artifact_changes = Vec::new();
    let mut restart_required_reasons = Vec::new();

    if let Some(previous) = &applied {
        if previous.provider_fingerprint != current.provider_fingerprint {
            provider_changes.push(format!(
                "Provider/model routing changed: {} -> {}",
                previous.default_provider, current.default_provider
            ));
            live_reload_changes.push(
                "Provider and model routing can rebind live through the shipped runtime reload path."
                    .to_string(),
            );
        }

        if previous.artifact_fingerprint != current.artifact_fingerprint {
            artifact_changes.push(
                "Workspace persona/instruction artifacts changed and will be read on the next turn."
                    .to_string(),
            );
            live_reload_changes.push(
                "Prompt artifact changes are live-safe because the agent runtime resolves workspace artifacts per request."
                    .to_string(),
            );
        }

        if previous.gateway_fingerprint != current.gateway_fingerprint {
            restart_required_reasons.push(
                "Gateway host/port settings changed and require a process restart.".to_string(),
            );
        }
        if previous.security_fingerprint != current.security_fingerprint {
            restart_required_reasons.push(
                "Gateway auth/origin policy changed and requires a process restart.".to_string(),
            );
        }
        if previous.scheduler_fingerprint != current.scheduler_fingerprint {
            restart_required_reasons
                .push("Scheduler timing changed and requires worker restart.".to_string());
        }
        if previous.sidecar_fingerprint != current.sidecar_fingerprint {
            restart_required_reasons
                .push("Sidecar launch settings changed and require process restart.".to_string());
        }
        if previous.enabled_channels != current.enabled_channels {
            restart_required_reasons.push(
                "Enabled channel set changed and requires channel transport restart.".to_string(),
            );
        } else if previous.channel_route_fingerprint != current.channel_route_fingerprint {
            restart_required_reasons.push(
                "Webhook or transport-routing paths changed and require process restart."
                    .to_string(),
            );
        } else if previous.channel_fingerprint != current.channel_fingerprint {
            restart_required_reasons.push(
                "Running channel transport settings changed; restart is required to reconnect safely."
                    .to_string(),
            );
        }
    }

    let changed_artifacts = applied
        .as_ref()
        .map(|previous| {
            current
                .artifact_paths
                .iter()
                .filter(|path| !previous.artifact_paths.contains(*path))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let restart_required = !restart_required_reasons.is_empty();
    let live_reload_ready = !restart_required
        && (!live_reload_changes.is_empty()
            || applied
                .as_ref()
                .map(|previous| previous.provider_fingerprint != current.provider_fingerprint)
                .unwrap_or(false));
    let status = if restart_required {
        "restart_required"
    } else if live_reload_ready {
        "live_reload_ready"
    } else {
        "up_to_date"
    };

    Ok(RuntimeReloadPlan {
        generated_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        applied_snapshot_at: applied.map(|snapshot| snapshot.captured_at),
        status: status.to_string(),
        live_reload_ready,
        restart_required,
        live_reload_changes,
        provider_changes,
        artifact_changes,
        restart_required_reasons,
        changed_artifacts,
    })
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
        "ollama" => config.providers.ollama.model.clone(),
        _ => String::new(),
    };

    let result = match provider {
        "ollama" => validate_ollama_provider(config).await,
        _ => validate_runtime_provider(config, provider),
    };

    match result {
        Ok(()) => RuntimeHealthProviderEntry {
            provider: provider.to_string(),
            role,
            model,
            configured: true,
            healthy: true,
            recommendation: provider_recommendation(provider, config),
            issue: None,
        },
        Err(error) => RuntimeHealthProviderEntry {
            provider: provider.to_string(),
            role,
            model,
            configured: !error.to_string().contains("environment variable not set"),
            healthy: false,
            recommendation: provider_recommendation(provider, config),
            issue: Some(error.to_string()),
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
    warnings
}

async fn validate_ollama_provider(config: &AppConfig) -> Result<()> {
    let base_url = config.providers.ollama.base_url.trim_end_matches('/');
    reqwest::Client::new()
        .get(format!("{base_url}/api/tags"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .context("Failed to reach Ollama")?
        .error_for_status()
        .context("Ollama health probe returned an error status")?;
    Ok(())
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
    let mut vault = load_vault(workspace_root, None).unwrap_or_default();
    vault.version = 1;
    vault.entries.insert(key.to_string(), value.to_string());
    vault.updated_at = Some(Utc::now().to_rfc3339());
    save_vault(workspace_root, &vault)
}

pub fn delete_vault_secret(workspace_root: &Path, key: &str) -> Result<()> {
    let mut vault = load_vault(workspace_root, None)?;
    vault.entries.remove(key);
    vault.updated_at = Some(Utc::now().to_rfc3339());
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
    let mut config = load_effective_config(config_path, workspace_root)?;
    match provider {
        "anthropic" => {
            if let Some(model) = model {
                config.providers.anthropic.model = model.to_string();
            }
            if let Some(api_key_env) = api_key_env {
                config.providers.anthropic.api_key_env = Some(api_key_env.to_string());
            }
        }
        "openai" => {
            if let Some(model) = model {
                config.providers.openai.model = model.to_string();
            }
            if let Some(api_key_env) = api_key_env {
                config.providers.openai.api_key_env = Some(api_key_env.to_string());
            }
        }
        "openrouter" => {
            if let Some(model) = model {
                config.providers.openrouter.model = model.to_string();
            }
            if let Some(api_key_env) = api_key_env {
                config.providers.openrouter.api_key_env = Some(api_key_env.to_string());
            }
        }
        "ollama" => {
            if let Some(model) = model {
                config.providers.ollama.model = model.to_string();
            }
        }
        _ => anyhow::bail!("Unknown provider '{}'", provider),
    }

    config.providers.default_provider = provider.to_string();
    if let Some(fallback_chain) = fallback_chain {
        config.providers.fallback_chain = fallback_chain;
    }
    validate_runtime_provider(&config, provider)?;
    write_config_with_backup(config_path, &config)?;
    Ok(config)
}

pub fn switch_model(
    config_path: &str,
    workspace_root: &Path,
    provider: &str,
    model: &str,
) -> Result<AppConfig> {
    switch_provider(
        config_path,
        workspace_root,
        provider,
        Some(model),
        None,
        None,
    )
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
        "ollama" => Ok(Arc::new(OllamaProvider::with_base_url(
            config.providers.ollama.model.clone(),
            config.providers.ollama.base_url.clone(),
        ))),
        _ => anyhow::bail!(
            "Unknown provider '{}'. Available: anthropic, openai, openrouter, ollama",
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

fn user_systemd_service_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("systemd/user").join(DEFAULT_SYSTEMD_SERVICE_NAME))
}

fn resolve_runtime_config_path(workspace_root: &Path, config_path: &str) -> PathBuf {
    let config_path = PathBuf::from(config_path);
    if config_path.is_absolute() {
        config_path
    } else {
        workspace_root.join(config_path)
    }
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
    use tempfile::tempdir;

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
                    recommendation: Some("configured task/runtime primary".to_string()),
                    issue: None,
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
}
