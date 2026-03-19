//! Runtime configuration, vault, and provider/model switching helpers.

use std::collections::BTreeMap;
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
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider,
    openrouter::RouteStrategy,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

pub const DEFAULT_VAULT_PATH: &str = ".claw/control/runtime-vault.json";
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

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn vault_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_VAULT_PATH)
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

pub fn validate_runtime_reload(config_path: &str, workspace_root: &Path) -> Result<RuntimeStatus> {
    let config = load_effective_config(config_path, workspace_root)?;
    validate_runtime_provider(&config, &config.providers.default_provider)?;
    for provider in &config.providers.fallback_chain {
        validate_runtime_provider(&config, provider)?;
    }
    runtime_status(config_path, workspace_root)
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
