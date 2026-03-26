//! File-backed local tool profiling and host-artifact generation.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub const DEFAULT_TOOL_REGISTRY_PATH: &str = ".claw/control/tool-profiles.json";
pub const DEFAULT_TOOL_ARTIFACT_ROOT: &str = ".claw/control/tool-hosts";
const DEFAULT_PROBE_TIMEOUT_SECS: u64 = 5;
const DEFAULT_COMMON_TOOLS: &[&str] = &["git", "gh", "cargo", "docker"];

fn default_registry_version() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum AiHost {
    ClaudeCode,
    Cursor,
    Codex,
    GeminiCli,
    GithubCopilot,
}

impl AiHost {
    pub fn as_id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::GeminiCli => "gemini-cli",
            Self::GithubCopilot => "github-copilot",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Codex => "Codex",
            Self::GeminiCli => "Gemini CLI",
            Self::GithubCopilot => "GitHub Copilot",
        }
    }

    fn startup_hint(self) -> &'static str {
        match self {
            Self::ClaudeCode => {
                "Use this as a compact startup reference for local tool behavior and safe invocation patterns."
            }
            Self::Cursor => {
                "Use this alongside project rules or MCP setup so the IDE agent sees the same local tool contract."
            }
            Self::Codex => {
                "Use this as a deterministic briefing for command discovery instead of relying on ad hoc probing."
            }
            Self::GeminiCli => {
                "Use this as a repo-local tool briefing before asking the model to call or wrap the tool."
            }
            Self::GithubCopilot => {
                "Use this as a workspace reference for local CLI behavior and common operator-safe patterns."
            }
        }
    }

    fn from_id(value: &str) -> Option<Self> {
        match value {
            "claude-code" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            "codex" => Some(Self::Codex),
            "gemini-cli" => Some(Self::GeminiCli),
            "github-copilot" => Some(Self::GithubCopilot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGeneratedArtifact {
    pub host: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProfile {
    pub name: String,
    pub executable: String,
    pub executable_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub help_digest: String,
    pub executable_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(default)]
    pub detected_subcommands: Vec<String>,
    #[serde(default)]
    pub detected_flags: Vec<String>,
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub generated_artifacts: Vec<ToolGeneratedArtifact>,
    pub inspected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistry {
    #[serde(default = "default_registry_version")]
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ToolProfile>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self {
            version: default_registry_version(),
            updated_at: None,
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatusEntry {
    pub profile: ToolProfile,
    pub available: bool,
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_executable_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatusReport {
    pub registry_path: String,
    pub artifact_root: String,
    pub entries: Vec<ToolStatusEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDetailReport {
    pub registry_path: String,
    pub artifact_root: String,
    pub entry: ToolStatusEntry,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolAddReport {
    pub registry_path: String,
    pub artifact_root: String,
    pub profile: ToolProfile,
    pub generated_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSetupReport {
    pub registry_path: String,
    pub artifact_root: String,
    pub added: Vec<String>,
    pub skipped: Vec<String>,
    pub generated_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSyncChange {
    pub name: String,
    pub changed: bool,
    pub stale_before_sync: bool,
    pub applied: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSyncReport {
    pub registry_path: String,
    pub artifact_root: String,
    pub applied: bool,
    pub changes: Vec<ToolSyncChange>,
}

pub fn tool_registry_path_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_TOOL_REGISTRY_PATH)
}

pub fn tool_artifact_root_for(workspace_root: impl AsRef<Path>) -> PathBuf {
    workspace_root.as_ref().join(DEFAULT_TOOL_ARTIFACT_ROOT)
}

pub fn load_registry(workspace_root: impl AsRef<Path>) -> Result<ToolRegistry> {
    let path = tool_registry_path_for(workspace_root);
    if !path.exists() {
        return Ok(ToolRegistry::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_registry(workspace_root: impl AsRef<Path>, registry: &ToolRegistry) -> Result<()> {
    let path = tool_registry_path_for(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let rendered =
        serde_json::to_string_pretty(registry).context("Failed to serialize tool registry")?;
    fs::write(&path, rendered.as_bytes())
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub async fn add_tool(
    workspace_root: &Path,
    name: &str,
    explicit_path: Option<&str>,
    hosts: &[AiHost],
) -> Result<ToolAddReport> {
    let mut registry = load_registry(workspace_root)?;
    let requested_hosts = normalized_hosts(None, hosts);
    let mut profile = probe_tool(name, explicit_path, &requested_hosts).await?;
    registry
        .profiles
        .insert(profile.name.clone(), profile.clone());
    let generated = sync_artifacts(workspace_root, &mut registry, Some(&[profile.name.clone()]))?;
    registry.updated_at = Some(Utc::now().to_rfc3339());
    profile = registry
        .profiles
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("Tool profile '{}' was not persisted", name))?;
    save_registry(workspace_root, &registry)?;

    Ok(ToolAddReport {
        registry_path: display_path(tool_registry_path_for(workspace_root)),
        artifact_root: display_path(tool_artifact_root_for(workspace_root)),
        profile,
        generated_paths: generated,
    })
}

pub async fn setup_tools(
    workspace_root: &Path,
    tool_names: &[String],
    hosts: &[AiHost],
) -> Result<ToolSetupReport> {
    let requested = if tool_names.is_empty() {
        DEFAULT_COMMON_TOOLS
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>()
    } else {
        tool_names.to_vec()
    };

    let mut added = Vec::new();
    let mut skipped = Vec::new();
    let mut generated_paths = Vec::new();

    for tool_name in requested {
        match add_tool(workspace_root, &tool_name, None, hosts).await {
            Ok(report) => {
                added.push(report.profile.name.clone());
                generated_paths.extend(report.generated_paths);
            }
            Err(error) if error.to_string().contains("not found on PATH") => {
                skipped.push(format!("{} (not found)", tool_name));
            }
            Err(error) => return Err(error),
        }
    }

    generated_paths.sort();
    generated_paths.dedup();

    Ok(ToolSetupReport {
        registry_path: display_path(tool_registry_path_for(workspace_root)),
        artifact_root: display_path(tool_artifact_root_for(workspace_root)),
        added,
        skipped,
        generated_paths,
    })
}

pub fn status_data(workspace_root: &Path, name: Option<&str>) -> Result<ToolStatusReport> {
    let registry = load_registry(workspace_root)?;
    let mut entries = registry
        .profiles
        .values()
        .filter(|profile| name.is_none_or(|filter| profile.name == filter))
        .map(tool_status_entry)
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by(|left, right| left.profile.name.cmp(&right.profile.name));
    Ok(ToolStatusReport {
        registry_path: display_path(tool_registry_path_for(workspace_root)),
        artifact_root: display_path(tool_artifact_root_for(workspace_root)),
        entries,
    })
}

pub fn detail_data(workspace_root: &Path, name: &str) -> Result<ToolDetailReport> {
    let registry = load_registry(workspace_root)?;
    let profile = registry
        .profiles
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("Tool '{}' is not registered", name))?;

    Ok(ToolDetailReport {
        registry_path: display_path(tool_registry_path_for(workspace_root)),
        artifact_root: display_path(tool_artifact_root_for(workspace_root)),
        entry: tool_status_entry(&profile)?,
    })
}

pub async fn sync_tools(
    workspace_root: &Path,
    name: Option<&str>,
    hosts: &[AiHost],
    apply: bool,
) -> Result<ToolSyncReport> {
    let mut registry = load_registry(workspace_root)?;
    let target_names = registry
        .profiles
        .keys()
        .filter(|tool_name| name.is_none_or(|filter| *tool_name == filter))
        .cloned()
        .collect::<Vec<_>>();

    if target_names.is_empty() {
        bail!("No tool profiles matched the requested sync target");
    }

    let mut changes = Vec::new();

    for tool_name in &target_names {
        let existing = registry
            .profiles
            .get(tool_name)
            .cloned()
            .ok_or_else(|| anyhow!("Tool '{}' disappeared during sync", tool_name))?;
        let stale_before_sync = tool_status_entry(&existing)?.stale;
        let resolved_hosts = normalized_hosts(Some(&existing.hosts), hosts);

        if !Path::new(&existing.executable_path).exists() {
            changes.push(ToolSyncChange {
                name: existing.name.clone(),
                changed: false,
                stale_before_sync,
                applied: false,
                notes: vec!["executable is missing; registry entry left unchanged".to_string()],
                generated_paths: Vec::new(),
            });
            continue;
        }

        let refreshed = probe_tool(
            &existing.name,
            Some(existing.executable_path.as_str()),
            &resolved_hosts,
        )
        .await?;
        let changed = tool_profiles_changed(&existing, &refreshed);

        if apply {
            registry.profiles.insert(existing.name.clone(), refreshed);
        }

        changes.push(ToolSyncChange {
            name: existing.name.clone(),
            changed,
            stale_before_sync,
            applied: apply,
            notes: if changed {
                vec!["tool profile refreshed from local probe output".to_string()]
            } else {
                vec!["tool profile already matched the current local executable".to_string()]
            },
            generated_paths: Vec::new(),
        });
    }

    if apply {
        let generated = sync_artifacts(workspace_root, &mut registry, Some(&target_names))?;
        registry.updated_at = Some(Utc::now().to_rfc3339());
        save_registry(workspace_root, &registry)?;
        let generated_set = generated.into_iter().collect::<BTreeSet<_>>();
        for change in &mut changes {
            let profile = registry
                .profiles
                .get(&change.name)
                .ok_or_else(|| anyhow!("Tool '{}' missing after sync", change.name))?;
            change.generated_paths = profile
                .generated_artifacts
                .iter()
                .map(|artifact| artifact.path.clone())
                .filter(|path| generated_set.contains(path))
                .collect();
        }
    }

    Ok(ToolSyncReport {
        registry_path: display_path(tool_registry_path_for(workspace_root)),
        artifact_root: display_path(tool_artifact_root_for(workspace_root)),
        applied: apply,
        changes,
    })
}

fn tool_status_entry(profile: &ToolProfile) -> Result<ToolStatusEntry> {
    let executable_path = Path::new(&profile.executable_path);
    if !executable_path.exists() {
        return Ok(ToolStatusEntry {
            profile: profile.clone(),
            available: false,
            stale: true,
            drift_reason: Some("executable path is missing".to_string()),
            current_executable_sha256: None,
            current_modified_at: None,
        });
    }

    let current_sha = sha256_file(executable_path)?;
    let current_modified_at = file_modified_at(executable_path);
    let artifacts_missing = profile
        .generated_artifacts
        .iter()
        .any(|artifact| !Path::new(&artifact.path).exists());

    let (stale, drift_reason) = if current_sha != profile.executable_sha256 {
        (
            true,
            Some("local executable hash changed since the last inspection".to_string()),
        )
    } else if artifacts_missing {
        (
            true,
            Some("generated host artifacts are missing or incomplete".to_string()),
        )
    } else {
        (false, None)
    };

    Ok(ToolStatusEntry {
        profile: profile.clone(),
        available: true,
        stale,
        drift_reason,
        current_executable_sha256: Some(current_sha),
        current_modified_at,
    })
}

async fn probe_tool(
    name: &str,
    explicit_path: Option<&str>,
    hosts: &[AiHost],
) -> Result<ToolProfile> {
    let resolved = resolve_executable(name, explicit_path)?;
    let version_output = probe_command(&resolved, &["--version"]).await.ok();
    let help_output = probe_command(&resolved, &["--help"])
        .await
        .unwrap_or_default();
    let summary = extract_summary(&help_output);
    let detected_subcommands = extract_subcommands(&help_output);
    let detected_flags = extract_flags(&help_output);
    let executable_sha256 = sha256_file(&resolved)?;
    let help_digest = sha256_text(&help_output);

    Ok(ToolProfile {
        name: name.to_string(),
        executable: resolved
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(name)
            .to_string(),
        executable_path: display_path(&resolved),
        version_text: version_output.as_deref().and_then(extract_version),
        summary,
        help_digest,
        executable_sha256,
        modified_at: file_modified_at(&resolved),
        detected_subcommands,
        detected_flags,
        hosts: hosts.iter().map(|host| host.as_id().to_string()).collect(),
        generated_artifacts: Vec::new(),
        inspected_at: Utc::now().to_rfc3339(),
    })
}

fn resolve_executable(name: &str, explicit_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        let candidate = PathBuf::from(path);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            env::current_dir()
                .context("Failed to determine current directory")?
                .join(candidate)
        };
        return resolved
            .canonicalize()
            .with_context(|| format!("Failed to resolve executable path {}", resolved.display()));
    }

    let candidate = PathBuf::from(name);
    if candidate.components().count() > 1 && candidate.exists() {
        return candidate
            .canonicalize()
            .with_context(|| format!("Failed to resolve executable path {}", candidate.display()));
    }

    let path_env = env::var_os("PATH").ok_or_else(|| anyhow!("PATH is not configured"))?;
    for directory in env::split_paths(&path_env) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return candidate.canonicalize().with_context(|| {
                format!("Failed to resolve executable path {}", candidate.display())
            });
        }
    }

    bail!("Tool '{}' was not found on PATH", name)
}

async fn probe_command(executable: &Path, args: &[&str]) -> Result<String> {
    let output = timeout(
        Duration::from_secs(DEFAULT_PROBE_TIMEOUT_SECS),
        Command::new(executable)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output(),
    )
    .await
    .with_context(|| {
        format!(
            "Timed out probing {} {}",
            executable.display(),
            args.join(" ")
        )
    })??;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Ok(stdout);
        }
        return Ok(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    bail!(
        "Probe failed for {} {}: {}",
        executable.display(),
        args.join(" "),
        if !stderr.is_empty() { stderr } else { stdout }
    )
}

fn extract_version(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_summary(help_output: &str) -> Option<String> {
    help_output
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with("Usage:")
                && !line.starts_with("USAGE:")
                && !line.starts_with("Options:")
                && !line.starts_with("OPTIONS:")
                && !line.starts_with("Commands:")
                && !line.starts_with("SUBCOMMANDS:")
                && !line.starts_with('-')
        })
        .map(ToOwned::to_owned)
}

fn extract_subcommands(help_output: &str) -> Vec<String> {
    let mut in_section = false;
    let mut values = BTreeSet::new();
    for line in help_output.lines() {
        let trimmed = line.trim_end();
        let normalized = trimmed.trim();
        if normalized.eq_ignore_ascii_case("commands:")
            || normalized.eq_ignore_ascii_case("subcommands:")
            || normalized.eq_ignore_ascii_case("available commands:")
        {
            in_section = true;
            continue;
        }
        if in_section && normalized.is_empty() {
            in_section = false;
            continue;
        }
        if !in_section || normalized.is_empty() {
            continue;
        }

        let token = normalized
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(':');
        if !token.is_empty()
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            values.insert(token.to_string());
        }
    }
    values.into_iter().collect()
}

fn extract_flags(help_output: &str) -> Vec<String> {
    let regex = Regex::new(r"(?m)(--[a-zA-Z0-9][a-zA-Z0-9-]*)").expect("valid flag regex");
    let mut values = regex
        .captures_iter(help_output)
        .filter_map(|capture| capture.get(1))
        .map(|flag| flag.as_str().to_string())
        .collect::<BTreeSet<_>>();
    values.retain(|flag| !flag.is_empty());
    values.into_iter().collect()
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn sha256_text(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn file_modified_at(path: &Path) -> Option<String> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map(|timestamp| timestamp.to_rfc3339())
}

fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn normalized_hosts(existing: Option<&[String]>, requested: &[AiHost]) -> Vec<AiHost> {
    if !requested.is_empty() {
        return requested.to_vec();
    }

    let from_existing = existing
        .unwrap_or_default()
        .iter()
        .filter_map(|value| AiHost::from_id(value))
        .collect::<Vec<_>>();
    if !from_existing.is_empty() {
        return from_existing;
    }

    vec![
        AiHost::ClaudeCode,
        AiHost::Cursor,
        AiHost::Codex,
        AiHost::GeminiCli,
        AiHost::GithubCopilot,
    ]
}

fn tool_profiles_changed(left: &ToolProfile, right: &ToolProfile) -> bool {
    left.executable_sha256 != right.executable_sha256
        || left.help_digest != right.help_digest
        || left.version_text != right.version_text
        || left.detected_subcommands != right.detected_subcommands
        || left.detected_flags != right.detected_flags
        || left.hosts != right.hosts
}

fn sync_artifacts(
    workspace_root: &Path,
    registry: &mut ToolRegistry,
    target_names: Option<&[String]>,
) -> Result<Vec<String>> {
    let artifact_root = tool_artifact_root_for(workspace_root);
    fs::create_dir_all(&artifact_root)
        .with_context(|| format!("Failed to create {}", artifact_root.display()))?;

    let selected = target_names
        .map(|names| names.iter().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let limit_targets = target_names.is_some();

    let mut generated = Vec::new();
    for profile in registry.profiles.values_mut() {
        if limit_targets && !selected.contains(&profile.name) {
            continue;
        }

        profile.generated_artifacts.clear();
        let hosts = normalized_hosts(Some(&profile.hosts), &[]);
        profile.hosts = hosts.iter().map(|host| host.as_id().to_string()).collect();

        for host in &hosts {
            let host_root = artifact_root.join(host.as_id());
            fs::create_dir_all(&host_root)
                .with_context(|| format!("Failed to create {}", host_root.display()))?;

            let markdown_path = host_root.join(format!("{}.md", sanitize_name(&profile.name)));
            let json_path = host_root.join(format!("{}.json", sanitize_name(&profile.name)));

            fs::write(
                &markdown_path,
                render_host_markdown(*host, profile).as_bytes(),
            )
            .with_context(|| format!("Failed to write {}", markdown_path.display()))?;
            fs::write(
                &json_path,
                serde_json::to_string_pretty(profile)
                    .context("Failed to serialize tool profile")?
                    .as_bytes(),
            )
            .with_context(|| format!("Failed to write {}", json_path.display()))?;

            profile.generated_artifacts.push(ToolGeneratedArtifact {
                host: host.as_id().to_string(),
                kind: "startup_markdown".to_string(),
                path: display_path(&markdown_path),
            });
            profile.generated_artifacts.push(ToolGeneratedArtifact {
                host: host.as_id().to_string(),
                kind: "tool_profile_json".to_string(),
                path: display_path(&json_path),
            });
            generated.push(display_path(markdown_path));
            generated.push(display_path(json_path));
        }
    }

    write_host_indexes(&artifact_root, &registry.profiles)?;
    generated.sort();
    generated.dedup();
    Ok(generated)
}

fn write_host_indexes(
    artifact_root: &Path,
    profiles: &BTreeMap<String, ToolProfile>,
) -> Result<()> {
    let mut by_host: BTreeMap<String, Vec<&ToolProfile>> = BTreeMap::new();
    for profile in profiles.values() {
        for host in &profile.hosts {
            by_host.entry(host.clone()).or_default().push(profile);
        }
    }

    for (host_id, mut host_profiles) in by_host {
        host_profiles.sort_by(|left, right| left.name.cmp(&right.name));
        let host =
            AiHost::from_id(&host_id).ok_or_else(|| anyhow!("Unsupported host '{}'", host_id))?;
        let index_path = artifact_root.join(&host_id).join("STARTUP.md");
        fs::create_dir_all(index_path.parent().expect("host index parent exists"))
            .with_context(|| format!("Failed to create {}", index_path.display()))?;
        fs::write(
            &index_path,
            render_host_index(host, &host_profiles).as_bytes(),
        )
        .with_context(|| format!("Failed to write {}", index_path.display()))?;
    }

    Ok(())
}

fn render_host_markdown(host: AiHost, profile: &ToolProfile) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "# {} tool briefing for {}\n\n",
        profile.name,
        host.display_name()
    ));
    body.push_str(host.startup_hint());
    body.push_str("\n\n");
    body.push_str(&format!(
        "- Executable: `{}`\n- Path: `{}`\n",
        profile.executable, profile.executable_path
    ));
    if let Some(version) = &profile.version_text {
        body.push_str(&format!("- Version: `{}`\n", version));
    }
    if let Some(summary) = &profile.summary {
        body.push_str(&format!("- Summary: {}\n", summary));
    }
    body.push_str(&format!("- Inspected: `{}`\n", profile.inspected_at));
    body.push_str("\n## Suggested use\n\n");
    body.push_str(
        "Prefer these bounded invocation patterns before improvising new flags or subcommands.\n\n",
    );

    if !profile.detected_subcommands.is_empty() {
        body.push_str("### Discovered subcommands\n\n");
        for subcommand in &profile.detected_subcommands {
            body.push_str(&format!("- `{}`\n", subcommand));
        }
        body.push('\n');
    }

    if !profile.detected_flags.is_empty() {
        body.push_str("### Common flags\n\n");
        for flag in profile.detected_flags.iter().take(24) {
            body.push_str(&format!("- `{}`\n", flag));
        }
        body.push('\n');
    }

    body.push_str("### Safe operator note\n\n");
    body.push_str(
        "This profile came from local `--help` and `--version` probing. Re-run `openrustclaw tools sync` when the local executable changes.\n",
    );
    body
}

fn render_host_index(host: AiHost, profiles: &[&ToolProfile]) -> String {
    let mut body = format!("# {} startup bundle\n\n", host.display_name());
    body.push_str(host.startup_hint());
    body.push_str("\n\n");
    body.push_str("Generated tool briefings:\n\n");
    for profile in profiles {
        body.push_str(&format!(
            "- `{}`: {}{}\n",
            profile.name,
            profile
                .summary
                .as_deref()
                .unwrap_or("Local tool profile generated from CLI probing."),
            profile
                .version_text
                .as_ref()
                .map(|version| format!(" (`{}`)", version))
                .unwrap_or_default()
        ));
    }
    body
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn make_executable(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
        Ok(())
    }

    fn write_fake_tool(path: &Path, version: &str, help: &str) -> Result<()> {
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo '{}'\n  exit 0\nfi\nif [ \"$1\" = \"--help\" ]; then\n  cat <<'EOF'\n{}\nEOF\n  exit 0\nfi\necho \"unsupported\"\nexit 1\n",
            version, help
        );
        fs::write(path, script)?;
        #[cfg(unix)]
        make_executable(path)?;
        Ok(())
    }

    #[tokio::test]
    async fn add_tool_persists_profile_and_artifacts() -> Result<()> {
        let dir = tempdir()?;
        let tool_path = dir.path().join("fake-tool");
        write_fake_tool(
            &tool_path,
            "fake-tool 1.0.0",
            "Fake tool summary\n\nUsage: fake-tool [OPTIONS] <COMMAND>\n\nCommands:\n  sync      Sync state\n  status    Show status\n\nOptions:\n  --help     Show help\n  --json     JSON output\n",
        )?;

        let report = add_tool(
            dir.path(),
            "fake-tool",
            Some(tool_path.to_str().unwrap()),
            &[],
        )
        .await?;
        assert_eq!(report.profile.name, "fake-tool");
        assert!(
            report
                .profile
                .detected_subcommands
                .contains(&"sync".to_string())
        );
        assert!(
            report
                .profile
                .detected_flags
                .contains(&"--json".to_string())
        );
        assert!(tool_registry_path_for(dir.path()).exists());
        assert!(
            tool_artifact_root_for(dir.path())
                .join("cursor")
                .join("fake-tool.md")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn status_marks_missing_artifacts_as_stale() -> Result<()> {
        let dir = tempdir()?;
        let artifact_path = dir.path().join(".claw/control/tool-hosts/cursor/tool.md");
        let profile = ToolProfile {
            name: "tool".to_string(),
            executable: "tool".to_string(),
            executable_path: "/bin/sh".to_string(),
            version_text: Some("tool 1.0".to_string()),
            summary: Some("summary".to_string()),
            help_digest: "abc".to_string(),
            executable_sha256: sha256_file(Path::new("/bin/sh"))?,
            modified_at: file_modified_at(Path::new("/bin/sh")),
            detected_subcommands: vec![],
            detected_flags: vec![],
            hosts: vec!["cursor".to_string()],
            generated_artifacts: vec![ToolGeneratedArtifact {
                host: "cursor".to_string(),
                kind: "startup_markdown".to_string(),
                path: display_path(&artifact_path),
            }],
            inspected_at: Utc::now().to_rfc3339(),
        };
        let status = tool_status_entry(&profile)?;
        assert!(status.stale);
        Ok(())
    }

    #[tokio::test]
    async fn sync_refreshes_changed_tool_profile() -> Result<()> {
        let dir = tempdir()?;
        let tool_path = dir.path().join("fake-tool");
        write_fake_tool(
            &tool_path,
            "fake-tool 1.0.0",
            "Summary\n\nCommands:\n  status   Show status\n",
        )?;
        add_tool(
            dir.path(),
            "fake-tool",
            Some(tool_path.to_str().unwrap()),
            &[],
        )
        .await?;

        write_fake_tool(
            &tool_path,
            "fake-tool 2.0.0",
            "Summary\n\nCommands:\n  status   Show status\n  sync     Sync state\n",
        )?;

        let report = sync_tools(dir.path(), Some("fake-tool"), &[], true).await?;
        assert!(report.changes.iter().any(|change| change.changed));
        let detail = detail_data(dir.path(), "fake-tool")?;
        assert_eq!(
            detail.entry.profile.version_text.as_deref(),
            Some("fake-tool 2.0.0")
        );
        assert!(
            detail
                .entry
                .profile
                .detected_subcommands
                .contains(&"sync".to_string())
        );
        Ok(())
    }
}
