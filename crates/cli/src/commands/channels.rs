//! File-backed channel account and binding manifests plus CLI controls.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use openrustclaw_core::config::SessionRoutingConfig;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelRouteStatus {
    Allowed,
    PendingApproval,
    Blocked,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRoutePreview {
    pub status: ChannelRouteStatus,
    pub route_key: String,
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: String,
    pub send_policy: ChannelSendPolicy,
    pub account_id: String,
    pub binding_id: Option<String>,
    pub should_respond: bool,
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

fn binding_manifest_path(root: &Path, binding_id: &str) -> PathBuf {
    bindings_dir(root).join(format!("{}.yaml", slugify(binding_id)))
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

pub fn show_account(root: Option<&str>, id: &str) -> Result<()> {
    let account = read_account(resolve_root(root)?, id)?;
    println!(
        "{}",
        serde_yaml::to_string(&ChannelAccountManifest {
            version: 1,
            account,
        })
        .context("Failed to render account manifest")?
    );
    Ok(())
}

pub fn create_account(
    root: Option<&str>,
    id: &str,
    platform: &str,
    external_user_id: &str,
    display_name: Option<&str>,
    workspace_id: Option<&str>,
    channel_scope: Option<&str>,
    workspace_target: Option<&str>,
    agent_id: Option<&str>,
    approved: bool,
    blocked: bool,
    enabled: bool,
    activation_mode: Option<&str>,
) -> Result<()> {
    let root = resolve_root(root)?;
    write_account_manifest(
        &root,
        ChannelAccountSpec {
            id: id.to_string(),
            platform: platform.to_string(),
            external_user_id: external_user_id.to_string(),
            display_name: display_name.map(ToString::to_string),
            workspace_id: workspace_id.map(ToString::to_string),
            channel_scope: channel_scope.map(ToString::to_string),
            enabled,
            approved,
            blocked,
            workspace_target: workspace_target.map(ToString::to_string),
            agent_id: agent_id.map(ToString::to_string),
            activation_mode: activation_mode.map(ToString::to_string),
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({}),
        },
    )?;
    println!(
        "Wrote account manifest {}",
        account_manifest_path(&root, id).display()
    );
    Ok(())
}

pub fn delete_account(root: Option<&str>, id: &str) -> Result<()> {
    let root = resolve_root(root)?;
    let path = account_manifest_path(&root, id);
    if !path.exists() {
        anyhow::bail!("Account '{}' not found", id);
    }
    fs::remove_file(&path).with_context(|| format!("Failed to remove '{}'", path.display()))?;
    println!("Deleted {}", path.display());
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

    write_binding_manifest(
        &root,
        ChannelBindingSpec {
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
    )?;
    let path = binding_manifest_path(&root, id);
    println!("Wrote binding manifest {}", path.display());
    Ok(())
}

pub fn show_binding(root: Option<&str>, id: &str) -> Result<()> {
    let binding = read_binding(resolve_root(root)?, id)?;
    println!(
        "{}",
        serde_yaml::to_string(&ChannelBindingManifest {
            version: 1,
            binding,
        })
        .context("Failed to render binding manifest")?
    );
    Ok(())
}

pub fn delete_binding(root: Option<&str>, id: &str) -> Result<()> {
    let root = resolve_root(root)?;
    let path = binding_manifest_path(&root, id);
    if !path.exists() {
        anyhow::bail!("Binding '{}' not found", id);
    }
    fs::remove_file(&path).with_context(|| format!("Failed to remove '{}'", path.display()))?;
    println!("Deleted {}", path.display());
    Ok(())
}

pub fn upsert_account(root: Option<&str>, account: ChannelAccountSpec) -> Result<()> {
    let root = resolve_root(root)?;
    write_account_manifest(&root, account)
}

pub fn upsert_binding(root: Option<&str>, binding: ChannelBindingSpec) -> Result<()> {
    let root = resolve_root(root)?;
    write_binding_manifest(&root, binding)
}

pub fn read_account(root: PathBuf, id: &str) -> Result<ChannelAccountSpec> {
    let path = account_manifest_path(&root, id);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read account manifest '{}'", path.display()))?;
    let manifest: ChannelAccountManifest =
        serde_yaml::from_str(&raw).context("Failed to parse account manifest")?;
    Ok(manifest.account)
}

pub fn read_binding(root: PathBuf, id: &str) -> Result<ChannelBindingSpec> {
    let path = binding_manifest_path(&root, id);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read binding manifest '{}'", path.display()))?;
    let manifest: ChannelBindingManifest =
        serde_yaml::from_str(&raw).context("Failed to parse binding manifest")?;
    Ok(manifest.binding)
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

fn write_account_manifest(root: &Path, account: ChannelAccountSpec) -> Result<()> {
    fs::create_dir_all(accounts_dir(root))
        .with_context(|| format!("Failed to create '{}'", accounts_dir(root).display()))?;
    let manifest = ChannelAccountManifest {
        version: 1,
        account,
    };
    let path = account_manifest_path(root, &manifest.account.id);
    fs::write(
        &path,
        serde_yaml::to_string(&manifest).context("Failed to render account manifest")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
    Ok(())
}

fn write_binding_manifest(root: &Path, binding: ChannelBindingSpec) -> Result<()> {
    fs::create_dir_all(bindings_dir(root))
        .with_context(|| format!("Failed to create '{}'", bindings_dir(root).display()))?;
    let manifest = ChannelBindingManifest {
        version: 1,
        binding,
    };
    let path = binding_manifest_path(root, &manifest.binding.id);
    fs::write(
        &path,
        serde_yaml::to_string(&manifest).context("Failed to render binding manifest")?,
    )
    .with_context(|| format!("Failed to write '{}'", path.display()))?;
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
        "teams_conversation_id",
        "mattermost_team_id",
        "google_chat_space",
        "matrix_room_id",
        "telegram_chat_id",
        "whatsapp_workspace_id",
        "imessage_workspace_id",
        "signal_group_id",
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
        "teams_conversation_id",
        "mattermost_root_id",
        "mattermost_channel_id",
        "google_chat_thread",
        "google_chat_space",
        "matrix_thread_root",
        "matrix_room_id",
        "whatsapp_group_id",
        "imessage_chat_guid",
        "signal_group_id",
        "signal_source_number",
        "signal_source_uuid",
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
    if let Some(value) = metadata
        .get("signal_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("mattermost_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("teams_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("google_chat_is_group")
        .and_then(|value| value.as_bool())
    {
        return value;
    }
    if let Some(value) = metadata
        .get("matrix_is_group")
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
        "signal_bot_mentioned",
        "mattermost_bot_mentioned",
        "teams_bot_mentioned",
        "google_chat_bot_mentioned",
        "webchat_bot_mentioned",
    ] {
        if metadata.get(key).and_then(|value| value.as_bool()) == Some(true) {
            return true;
        }
    }
    false
}

pub fn channel_scope_from_metadata(
    metadata: &serde_json::Value,
    thread_overrides_channel: bool,
) -> Option<String> {
    let primary_keys: &[&str] = if thread_overrides_channel {
        &[
            "slack_thread_ts",
            "slack_channel",
            "telegram_chat_id",
            "discord_thread_id",
            "discord_channel_id",
            "google_chat_thread",
            "google_chat_space",
            "teams_conversation_id",
            "matrix_room_id",
            "whatsapp_chat_id",
            "line_room_id",
            "meta_thread_id",
        ]
    } else {
        &[
            "slack_channel",
            "telegram_chat_id",
            "discord_channel_id",
            "google_chat_space",
            "teams_conversation_id",
            "matrix_room_id",
            "whatsapp_chat_id",
            "line_room_id",
            "meta_thread_id",
        ]
    };

    for key in primary_keys {
        if let Some(value) = metadata.get(*key) {
            if let Some(text) = value.as_str() {
                return Some(format!("{}={}", key, text));
            }
            if let Some(number) = value.as_i64() {
                return Some(format!("{}={}", key, number));
            }
            if let Some(number) = value.as_u64() {
                return Some(format!("{}={}", key, number));
            }
        }
    }

    None
}

pub fn parent_channel_scope_from_metadata(metadata: &serde_json::Value) -> Option<String> {
    for key in ["discord_parent_channel_id", "slack_channel"] {
        if let Some(value) = metadata.get(key) {
            if let Some(text) = value.as_str() {
                return Some(format!("{}={}", key, text));
            }
            if let Some(number) = value.as_i64() {
                return Some(format!("{}={}", key, number));
            }
            if let Some(number) = value.as_u64() {
                return Some(format!("{}={}", key, number));
            }
        }
    }
    None
}

pub fn channel_scope_candidates(
    metadata: &serde_json::Value,
    thread_overrides_channel: bool,
) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(primary) = channel_scope_from_metadata(metadata, thread_overrides_channel) {
        candidates.push(primary);
    }
    if let Some(parent) = parent_channel_scope_from_metadata(metadata)
        && !candidates.iter().any(|existing| existing == &parent)
    {
        candidates.push(parent);
    }
    candidates
}

pub fn channel_route_key_with_binding(
    message: &IncomingMessage,
    direct_strategy: &str,
    group_strategy: &str,
    thread_overrides_channel: bool,
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
    account_id: Option<&str>,
) -> String {
    let mut prefix = vec![message.platform.to_string()];
    if let Some(workspace_id) = workspace_id {
        prefix.push(format!("workspace={workspace_id}"));
    }
    if let Some(agent_id) = agent_id {
        prefix.push(format!("agent={agent_id}"));
    }
    if let Some(account_id) = account_id {
        prefix.push(format!("account={account_id}"));
    }

    if let Some(scope) = channel_scope_from_metadata(&message.metadata, thread_overrides_channel) {
        if group_strategy == "shared_channel" {
            prefix.push(scope);
            prefix.push("shared".to_string());
            return prefix.join(":");
        }
        prefix.push(scope);
        prefix.push(message.user_id.clone());
        return prefix.join(":");
    }

    match direct_strategy {
        "shared_main" => {
            prefix.push("main".to_string());
            prefix.join(":")
        }
        _ => {
            prefix.push("direct".to_string());
            prefix.push(message.user_id.clone());
            prefix.join(":")
        }
    }
}

pub fn resolve_channel_binding<'a>(
    registry: &'a ChannelRegistry,
    platform: Platform,
    workspace_id: Option<&str>,
    account_id: Option<&str>,
    channel_scopes: &[String],
) -> Option<&'a ChannelBindingSpec> {
    registry
        .bindings
        .iter()
        .filter(|binding| binding.enabled && binding.platform == platform.to_string())
        .filter(|binding| {
            binding
                .workspace_match
                .as_deref()
                .map(|value| workspace_id == Some(value))
                .unwrap_or(true)
        })
        .filter(|binding| {
            binding
                .account_match
                .as_deref()
                .map(|value| account_id == Some(value))
                .unwrap_or(true)
        })
        .filter(|binding| {
            binding
                .channel_match
                .as_deref()
                .map(|value| channel_scopes.iter().any(|scope| scope == value))
                .unwrap_or(true)
        })
        .max_by_key(|binding| {
            let specificity = usize::from(binding.workspace_match.is_some())
                + usize::from(binding.account_match.is_some())
                + usize::from(binding.channel_match.is_some());
            (specificity, -(binding.priority as isize))
        })
}

pub fn default_send_policy(policy: &SessionRoutingConfig) -> ChannelSendPolicy {
    ChannelSendPolicy {
        mode: policy.default_send_mode.clone(),
        max_chunk_chars: policy.default_chunk_chars,
        chunk_delay_ms: policy.default_chunk_delay_ms,
        coalesce_below_chars: Some(320),
        preview_chars: 280,
    }
}

pub fn preview_route(
    registry: &mut ChannelRegistry,
    incoming: &IncomingMessage,
    policy: &SessionRoutingConfig,
) -> Result<ChannelRoutePreview> {
    let identity = identity_from_message(incoming);
    let account =
        ensure_account_manifest(&registry.root, &identity, policy.pairing_approval_required)?;
    registry
        .accounts
        .insert(account.id.clone(), account.clone());

    let channel_scopes =
        channel_scope_candidates(&incoming.metadata, policy.thread_overrides_channel);
    let binding = resolve_channel_binding(
        registry,
        incoming.platform,
        identity.workspace_id.as_deref(),
        Some(account.id.as_str()),
        &channel_scopes,
    );

    let direct_strategy = account
        .direct_strategy
        .clone()
        .or_else(|| binding.and_then(|value| value.direct_strategy.clone()))
        .unwrap_or_else(|| policy.direct_strategy.clone());
    let group_strategy = account
        .group_strategy
        .clone()
        .or_else(|| binding.and_then(|value| value.group_strategy.clone()))
        .unwrap_or_else(|| policy.group_strategy.clone());
    let activation_mode = account
        .activation_mode
        .clone()
        .or_else(|| binding.and_then(|value| value.activation_mode.clone()))
        .unwrap_or_else(|| policy.default_group_activation.clone());
    let send_policy = account
        .send_policy
        .clone()
        .or_else(|| binding.and_then(|value| value.send_policy.clone()))
        .unwrap_or_else(|| default_send_policy(policy));
    let workspace_id = account
        .workspace_target
        .clone()
        .or_else(|| binding.and_then(|value| value.workspace_target.clone()))
        .or_else(|| identity.workspace_id.clone());
    let agent_id = account
        .agent_id
        .clone()
        .or_else(|| binding.and_then(|value| value.agent_id.clone()));
    let route_key = channel_route_key_with_binding(
        incoming,
        &direct_strategy,
        &group_strategy,
        policy.thread_overrides_channel,
        workspace_id.as_deref(),
        agent_id.as_deref(),
        Some(account.id.as_str()),
    );

    let status = if account.blocked {
        ChannelRouteStatus::Blocked
    } else if !account.enabled {
        ChannelRouteStatus::Disabled
    } else if !account.approved {
        ChannelRouteStatus::PendingApproval
    } else {
        ChannelRouteStatus::Allowed
    };
    let should_respond =
        !identity.is_group || activation_mode != "mention" || identity.bot_mentioned;

    Ok(ChannelRoutePreview {
        status,
        route_key,
        workspace_id,
        agent_id,
        activation_mode,
        send_policy,
        account_id: account.id,
        binding_id: binding.map(|value| value.id.clone()),
        should_respond,
    })
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
    use tempfile::tempdir;

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

    #[test]
    fn message_identity_uses_signal_group_metadata() {
        let identity = identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "+15551234567".to_string(),
            content: "hello".to_string(),
            platform: Platform::Signal,
            metadata: serde_json::json!({
                "signal_group_id": "group-123",
                "signal_source_number": "+15551234567",
                "signal_is_group": true,
                "signal_bot_mentioned": true
            }),
        });

        assert_eq!(identity.account_id, "signal:group-123:+15551234567");
        assert_eq!(identity.workspace_id.as_deref(), Some("group-123"));
        assert_eq!(identity.channel_scope.as_deref(), Some("group-123"));
        assert!(identity.is_group);
        assert!(identity.bot_mentioned);
    }

    #[test]
    fn message_identity_uses_google_chat_thread_metadata() {
        let identity = identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "users/123".to_string(),
            content: "hello".to_string(),
            platform: Platform::GoogleChat,
            metadata: serde_json::json!({
                "google_chat_space": "spaces/AAA",
                "google_chat_thread": "spaces/AAA/threads/BBB",
                "google_chat_is_group": true,
                "google_chat_bot_mentioned": true
            }),
        });

        assert_eq!(identity.account_id, "google_chat:spaces/AAA:users/123");
        assert_eq!(identity.workspace_id.as_deref(), Some("spaces/AAA"));
        assert_eq!(
            identity.channel_scope.as_deref(),
            Some("spaces/AAA/threads/BBB")
        );
        assert!(identity.is_group);
        assert!(identity.bot_mentioned);
    }

    #[test]
    fn message_identity_uses_teams_and_matrix_routing_metadata() {
        let teams_identity = identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "29:user".to_string(),
            content: "hello".to_string(),
            platform: Platform::Teams,
            metadata: serde_json::json!({
                "teams_conversation_id": "19:conversation",
                "teams_is_group": true,
                "teams_bot_mentioned": false
            }),
        });
        assert_eq!(teams_identity.account_id, "teams:19:conversation:29:user");
        assert_eq!(
            teams_identity.workspace_id.as_deref(),
            Some("19:conversation")
        );
        assert_eq!(
            teams_identity.channel_scope.as_deref(),
            Some("19:conversation")
        );
        assert!(teams_identity.is_group);

        let matrix_identity = identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "@user:matrix.org".to_string(),
            content: "hello".to_string(),
            platform: Platform::Matrix,
            metadata: serde_json::json!({
                "matrix_room_id": "!room:matrix.org",
                "matrix_thread_root": "$event",
                "matrix_is_group": true
            }),
        });
        assert_eq!(
            matrix_identity.account_id,
            "matrix:!room:matrix.org:@user:matrix.org"
        );
        assert_eq!(
            matrix_identity.workspace_id.as_deref(),
            Some("!room:matrix.org")
        );
        assert_eq!(matrix_identity.channel_scope.as_deref(), Some("$event"));
        assert!(matrix_identity.is_group);
    }

    #[test]
    fn account_and_binding_crud_round_trip() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join(".claw/channels");
        let root_str = root.to_string_lossy().to_string();

        init(Some(&root_str)).expect("init registry");
        create_account(
            Some(&root_str),
            "acct-1",
            "slack",
            "U123",
            Some("Workspace Bot"),
            Some("T123"),
            Some("C123"),
            Some("workspace-a"),
            Some("agent-a"),
            true,
            false,
            true,
            Some("mention"),
        )
        .expect("create account");
        bind(
            Some(&root_str),
            "binding-1",
            "slack",
            Some("T123"),
            Some("acct-1"),
            Some("C123"),
            Some("workspace-a"),
            Some("agent-a"),
            Some("mention"),
        )
        .expect("create binding");

        let account = read_account(root.clone(), "acct-1").expect("read account");
        assert_eq!(account.external_user_id, "U123");
        assert!(account.approved);

        let binding = read_binding(root.clone(), "binding-1").expect("read binding");
        assert_eq!(binding.account_match.as_deref(), Some("acct-1"));

        delete_binding(Some(&root_str), "binding-1").expect("delete binding");
        delete_account(Some(&root_str), "acct-1").expect("delete account");

        assert!(read_binding(root.clone(), "binding-1").is_err());
        assert!(read_account(root, "acct-1").is_err());
    }
}
