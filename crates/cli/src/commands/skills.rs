//! Skill management commands.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use openrustclaw_core::types::SkillSource;
use openrustclaw_scheduler::DurableEventBus;
use openrustclaw_security::SkillVerifier;
use openrustclaw_security::sso::{OidcClient, OidcConfig, SsoClient};
use openrustclaw_skills::{
    ClawHubRegistry, CompiledBackgroundService, CompiledSkillArtifact, CompiledSkillManifest,
    ExtensionManifest, SearchFilters, SortBy, compile_skill_to_dir,
    compiled_skill_background_services, execute_compiled_skill_artifact, list_compiled_manifests,
    list_extension_manifests, load_compiled_artifact, load_extension_manifest,
    normalize_capability_names, remove_compiled_artifact,
    resolve_compiled_skill_background_service,
};

use super::channels::{
    ChannelBindingSpec, load_registry, read_binding, resolve_root, upsert_binding,
};
use super::{control, runtime, voice_runtime};

fn parse_hex_bytes(input: &str) -> Result<Vec<u8>> {
    let trimmed = input.trim();
    if trimmed.len() % 2 != 0 {
        anyhow::bail!("hex value must contain an even number of characters");
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    let chars: Vec<char> = trimmed.chars().collect();
    for pair in chars.chunks(2) {
        let hi = pair[0]
            .to_digit(16)
            .ok_or_else(|| anyhow::anyhow!("invalid hex character '{}'", pair[0]))?;
        let lo = pair[1]
            .to_digit(16)
            .ok_or_else(|| anyhow::anyhow!("invalid hex character '{}'", pair[1]))?;
        bytes.push(((hi << 4) | lo) as u8);
    }

    Ok(bytes)
}

fn load_configured_skill_verifier(
    config: &openrustclaw_core::config::AppConfig,
    required: bool,
) -> Result<SkillVerifier> {
    let Some(key_hex) = config.security.skill_verifying_key.as_deref() else {
        anyhow::bail!(
            "No skill verifying key configured. Set [security].skill_verifying_key to the Ed25519 public key hex before verifying external skills."
        );
    };

    let key_bytes = parse_hex_bytes(key_hex).context("Invalid skill verifying key hex")?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Skill verifying key must be exactly 32 bytes"))?;

    SkillVerifier::new(Some(&key_bytes), required)
        .context("Failed to construct skill verifier from configured key")
}

async fn publish_plugin_event(
    pool: &sqlx::SqlitePool,
    event_name: &str,
    payload: serde_json::Value,
) {
    let event_bus = DurableEventBus::new(pool.clone(), 64);
    if let Err(error) = event_bus
        .publish_named(event_name, "plugin_runtime_event", None, &payload, None)
        .await
    {
        tracing::warn!(error = %error, event_name, "Failed to publish plugin runtime event");
    }
}

fn resolve_workspace_skill_file(name: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new("skills").join(name).join("SKILL.md"),
        Path::new("skills").join(format!("{name}.md")),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn serialize_capabilities(capabilities: &[String]) -> Result<String> {
    let normalized = normalize_capability_names(capabilities)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to validate skill capabilities")?;
    serde_json::to_string(&normalized).context("Failed to serialize skill capabilities")
}

fn sensitive_capabilities(capabilities: &[String]) -> Result<Vec<String>> {
    let normalized = normalize_capability_names(capabilities)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to validate skill capabilities")?;
    let sensitive: HashSet<&str> = [
        "file_write",
        "network_access",
        "shell_exec",
        "database_access",
    ]
    .into_iter()
    .collect();
    Ok(normalized
        .into_iter()
        .filter(|capability| sensitive.contains(capability.as_str()))
        .collect())
}

fn enforce_external_skill_policy(
    config: &openrustclaw_core::config::AppConfig,
    capabilities: &[String],
    signature: Option<&str>,
    skill_name: &str,
) -> Result<()> {
    let sensitive = sensitive_capabilities(capabilities)?;

    if config.security.skill_signature_required && signature.is_none() {
        anyhow::bail!(
            "Skill '{}' is unsigned and [security].skill_signature_required is enabled",
            skill_name
        );
    }

    if !sensitive.is_empty() && signature.is_none() {
        anyhow::bail!(
            "Skill '{}' requests sensitive capabilities ({}) and must be signed before install",
            skill_name,
            sensitive.join(", ")
        );
    }

    Ok(())
}

async fn upsert_skill_record(
    pool: &sqlx::SqlitePool,
    name: &str,
    description: Option<&str>,
    source: &str,
    version: Option<&str>,
    signature: Option<&str>,
    verified: bool,
    capabilities_json: Option<&str>,
    schema: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO skills (
            id, name, description, source, version, signature,
            verified, enabled, capabilities, schema, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, datetime('now'), datetime('now'))
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            source = excluded.source,
            version = excluded.version,
            signature = excluded.signature,
            verified = excluded.verified,
            enabled = 1,
            capabilities = excluded.capabilities,
            schema = excluded.schema,
            updated_at = datetime('now')
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(name)
    .bind(description)
    .bind(source)
    .bind(version)
    .bind(signature)
    .bind(if verified { 1 } else { 0 })
    .bind(capabilities_json)
    .bind(schema)
    .execute(pool)
    .await
    .context("Failed to upsert skill record")?;

    Ok(())
}

async fn persist_verified_state(pool: &sqlx::SqlitePool, id: &str, verified: bool) -> Result<()> {
    sqlx::query("UPDATE skills SET verified = ? WHERE id = ?")
        .bind(if verified { 1 } else { 0 })
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to update skill verification state")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledSkillSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub verified: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub capabilities: Vec<String>,
    pub signature_present: bool,
    pub schema_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledSkillDetail {
    pub skill: InstalledSkillSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistrySkillSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySkillSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository: String,
    pub license: String,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub capabilities: Vec<String>,
    pub signed: bool,
    pub published_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillCatalogResult {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    pub skills: Vec<RegistrySkillSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillMutationResult {
    pub status: String,
    pub action: String,
    pub skill_name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<InstalledSkillDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillCompileResult {
    pub status: String,
    pub output_root: String,
    pub compiled_count: usize,
    pub blocked_count: usize,
    pub compiled: Vec<CompiledSkillManifest>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillInvokeOptions<'a> {
    pub args: Option<&'a str>,
    pub reference: Option<&'a str>,
    pub max_chars: Option<usize>,
    pub detail: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInvokeResult {
    pub status: String,
    pub skill_name: String,
    pub blocked: bool,
    pub detail: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_reference: Option<String>,
    pub command_preview: String,
    pub invocation: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillExecuteOptions<'a> {
    pub component: Option<&'a str>,
    pub input: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillExecuteResult {
    pub status: String,
    pub skill_name: String,
    pub component: String,
    pub source_path: String,
    pub verified_execution: bool,
    pub runtime: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillBackgroundServicesResult {
    pub status: String,
    pub skill_name: String,
    pub blocked: bool,
    pub services: Vec<CompiledBackgroundService>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillScheduleBackgroundOptions<'a> {
    pub service: Option<&'a str>,
    pub component: Option<&'a str>,
    pub input: Option<&'a str>,
    pub every_seconds: Option<u64>,
    pub at: Option<&'a str>,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillScheduleBackgroundResult {
    pub status: String,
    pub job_id: String,
    pub workflow_id: String,
    pub skill_name: String,
    pub service: String,
    pub component: String,
    pub compiled_root: String,
    pub trigger_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillChannelExtensionSummary {
    pub binding_id: String,
    pub platform: String,
    pub enabled: bool,
    pub trigger: String,
    pub skill_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_match: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillChannelExtensionsResult {
    pub status: String,
    pub count: usize,
    pub extensions: Vec<SkillChannelExtensionSummary>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillBindChannelExtensionOptions<'a> {
    pub service: Option<&'a str>,
    pub component: Option<&'a str>,
    pub trigger: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillBindChannelExtensionResult {
    pub status: String,
    pub binding_id: String,
    pub skill_name: String,
    pub trigger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillAuthPluginBinding {
    pub provider_id: String,
    pub skill_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    pub client_id_key: String,
    pub client_secret_key: String,
    pub scopes: Vec<String>,
    pub vault_key_prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub configured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillAuthPluginRegistry {
    pub bindings: BTreeMap<String, SkillAuthPluginBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillAuthPendingSession {
    pub provider_id: String,
    pub state: String,
    pub redirect_uri: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillAuthPendingSessions {
    pub sessions: Vec<SkillAuthPendingSession>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAuthPluginSummary {
    pub provider_id: String,
    pub skill_name: String,
    pub blocked: bool,
    pub declared_by_skill: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    pub client_id_key: String,
    pub client_secret_key: String,
    pub scopes: Vec<String>,
    pub vault_key_prefix: String,
    pub client_id_present: bool,
    pub client_secret_present: bool,
    pub access_token_present: bool,
    pub refresh_token_present: bool,
    pub pending_states: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAuthPluginsResult {
    pub status: String,
    pub count: usize,
    pub auth_plugins: Vec<SkillAuthPluginSummary>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillBindAuthPluginOptions<'a> {
    pub redirect_uri: Option<&'a str>,
    pub issuer: Option<&'a str>,
    pub authorization_endpoint: Option<&'a str>,
    pub token_endpoint: Option<&'a str>,
    pub client_id_key: Option<&'a str>,
    pub client_secret_key: Option<&'a str>,
    pub scopes: Option<&'a str>,
    pub vault_key_prefix: Option<&'a str>,
    pub service: Option<&'a str>,
    pub component: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillBindAuthPluginResult {
    pub status: String,
    pub provider_id: String,
    pub skill_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    pub client_id_key: String,
    pub client_secret_key: String,
    pub vault_key_prefix: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillAuthAuthorizeOptions<'a> {
    pub redirect_uri: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAuthAuthorizeResult {
    pub status: String,
    pub provider_id: String,
    pub skill_name: String,
    pub redirect_uri: String,
    pub state: String,
    pub authorization_url: String,
    pub vault_key_prefix: String,
    pub pending_states: usize,
}

#[derive(Debug, Clone)]
pub struct SkillAuthExchangeOptions<'a> {
    pub code: &'a str,
    pub state: &'a str,
    pub redirect_uri: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAuthExchangeResult {
    pub status: String,
    pub provider_id: String,
    pub skill_name: String,
    pub redirect_uri: String,
    pub stored_keys: Vec<String>,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillVoicePluginBinding {
    pub plugin_id: String,
    pub skill_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_voice: Option<String>,
    pub configured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillVoicePluginRegistry {
    pub bindings: BTreeMap<String, SkillVoicePluginBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVoiceCallRecord {
    pub call_id: String,
    pub plugin_id: String,
    pub skill_name: String,
    pub status: String,
    #[serde(default = "default_voice_call_health")]
    pub health: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_hook_output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_hook_output: Option<serde_json::Value>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
    #[serde(default = "default_voice_call_stale_after_secs")]
    pub stale_after_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SkillVoiceCallRegistry {
    pub calls: Vec<SkillVoiceCallRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillVoicePluginSummary {
    pub plugin_id: String,
    pub skill_name: String,
    pub blocked: bool,
    pub declared_by_skill: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_voice: Option<String>,
    pub active_calls: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillVoicePluginsResult {
    pub status: String,
    pub count: usize,
    pub voice_plugins: Vec<SkillVoicePluginSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillVoiceCallsResult {
    pub status: String,
    pub count: usize,
    pub calls: Vec<SkillVoiceCallRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillVoiceCallHealthSummary {
    pub total: usize,
    pub active: usize,
    pub stale: usize,
    pub ended: usize,
    pub reaped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_active_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillVoiceCallHealthResult {
    pub status: String,
    pub health: SkillVoiceCallHealthSummary,
}

#[derive(Debug, Clone, Default)]
pub struct SkillBindVoicePluginOptions<'a> {
    pub service: Option<&'a str>,
    pub component: Option<&'a str>,
    pub greeting_text: Option<&'a str>,
    pub default_voice: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillBindVoicePluginResult {
    pub status: String,
    pub plugin_id: String,
    pub skill_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_voice: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillStartVoiceCallOptions<'a> {
    pub remote: Option<&'a str>,
    pub greeting_text: Option<&'a str>,
    pub voice: Option<&'a str>,
    pub metadata: Option<&'a str>,
    pub stale_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillStartVoiceCallResult {
    pub status: String,
    pub call: SkillVoiceCallRecord,
}

#[derive(Debug, Clone, Default)]
pub struct SkillEndVoiceCallOptions<'a> {
    pub reason: Option<&'a str>,
    pub metadata: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillEndVoiceCallResult {
    pub status: String,
    pub call: SkillVoiceCallRecord,
}

#[derive(Debug, Clone, Default)]
pub struct SkillPrewarmVoicePluginOptions<'a> {
    pub greeting_text: Option<&'a str>,
    pub voice: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillPrewarmVoicePluginResult {
    pub status: String,
    pub plugin_id: String,
    pub skill_name: String,
    pub greeting_text: String,
    pub voice: String,
    pub output_path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SkillReapVoiceCallsOptions {
    pub stale_after_secs: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillReapVoiceCallsResult {
    pub status: String,
    pub checked: usize,
    pub reaped_count: usize,
    pub calls: Vec<SkillVoiceCallRecord>,
}

async fn load_skill_config_and_pool()
-> Result<(openrustclaw_core::config::AppConfig, sqlx::SqlitePool)> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    Ok((config, pool))
}

fn skill_registry_endpoint(config: &openrustclaw_core::config::AppConfig) -> String {
    config
        .skills
        .as_ref()
        .and_then(|skills| skills.registry_url.clone())
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string())
}

fn compiled_skill_root() -> PathBuf {
    Path::new(".claw").join("skills").join("compiled")
}

fn source_from_string(source: &str) -> SkillSource {
    match source {
        "bundled" => SkillSource::Bundled,
        "managed" => SkillSource::Managed,
        "marketplace" => SkillSource::Marketplace,
        _ => SkillSource::Workspace,
    }
}

fn default_voice_call_stale_after_secs() -> u64 {
    900
}

fn default_voice_call_health() -> String {
    "active".to_string()
}

fn ensure_compiled_root() -> Result<PathBuf> {
    let root = compiled_skill_root();
    std::fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create {}", root.display()))?;
    Ok(root)
}

fn current_workspace_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current workspace root")
}

fn auth_plugin_registry_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root).join("skill-auth-plugins.json")
}

fn auth_plugin_sessions_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root).join("skill-auth-plugin-sessions.json")
}

fn voice_plugin_registry_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root).join("skill-voice-plugins.json")
}

fn voice_call_registry_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root).join("skill-voice-calls.json")
}

fn load_auth_plugin_registry(workspace_root: &Path) -> Result<SkillAuthPluginRegistry> {
    let path = auth_plugin_registry_path(workspace_root);
    if !path.exists() {
        return Ok(SkillAuthPluginRegistry::default());
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_auth_plugin_registry(
    workspace_root: &Path,
    registry: &SkillAuthPluginRegistry,
) -> Result<()> {
    let path = auth_plugin_registry_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(registry)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn load_auth_plugin_sessions(workspace_root: &Path) -> Result<SkillAuthPendingSessions> {
    let path = auth_plugin_sessions_path(workspace_root);
    if !path.exists() {
        return Ok(SkillAuthPendingSessions::default());
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_auth_plugin_sessions(
    workspace_root: &Path,
    sessions: &SkillAuthPendingSessions,
) -> Result<()> {
    let path = auth_plugin_sessions_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(sessions)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn sanitize_env_fragment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let collapsed = sanitized
        .split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if collapsed.is_empty() {
        "SKILL".to_string()
    } else {
        collapsed
    }
}

fn default_auth_prefix(provider_id: &str) -> String {
    format!("AUTH_PLUGIN_{}", sanitize_env_fragment(provider_id))
}

fn auth_scope_list(raw: Option<&str>) -> Vec<String> {
    let scopes = raw
        .map(|value| {
            value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .filter(|segment| !segment.trim().is_empty())
                .map(|segment| segment.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if scopes.is_empty() {
        vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ]
    } else {
        scopes
    }
}

fn access_token_key(binding: &SkillAuthPluginBinding) -> String {
    format!("{}_ACCESS_TOKEN", binding.vault_key_prefix)
}

fn refresh_token_key(binding: &SkillAuthPluginBinding) -> String {
    format!("{}_REFRESH_TOKEN", binding.vault_key_prefix)
}

fn id_token_key(binding: &SkillAuthPluginBinding) -> String {
    format!("{}_ID_TOKEN", binding.vault_key_prefix)
}

fn expires_in_key(binding: &SkillAuthPluginBinding) -> String {
    format!("{}_EXPIRES_IN", binding.vault_key_prefix)
}

fn scope_key(binding: &SkillAuthPluginBinding) -> String {
    format!("{}_SCOPE", binding.vault_key_prefix)
}

fn provider_from_pending_state(sessions: &SkillAuthPendingSessions, state: &str) -> Option<String> {
    sessions
        .sessions
        .iter()
        .find(|session| session.state == state)
        .map(|session| session.provider_id.clone())
}

fn load_voice_plugin_registry(workspace_root: &Path) -> Result<SkillVoicePluginRegistry> {
    let path = voice_plugin_registry_path(workspace_root);
    if !path.exists() {
        return Ok(SkillVoicePluginRegistry::default());
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_voice_plugin_registry(
    workspace_root: &Path,
    registry: &SkillVoicePluginRegistry,
) -> Result<()> {
    let path = voice_plugin_registry_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(registry)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn load_voice_call_registry(workspace_root: &Path) -> Result<SkillVoiceCallRegistry> {
    let path = voice_call_registry_path(workspace_root);
    if !path.exists() {
        return Ok(SkillVoiceCallRegistry::default());
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_voice_call_registry(
    workspace_root: &Path,
    registry: &SkillVoiceCallRegistry,
) -> Result<()> {
    let path = voice_call_registry_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(registry)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

async fn build_registry_client(
    config: &openrustclaw_core::config::AppConfig,
) -> Result<ClawHubRegistry> {
    ClawHubRegistry::new(skill_registry_endpoint(config))
        .await
        .context("Failed to connect to ClawHub registry")
}

fn parse_capabilities_json(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .unwrap_or_default()
}

fn registry_skill_summary(metadata: openrustclaw_skills::SkillMetadata) -> RegistrySkillSummary {
    RegistrySkillSummary {
        name: metadata.name,
        version: metadata.version.to_string(),
        description: metadata.description,
        author: metadata.author,
        repository: metadata.repository,
        license: metadata.license,
        downloads: metadata.downloads,
        rating: metadata.rating,
        rating_count: metadata.rating_count,
        categories: metadata.categories,
        keywords: metadata.keywords,
        capabilities: metadata.capabilities,
        signed: metadata.signature.is_some(),
        published_at: metadata.published_at.to_rfc3339(),
        updated_at: metadata.updated_at.to_rfc3339(),
    }
}

async fn registry_install_paths(
    config: &openrustclaw_core::config::AppConfig,
) -> HashSet<(String, String)> {
    match build_registry_client(config).await {
        Ok(registry) => match registry.list_installed().await {
            Ok(skills) => skills
                .into_iter()
                .map(|skill| {
                    (
                        skill.name,
                        skill.install_path.join("SKILL.md").display().to_string(),
                    )
                })
                .collect(),
            Err(error) => {
                debug!(error = %error, "Failed to read installed registry skills");
                HashSet::new()
            }
        },
        Err(error) => {
            debug!(error = %error, "Failed to build registry client for install-path lookup");
            HashSet::new()
        }
    }
}

fn resolve_skill_local_path(
    name: &str,
    source: &str,
    registry_paths: &HashSet<(String, String)>,
) -> Option<String> {
    if source == "workspace" || source == "bundled" {
        return resolve_workspace_skill_file(name).map(|path| path.display().to_string());
    }

    registry_paths
        .iter()
        .find_map(|(skill_name, path)| (skill_name == name).then(|| path.clone()))
        .or_else(|| {
            let candidate = Path::new("skills").join(name).join("SKILL.md");
            candidate.exists().then(|| candidate.display().to_string())
        })
}

async fn list_installed_rows(
    pool: &sqlx::SqlitePool,
    config: &openrustclaw_core::config::AppConfig,
) -> Result<Vec<InstalledSkillSummary>> {
    let rows = sqlx::query(
        r#"
        SELECT
            name,
            description,
            source,
            version,
            verified,
            enabled,
            created_at,
            capabilities,
            signature,
            schema
        FROM skills
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to query skills from database")?;

    let registry_paths = registry_install_paths(config).await;
    let mut skills = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row.get("name");
        let source: String = row.get("source");
        let capabilities_json: Option<String> = row.get("capabilities");
        let signature: Option<String> = row.get("signature");
        let schema: Option<String> = row.get("schema");
        skills.push(InstalledSkillSummary {
            name: name.clone(),
            description: row.get("description"),
            source: source.clone(),
            version: row.get("version"),
            verified: row.get::<i64, _>("verified") == 1,
            enabled: row.get::<i64, _>("enabled") == 1,
            created_at: row.get("created_at"),
            capabilities: parse_capabilities_json(capabilities_json.as_deref()),
            signature_present: signature.is_some(),
            schema_present: schema
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()),
            local_path: resolve_skill_local_path(&name, &source, &registry_paths),
        });
    }

    Ok(skills)
}

pub async fn installed_skills_data() -> Result<Vec<InstalledSkillSummary>> {
    let (config, pool) = load_skill_config_and_pool().await?;
    list_installed_rows(&pool, &config).await
}

pub async fn installed_skill_detail_data(name: &str) -> Result<InstalledSkillDetail> {
    let (config, pool) = load_skill_config_and_pool().await?;
    let registry_paths = registry_install_paths(&config).await;
    let row = sqlx::query(
        r#"
        SELECT
            name,
            description,
            source,
            version,
            verified,
            enabled,
            created_at,
            capabilities,
            signature,
            schema
        FROM skills
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(&pool)
    .await
    .context("Failed to query skill detail from database")?;

    let Some(row) = row else {
        anyhow::bail!("Skill '{}' not found", name);
    };

    let skill_name: String = row.get("name");
    let source: String = row.get("source");
    let capabilities_json: Option<String> = row.get("capabilities");
    let signature: Option<String> = row.get("signature");
    let schema: Option<String> = row.get("schema");
    let registry = if source == "marketplace" {
        match build_registry_client(&config).await {
            Ok(client) => match client.get_skill(&skill_name).await {
                Ok(metadata) => Some(registry_skill_summary(metadata)),
                Err(error) => {
                    debug!(error = %error, skill = %skill_name, "Failed to load registry metadata for installed skill");
                    None
                }
            },
            Err(error) => {
                debug!(error = %error, "Failed to build registry client for skill inspection");
                None
            }
        }
    } else {
        None
    };

    Ok(InstalledSkillDetail {
        skill: InstalledSkillSummary {
            name: skill_name.clone(),
            description: row.get("description"),
            source: source.clone(),
            version: row.get("version"),
            verified: row.get::<i64, _>("verified") == 1,
            enabled: row.get::<i64, _>("enabled") == 1,
            created_at: row.get("created_at"),
            capabilities: parse_capabilities_json(capabilities_json.as_deref()),
            signature_present: signature.is_some(),
            schema_present: schema
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty()),
            local_path: resolve_skill_local_path(&skill_name, &source, &registry_paths),
        },
        schema,
        signature,
        registry,
    })
}

fn discover_workspace_skill_paths() -> BTreeMap<String, (PathBuf, SkillSource, bool)> {
    let mut discovered = BTreeMap::new();
    let skills_root = Path::new("skills");
    if !skills_root.exists() {
        return discovered;
    }

    if let Ok(entries) = std::fs::read_dir(skills_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let skill_file = if path.is_dir() {
                let candidate = path.join("SKILL.md");
                candidate.exists().then_some(candidate)
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                Some(path.clone())
            } else {
                None
            };

            if let Some(skill_file) = skill_file {
                let name = if path.is_dir() {
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                };
                discovered.insert(name, (skill_file, SkillSource::Workspace, true));
            }
        }
    }

    discovered
}

async fn compile_skill_target(
    name: &str,
    path: &Path,
    source: SkillSource,
    verified: bool,
) -> Result<CompiledSkillArtifact> {
    let root = ensure_compiled_root()?;
    compile_skill_to_dir(path, &root, source, verified)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .with_context(|| format!("Failed to compile skill '{}'", name))
}

async fn compile_skill_by_name_internal(name: &str) -> Result<CompiledSkillArtifact> {
    if let Some(path) = resolve_workspace_skill_file(name) {
        return compile_skill_target(name, &path, SkillSource::Workspace, true).await;
    }

    if let Ok(detail) = installed_skill_detail_data(name).await
        && let Some(local_path) = detail.skill.local_path.clone()
    {
        let source = source_from_string(&detail.skill.source);
        return compile_skill_target(name, Path::new(&local_path), source, detail.skill.verified)
            .await;
    }

    anyhow::bail!("No local skill file found for '{}'", name)
}

pub async fn compiled_skills_data() -> Result<Vec<CompiledSkillManifest>> {
    let root = compiled_skill_root();
    list_compiled_manifests(&root)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("Failed to load compiled skill manifests")
}

pub async fn compiled_skill_detail_data(name: &str) -> Result<CompiledSkillArtifact> {
    let root = compiled_skill_root();
    load_compiled_artifact(&root, name)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .with_context(|| format!("Failed to load compiled skill '{}'", name))
}

pub async fn extension_manifests_data() -> Result<Vec<ExtensionManifest>> {
    let root = compiled_skill_root();
    list_extension_manifests(&root)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("Failed to load compiled extension manifests")
}

pub async fn extension_manifest_data(name: &str) -> Result<ExtensionManifest> {
    let root = compiled_skill_root();
    match load_extension_manifest(&root, name) {
        Ok(manifest) => Ok(manifest),
        Err(_) => {
            let _ = compile_skill_by_name_internal(name).await?;
            load_extension_manifest(&root, name)
                .map_err(|error| anyhow::anyhow!(error.to_string()))
                .with_context(|| format!("Failed to load extension manifest for '{}'", name))
        }
    }
}

async fn compiled_skill_detail_or_compile(name: &str) -> Result<CompiledSkillArtifact> {
    match compiled_skill_detail_data(name).await {
        Ok(artifact) => Ok(artifact),
        Err(_) => compile_skill_by_name_internal(name).await,
    }
}

fn read_compiled_skill_reference(
    artifact: &CompiledSkillArtifact,
    reference: &str,
    max_chars: Option<usize>,
) -> Result<serde_json::Value> {
    if !artifact
        .manifest
        .references
        .iter()
        .any(|entry| entry == reference)
    {
        anyhow::bail!(
            "Reference '{}' is not part of compiled skill '{}'",
            reference,
            artifact.manifest.name
        );
    }

    let canonical_candidate = resolve_skill_relative_path(artifact, reference)?;

    let bytes = fs::read(&canonical_candidate)
        .with_context(|| format!("Failed to read {}", canonical_candidate.display()))?;
    let metadata = fs::metadata(&canonical_candidate)
        .with_context(|| format!("Failed to stat {}", canonical_candidate.display()))?;
    let max_chars = max_chars.unwrap_or(4000).max(1);
    match String::from_utf8(bytes) {
        Ok(text) => {
            let char_len = text.chars().count();
            let truncated = char_len > max_chars;
            let content = if truncated {
                text.chars().take(max_chars).collect::<String>()
            } else {
                text
            };
            Ok(serde_json::json!({
                "reference": reference,
                "path": canonical_candidate.display().to_string(),
                "binary": false,
                "bytes": metadata.len(),
                "truncated": truncated,
                "content": content,
            }))
        }
        Err(error) => Ok(serde_json::json!({
            "reference": reference,
            "path": canonical_candidate.display().to_string(),
            "binary": true,
            "bytes": metadata.len(),
            "encoding_error": error.to_string(),
        })),
    }
}

fn resolve_skill_relative_path(
    artifact: &CompiledSkillArtifact,
    relative: &str,
) -> Result<PathBuf> {
    let skill_file = PathBuf::from(&artifact.manifest.local_path);
    let skill_root = skill_file.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Compiled skill '{}' does not have a resolvable root",
            artifact.manifest.name
        )
    })?;
    let canonical_root = skill_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", skill_root.display()))?;
    let candidate = skill_root.join(relative);
    let canonical_candidate = candidate
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", candidate.display()))?;
    if !canonical_candidate.starts_with(&canonical_root) {
        anyhow::bail!(
            "Path '{}' escapes the skill root for '{}'",
            relative,
            artifact.manifest.name
        );
    }
    Ok(canonical_candidate)
}

pub async fn invoke_data(name: &str, options: SkillInvokeOptions<'_>) -> Result<SkillInvokeResult> {
    let artifact = compiled_skill_detail_or_compile(name).await?;
    invoke_compiled_skill(&artifact, options)
}

pub async fn execute_data(
    name: &str,
    options: SkillExecuteOptions<'_>,
) -> Result<SkillExecuteResult> {
    let artifact = compiled_skill_detail_or_compile(name).await?;
    execute_compiled_artifact_data(&artifact, options).await
}

pub async fn background_services_data(name: &str) -> Result<SkillBackgroundServicesResult> {
    let artifact = compiled_skill_detail_or_compile(name).await?;
    Ok(SkillBackgroundServicesResult {
        status: "ok".to_string(),
        skill_name: artifact.manifest.name.clone(),
        blocked: matches!(
            artifact.manifest.status,
            openrustclaw_skills::CompiledSkillStatus::Blocked
        ),
        services: compiled_skill_background_services(&artifact),
    })
}

pub async fn execute_compiled_artifact_data(
    artifact: &CompiledSkillArtifact,
    options: SkillExecuteOptions<'_>,
) -> Result<SkillExecuteResult> {
    execute_compiled_skill(artifact, options).await
}

fn invoke_compiled_skill(
    artifact: &CompiledSkillArtifact,
    options: SkillInvokeOptions<'_>,
) -> Result<SkillInvokeResult> {
    let blocked = matches!(
        artifact.manifest.status,
        openrustclaw_skills::CompiledSkillStatus::Blocked
    );
    let requested_args = options.args.map(str::to_string);
    let requested_reference = options.reference.map(str::to_string);
    let command_preview = match requested_args.as_deref() {
        Some(args) if !args.trim().is_empty() => {
            format!("{} {}", artifact.cli_schema.command, args.trim())
        }
        _ => artifact.cli_schema.usage.clone(),
    };

    let invocation = serde_json::json!({
        "manifest": &artifact.manifest,
        "help": {
            "summary": &artifact.help_index.summary,
            "body_excerpt": if blocked { serde_json::Value::Null } else if options.detail { serde_json::json!(&artifact.help_index.body_excerpt) } else { serde_json::Value::Null },
            "argument_hint": &artifact.help_index.argument_hint,
            "allowed_tools": &artifact.help_index.allowed_tools,
            "capabilities": &artifact.help_index.capabilities,
            "scripts": &artifact.help_index.scripts,
            "references": &artifact.help_index.references,
            "examples": if blocked || !options.detail { serde_json::json!([]) } else { serde_json::json!(&artifact.help_index.examples) },
            "safety_notes": &artifact.help_index.safety_notes,
        },
        "cli": &artifact.cli_schema,
        "mcp": &artifact.mcp_schema,
        "scan_report": &artifact.scan_report,
    });

    let reference_result = match options.reference {
        Some(reference) if blocked => {
            anyhow::bail!(
                "Compiled skill '{}' is blocked; reference access is disabled",
                artifact.manifest.name
            );
        }
        Some(reference) => Some(read_compiled_skill_reference(
            &artifact,
            reference,
            options.max_chars,
        )?),
        None => None,
    };

    let guidance = if blocked {
        Some(format!(
            "Skill '{}' is blocked by the compile scanner and remains inspection-only.",
            artifact.manifest.name
        ))
    } else if artifact.help_index.scripts.is_empty() {
        Some(format!(
            "Skill '{}' is currently exposed through compiled help/reference surfaces. Executable plugin runtime parity is still a later Phase 7 track.",
            artifact.manifest.name
        ))
    } else {
        Some(format!(
            "Skill '{}' exposes {} script entr{} through compiled metadata. This invoke surface currently returns the cached command/help bundle instead of executing those scripts directly.",
            artifact.manifest.name,
            artifact.help_index.scripts.len(),
            if artifact.help_index.scripts.len() == 1 {
                "y"
            } else {
                "ies"
            }
        ))
    };

    Ok(SkillInvokeResult {
        status: "ok".to_string(),
        skill_name: artifact.manifest.name.clone(),
        blocked,
        detail: options.detail,
        requested_args,
        requested_reference,
        command_preview,
        invocation,
        reference_result,
        guidance,
    })
}

async fn execute_compiled_skill(
    artifact: &CompiledSkillArtifact,
    options: SkillExecuteOptions<'_>,
) -> Result<SkillExecuteResult> {
    let input = match options.input {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw).with_context(|| {
            format!(
                "Failed to parse JSON input for '{}'",
                artifact.manifest.name
            )
        })?,
        _ => serde_json::json!({}),
    };
    let execution = execute_compiled_skill_artifact(artifact, options.component, input)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(SkillExecuteResult {
        status: "ok".to_string(),
        skill_name: execution.skill_name,
        component: execution.component,
        source_path: execution.source_path,
        verified_execution: execution.verified_execution,
        runtime: execution.runtime,
        input: execution.input,
        output: execution.output,
        capabilities: execution.capabilities,
    })
}

fn background_trigger(
    every_seconds: Option<u64>,
    at: Option<&str>,
) -> Result<(String, serde_json::Value, Option<DateTime<Utc>>)> {
    if every_seconds.is_some() && at.is_some() {
        anyhow::bail!("Use either --every-seconds or --at, not both");
    }

    if let Some(run_at) = at {
        let run_at = DateTime::parse_from_rfc3339(run_at)
            .with_context(|| format!("Invalid RFC3339 timestamp for --at: {}", run_at))?
            .with_timezone(&Utc);
        Ok((
            "absolute".to_string(),
            serde_json::json!({
                "type": "absolute",
                "run_at": run_at.to_rfc3339(),
            }),
            Some(run_at),
        ))
    } else {
        let every_seconds = every_seconds.unwrap_or(3600);
        Ok((
            "interval".to_string(),
            serde_json::json!({
                "type": "interval",
                "interval_secs": every_seconds,
            }),
            Some(Utc::now() + chrono::Duration::seconds(every_seconds as i64)),
        ))
    }
}

pub async fn schedule_background_service_data(
    name: &str,
    options: SkillScheduleBackgroundOptions<'_>,
) -> Result<SkillScheduleBackgroundResult> {
    let artifact = compiled_skill_detail_or_compile(name).await?;
    let resolved =
        resolve_compiled_skill_background_service(&artifact, options.service, options.component)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let component = resolved.component.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Background service '{}' for '{}' does not resolve to an executable component",
            resolved.name,
            artifact.manifest.name
        )
    })?;

    let (config, pool) = load_skill_config_and_pool().await?;
    let compiled_root = ensure_compiled_root()?;
    let (trigger_type, trigger_config, next_run_at) =
        background_trigger(options.every_seconds, options.at)?;
    let skill_name = artifact.manifest.name.clone();
    let service_name = resolved.name.clone();
    let skill_input = match options.input {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str::<serde_json::Value>(raw)
            .with_context(|| format!("Invalid JSON for --input: {}", raw))?,
        _ => serde_json::json!({}),
    };

    let job_id = uuid::Uuid::new_v4().to_string();
    let idempotency_key = format!("{}:{}", job_id, uuid::Uuid::new_v4());
    let job_name = format!(
        "skill-bg-{}-{}",
        skill_name.replace(' ', "-").to_lowercase(),
        service_name.replace(' ', "-").to_lowercase()
    );
    let metadata = serde_json::json!({
        "input": {
            "compiled_root": compiled_root.display().to_string(),
            "skill_name": &skill_name,
            "service": &service_name,
            "component": &component,
            "skill_input": skill_input,
        },
        "workflow_metadata": {
            "skill_name": &skill_name,
            "background_service": &service_name,
            "component": &component,
            "source_kind": "plugin_background_workflow",
            "workspace_database": config.database.url,
        },
        "task": {
            "priority": options.priority,
            "source_kind": "plugin_background_workflow",
            "owner": "skills",
            "tags": ["skill", "background-service", &service_name],
        }
    });

    sqlx::query(
        r#"
        INSERT INTO scheduled_jobs (
            id, name, description, workflow_id, trigger_type, trigger_config,
            idempotency_key, state, timezone, max_retries, priority, source_kind, owner,
            tags, next_run_at, run_count, metadata, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 'UTC', 3, ?, 'skills', ?, ?, ?, 0, ?, datetime('now'))
        "#,
    )
    .bind(&job_id)
    .bind(&job_name)
    .bind(format!(
        "Compiled skill background workflow for {}:{}",
        skill_name, service_name
    ))
    .bind("skill_background")
    .bind(&trigger_type)
    .bind(trigger_config.to_string())
    .bind(idempotency_key)
    .bind(options.priority)
    .bind("skills")
    .bind(serde_json::to_string(&vec![
        "skill".to_string(),
        "background-service".to_string(),
        service_name.clone(),
    ])?)
    .bind(next_run_at.map(|value| value.to_rfc3339()))
    .bind(metadata.to_string())
    .execute(&pool)
    .await
    .context("Failed to create background skill workflow job")?;

    publish_plugin_event(
        &pool,
        "plugin.background_workflow_scheduled",
        serde_json::json!({
            "job_id": &job_id,
            "skill_name": &skill_name,
            "service": &service_name,
            "component": &component,
            "trigger_type": &trigger_type,
        }),
    )
    .await;

    Ok(SkillScheduleBackgroundResult {
        status: "ok".to_string(),
        job_id,
        workflow_id: "skill_background".to_string(),
        skill_name,
        service: service_name,
        component,
        compiled_root: compiled_root.display().to_string(),
        trigger_type,
        next_run_at: next_run_at.map(|value| value.to_rfc3339()),
    })
}

fn channel_extension_summary(binding: &ChannelBindingSpec) -> Option<SkillChannelExtensionSummary> {
    let extension = binding
        .metadata
        .get("skill_channel_extension")?
        .as_object()?;
    let trigger = extension
        .get("trigger")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("message")
        .to_string();
    let skill_name = extension
        .get("skill_name")
        .and_then(serde_json::Value::as_str)?
        .to_string();

    Some(SkillChannelExtensionSummary {
        binding_id: binding.id.clone(),
        platform: binding.platform.clone(),
        enabled: binding.enabled,
        trigger,
        skill_name,
        service: extension
            .get("service")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        component: extension
            .get("component")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        workspace_match: binding.workspace_match.clone(),
        account_match: binding.account_match.clone(),
        channel_match: binding.channel_match.clone(),
    })
}

pub async fn channel_extensions_data() -> Result<SkillChannelExtensionsResult> {
    let root = resolve_root(None)?;
    let registry = load_registry(root)?;
    let mut extensions = registry
        .bindings
        .iter()
        .filter_map(channel_extension_summary)
        .collect::<Vec<_>>();
    extensions.sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
    Ok(SkillChannelExtensionsResult {
        status: "ok".to_string(),
        count: extensions.len(),
        extensions,
    })
}

pub async fn bind_channel_extension_data(
    binding_id: &str,
    skill_name: &str,
    options: SkillBindChannelExtensionOptions<'_>,
) -> Result<SkillBindChannelExtensionResult> {
    let root = resolve_root(None)?;
    let mut binding = read_binding(root.clone(), binding_id)?;
    let artifact = compiled_skill_detail_or_compile(skill_name).await?;
    let resolved =
        resolve_compiled_skill_background_service(&artifact, options.service, options.component)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let component = resolved.component.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Channel extension service '{}' for '{}' has no executable component mapping",
            resolved.name,
            artifact.manifest.name
        )
    })?;
    let trigger = options.trigger.unwrap_or("message");
    if !matches!(trigger, "message" | "mentioned") {
        anyhow::bail!(
            "Unsupported trigger '{}'; use `message` or `mentioned`",
            trigger
        );
    }

    binding.metadata["skill_channel_extension"] = serde_json::json!({
        "skill_name": artifact.manifest.name,
        "service": resolved.name,
        "component": component,
        "trigger": trigger,
        "mode": "background_schedule",
    });
    upsert_binding(None, binding.clone())?;

    let (_, pool) = load_skill_config_and_pool().await?;
    publish_plugin_event(
        &pool,
        "plugin.channel_extension_bound",
        serde_json::json!({
            "binding_id": binding.id,
            "platform": binding.platform,
            "skill_name": artifact.manifest.name,
            "service": resolved.name,
            "component": component,
            "trigger": trigger,
        }),
    )
    .await;

    Ok(SkillBindChannelExtensionResult {
        status: "ok".to_string(),
        binding_id: binding.id,
        skill_name: artifact.manifest.name,
        trigger: trigger.to_string(),
        service: Some(resolved.name),
        component: Some(component),
    })
}

fn auth_plugin_summary(
    binding: &SkillAuthPluginBinding,
    artifact: Option<&CompiledSkillArtifact>,
    vault_keys: &HashSet<String>,
    pending_states: usize,
) -> SkillAuthPluginSummary {
    let declared_by_skill = artifact.is_some_and(|artifact| {
        artifact
            .manifest
            .declared_auth_providers
            .iter()
            .any(|provider| provider == &binding.provider_id)
    });
    let blocked = artifact.is_some_and(|artifact| {
        matches!(
            artifact.manifest.status,
            openrustclaw_skills::CompiledSkillStatus::Blocked
        )
    });

    SkillAuthPluginSummary {
        provider_id: binding.provider_id.clone(),
        skill_name: binding.skill_name.clone(),
        blocked,
        declared_by_skill,
        redirect_uri: binding.redirect_uri.clone(),
        issuer: binding.issuer.clone(),
        authorization_endpoint: binding.authorization_endpoint.clone(),
        token_endpoint: binding.token_endpoint.clone(),
        client_id_key: binding.client_id_key.clone(),
        client_secret_key: binding.client_secret_key.clone(),
        scopes: binding.scopes.clone(),
        vault_key_prefix: binding.vault_key_prefix.clone(),
        client_id_present: vault_keys.contains(&binding.client_id_key),
        client_secret_present: vault_keys.contains(&binding.client_secret_key),
        access_token_present: vault_keys.contains(&access_token_key(binding)),
        refresh_token_present: vault_keys.contains(&refresh_token_key(binding)),
        pending_states,
        service: binding.service.clone(),
        component: binding.component.clone(),
        updated_at: binding.updated_at.clone(),
    }
}

async fn hydrated_auth_binding(
    workspace_root: &Path,
    provider_id: &str,
) -> Result<(SkillAuthPluginBinding, OidcClient)> {
    let mut registry = load_auth_plugin_registry(workspace_root)?;
    let mut binding = registry
        .bindings
        .get(provider_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Auth plugin '{}' not found", provider_id))?;
    let issuer = binding.issuer.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Auth plugin '{}' is missing an issuer URL; re-bind with --issuer",
            provider_id
        )
    })?;

    let force_discovery =
        binding.authorization_endpoint.is_none() || binding.token_endpoint.is_none();
    let mut config = OidcConfig::new(
        provider_id,
        runtime::get_vault_secret(workspace_root, &binding.client_id_key)?,
        runtime::get_vault_secret(workspace_root, &binding.client_secret_key)?,
        issuer,
    );
    config.scopes = binding.scopes.clone();
    if !force_discovery {
        config.authorization_endpoint = binding.authorization_endpoint.clone();
        config.token_endpoint = binding.token_endpoint.clone();
    }

    let mut client = OidcClient::new(config);
    if force_discovery {
        client
            .discover_metadata()
            .await
            .map_err(|error| anyhow::anyhow!("OIDC discovery failed: {}", error))?;
        let metadata = client.metadata().clone();
        binding.authorization_endpoint = Some(metadata.authorization_endpoint);
        binding.token_endpoint = Some(metadata.token_endpoint);
        binding.updated_at = Utc::now().to_rfc3339();
        registry
            .bindings
            .insert(provider_id.to_string(), binding.clone());
        save_auth_plugin_registry(workspace_root, &registry)?;
    }

    Ok((binding, client))
}

pub async fn auth_plugins_data() -> Result<SkillAuthPluginsResult> {
    let workspace_root = current_workspace_root()?;
    let registry = load_auth_plugin_registry(&workspace_root)?;
    let sessions = load_auth_plugin_sessions(&workspace_root)?;
    let vault_keys = runtime::list_vault_keys(&workspace_root)
        .unwrap_or_default()
        .into_iter()
        .collect::<HashSet<_>>();
    let mut auth_plugins = Vec::new();

    for binding in registry.bindings.values() {
        let artifact = compiled_skill_detail_or_compile(&binding.skill_name)
            .await
            .ok();
        let pending_count = sessions
            .sessions
            .iter()
            .filter(|session| session.provider_id == binding.provider_id)
            .count();
        auth_plugins.push(auth_plugin_summary(
            binding,
            artifact.as_ref(),
            &vault_keys,
            pending_count,
        ));
    }

    auth_plugins.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
    Ok(SkillAuthPluginsResult {
        status: "ok".to_string(),
        count: auth_plugins.len(),
        auth_plugins,
    })
}

pub async fn auth_plugins_for_skill_data(skill_name: &str) -> Result<SkillAuthPluginsResult> {
    let mut result = auth_plugins_data().await?;
    result
        .auth_plugins
        .retain(|binding| binding.skill_name == skill_name);
    result.count = result.auth_plugins.len();
    Ok(result)
}

pub async fn bind_auth_plugin_data(
    provider_id: &str,
    skill_name: &str,
    options: SkillBindAuthPluginOptions<'_>,
) -> Result<SkillBindAuthPluginResult> {
    let workspace_root = current_workspace_root()?;
    let artifact = compiled_skill_detail_or_compile(skill_name).await?;
    if matches!(
        artifact.manifest.status,
        openrustclaw_skills::CompiledSkillStatus::Blocked
    ) {
        anyhow::bail!(
            "Compiled skill '{}' is blocked and cannot be bound as an auth plugin",
            artifact.manifest.name
        );
    }
    if !artifact.manifest.declared_auth_providers.is_empty()
        && !artifact
            .manifest
            .declared_auth_providers
            .iter()
            .any(|provider| provider == provider_id)
    {
        anyhow::bail!(
            "Compiled skill '{}' declares auth providers [{}], not '{}'",
            artifact.manifest.name,
            artifact.manifest.declared_auth_providers.join(", "),
            provider_id
        );
    }
    if options.issuer.is_none()
        && (options.authorization_endpoint.is_none() || options.token_endpoint.is_none())
    {
        anyhow::bail!(
            "Bind auth plugin '{}' with either --issuer for OIDC discovery or both --authorization-endpoint and --token-endpoint",
            provider_id
        );
    }

    if options.service.is_some() || options.component.is_some() {
        let _ = resolve_compiled_skill_background_service(
            &artifact,
            options.service,
            options.component,
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }

    let default_prefix = default_auth_prefix(provider_id);
    let vault_key_prefix = options
        .vault_key_prefix
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(default_prefix);
    let client_id_key = options
        .client_id_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{}_CLIENT_ID", vault_key_prefix));
    let client_secret_key = options
        .client_secret_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{}_CLIENT_SECRET", vault_key_prefix));
    let now = Utc::now().to_rfc3339();

    let mut registry = load_auth_plugin_registry(&workspace_root)?;
    let existing_configured_at = registry
        .bindings
        .get(provider_id)
        .map(|binding| binding.configured_at.clone())
        .unwrap_or_else(|| now.clone());

    let binding = SkillAuthPluginBinding {
        provider_id: provider_id.to_string(),
        skill_name: artifact.manifest.name.clone(),
        redirect_uri: options.redirect_uri.map(ToString::to_string),
        issuer: options.issuer.map(ToString::to_string),
        authorization_endpoint: options.authorization_endpoint.map(ToString::to_string),
        token_endpoint: options.token_endpoint.map(ToString::to_string),
        client_id_key: client_id_key.clone(),
        client_secret_key: client_secret_key.clone(),
        scopes: auth_scope_list(options.scopes),
        vault_key_prefix: vault_key_prefix.clone(),
        service: options.service.map(ToString::to_string),
        component: options.component.map(ToString::to_string),
        configured_at: existing_configured_at,
        updated_at: now,
    };
    registry
        .bindings
        .insert(provider_id.to_string(), binding.clone());
    save_auth_plugin_registry(&workspace_root, &registry)?;

    let (_, pool) = load_skill_config_and_pool().await?;
    publish_plugin_event(
        &pool,
        "plugin.auth_plugin_bound",
        serde_json::json!({
            "provider_id": provider_id,
            "skill_name": binding.skill_name,
            "redirect_uri": binding.redirect_uri,
            "issuer": binding.issuer,
            "vault_key_prefix": binding.vault_key_prefix,
        }),
    )
    .await;

    Ok(SkillBindAuthPluginResult {
        status: "ok".to_string(),
        provider_id: provider_id.to_string(),
        skill_name: binding.skill_name,
        redirect_uri: binding.redirect_uri,
        authorization_endpoint: binding.authorization_endpoint,
        token_endpoint: binding.token_endpoint,
        client_id_key,
        client_secret_key,
        vault_key_prefix,
        scopes: binding.scopes,
    })
}

pub async fn authorize_auth_plugin_data(
    provider_id: &str,
    options: SkillAuthAuthorizeOptions<'_>,
) -> Result<SkillAuthAuthorizeResult> {
    let workspace_root = current_workspace_root()?;
    let (binding, client) = hydrated_auth_binding(&workspace_root, provider_id).await?;
    let redirect_uri = options
        .redirect_uri
        .map(ToString::to_string)
        .or_else(|| binding.redirect_uri.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Auth plugin '{}' has no redirect URI configured; pass --redirect-uri or re-bind with one",
                provider_id
            )
        })?;
    let state = uuid::Uuid::new_v4().to_string();
    let authorization_url = client.authorization_url(&state, &redirect_uri);

    let mut sessions = load_auth_plugin_sessions(&workspace_root)?;
    sessions.sessions.retain(|session| {
        !(session.provider_id == provider_id && session.redirect_uri == redirect_uri)
    });
    sessions.sessions.push(SkillAuthPendingSession {
        provider_id: provider_id.to_string(),
        state: state.clone(),
        redirect_uri: redirect_uri.clone(),
        created_at: Utc::now().to_rfc3339(),
    });
    save_auth_plugin_sessions(&workspace_root, &sessions)?;

    Ok(SkillAuthAuthorizeResult {
        status: "ok".to_string(),
        provider_id: provider_id.to_string(),
        skill_name: binding.skill_name,
        redirect_uri,
        state,
        authorization_url,
        vault_key_prefix: binding.vault_key_prefix,
        pending_states: sessions
            .sessions
            .iter()
            .filter(|session| session.provider_id == provider_id)
            .count(),
    })
}

pub async fn exchange_auth_plugin_data(
    provider_id: &str,
    options: SkillAuthExchangeOptions<'_>,
) -> Result<SkillAuthExchangeResult> {
    let workspace_root = current_workspace_root()?;
    let (binding, client) = hydrated_auth_binding(&workspace_root, provider_id).await?;
    let mut sessions = load_auth_plugin_sessions(&workspace_root)?;
    let session_index = sessions
        .sessions
        .iter()
        .position(|session| session.provider_id == provider_id && session.state == options.state)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No pending authorization state '{}' was found for auth plugin '{}'",
                options.state,
                provider_id
            )
        })?;
    let pending = sessions.sessions.remove(session_index);
    save_auth_plugin_sessions(&workspace_root, &sessions)?;

    let redirect_uri = options
        .redirect_uri
        .map(ToString::to_string)
        .unwrap_or(pending.redirect_uri);
    let tokens = client
        .exchange_code(options.code, &redirect_uri)
        .await
        .map_err(|error| anyhow::anyhow!("OIDC code exchange failed: {}", error))?;

    let mut stored_keys = Vec::new();
    runtime::set_vault_secret(
        &workspace_root,
        &access_token_key(&binding),
        &tokens.access_token,
    )?;
    stored_keys.push(access_token_key(&binding));
    if let Some(refresh_token) = tokens.refresh_token.as_deref() {
        runtime::set_vault_secret(&workspace_root, &refresh_token_key(&binding), refresh_token)?;
        stored_keys.push(refresh_token_key(&binding));
    }
    if let Some(id_token) = tokens.id_token.as_deref() {
        runtime::set_vault_secret(&workspace_root, &id_token_key(&binding), id_token)?;
        stored_keys.push(id_token_key(&binding));
    }
    runtime::set_vault_secret(
        &workspace_root,
        &expires_in_key(&binding),
        &tokens.expires_in.to_string(),
    )?;
    stored_keys.push(expires_in_key(&binding));
    if let Some(scope) = tokens.scope.as_deref() {
        runtime::set_vault_secret(&workspace_root, &scope_key(&binding), scope)?;
        stored_keys.push(scope_key(&binding));
    }

    let (_, pool) = load_skill_config_and_pool().await?;
    publish_plugin_event(
        &pool,
        "plugin.auth_plugin_connected",
        serde_json::json!({
            "provider_id": provider_id,
            "skill_name": binding.skill_name,
            "redirect_uri": redirect_uri,
            "stored_keys": stored_keys,
        }),
    )
    .await;

    Ok(SkillAuthExchangeResult {
        status: "ok".to_string(),
        provider_id: provider_id.to_string(),
        skill_name: binding.skill_name,
        redirect_uri,
        stored_keys,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
        scope: tokens.scope,
    })
}

pub async fn exchange_auth_plugin_callback_data(
    provider_id: Option<&str>,
    code: &str,
    state: &str,
    redirect_uri: Option<&str>,
) -> Result<SkillAuthExchangeResult> {
    let workspace_root = current_workspace_root()?;
    let sessions = load_auth_plugin_sessions(&workspace_root)?;
    let resolved_provider = provider_id
        .map(ToString::to_string)
        .or_else(|| provider_from_pending_state(&sessions, state))
        .ok_or_else(|| anyhow::anyhow!("No auth plugin could be resolved for state '{}'", state))?;
    exchange_auth_plugin_data(
        &resolved_provider,
        SkillAuthExchangeOptions {
            code,
            state,
            redirect_uri,
        },
    )
    .await
}

fn voice_plugin_summary(
    binding: &SkillVoicePluginBinding,
    artifact: Option<&CompiledSkillArtifact>,
    call_registry: &SkillVoiceCallRegistry,
) -> SkillVoicePluginSummary {
    let declared_by_skill = artifact.is_some_and(|artifact| {
        artifact
            .manifest
            .declared_voice_call_plugins
            .iter()
            .any(|plugin| plugin == &binding.plugin_id)
    });
    let blocked = artifact.is_some_and(|artifact| {
        matches!(
            artifact.manifest.status,
            openrustclaw_skills::CompiledSkillStatus::Blocked
        )
    });
    let active_calls = call_registry
        .calls
        .iter()
        .filter(|call| call.plugin_id == binding.plugin_id && call.ended_at.is_none())
        .count();

    SkillVoicePluginSummary {
        plugin_id: binding.plugin_id.clone(),
        skill_name: binding.skill_name.clone(),
        blocked,
        declared_by_skill,
        service: binding.service.clone(),
        component: binding.component.clone(),
        greeting_text: binding.greeting_text.clone(),
        default_voice: binding.default_voice.clone(),
        active_calls,
        updated_at: binding.updated_at.clone(),
    }
}

pub async fn voice_plugins_data() -> Result<SkillVoicePluginsResult> {
    let workspace_root = current_workspace_root()?;
    let registry = load_voice_plugin_registry(&workspace_root)?;
    let call_registry = load_voice_call_registry(&workspace_root)?;
    let mut voice_plugins = Vec::new();

    for binding in registry.bindings.values() {
        let artifact = compiled_skill_detail_or_compile(&binding.skill_name)
            .await
            .ok();
        voice_plugins.push(voice_plugin_summary(
            binding,
            artifact.as_ref(),
            &call_registry,
        ));
    }

    voice_plugins.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    Ok(SkillVoicePluginsResult {
        status: "ok".to_string(),
        count: voice_plugins.len(),
        voice_plugins,
    })
}

pub async fn voice_plugins_for_skill_data(skill_name: &str) -> Result<SkillVoicePluginsResult> {
    let mut result = voice_plugins_data().await?;
    result
        .voice_plugins
        .retain(|binding| binding.skill_name == skill_name);
    result.count = result.voice_plugins.len();
    Ok(result)
}

pub async fn bind_voice_plugin_data(
    plugin_id: &str,
    skill_name: &str,
    options: SkillBindVoicePluginOptions<'_>,
) -> Result<SkillBindVoicePluginResult> {
    let workspace_root = current_workspace_root()?;
    let artifact = compiled_skill_detail_or_compile(skill_name).await?;
    if matches!(
        artifact.manifest.status,
        openrustclaw_skills::CompiledSkillStatus::Blocked
    ) {
        anyhow::bail!(
            "Compiled skill '{}' is blocked and cannot be bound as a voice plugin",
            artifact.manifest.name
        );
    }
    if !artifact.manifest.declared_voice_call_plugins.is_empty()
        && !artifact
            .manifest
            .declared_voice_call_plugins
            .iter()
            .any(|plugin| plugin == plugin_id)
    {
        anyhow::bail!(
            "Compiled skill '{}' declares voice call plugins [{}], not '{}'",
            artifact.manifest.name,
            artifact.manifest.declared_voice_call_plugins.join(", "),
            plugin_id
        );
    }

    let (service, component) = if options.service.is_some() || options.component.is_some() {
        let resolved = resolve_compiled_skill_background_service(
            &artifact,
            options.service,
            options.component,
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        (Some(resolved.name), resolved.component)
    } else {
        (None, None)
    };

    let now = Utc::now().to_rfc3339();
    let mut registry = load_voice_plugin_registry(&workspace_root)?;
    let configured_at = registry
        .bindings
        .get(plugin_id)
        .map(|binding| binding.configured_at.clone())
        .unwrap_or_else(|| now.clone());
    let binding = SkillVoicePluginBinding {
        plugin_id: plugin_id.to_string(),
        skill_name: artifact.manifest.name.clone(),
        service: service.clone(),
        component: component.clone(),
        greeting_text: options.greeting_text.map(ToString::to_string),
        default_voice: options.default_voice.map(ToString::to_string),
        configured_at,
        updated_at: now,
    };
    registry
        .bindings
        .insert(plugin_id.to_string(), binding.clone());
    save_voice_plugin_registry(&workspace_root, &registry)?;

    let (_, pool) = load_skill_config_and_pool().await?;
    publish_plugin_event(
        &pool,
        "plugin.voice_call_bound",
        serde_json::json!({
            "plugin_id": plugin_id,
            "skill_name": binding.skill_name,
            "service": binding.service,
            "component": binding.component,
        }),
    )
    .await;

    Ok(SkillBindVoicePluginResult {
        status: "ok".to_string(),
        plugin_id: plugin_id.to_string(),
        skill_name: binding.skill_name,
        service: binding.service,
        component: binding.component,
        greeting_text: binding.greeting_text,
        default_voice: binding.default_voice,
    })
}

pub async fn voice_calls_data() -> Result<SkillVoiceCallsResult> {
    let workspace_root = current_workspace_root()?;
    let mut registry = load_voice_call_registry(&workspace_root)?;
    let now = Utc::now();
    for call in &mut registry.calls {
        refresh_voice_call_health(call, now);
    }
    registry
        .calls
        .sort_by(|left, right| right.started_at.cmp(&left.started_at));
    Ok(SkillVoiceCallsResult {
        status: "ok".to_string(),
        count: registry.calls.len(),
        calls: registry.calls,
    })
}

pub async fn voice_call_health_data() -> Result<SkillVoiceCallHealthResult> {
    let result = voice_calls_data().await?;
    Ok(SkillVoiceCallHealthResult {
        status: "ok".to_string(),
        health: voice_call_health_summary(&result.calls),
    })
}

async fn execute_voice_call_hook(
    artifact: &CompiledSkillArtifact,
    service: Option<&str>,
    component: Option<&str>,
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    if service.is_none() && component.is_none() {
        return Ok(None);
    }
    let resolved = resolve_compiled_skill_background_service(artifact, service, component)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let component = resolved.component.ok_or_else(|| {
        anyhow::anyhow!(
            "Voice call hook '{}' for '{}' has no executable component mapping",
            resolved.name,
            artifact.manifest.name
        )
    })?;
    let result = execute_compiled_skill_artifact(artifact, Some(&component), input)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(Some(result.output))
}

fn parse_optional_json(raw: Option<&str>, field_name: &str) -> Result<serde_json::Value> {
    match raw {
        Some(value) if !value.trim().is_empty() => {
            serde_json::from_str(value).with_context(|| format!("Invalid JSON for {}", field_name))
        }
        _ => Ok(serde_json::json!({})),
    }
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn refresh_voice_call_health(call: &mut SkillVoiceCallRecord, now: DateTime<Utc>) {
    if call.ended_at.is_some() {
        if call.status == "reaped" {
            call.health = "reaped".to_string();
        } else {
            call.health = "ended".to_string();
        }
        return;
    }

    let reference = call
        .last_seen_at
        .as_deref()
        .and_then(parse_rfc3339_utc)
        .or_else(|| parse_rfc3339_utc(&call.started_at))
        .unwrap_or(now);
    let stale_after_secs = call.stale_after_secs.max(1);
    let age = now.signed_duration_since(reference).num_seconds().max(0) as u64;
    call.health = if age > stale_after_secs {
        "stale".to_string()
    } else {
        "active".to_string()
    };
}

fn voice_call_health_summary(calls: &[SkillVoiceCallRecord]) -> SkillVoiceCallHealthSummary {
    let mut active = 0usize;
    let mut stale = 0usize;
    let mut ended = 0usize;
    let mut reaped = 0usize;
    let mut oldest_active: Option<(DateTime<Utc>, String)> = None;

    for call in calls {
        match call.health.as_str() {
            "active" => {
                active += 1;
                if let Some(started_at) = parse_rfc3339_utc(&call.started_at) {
                    let replace = oldest_active
                        .as_ref()
                        .map(|(current, _)| started_at < *current)
                        .unwrap_or(true);
                    if replace {
                        oldest_active = Some((started_at, call.call_id.clone()));
                    }
                }
            }
            "stale" => stale += 1,
            "reaped" => reaped += 1,
            _ => ended += 1,
        }
    }

    SkillVoiceCallHealthSummary {
        total: calls.len(),
        active,
        stale,
        ended,
        reaped,
        oldest_active_call_id: oldest_active.map(|(_, id)| id),
    }
}

pub async fn start_voice_call_data(
    plugin_id: &str,
    options: SkillStartVoiceCallOptions<'_>,
) -> Result<SkillStartVoiceCallResult> {
    let workspace_root = current_workspace_root()?;
    let registry = load_voice_plugin_registry(&workspace_root)?;
    let binding = registry
        .bindings
        .get(plugin_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Voice plugin '{}' not found", plugin_id))?;
    let artifact = compiled_skill_detail_or_compile(&binding.skill_name).await?;
    let (config, pool) = load_skill_config_and_pool().await?;
    let call_id = uuid::Uuid::new_v4().to_string();
    let metadata = parse_optional_json(options.metadata, "--metadata")?;
    let greeting_text = options
        .greeting_text
        .map(ToString::to_string)
        .or_else(|| binding.greeting_text.clone());
    let greeting_audio_path = if let Some(text) = greeting_text.as_deref() {
        Some(
            voice_runtime::synthesize_with_config(
                &config,
                &workspace_root,
                voice_runtime::VoiceSynthesizeRequest {
                    text: text.to_string(),
                    provider: None,
                    model: None,
                    voice: options
                        .voice
                        .map(ToString::to_string)
                        .or_else(|| binding.default_voice.clone()),
                    format: Some("mp3".to_string()),
                    output_path: None,
                },
            )
            .await?
            .output_path,
        )
    } else {
        None
    };
    let start_hook_output = execute_voice_call_hook(
        &artifact,
        binding.service.as_deref(),
        binding.component.as_deref(),
        serde_json::json!({
            "action": "call_start",
            "call_id": &call_id,
            "plugin_id": plugin_id,
            "skill_name": &binding.skill_name,
            "remote": options.remote,
            "metadata": &metadata,
        }),
    )
    .await?;

    let call = SkillVoiceCallRecord {
        call_id: call_id.clone(),
        plugin_id: plugin_id.to_string(),
        skill_name: binding.skill_name.clone(),
        status: "active".to_string(),
        health: "active".to_string(),
        remote: options.remote.map(ToString::to_string),
        greeting_text: greeting_text.clone(),
        greeting_audio_path,
        reason: None,
        metadata,
        start_hook_output,
        end_hook_output: None,
        started_at: Utc::now().to_rfc3339(),
        last_seen_at: Some(Utc::now().to_rfc3339()),
        stale_after_secs: options
            .stale_after_secs
            .unwrap_or_else(default_voice_call_stale_after_secs),
        ended_at: None,
    };

    let mut call_registry = load_voice_call_registry(&workspace_root)?;
    call_registry.calls.push(call.clone());
    save_voice_call_registry(&workspace_root, &call_registry)?;

    publish_plugin_event(
        &pool,
        "plugin.voice_call_started",
        serde_json::json!({
            "call_id": &call_id,
            "plugin_id": plugin_id,
            "skill_name": &binding.skill_name,
            "remote": options.remote,
        }),
    )
    .await;

    Ok(SkillStartVoiceCallResult {
        status: "ok".to_string(),
        call,
    })
}

pub async fn end_voice_call_data(
    call_id: &str,
    options: SkillEndVoiceCallOptions<'_>,
) -> Result<SkillEndVoiceCallResult> {
    let workspace_root = current_workspace_root()?;
    let registry = load_voice_plugin_registry(&workspace_root)?;
    let mut call_registry = load_voice_call_registry(&workspace_root)?;
    let index = call_registry
        .calls
        .iter()
        .position(|call| call.call_id == call_id)
        .ok_or_else(|| anyhow::anyhow!("Voice call '{}' not found", call_id))?;
    if call_registry.calls[index].ended_at.is_some() {
        anyhow::bail!("Voice call '{}' is already ended", call_id);
    }

    let binding = registry
        .bindings
        .get(&call_registry.calls[index].plugin_id)
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Voice plugin '{}' for call '{}' was not found",
                call_registry.calls[index].plugin_id,
                call_id
            )
        })?;
    let artifact = compiled_skill_detail_or_compile(&binding.skill_name).await?;
    let (_, pool) = load_skill_config_and_pool().await?;
    let extra_metadata = parse_optional_json(options.metadata, "--metadata")?;
    let end_hook_output = execute_voice_call_hook(
        &artifact,
        binding.service.as_deref(),
        binding.component.as_deref(),
        serde_json::json!({
            "action": "call_end",
            "call_id": call_id,
            "plugin_id": &binding.plugin_id,
            "skill_name": &binding.skill_name,
            "reason": options.reason,
            "metadata": &extra_metadata,
        }),
    )
    .await?;

    let call = &mut call_registry.calls[index];
    if !extra_metadata.is_null() {
        if call.metadata.is_object() && extra_metadata.is_object() {
            let target = call.metadata.as_object_mut().expect("object checked");
            for (key, value) in extra_metadata.as_object().expect("object checked") {
                target.insert(key.clone(), value.clone());
            }
        } else {
            call.metadata = extra_metadata;
        }
    }
    call.status = "ended".to_string();
    call.health = "ended".to_string();
    call.reason = options.reason.map(ToString::to_string);
    call.last_seen_at = Some(Utc::now().to_rfc3339());
    call.ended_at = Some(Utc::now().to_rfc3339());
    call.end_hook_output = end_hook_output;
    let result_call = call.clone();
    save_voice_call_registry(&workspace_root, &call_registry)?;

    publish_plugin_event(
        &pool,
        "plugin.voice_call_ended",
        serde_json::json!({
            "call_id": call_id,
            "plugin_id": &binding.plugin_id,
            "skill_name": &binding.skill_name,
            "reason": options.reason,
        }),
    )
    .await;

    Ok(SkillEndVoiceCallResult {
        status: "ok".to_string(),
        call: result_call,
    })
}

pub async fn prewarm_voice_plugin_data(
    plugin_id: &str,
    options: SkillPrewarmVoicePluginOptions<'_>,
) -> Result<SkillPrewarmVoicePluginResult> {
    let workspace_root = current_workspace_root()?;
    let registry = load_voice_plugin_registry(&workspace_root)?;
    let binding = registry
        .bindings
        .get(plugin_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Voice plugin '{}' not found", plugin_id))?;
    let (config, pool) = load_skill_config_and_pool().await?;
    let greeting_text = options
        .greeting_text
        .map(ToString::to_string)
        .or_else(|| binding.greeting_text.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "voice plugin '{}' has no greeting text to prewarm",
                plugin_id
            )
        })?;
    let voice = options
        .voice
        .map(ToString::to_string)
        .or_else(|| binding.default_voice.clone())
        .unwrap_or_else(|| config.voice.tts.voice.clone());
    let result = voice_runtime::synthesize_with_config(
        &config,
        &workspace_root,
        voice_runtime::VoiceSynthesizeRequest {
            text: greeting_text.clone(),
            provider: None,
            model: None,
            voice: Some(voice.clone()),
            format: Some("mp3".to_string()),
            output_path: None,
        },
    )
    .await?;

    publish_plugin_event(
        &pool,
        "plugin.voice_call_prewarmed",
        serde_json::json!({
            "plugin_id": plugin_id,
            "skill_name": &binding.skill_name,
            "voice": &voice,
            "output_path": &result.output_path,
        }),
    )
    .await;

    Ok(SkillPrewarmVoicePluginResult {
        status: "ok".to_string(),
        plugin_id: plugin_id.to_string(),
        skill_name: binding.skill_name,
        greeting_text,
        voice,
        output_path: result.output_path,
        bytes: result.bytes,
    })
}

pub async fn reap_voice_calls_data(
    options: SkillReapVoiceCallsOptions,
) -> Result<SkillReapVoiceCallsResult> {
    let workspace_root = current_workspace_root()?;
    let plugin_registry = load_voice_plugin_registry(&workspace_root)?;
    let mut call_registry = load_voice_call_registry(&workspace_root)?;
    let (_, pool) = load_skill_config_and_pool().await?;
    let now = Utc::now();
    let stale_override = options.stale_after_secs;
    let limit = options.limit.unwrap_or(usize::MAX);
    let mut reaped_calls = Vec::new();

    for call in &mut call_registry.calls {
        refresh_voice_call_health(call, now);
    }

    let checked = call_registry.calls.len();

    for call in &mut call_registry.calls {
        if reaped_calls.len() >= limit {
            break;
        }
        if call.ended_at.is_some() || call.health != "stale" {
            continue;
        }
        if let Some(override_secs) = stale_override {
            let reference = call
                .last_seen_at
                .as_deref()
                .and_then(parse_rfc3339_utc)
                .or_else(|| parse_rfc3339_utc(&call.started_at))
                .unwrap_or(now);
            let age = now.signed_duration_since(reference).num_seconds().max(0) as u64;
            if age <= override_secs.max(1) {
                continue;
            }
        }

        let binding = plugin_registry.bindings.get(&call.plugin_id).cloned();
        let end_hook_output = if let Some(binding) = binding.as_ref() {
            match compiled_skill_detail_or_compile(&binding.skill_name).await {
                Ok(artifact) => execute_voice_call_hook(
                    &artifact,
                    binding.service.as_deref(),
                    binding.component.as_deref(),
                    serde_json::json!({
                        "action": "call_reap",
                        "call_id": &call.call_id,
                        "plugin_id": &binding.plugin_id,
                        "skill_name": &binding.skill_name,
                        "reason": "stale_timeout",
                    }),
                )
                .await
                .unwrap_or_else(|error| Some(serde_json::json!({ "error": error.to_string() }))),
                Err(error) => Some(serde_json::json!({ "error": error.to_string() })),
            }
        } else {
            Some(serde_json::json!({ "error": "voice plugin binding not found during reap" }))
        };

        call.status = "reaped".to_string();
        call.health = "reaped".to_string();
        call.reason = Some("stale_timeout".to_string());
        call.last_seen_at = Some(now.to_rfc3339());
        call.ended_at = Some(now.to_rfc3339());
        call.end_hook_output = end_hook_output;
        reaped_calls.push(call.clone());

        publish_plugin_event(
            &pool,
            "plugin.voice_call_reaped",
            serde_json::json!({
                "call_id": &call.call_id,
                "plugin_id": &call.plugin_id,
                "skill_name": &call.skill_name,
                "reason": "stale_timeout",
            }),
        )
        .await;
    }

    save_voice_call_registry(&workspace_root, &call_registry)?;

    Ok(SkillReapVoiceCallsResult {
        status: "ok".to_string(),
        checked,
        reaped_count: reaped_calls.len(),
        calls: reaped_calls,
    })
}

pub async fn compile_data(name: Option<&str>) -> Result<SkillCompileResult> {
    let root = ensure_compiled_root()?;
    let mut compiled = Vec::new();

    if let Some(name) = name {
        compiled.push(compile_skill_by_name_internal(name).await?.manifest);
    } else {
        let mut targets = discover_workspace_skill_paths();
        for installed in installed_skills_data().await.unwrap_or_default() {
            if let Some(local_path) = installed.local_path {
                targets.insert(
                    installed.name.clone(),
                    (
                        PathBuf::from(local_path),
                        source_from_string(&installed.source),
                        installed.verified,
                    ),
                );
            }
        }

        for (name, (path, source, verified)) in targets {
            if !path.exists() {
                continue;
            }
            compiled.push(
                compile_skill_target(&name, &path, source, verified)
                    .await?
                    .manifest,
            );
        }
    }

    let blocked_count = compiled
        .iter()
        .filter(|manifest| {
            matches!(
                manifest.status,
                openrustclaw_skills::CompiledSkillStatus::Blocked
            )
        })
        .count();
    compiled.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(SkillCompileResult {
        status: "ok".to_string(),
        output_root: root.display().to_string(),
        compiled_count: compiled.len(),
        blocked_count,
        compiled,
    })
}

pub async fn search_data(
    query: &str,
    category: Option<&str>,
    sort: &str,
) -> Result<SkillCatalogResult> {
    let (config, _) = load_skill_config_and_pool().await?;
    let sort_by = match sort.to_lowercase().as_str() {
        "downloads" => SortBy::Downloads,
        "rating" => SortBy::Rating,
        "recent" => SortBy::Recent,
        _ => SortBy::Relevance,
    };
    let filters = SearchFilters {
        category: category.map(|value| value.to_string()),
        sort_by,
        min_rating: None,
        verified_only: false,
    };

    let registry = build_registry_client(&config).await?;
    let skills = registry
        .search(query, filters)
        .await?
        .into_iter()
        .map(registry_skill_summary)
        .collect();

    Ok(SkillCatalogResult {
        source: "clawhub".to_string(),
        query: Some(query.to_string()),
        category: category.map(|value| value.to_string()),
        sort: Some(sort.to_string()),
        limit: None,
        skills,
    })
}

pub async fn popular_data(limit: usize) -> Result<SkillCatalogResult> {
    let (config, _) = load_skill_config_and_pool().await?;
    let registry = build_registry_client(&config).await?;
    let skills = registry
        .get_popular(limit)
        .await?
        .into_iter()
        .map(registry_skill_summary)
        .collect();

    Ok(SkillCatalogResult {
        source: "clawhub".to_string(),
        query: None,
        category: None,
        sort: Some("popular".to_string()),
        limit: Some(limit),
        skills,
    })
}

pub async fn trending_data(limit: usize) -> Result<SkillCatalogResult> {
    let (config, _) = load_skill_config_and_pool().await?;
    let registry = build_registry_client(&config).await?;
    let skills = registry
        .get_trending(limit)
        .await?
        .into_iter()
        .map(registry_skill_summary)
        .collect();

    Ok(SkillCatalogResult {
        source: "clawhub".to_string(),
        query: None,
        category: None,
        sort: Some("trending".to_string()),
        limit: Some(limit),
        skills,
    })
}

pub async fn install_data(name: &str) -> Result<SkillMutationResult> {
    let (config, pool) = load_skill_config_and_pool().await?;

    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        return Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "install".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' is already installed", name),
            skill: Some(installed_skill_detail_data(name).await?),
            verified: None,
        });
    }

    let skills_dir = Path::new("skills");
    if !skills_dir.exists() {
        tokio::fs::create_dir_all(skills_dir).await?;
    }

    if let Some(skill_file) = resolve_workspace_skill_file(name) {
        let content = tokio::fs::read_to_string(&skill_file).await?;
        let metadata = parse_skill_metadata(&content)?;
        let capabilities_json = serialize_capabilities(&metadata.capabilities)?;

        upsert_skill_record(
            &pool,
            &metadata.name,
            metadata.description.as_deref(),
            "workspace",
            metadata.version.as_deref(),
            metadata.signature.as_deref(),
            true,
            Some(&capabilities_json),
            metadata.schema.as_deref(),
        )
        .await?;

        publish_plugin_event(
            &pool,
            "plugin.skill_installed",
            serde_json::json!({
                "name": metadata.name,
                "source": "workspace",
                "capabilities": metadata.capabilities,
            }),
        )
        .await;

        let _ =
            compile_skill_target(&metadata.name, &skill_file, SkillSource::Workspace, true).await;

        return Ok(SkillMutationResult {
            status: "ok".to_string(),
            action: "install".to_string(),
            skill_name: metadata.name.clone(),
            message: format!(
                "Installed workspace skill '{}' from {}",
                metadata.name,
                skill_file.display()
            ),
            skill: Some(installed_skill_detail_data(&metadata.name).await?),
            verified: Some(true),
        });
    }

    let registry = build_registry_client(&config).await?;
    let registry_metadata = registry
        .get_skill(name)
        .await
        .with_context(|| format!("Failed to fetch skill metadata for '{}'", name))?;

    enforce_external_skill_policy(
        &config,
        &registry_metadata.capabilities,
        registry_metadata.signature.as_deref(),
        name,
    )?;

    match registry.install(name, None).await? {
        openrustclaw_skills::InstallResult::AlreadyInstalled => Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "install".to_string(),
            skill_name: name.to_string(),
            message: format!(
                "Skill '{}' was already installed in the registry store",
                name
            ),
            skill: if sqlx::query_scalar::<_, String>("SELECT name FROM skills WHERE name = ?")
                .bind(name)
                .fetch_optional(&pool)
                .await?
                .is_some()
            {
                Some(installed_skill_detail_data(name).await?)
            } else {
                None
            },
            verified: Some(false),
        }),
        openrustclaw_skills::InstallResult::Installed {
            name,
            version,
            path,
        } => {
            let capabilities_json = serialize_capabilities(&registry_metadata.capabilities)?;
            let version_string = version.to_string();
            upsert_skill_record(
                &pool,
                &name,
                Some(&registry_metadata.description),
                "marketplace",
                Some(version_string.as_str()),
                registry_metadata.signature.as_deref(),
                false,
                Some(&capabilities_json),
                None,
            )
            .await?;

            publish_plugin_event(
                &pool,
                "plugin.skill_installed",
                serde_json::json!({
                    "name": name,
                    "source": "marketplace",
                    "version": version_string,
                }),
            )
            .await;

            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                let _ =
                    compile_skill_target(&name, &skill_file, SkillSource::Marketplace, false).await;
            }

            Ok(SkillMutationResult {
                status: "ok".to_string(),
                action: "install".to_string(),
                skill_name: name.clone(),
                message: format!("Installed marketplace skill '{}' v{}", name, version),
                skill: Some(installed_skill_detail_data(&name).await?),
                verified: Some(false),
            })
        }
    }
}

pub async fn update_data(name: &str) -> Result<SkillMutationResult> {
    let (config, pool) = load_skill_config_and_pool().await?;

    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        return Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "update".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' is not installed", name),
            skill: None,
            verified: None,
        });
    }

    let registry = build_registry_client(&config).await?;
    match registry.update(name).await? {
        openrustclaw_skills::UpdateResult::UpToDate => Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "update".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' is already up to date", name),
            skill: Some(installed_skill_detail_data(name).await?),
            verified: None,
        }),
        openrustclaw_skills::UpdateResult::Updated { from, to } => {
            let metadata = registry.get_skill(name).await?;
            enforce_external_skill_policy(
                &config,
                &metadata.capabilities,
                metadata.signature.as_deref(),
                name,
            )?;
            let capabilities_json = serialize_capabilities(&metadata.capabilities)?;
            let version_string = to.to_string();
            upsert_skill_record(
                &pool,
                name,
                Some(&metadata.description),
                "marketplace",
                Some(version_string.as_str()),
                metadata.signature.as_deref(),
                false,
                Some(&capabilities_json),
                None,
            )
            .await?;

            publish_plugin_event(
                &pool,
                "plugin.skill_updated",
                serde_json::json!({
                    "name": name,
                    "from": from.to_string(),
                    "to": version_string,
                }),
            )
            .await;

            let _ = compile_skill_by_name_internal(name).await;

            Ok(SkillMutationResult {
                status: "ok".to_string(),
                action: "update".to_string(),
                skill_name: name.to_string(),
                message: format!("Updated skill '{}' from v{} to v{}", name, from, to),
                skill: Some(installed_skill_detail_data(name).await?),
                verified: Some(false),
            })
        }
    }
}

pub async fn uninstall_data(name: &str) -> Result<SkillMutationResult> {
    let (config, pool) = load_skill_config_and_pool().await?;

    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        return Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "uninstall".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' is not installed", name),
            skill: None,
            verified: None,
        });
    }

    if let Ok(registry) = build_registry_client(&config).await
        && let Err(error) = registry.uninstall(name).await
    {
        debug!(error = %error, skill = %name, "Failed to uninstall skill through registry client");
    }

    sqlx::query("DELETE FROM skills WHERE name = ?")
        .bind(name)
        .execute(&pool)
        .await?;

    let _ = remove_compiled_artifact(&compiled_skill_root(), name);

    publish_plugin_event(
        &pool,
        "plugin.skill_uninstalled",
        serde_json::json!({
            "name": name,
        }),
    )
    .await;

    Ok(SkillMutationResult {
        status: "ok".to_string(),
        action: "uninstall".to_string(),
        skill_name: name.to_string(),
        message: format!("Uninstalled skill '{}'", name),
        skill: None,
        verified: None,
    })
}

pub async fn verify_data(name: &str) -> Result<SkillMutationResult> {
    let (config, pool) = load_skill_config_and_pool().await?;
    let row = sqlx::query("SELECT id, signature, source, verified FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    let Some(row) = row else {
        anyhow::bail!("Skill '{}' not found", name);
    };

    let id: String = row.get("id");
    let signature: Option<String> = row.get("signature");
    let source: String = row.get("source");
    let verified: i64 = row.get("verified");

    if source == "workspace" || source == "bundled" {
        persist_verified_state(&pool, &id, true).await?;
        let _ = compile_skill_by_name_internal(name).await;
        publish_plugin_event(
            &pool,
            "plugin.skill_verified",
            serde_json::json!({
                "name": name,
                "source": source,
                "verified": true,
                "verification_mode": "exempt",
            }),
        )
        .await;

        return Ok(SkillMutationResult {
            status: "ok".to_string(),
            action: "verify".to_string(),
            skill_name: name.to_string(),
            message: format!(
                "Skill '{}' is verification-exempt because it is {}",
                name, source
            ),
            skill: Some(installed_skill_detail_data(name).await?),
            verified: Some(true),
        });
    }

    if verified == 1 {
        return Ok(SkillMutationResult {
            status: "noop".to_string(),
            action: "verify".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' is already verified", name),
            skill: Some(installed_skill_detail_data(name).await?),
            verified: Some(true),
        });
    }

    let Some(skill_file) = resolve_workspace_skill_file(name) else {
        anyhow::bail!(
            "Skill '{}' is not a workspace skill and no local SKILL.md was found for manual verification",
            name
        );
    };

    let content = tokio::fs::read(&skill_file).await?;
    let verifier = load_configured_skill_verifier(&config, true)?;

    let Some(sig_hex) = signature else {
        persist_verified_state(&pool, &id, false).await?;
        return Ok(SkillMutationResult {
            status: "failed".to_string(),
            action: "verify".to_string(),
            skill_name: name.to_string(),
            message: format!("Skill '{}' has no signature", name),
            skill: Some(installed_skill_detail_data(name).await?),
            verified: Some(false),
        });
    };

    let signature_bytes = hex::decode(&sig_hex).context("Invalid signature format in database")?;
    let verified_result = match verifier.verify(&content, &signature_bytes) {
        Ok(value) => value,
        Err(error) => {
            persist_verified_state(&pool, &id, false).await?;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": false,
                    "verification_mode": "error",
                    "error": error.to_string(),
                }),
            )
            .await;
            return Ok(SkillMutationResult {
                status: "failed".to_string(),
                action: "verify".to_string(),
                skill_name: name.to_string(),
                message: format!("Error verifying skill '{}': {}", name, error),
                skill: Some(installed_skill_detail_data(name).await?),
                verified: Some(false),
            });
        }
    };

    persist_verified_state(&pool, &id, verified_result).await?;
    let _ = compile_skill_by_name_internal(name).await;
    publish_plugin_event(
        &pool,
        "plugin.skill_verified",
        serde_json::json!({
            "name": name,
            "source": source,
            "verified": verified_result,
            "verification_mode": "signature",
        }),
    )
    .await;

    Ok(SkillMutationResult {
        status: if verified_result { "ok" } else { "failed" }.to_string(),
        action: "verify".to_string(),
        skill_name: name.to_string(),
        message: if verified_result {
            format!("Skill '{}' signature verified successfully", name)
        } else {
            format!("Skill '{}' signature verification failed", name)
        },
        skill: Some(installed_skill_detail_data(name).await?),
        verified: Some(verified_result),
    })
}

/// List installed skills from database.
pub async fn list() -> Result<()> {
    // Load configuration to get DB path
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Query skills from database
    let rows = sqlx::query(
        r#"
        SELECT 
            name, 
            description, 
            source, 
            version, 
            verified, 
            enabled,
            created_at
        FROM skills 
        ORDER BY name
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query skills from database")?;

    if rows.is_empty() {
        println!("No skills installed.");
        println!();
        println!("To install a skill:");
        println!("  openrustclaw skills install <skill-name>");
        println!();
        println!("To search for skills:");
        println!("  openrustclaw skills search <query>");
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                 Installed Skills                         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    for row in rows {
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let source: String = row.get("source");
        let version: Option<String> = row.get("version");
        let verified: i64 = row.get("verified");
        let enabled: i64 = row.get("enabled");

        let status = if enabled == 1 {
            "\x1b[32m● enabled\x1b[0m"
        } else {
            "\x1b[90m○ disabled\x1b[0m"
        };

        let verified_icon = if verified == 1 {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[90m○\x1b[0m"
        };

        println!("{} {} {}", verified_icon, name, status);

        if let Some(desc) = description {
            println!("  {}", desc);
        }

        println!("  Source: {}", source);

        if let Some(ver) = version {
            println!("  Version: {}", ver);
        }

        println!();
    }

    Ok(())
}

/// Compile one skill or refresh compiled artifacts for all discoverable skills.
pub async fn compile(name: Option<&str>) -> Result<()> {
    let result = compile_data(name).await?;
    println!("Compiled skills root: {}", result.output_root);
    println!(
        "Compiled {} skill(s) ({} blocked).",
        result.compiled_count, result.blocked_count
    );
    for manifest in result.compiled {
        println!(
            "- {} [{}] {}",
            manifest.name,
            serde_json::to_string(&manifest.status)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"'),
            manifest.local_path
        );
    }
    Ok(())
}

/// Refresh the compiled skill registry for all discoverable local skills.
pub async fn refresh_compiled() -> Result<()> {
    compile(None).await
}

/// Inspect one compiled skill artifact bundle.
pub async fn inspect_compiled(name: &str) -> Result<()> {
    let artifact = compiled_skill_detail_data(name).await?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

/// List compiled extension manifests generated from cached skills.
pub async fn list_extensions() -> Result<()> {
    let manifests = extension_manifests_data().await?;
    if manifests.is_empty() {
        println!("No compiled extension manifests found.");
        println!("Compile skills first with: openrustclaw skills compile");
        return Ok(());
    }

    println!("Compiled extension manifests:");
    for manifest in manifests {
        println!(
            "- {} [{}] modes={} capabilities={}",
            manifest.name,
            format!("{:?}", manifest.compiled_status).to_lowercase(),
            if manifest.runtime_modes.is_empty() {
                "-".to_string()
            } else {
                manifest.runtime_modes.join(",")
            },
            if manifest.capabilities.is_empty() {
                "-".to_string()
            } else {
                manifest.capabilities.join(",")
            }
        );
    }
    Ok(())
}

/// Inspect one compiled extension manifest bundle.
pub async fn inspect_extension(name: &str) -> Result<()> {
    let manifest = extension_manifest_data(name).await?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

/// List declared and inferred background services for a compiled skill.
pub async fn background_services(name: &str) -> Result<()> {
    let result = background_services_data(name).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// List channel bindings that currently attach a compiled skill background extension.
pub async fn list_channel_extensions() -> Result<()> {
    let result = channel_extensions_data().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Invoke the generated CLI/help bridge for a compiled skill.
pub async fn invoke(
    name: &str,
    args: Option<&str>,
    reference: Option<&str>,
    detail: bool,
) -> Result<()> {
    let result = invoke_data(
        name,
        SkillInvokeOptions {
            args,
            reference,
            max_chars: None,
            detail,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Execute a bounded `.wasm` or `.wat` component from a compiled skill.
pub async fn execute(name: &str, component: Option<&str>, input: Option<&str>) -> Result<()> {
    let result = execute_data(name, SkillExecuteOptions { component, input }).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Schedule a compiled skill background workflow through the durable scheduler.
pub async fn schedule_background(
    name: &str,
    service: Option<&str>,
    component: Option<&str>,
    input: Option<&str>,
    every_seconds: Option<u64>,
    at: Option<&str>,
    priority: i64,
) -> Result<()> {
    let result = schedule_background_service_data(
        name,
        SkillScheduleBackgroundOptions {
            service,
            component,
            input,
            every_seconds,
            at,
            priority,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Bind a compiled skill background service to a channel binding.
pub async fn bind_channel_extension(
    binding_id: &str,
    skill_name: &str,
    service: Option<&str>,
    component: Option<&str>,
    trigger: Option<&str>,
) -> Result<()> {
    let result = bind_channel_extension_data(
        binding_id,
        skill_name,
        SkillBindChannelExtensionOptions {
            service,
            component,
            trigger,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// List configured auth-plugin bindings over compiled skills.
pub async fn list_auth_plugins() -> Result<()> {
    let result = auth_plugins_data().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Bind a compiled skill to a bounded OIDC auth-provider lane.
pub async fn bind_auth_plugin(
    provider_id: &str,
    skill_name: &str,
    redirect_uri: Option<&str>,
    issuer: Option<&str>,
    authorization_endpoint: Option<&str>,
    token_endpoint: Option<&str>,
    client_id_key: Option<&str>,
    client_secret_key: Option<&str>,
    scopes: Option<&str>,
    vault_key_prefix: Option<&str>,
    service: Option<&str>,
    component: Option<&str>,
) -> Result<()> {
    let result = bind_auth_plugin_data(
        provider_id,
        skill_name,
        SkillBindAuthPluginOptions {
            redirect_uri,
            issuer,
            authorization_endpoint,
            token_endpoint,
            client_id_key,
            client_secret_key,
            scopes,
            vault_key_prefix,
            service,
            component,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Start an authorization-code flow for a configured auth plugin.
pub async fn authorize_auth_plugin(provider_id: &str, redirect_uri: Option<&str>) -> Result<()> {
    let result =
        authorize_auth_plugin_data(provider_id, SkillAuthAuthorizeOptions { redirect_uri }).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Exchange an authorization code for tokens and store them in the runtime vault.
pub async fn exchange_auth_plugin(
    provider_id: &str,
    code: &str,
    state: &str,
    redirect_uri: Option<&str>,
) -> Result<()> {
    let result = exchange_auth_plugin_data(
        provider_id,
        SkillAuthExchangeOptions {
            code,
            state,
            redirect_uri,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// List configured voice-call plugin bindings over compiled skills.
pub async fn list_voice_plugins() -> Result<()> {
    let result = voice_plugins_data().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Bind a compiled skill to a bounded voice-call plugin lane.
pub async fn bind_voice_plugin(
    plugin_id: &str,
    skill_name: &str,
    service: Option<&str>,
    component: Option<&str>,
    greeting_text: Option<&str>,
    default_voice: Option<&str>,
) -> Result<()> {
    let result = bind_voice_plugin_data(
        plugin_id,
        skill_name,
        SkillBindVoicePluginOptions {
            service,
            component,
            greeting_text,
            default_voice,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// List persisted bounded voice-call session receipts.
pub async fn list_voice_calls() -> Result<()> {
    let result = voice_calls_data().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Summarize bounded voice-call lifecycle health.
pub async fn voice_call_health() -> Result<()> {
    let result = voice_call_health_data().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Start a bounded voice-call session for a configured plugin.
pub async fn start_voice_call(
    plugin_id: &str,
    remote: Option<&str>,
    greeting_text: Option<&str>,
    voice: Option<&str>,
    metadata: Option<&str>,
    stale_after_secs: Option<u64>,
) -> Result<()> {
    let result = start_voice_call_data(
        plugin_id,
        SkillStartVoiceCallOptions {
            remote,
            greeting_text,
            voice,
            metadata,
            stale_after_secs,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// End a bounded voice-call session and persist the receipt.
pub async fn end_voice_call(
    call_id: &str,
    reason: Option<&str>,
    metadata: Option<&str>,
) -> Result<()> {
    let result =
        end_voice_call_data(call_id, SkillEndVoiceCallOptions { reason, metadata }).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Prewarm the greeting audio artifact for a configured voice plugin.
pub async fn prewarm_voice_plugin(
    plugin_id: &str,
    greeting_text: Option<&str>,
    voice: Option<&str>,
) -> Result<()> {
    let result = prewarm_voice_plugin_data(
        plugin_id,
        SkillPrewarmVoicePluginOptions {
            greeting_text,
            voice,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Reap stale bounded voice-call sessions.
pub async fn reap_voice_calls(stale_after_secs: Option<u64>, limit: Option<usize>) -> Result<()> {
    let result = reap_voice_calls_data(SkillReapVoiceCallsOptions {
        stale_after_secs,
        limit,
    })
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Search for skills in the ClawHub registry.
pub async fn search(query: &str, category: Option<&str>, sort: &str) -> Result<()> {
    println!("🔍 Searching for: {}", query);

    // Parse sort option
    let sort_by = match sort.to_lowercase().as_str() {
        "downloads" => SortBy::Downloads,
        "rating" => SortBy::Rating,
        "recent" => SortBy::Recent,
        _ => SortBy::Relevance,
    };

    let filters = SearchFilters {
        category: category.map(|s| s.to_string()),
        sort_by,
        min_rating: None,
        verified_only: false,
    };

    // Get registry endpoint from config or use default
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    // Create registry client
    let registry = ClawHubRegistry::new(&endpoint)
        .await
        .context("Failed to connect to ClawHub registry")?;

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Searching registry...");

    // Search
    let results = registry.search(query, filters).await;

    pb.finish_and_clear();

    match results {
        Ok(skills) => {
            if skills.is_empty() {
                println!("No skills found matching '{}'", query);
                return Ok(());
            }

            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║              Search Results                              ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("Found {} skill(s) matching '{}'", skills.len(), query);
            println!();

            for skill in skills {
                let rating = if skill.rating_count > 0 {
                    format!("★ {:.1} ({})", skill.rating, skill.rating_count)
                } else {
                    "★ No ratings".to_string()
                };

                println!("📦 {} \x1b[32mv{}\x1b[0m", skill.name, skill.version);
                println!("   {}", skill.description);
                println!(
                    "   Author: {} | Downloads: {} | {}",
                    skill.author, skill.downloads, rating
                );

                if !skill.categories.is_empty() {
                    println!("   Categories: {}", skill.categories.join(", "));
                }

                println!();
                println!("   Install: openrustclaw skills install {}", skill.name);
                println!();
            }
        }
        Err(e) => {
            println!("⚠ Failed to search registry: {}", e);
            println!();
            println!("You can still install skills from local files:");
            println!("  openrustclaw skills install <skill-name>");
        }
    }

    Ok(())
}

/// Install a skill from marketplace.
pub async fn install(name: &str) -> Result<()> {
    println!("Installing skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill already exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        println!("Skill '{}' is already installed.", name);
        println!("Use `openrustclaw skills update {}` to update it.", name);
        println!(
            "Use `openrustclaw skills verify {}` to verify its signature.",
            name
        );
        return Ok(());
    }

    let skills_dir = std::path::Path::new("skills");
    let expected_skill_path = Path::new("skills").join(name).join("SKILL.md");
    if !skills_dir.exists() {
        tokio::fs::create_dir_all(skills_dir).await?;
    }

    if let Some(skill_file) = resolve_workspace_skill_file(name) {
        // Load and parse the skill
        let content = tokio::fs::read_to_string(&skill_file).await?;

        // Parse SKILL.md format
        let metadata = parse_skill_metadata(&content)?;
        let capabilities_json = serialize_capabilities(&metadata.capabilities)?;

        upsert_skill_record(
            &pool,
            &metadata.name,
            metadata.description.as_deref(),
            "workspace",
            metadata.version.as_deref(),
            metadata.signature.as_deref(),
            true,
            Some(&capabilities_json),
            metadata.schema.as_deref(),
        )
        .await?;

        publish_plugin_event(
            &pool,
            "plugin.skill_installed",
            serde_json::json!({
                "name": metadata.name,
                "source": "workspace",
                "capabilities": metadata.capabilities,
            }),
        )
        .await;

        println!("✓ Skill '{}' installed successfully", name);
        println!("  Path: {}", skill_file.display());
        println!("  Source: workspace");
        if !metadata.capabilities.is_empty() {
            println!("  Capabilities: {}", metadata.capabilities.join(", "));
        }
        let _ =
            compile_skill_target(&metadata.name, &skill_file, SkillSource::Workspace, true).await;
    } else {
        // Try to install from ClawHub registry
        println!("Skill not found locally. Checking ClawHub registry...");

        let endpoint = config
            .skills
            .clone()
            .and_then(|s| s.registry_url)
            .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

        match ClawHubRegistry::new(&endpoint).await {
            Ok(registry) => {
                let registry_metadata = registry.get_skill(name).await;
                let registry_metadata = match registry_metadata {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        println!("✗ Failed to fetch skill metadata from ClawHub: {}", e);
                        println!();
                        println!(
                            "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                        );
                        println!(
                            "Expected file: {}",
                            Path::new("skills").join(name).join("SKILL.md").display()
                        );
                        return Ok(());
                    }
                };

                enforce_external_skill_policy(
                    &config,
                    &registry_metadata.capabilities,
                    registry_metadata.signature.as_deref(),
                    name,
                )?;

                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap(),
                );
                pb.set_message("Downloading from ClawHub...");

                match registry.install(name, None).await {
                    Ok(result) => {
                        pb.finish_and_clear();
                        match result {
                            openrustclaw_skills::InstallResult::AlreadyInstalled => {
                                println!("Skill '{}' is already installed.", name);
                            }
                            openrustclaw_skills::InstallResult::Installed {
                                name,
                                version,
                                path,
                            } => {
                                let capabilities_json =
                                    serialize_capabilities(&registry_metadata.capabilities)?;
                                let version_string = version.to_string();
                                upsert_skill_record(
                                    &pool,
                                    &name,
                                    Some(&registry_metadata.description),
                                    "marketplace",
                                    Some(version_string.as_str()),
                                    registry_metadata.signature.as_deref(),
                                    false,
                                    Some(&capabilities_json),
                                    None,
                                )
                                .await?;

                                publish_plugin_event(
                                    &pool,
                                    "plugin.skill_installed",
                                    serde_json::json!({
                                        "name": name,
                                        "source": "marketplace",
                                        "version": version_string,
                                    }),
                                )
                                .await;

                                println!("✓ Skill '{}' v{} installed successfully", name, version);
                                println!("  Path: {}", path.display());
                                let skill_file = path.join("SKILL.md");
                                if skill_file.exists() {
                                    let _ = compile_skill_target(
                                        &name,
                                        &skill_file,
                                        SkillSource::Marketplace,
                                        false,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        pb.finish_and_clear();
                        println!("✗ Failed to install from ClawHub: {}", e);
                        println!();
                        println!(
                            "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                        );
                        println!("Expected file: {}", expected_skill_path.display());
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to connect to ClawHub: {}", e);
                println!();
                println!(
                    "To create a new skill locally, add a SKILL.md file to the ./skills directory."
                );
                println!("Expected file: {}", expected_skill_path.display());
            }
        }
    }

    Ok(())
}

/// Update an installed skill.
pub async fn update(name: &str) -> Result<()> {
    println!("Updating skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        println!("Skill '{}' is not installed.", name);
        println!("Use `openrustclaw skills install {}` to install it.", name);
        return Ok(());
    }

    // Try to update via ClawHub registry
    let endpoint = config
        .skills
        .clone()
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for updates...");

            match registry.update(name).await {
                Ok(result) => {
                    pb.finish_and_clear();
                    match result {
                        openrustclaw_skills::UpdateResult::UpToDate => {
                            println!("✓ Skill '{}' is already up to date.", name);
                        }
                        openrustclaw_skills::UpdateResult::Updated { from, to } => {
                            println!("✓ Skill '{}' updated from v{} to v{}", name, from, to);

                            let metadata = registry.get_skill(name).await?;
                            enforce_external_skill_policy(
                                &config,
                                &metadata.capabilities,
                                metadata.signature.as_deref(),
                                name,
                            )?;
                            let capabilities_json = serialize_capabilities(&metadata.capabilities)?;
                            let version_string = to.to_string();
                            upsert_skill_record(
                                &pool,
                                name,
                                Some(&metadata.description),
                                "marketplace",
                                Some(version_string.as_str()),
                                metadata.signature.as_deref(),
                                false,
                                Some(&capabilities_json),
                                None,
                            )
                            .await?;

                            publish_plugin_event(
                                &pool,
                                "plugin.skill_updated",
                                serde_json::json!({
                                    "name": name,
                                    "from": from.to_string(),
                                    "to": version_string,
                                }),
                            )
                            .await;
                            let _ = compile_skill_by_name_internal(name).await;
                        }
                    }
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("✗ Failed to update: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Uninstall a skill.
pub async fn uninstall(name: &str) -> Result<()> {
    println!("Uninstalling skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Check if skill exists
    let existing: Option<String> = sqlx::query_scalar("SELECT name FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    if existing.is_none() {
        println!("Skill '{}' is not installed.", name);
        return Ok(());
    }

    // Try to uninstall via ClawHub registry first
    let endpoint = config
        .skills
        .clone()
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    if let Ok(registry) = ClawHubRegistry::new(&endpoint).await
        && let Err(e) = registry.uninstall(name).await
    {
        debug!("Failed to uninstall via registry: {}", e);
    }

    // Remove from database
    sqlx::query("DELETE FROM skills WHERE name = ?")
        .bind(name)
        .execute(&pool)
        .await?;
    let _ = remove_compiled_artifact(&compiled_skill_root(), name);

    publish_plugin_event(
        &pool,
        "plugin.skill_uninstalled",
        serde_json::json!({
            "name": name,
        }),
    )
    .await;

    println!("✓ Skill '{}' uninstalled successfully.", name);

    Ok(())
}

/// Verify skill signatures.
pub async fn verify(name: &str) -> Result<()> {
    println!("Verifying skill: {}", name);

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool and run migrations
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;

    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Get skill from database
    let row = sqlx::query("SELECT id, signature, source, verified FROM skills WHERE name = ?")
        .bind(name)
        .fetch_optional(&pool)
        .await?;

    let Some(row) = row else {
        anyhow::bail!("Skill '{}' not found", name);
    };

    let id: String = row.get("id");
    let signature: Option<String> = row.get("signature");
    let source: String = row.get("source");
    let verified: i64 = row.get("verified");

    // Workspace and bundled skills are exempt from verification
    if source == "workspace" || source == "bundled" {
        println!(
            "✓ Skill '{}' is from {} - verification exempt",
            name, source
        );

        persist_verified_state(&pool, &id, true).await?;
        let _ = compile_skill_by_name_internal(name).await;
        publish_plugin_event(
            &pool,
            "plugin.skill_verified",
            serde_json::json!({
                "name": name,
                "source": source,
                "verified": true,
                "verification_mode": "exempt",
            }),
        )
        .await;

        return Ok(());
    }

    if verified == 1 {
        println!(
            "✓ Skill '{}' is already verified from its managed install record",
            name
        );
        return Ok(());
    }

    // Load skill content
    let Some(skill_file) = resolve_workspace_skill_file(name) else {
        anyhow::bail!(
            "Skill '{}' is not a workspace skill and no local SKILL.md was found for manual verification",
            name
        );
    };

    let content = tokio::fs::read(&skill_file).await?;

    // External skills require a configured public key for real verification.
    let verifier = load_configured_skill_verifier(&config, true)?;

    // Check if skill has signature
    let Some(sig_hex) = signature else {
        println!("✗ Skill '{}' has no signature", name);
        persist_verified_state(&pool, &id, false).await?;
        return Ok(());
    };

    // Parse signature
    let signature_bytes = hex::decode(&sig_hex).context("Invalid signature format in database")?;

    // Verify
    match verifier.verify(&content, &signature_bytes) {
        Ok(true) => {
            println!("✓ Skill '{}' signature verified successfully", name);
            persist_verified_state(&pool, &id, true).await?;
            let _ = compile_skill_by_name_internal(name).await;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": true,
                    "verification_mode": "signature",
                }),
            )
            .await;
        }
        Ok(false) => {
            println!("✗ Skill '{}' signature verification failed", name);
            persist_verified_state(&pool, &id, false).await?;
            let _ = compile_skill_by_name_internal(name).await;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": false,
                    "verification_mode": "signature",
                }),
            )
            .await;
        }
        Err(e) => {
            println!("✗ Error verifying skill '{}': {}", name, e);
            persist_verified_state(&pool, &id, false).await?;
            let _ = compile_skill_by_name_internal(name).await;
            publish_plugin_event(
                &pool,
                "plugin.skill_verified",
                serde_json::json!({
                    "name": name,
                    "source": source,
                    "verified": false,
                    "verification_mode": "error",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    }

    Ok(())
}

/// Show popular skills from the registry.
pub async fn popular(limit: usize) -> Result<()> {
    println!("🔥 Popular Skills");
    println!();

    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Fetching popular skills...");

            match registry.get_popular(limit).await {
                Ok(skills) => {
                    pb.finish_and_clear();

                    if skills.is_empty() {
                        println!("No popular skills found.");
                        return Ok(());
                    }

                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║              Popular Skills                              ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    for (i, skill) in skills.iter().enumerate() {
                        let rating = if skill.rating_count > 0 {
                            format!("★ {:.1}", skill.rating)
                        } else {
                            "★ -".to_string()
                        };

                        println!(
                            "{}. 📦 {} \x1b[32mv{}\x1b[0m",
                            i + 1,
                            skill.name,
                            skill.version
                        );
                        println!("   {}", skill.description);
                        println!(
                            "   {} downloads | {} | {}",
                            skill.downloads, rating, skill.author
                        );
                        println!();
                    }

                    println!("Install a skill with: openrustclaw skills install <name>");
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("⚠ Failed to fetch popular skills: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Show trending skills from the registry.
pub async fn trending(limit: usize) -> Result<()> {
    println!("📈 Trending Skills");
    println!();

    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let endpoint = config
        .skills
        .and_then(|s| s.registry_url)
        .unwrap_or_else(|| "https://clawhub.openrustclaw.dev".to_string());

    match ClawHubRegistry::new(&endpoint).await {
        Ok(registry) => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Fetching trending skills...");

            match registry.get_trending(limit).await {
                Ok(skills) => {
                    pb.finish_and_clear();

                    if skills.is_empty() {
                        println!("No trending skills found.");
                        return Ok(());
                    }

                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║              Trending Skills                             ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    for (i, skill) in skills.iter().enumerate() {
                        let rating = if skill.rating_count > 0 {
                            format!("★ {:.1}", skill.rating)
                        } else {
                            "★ -".to_string()
                        };

                        println!(
                            "{}. 🚀 {} \x1b[32mv{}\x1b[0m",
                            i + 1,
                            skill.name,
                            skill.version
                        );
                        println!("   {}", skill.description);
                        println!(
                            "   {} downloads | {} | {}",
                            skill.downloads, rating, skill.author
                        );
                        println!();
                    }

                    println!("Install a skill with: openrustclaw skills install <name>");
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("⚠ Failed to fetch trending skills: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Failed to connect to ClawHub: {}", e);
        }
    }

    Ok(())
}

/// Parsed skill metadata from SKILL.md format.
struct ParsedSkillMetadata {
    name: String,
    description: Option<String>,
    version: Option<String>,
    capabilities: Vec<String>,
    schema: Option<String>,
    signature: Option<String>,
}

/// Parse SKILL.md metadata.
fn parse_skill_metadata(content: &str) -> Result<ParsedSkillMetadata> {
    // Simple frontmatter parser
    let mut name = "unknown".to_string();
    let mut description = None;
    let mut version = None;
    let mut capabilities = Vec::new();
    let mut schema = None;
    let mut signature = None;

    // Extract name from title (# Title)
    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            name = title.trim().to_string();
            break;
        }
    }

    // Extract description from first paragraph after title
    let lines: Vec<_> = content.lines().collect();
    for (i, line_text) in lines.iter().enumerate() {
        if line_text.starts_with("# ") && i + 1 < lines.len() {
            // Find first non-empty line after title
            for other_line in &lines[(i + 1)..] {
                if !other_line.trim().is_empty() && !other_line.starts_with("##") {
                    description = Some(other_line.trim().to_string());
                    break;
                }
            }
            break;
        }
    }

    // Parse version if present in frontmatter or metadata section
    for line in content.lines() {
        if line.starts_with("version:") || line.starts_with("Version:") {
            version = line.split_once(':').map(|(_, s)| s.trim().to_string());
        }
        if line.starts_with("capabilities:") || line.starts_with("Capabilities:") {
            if let Some((_, raw)) = line.split_once(':') {
                capabilities = raw
                    .split(',')
                    .map(|value| value.trim().trim_matches('"'))
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
        if line.starts_with("signature:") || line.starts_with("Signature:") {
            signature = line
                .split_once(':')
                .map(|(_, s)| s.trim().trim_matches('"').to_string())
                .filter(|value| !value.is_empty());
        }
    }

    // Extract JSON schema if present
    if let Some(start) = content.find("```json")
        && let Some(end) = content[start + 7..].find("```")
    {
        schema = Some(content[start + 7..start + 7 + end].trim().to_string());
    }

    Ok(ParsedSkillMetadata {
        name,
        description,
        version,
        capabilities,
        schema,
        signature,
    })
}

// Simple hex decode helper for when the hex crate isn't available
mod hex {
    pub fn decode(s: &str) -> anyhow::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(s.len() / 2);
        let chars: Vec<_> = s.chars().collect();

        for chunk in chars.chunks(2) {
            if chunk.len() != 2 {
                anyhow::bail!("Invalid hex string length");
            }
            let high = chunk[0]
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
            let low = chunk[1]
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("Invalid hex char"))?;
            result.push((high << 4 | low) as u8);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- hex module tests ---

    #[test]
    fn test_hex_decode_valid() {
        let result = hex::decode("48656c6c6f").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_hex_decode_empty() {
        let result = hex::decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_hex_decode_all_zeros() {
        let result = hex::decode("000000").unwrap();
        assert_eq!(result, vec![0, 0, 0]);
    }

    #[test]
    fn test_hex_decode_all_ff() {
        let result = hex::decode("ffffff").unwrap();
        assert_eq!(result, vec![0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_hex_decode_invalid_char() {
        let result = hex::decode("zz");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_decode_odd_length() {
        let result = hex::decode("abc");
        // odd length: last chunk is 1 char, so it should fail
        assert!(result.is_err());
    }

    // --- parse_skill_metadata tests ---

    #[test]
    fn test_parse_skill_metadata_basic() {
        let content = "# My Skill\n\nThis is a great skill.\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "My Skill");
        assert_eq!(
            metadata.description.as_deref(),
            Some("This is a great skill.")
        );
    }

    #[test]
    fn test_parse_skill_metadata_with_version() {
        let content = "# Test Skill\n\nA test.\n\nversion: 1.2.3\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "Test Skill");
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn test_parse_skill_metadata_with_capabilities() {
        let content = "# Net Skill\n\nA net skill.\n\ncapabilities: network_access, file_read\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.capabilities, vec!["network_access", "file_read"]);
    }

    #[test]
    fn test_parse_skill_metadata_with_json_schema() {
        let content = "# Schema Skill\n\nHas schema.\n\n```json\n{\"type\": \"object\"}\n```\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.schema.as_deref(), Some("{\"type\": \"object\"}"));
    }

    #[test]
    fn test_parse_skill_metadata_with_signature() {
        let content = "# Signed Skill\n\nSigned.\n\nsignature: deadbeef\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.signature.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn test_parse_skill_metadata_no_title() {
        let content = "No title here\n\nJust text.\n";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "unknown");
    }

    #[test]
    fn test_parse_skill_metadata_empty_content() {
        let content = "";
        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "unknown");
        assert!(metadata.description.is_none());
        assert!(metadata.version.is_none());
        assert!(metadata.capabilities.is_empty());
    }

    #[test]
    fn test_serialize_capabilities_normalizes_and_deduplicates() {
        let serialized = serialize_capabilities(&[
            "network".to_string(),
            "file-read".to_string(),
            "network_access".to_string(),
        ])
        .unwrap();
        assert_eq!(serialized, r#"["file_read","network_access"]"#);
    }

    #[test]
    fn test_serialize_capabilities_rejects_unknown_values() {
        let error = serialize_capabilities(&["launch_missiles".to_string()]).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("Failed to validate skill capabilities")
                || message.contains("Unknown skill capability")
        );
    }

    #[test]
    fn test_sensitive_capabilities_filters_safe_entries() {
        let sensitive = sensitive_capabilities(&[
            "network_access".to_string(),
            "file_read".to_string(),
            "shell_exec".to_string(),
        ])
        .unwrap();
        assert_eq!(
            sensitive,
            vec!["network_access".to_string(), "shell_exec".to_string()]
        );
    }

    #[test]
    fn test_external_skill_policy_rejects_unsigned_sensitive_skills() {
        let config = openrustclaw_core::config::AppConfig::default();
        let error = enforce_external_skill_policy(
            &config,
            &["shell_exec".to_string()],
            None,
            "dangerous-skill",
        )
        .unwrap_err();
        assert!(error.to_string().contains("must be signed"));
    }

    #[test]
    fn test_external_skill_policy_allows_signed_sensitive_skills() {
        let config = openrustclaw_core::config::AppConfig::default();
        enforce_external_skill_policy(
            &config,
            &["shell_exec".to_string()],
            Some("deadbeef"),
            "signed-skill",
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_persist_verified_state_updates_row() {
        let db_path =
            std::env::temp_dir().join(format!("skills-verify-{}.db", uuid::Uuid::new_v4()));
        let pool = openrustclaw_db::init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        openrustclaw_db::run_migrations(&pool).await.unwrap();

        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, description, source, version, signature,
                verified, enabled, capabilities, schema, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
        )
        .bind("skill-1")
        .bind("test-skill")
        .bind("desc")
        .bind("managed")
        .bind("1.0.0")
        .bind(Option::<String>::None)
        .bind(1i64)
        .bind(1i64)
        .bind("[]")
        .bind(Option::<String>::None)
        .execute(&pool)
        .await
        .unwrap();

        persist_verified_state(&pool, "skill-1", false)
            .await
            .unwrap();
        let verified: i64 = sqlx::query_scalar("SELECT verified FROM skills WHERE id = ?")
            .bind("skill-1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(verified, 0);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_invoke_compiled_skill_includes_reference_preview() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("demo");
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "# Demo Skill\n\nUseful compiled skill.\n",
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("references").join("guide.md"),
            "This is the guide for the demo skill.",
        )
        .unwrap();
        let artifact = compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &tmp.path().join("compiled"),
            SkillSource::Workspace,
            true,
        )
        .unwrap();

        let result = invoke_compiled_skill(
            &artifact,
            SkillInvokeOptions {
                args: Some("--topic rust"),
                reference: Some("references/guide.md"),
                max_chars: Some(4),
                detail: true,
            },
        )
        .unwrap();

        assert_eq!(result.skill_name, "Demo Skill");
        assert_eq!(
            result.command_preview,
            "openrustclaw skills invoke Demo Skill --topic rust"
        );
        assert_eq!(result.reference_result.as_ref().unwrap()["content"], "This");
        assert_eq!(result.reference_result.as_ref().unwrap()["truncated"], true);
        assert!(
            result.invocation["help"]["body_excerpt"]
                .as_str()
                .unwrap()
                .contains("Useful compiled skill.")
        );
    }

    #[test]
    fn test_invoke_compiled_skill_rejects_blocked_reference_access() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("danger");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Danger Skill\n\nNope.\n").unwrap();
        std::fs::write(skill_dir.join("scripts").join("run.sh"), "rm -rf /\n").unwrap();
        std::fs::write(
            skill_dir.join("references").join("guide.md"),
            "blocked reference",
        )
        .unwrap();
        let artifact = compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &tmp.path().join("compiled"),
            SkillSource::Marketplace,
            false,
        )
        .unwrap();

        let error = invoke_compiled_skill(
            &artifact,
            SkillInvokeOptions {
                args: None,
                reference: Some("references/guide.md"),
                max_chars: None,
                detail: true,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("blocked"));
    }

    #[tokio::test]
    async fn test_execute_compiled_skill_runs_wat_component() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("runner");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: runner
description: "Runs a bounded wasm component"
capabilities:
  - file_read
---

# Runner
"#,
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("scripts").join("echo.wat"),
            r#"(module
              (memory (export "memory") 1 1)
              (data (i32.const 1024) "{\"ok\":true}")
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64)
                (i64.or
                  (i64.shl (i64.extend_i32_u (i32.const 1024)) (i64.const 32))
                  (i64.extend_i32_u (i32.const 11)))))"#,
        )
        .unwrap();
        let artifact = compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &tmp.path().join("compiled"),
            SkillSource::Workspace,
            true,
        )
        .unwrap();

        let result = execute_compiled_skill(
            &artifact,
            SkillExecuteOptions {
                component: Some("scripts/echo.wat"),
                input: Some(r#"{"text":"hello"}"#),
            },
        )
        .await
        .unwrap();

        assert_eq!(result.skill_name, "runner");
        assert_eq!(result.component, "scripts/echo.wat");
        assert_eq!(result.output, serde_json::json!({"ok": true}));
        assert_eq!(result.input, serde_json::json!({"text": "hello"}));
    }
}
