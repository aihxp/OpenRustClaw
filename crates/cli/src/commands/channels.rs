//! File-backed channel account and binding manifests plus CLI controls.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use openrustclaw_core::types::{IncomingMessage, Platform};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

pub const DEFAULT_CHANNELS_DIR: &str = ".claw/channels";

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_priority() -> i32 {
    100
}

fn default_activation_mode() -> String {
    "mention".to_string()
}

fn default_send_mode() -> String {
    "blocks".to_string()
}

fn default_chunk_chars() -> usize {
    1600
}

fn default_preview_chars() -> usize {
    280
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccountManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub account: ChannelAccountSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccountSpec {
    pub id: String,
    pub platform: String,
    pub external_user_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub channel_scope: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub workspace_target: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub activation_mode: Option<String>,
    #[serde(default)]
    pub direct_strategy: Option<String>,
    #[serde(default)]
    pub group_strategy: Option<String>,
    #[serde(default)]
    pub send_policy: Option<ChannelSendPolicy>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBindingManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub binding: ChannelBindingSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBindingSpec {
    pub id: String,
    pub platform: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub workspace_match: Option<String>,
    #[serde(default)]
    pub account_match: Option<String>,
    #[serde(default)]
    pub channel_match: Option<String>,
    #[serde(default)]
    pub workspace_target: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub activation_mode: Option<String>,
    #[serde(default)]
    pub direct_strategy: Option<String>,
    #[serde(default)]
    pub group_strategy: Option<String>,
    #[serde(default)]
    pub send_policy: Option<ChannelSendPolicy>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSendPolicy {
    #[serde(default = "default_send_mode")]
    pub mode: String,
    #[serde(default = "default_chunk_chars")]
    pub max_chunk_chars: usize,
    #[serde(default)]
    pub chunk_delay_ms: u64,
    #[serde(default)]
    pub coalesce_below_chars: Option<usize>,
    #[serde(default = "default_preview_chars")]
    pub preview_chars: usize,
}

impl Default for ChannelSendPolicy {
    fn default() -> Self {
        Self {
            mode: default_send_mode(),
            max_chunk_chars: default_chunk_chars(),
            chunk_delay_ms: 250,
            coalesce_below_chars: Some(320),
            preview_chars: default_preview_chars(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelRegistry {
    pub root: PathBuf,
    pub accounts: HashMap<String, ChannelAccountSpec>,
    pub bindings: Vec<ChannelBindingSpec>,
}

#[derive(Debug, Clone)]
pub struct ChannelIdentity {
    pub platform: Platform,
    pub account_id: String,
    pub external_user_id: String,
    pub workspace_id: Option<String>,
    pub channel_scope: Option<String>,
    pub is_group: bool,
    pub bot_mentioned: bool,
}

pub fn channels_root_for(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(DEFAULT_CHANNELS_DIR)
}

fn accounts_dir(root: &Path) -> PathBuf {
    root.join("accounts")
}

fn bindings_dir(root: &Path) -> PathBuf {
    root.join("bindings")
}

fn account_manifest_path(root: &Path, account_id: &str) -> PathBuf {
    accounts_dir(root).join(format!("{}.yaml", slugify(account_id)))
}

pub fn init(root: Option<&str>) -> Result<()> {
    let root = resolve_root(root)?;
    fs::create_dir_all(accounts_dir(&root))
        .with_context(|| format!("Failed to create '{}'", accounts_dir(&root).display()))?;
    fs::create_dir_all(bindings_dir(&root))
        .with_context(|| format!("Failed to create '{}'", bindings_dir(&root).display()))?;
    println!("Initialized channel registry at {}", root.display());
    Ok(())
}

pub fn list(root: Option<&str>) -> Result<()> {
    let registry = load_registry(resolve_root(root)?)?;
    if registry.accounts.is_empty() && registry.bindings.is_empty() {
        println!("No channel accounts or bindings found.");
        return Ok(());
    }

    println!("Accounts:");
    for account in registry.accounts.values() {
        println!(
            "- {} [{}] approved={} blocked={} workspace={}",
            account.id,
            account.platform,
            account.approved,
            account.blocked,
            account.workspace_id.as_deref().unwrap_or("-")
        );
    }
    println!("Bindings:");
    for binding in &registry.bindings {
        println!(
            "- {} [{}] priority={} workspace_match={} account_match={} channel_match={}",
            binding.id,
            binding.platform,
            binding.priority,
            binding.workspace_match.as_deref().unwrap_or("-"),
            binding.account_match.as_deref().unwrap_or("-"),
            binding.channel_match.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub fn approve(root: Option<&str>, id: &str) -> Result<()> {
    mutate_account(root, id, |account| {
        account.approved = true;
        account.blocked = false;
    })
}

pub fn block(root: Option<&str>, id: &str) -> Result<()> {
    mutate_account(root, id, |account| {
        account.blocked = true;
        account.approved = false;
    })
}

pub fn activation(root: Option<&str>, id: &str, mode: &str) -> Result<()> {
    if !matches!(mode, "mention" | "always") {
        anyhow::bail!("activation mode must be 'mention' or 'always'");
    }
    mutate_account(root, id, |account| {
        account.activation_mode = Some(mode.to_string());
    })
}

pub fn bind(
    root: Option<&str>,
    id: &str,
    platform: &str,
    workspace_match: Option<&str>,
    account_match: Option<&str>,
    channel_match: Option<&str>,
    workspace_target: Option<&str>,
    agent_id: Option<&str>,
    activation_mode: Option<&str>,
) -> Result<()> {
    let root = resolve_root(root)?;
    fs::create_dir_all(bindings_dir(&root))
        .with_context(|| format!("Failed to create '{}'", bindings_dir(&root).display()))?;

    let manifest = ChannelBindingManifest {
        version: 1,
        binding: ChannelBindingSpec {
            id: id.to_string(),
            platform: platform.to_string(),
            enabled: true,
            priority: 100,
            workspace_match: workspace_match.map(ToString::to_string),
            account_match: account_match.map(ToString::to_string),
            channel_match: channel_match.map(ToString::to_string),
            workspace_target: workspace_target.map(ToString::to_string),
            agent_id: agent_id.map(ToString::to_string),
            activation_mode: activation_mode.map(ToString::to_string),
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({}),
        },
    };

    let path = bindings_dir(&root).join(format!("{}.yaml", slugify(id)));
    fs::write(
        &path,
        serde_yaml::to_string(&manifest).context("Failed to render binding manifest")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    println!("Wrote binding manifest {}", path.display());
    Ok(())
}

fn mutate_account(
    root: Option<&str>,
    id: &str,
    mutator: impl FnOnce(&mut ChannelAccountSpec),
) -> Result<()> {
    let root = resolve_root(root)?;
    let path = account_manifest_path(&root, id);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read account manifest '{}'", path.display()))?;
    let mut manifest: ChannelAccountManifest =
        serde_yaml::from_str(&raw).context("Failed to parse account manifest")?;
    mutator(&mut manifest.account);
    fs::write(
        &path,
        serde_yaml::to_string(&manifest).context("Failed to render account manifest")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    println!("Updated {}", path.display());
    Ok(())
}

pub fn resolve_root(root: Option<&str>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to determine current workspace root")?;
    Ok(match root {
        Some(root) => {
            let path = PathBuf::from(root);
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        }
        None => channels_root_for(cwd),
    })
}

pub fn load_registry(root: PathBuf) -> Result<ChannelRegistry> {
    let mut registry = ChannelRegistry {
        root: root.clone(),
        accounts: HashMap::new(),
        bindings: Vec::new(),
    };

    for path in detect_yaml_paths(&accounts_dir(&root)) {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let manifest: ChannelAccountManifest =
            serde_yaml::from_str(&raw).with_context(|| format!("Invalid '{}'", path.display()))?;
        registry
            .accounts
            .insert(manifest.account.id.clone(), manifest.account);
    }

    for path in detect_yaml_paths(&bindings_dir(&root)) {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let manifest: ChannelBindingManifest =
            serde_yaml::from_str(&raw).with_context(|| format!("Invalid '{}'", path.display()))?;
        registry.bindings.push(manifest.binding);
    }
    registry
        .bindings
        .sort_by_key(|binding| (binding.priority, binding.id.clone()));

    Ok(registry)
}

pub fn ensure_account_manifest(
    root: &Path,
    identity: &ChannelIdentity,
    require_approval: bool,
) -> Result<ChannelAccountSpec> {
    fs::create_dir_all(accounts_dir(root))
        .with_context(|| format!("Failed to create '{}'", accounts_dir(root).display()))?;
    let path = account_manifest_path(root, &identity.account_id);
    if path.exists() {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let manifest: ChannelAccountManifest =
            serde_yaml::from_str(&raw).with_context(|| format!("Invalid '{}'", path.display()))?;
        return Ok(manifest.account);
    }

    let manifest = ChannelAccountManifest {
        version: 1,
        account: ChannelAccountSpec {
            id: identity.account_id.clone(),
            platform: identity.platform.to_string(),
            external_user_id: identity.external_user_id.clone(),
            display_name: None,
            workspace_id: identity.workspace_id.clone(),
            channel_scope: identity.channel_scope.clone(),
            enabled: true,
            approved: !require_approval,
            blocked: false,
            workspace_target: identity.workspace_id.clone(),
            agent_id: None,
            activation_mode: Some(default_activation_mode()),
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({
                "is_group": identity.is_group,
                "bot_mentioned": identity.bot_mentioned,
                "created_by_runtime": true,
            }),
        },
    };
    fs::write(
        &path,
        serde_yaml::to_string(&manifest).context("Failed to render account manifest")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(manifest.account)
}

pub fn identity_from_message(message: &IncomingMessage) -> ChannelIdentity {
    let workspace_id = message_workspace_id(message);
    let channel_scope = message_channel_scope(message);
    let is_group = message_is_group(message);
    let bot_mentioned = message_bot_mentioned(message);
    let external_user_id = message.user_id.clone();
    let account_id = format!(
        "{}:{}:{}",
        message.platform,
        workspace_id.as_deref().unwrap_or("direct"),
        external_user_id
    );

    ChannelIdentity {
        platform: message.platform,
        account_id,
        external_user_id,
        workspace_id,
        channel_scope,
        is_group,
        bot_mentioned,
    }
}

pub fn message_workspace_id(message: &IncomingMessage) -> Option<String> {
    let metadata = &message.metadata;
    let keys = [
        "workspace_id",
        "slack_team_id",
        "discord_guild_id",
        "telegram_chat_id",
        "whatsapp_workspace_id",
        "imessage_workspace_id",
        "webchat_workspace_id",
    ];
    lookup_string(metadata, &keys)
}

pub fn message_channel_scope(message: &IncomingMessage) -> Option<String> {
    let metadata = &message.metadata;
    let keys = [
        "slack_thread_ts",
        "slack_channel",
        "discord_thread_id",
        "discord_channel_id",
        "whatsapp_group_id",
        "imessage_chat_guid",
        "telegram_chat_id",
        "webchat_room_id",
    ];
    lookup_string(metadata, &keys)
}

pub fn message_is_group(message: &IncomingMessage) -> bool {
    let metadata = &message.metadata;
    if let Some(value) = metadata
        .get("telegram_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("discord_is_dm")
        .and_then(|value| value.as_bool())
    {
        return !value;
    }
    if let Some(value) = metadata
        .get("slack_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("webchat_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("whatsapp_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("imessage_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    false
}

pub fn message_bot_mentioned(message: &IncomingMessage) -> bool {
    let metadata = &message.metadata;
    for key in [
        "slack_bot_mentioned",
        "discord_bot_mentioned",
        "telegram_bot_mentioned",
        "whatsapp_bot_mentioned",
        "imessage_bot_mentioned",
        "webchat_bot_mentioned",
    ] {
        if metadata.get(key).and_then(|value| value.as_bool()) == Some(true) {
            return true;
        }
    }
    false
}

fn detect_yaml_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if !root.exists() {
        return paths;
    }
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|value| value.to_str());
        if matches!(ext, Some("yaml" | "yml")) {
            paths.push(path.to_path_buf());
        }
    }
    paths.sort();
    paths
}

fn lookup_string(metadata: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = metadata.get(*key) {
            if let Some(text) = value.as_str() {
                return Some(text.to_string());
            }
            if let Some(number) = value.as_i64() {
                return Some(number.to_string());
            }
            if let Some(number) = value.as_u64() {
                return Some(number.to_string());
            }
        }
    }
    None
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch.to_ascii_lowercase()
        } else if !last_dash {
            last_dash = true;
            '-'
        } else {
            continue;
        };
        slug.push(next);
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_identity_uses_workspace_and_user() {
        let identity = identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "U123".to_string(),
            content: "hello".to_string(),
            platform: Platform::Slack,
            metadata: serde_json::json!({
                "slack_team_id": "T123",
                "slack_channel": "C123",
                "slack_is_group": true,
                "slack_bot_mentioned": true
            }),
        });

        assert_eq!(identity.account_id, "slack:T123:U123");
        assert_eq!(identity.channel_scope.as_deref(), Some("C123"));
        assert!(identity.is_group);
        assert!(identity.bot_mentioned);
    }
}
