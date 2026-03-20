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
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    pub anthropic_model: String,
    pub openai_model: String,
    pub openrouter_model: String,
    pub ollama_model: String,
    pub vault_path: String,
    pub vault_present: bool,
    pub vault_unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealthProviderEntry {
    pub provider: String,
    pub role: String,
    pub model: String,
    pub configured: bool,
    pub healthy: bool,
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
    pub recommended_control_plane_provider: Option<String>,
    pub startup_fallback_valid: bool,
    pub degraded_control_plane_mode: bool,
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
        default_provider: config.providers.default_provider.clone(),
        fallback_chain: config.providers.fallback_chain.clone(),
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

pub async fn scan_runtime_health(
    config_path: &str,
    workspace_root: &Path,
) -> Result<RuntimeHealthReport> {
    let config = load_effective_config(config_path, workspace_root)?;

    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();
    for provider in std::iter::once(config.providers.default_provider.clone())
        .chain(config.providers.fallback_chain.clone().into_iter())
    {
        if seen.insert(provider.clone()) {
            ordered.push(provider);
        }
    }

    let mut entries = Vec::new();
    for provider in &ordered {
        entries.push(scan_provider_health(provider, &config).await);
    }

    let startup_fallback_valid = entries
        .iter()
        .any(|entry| entry.provider != config.providers.default_provider && entry.healthy);
    let default_healthy = entries
        .iter()
        .find(|entry| entry.provider == config.providers.default_provider)
        .map(|entry| entry.healthy)
        .unwrap_or(false);
    let recommended_control_plane_provider = entries
        .iter()
        .find(|entry| entry.provider != config.providers.default_provider && entry.healthy)
        .map(|entry| entry.provider.clone())
        .or_else(|| default_healthy.then(|| config.providers.default_provider.clone()));

    let artifacts = summarize_runtime_artifacts(workspace_root, &config)?;
    let report = RuntimeHealthReport {
        generated_at: Utc::now().to_rfc3339(),
        config_path: config_path.to_string(),
        default_provider: config.providers.default_provider.clone(),
        fallback_chain: config.providers.fallback_chain.clone(),
        recommended_control_plane_provider,
        startup_fallback_valid,
        degraded_control_plane_mode: !default_healthy && startup_fallback_valid,
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
    let role = if provider == config.providers.default_provider {
        "primary".to_string()
    } else {
        let index = config
            .providers
            .fallback_chain
            .iter()
            .position(|value| value == provider)
            .map(|value| value + 1)
            .unwrap_or(0);
        format!("fallback_{index}")
    };

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
            issue: None,
        },
        Err(error) => RuntimeHealthProviderEntry {
            provider: provider.to_string(),
            role,
            model,
            configured: !error.to_string().contains("environment variable not set"),
            healthy: false,
            issue: Some(error.to_string()),
        },
    }
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
