use crate::tool_host_service::{AiHost, ToolHostService};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendAuthStatus {
    LoggedIn,
    LoggedOut,
    Unknown,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendCapability {
    Supported,
    Candidate,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendReadiness {
    Ready,
    Candidate,
    DetectionOnly,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBackendCatalogEntry {
    pub host: AiHost,
    pub binary_name: String,
    pub detected: bool,
    pub executable_path: Option<String>,
    pub version_text: Option<String>,
    pub summary: Option<String>,
    pub auth_status: AgentBackendAuthStatus,
    pub auth_method: Option<String>,
    pub model_discovery: AgentBackendCapability,
    pub delegated_execution: AgentBackendCapability,
    pub policy_classification: String,
    pub readiness: AgentBackendReadiness,
    pub readiness_reason: Option<String>,
    pub detected_subcommands: Vec<String>,
    pub detected_flags: Vec<String>,
    pub inspected_at: String,
    pub notes: Vec<String>,
}

impl AgentBackendCatalogEntry {
    pub fn display_name(&self) -> &'static str {
        self.host.display_name()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentBackendCatalogService;

impl AgentBackendCatalogService {
    pub fn new() -> Self {
        Self
    }

    pub fn discover(&self) -> Vec<AgentBackendCatalogEntry> {
        [
            BackendProbe::claude_code(),
            BackendProbe::codex(),
            BackendProbe::gemini(),
            BackendProbe::cursor(),
        ]
        .into_iter()
        .map(|probe| self.inspect_probe(probe))
        .collect()
    }

    fn inspect_probe(&self, probe: BackendProbe) -> AgentBackendCatalogEntry {
        let inspected_at = chrono::Utc::now().to_rfc3339();
        let Some(executable_path) = resolve_on_path(probe.binary_name) else {
            return AgentBackendCatalogEntry {
                host: probe.host,
                binary_name: probe.binary_name.to_string(),
                detected: false,
                executable_path: None,
                version_text: None,
                summary: None,
                auth_status: if probe.auth_probe.is_some() {
                    AgentBackendAuthStatus::LoggedOut
                } else {
                    AgentBackendAuthStatus::NotSupported
                },
                auth_method: None,
                model_discovery: probe.model_discovery,
                delegated_execution: probe.delegated_execution,
                policy_classification: probe.policy_classification.to_string(),
                readiness: AgentBackendReadiness::Unavailable,
                readiness_reason: Some(format!("`{}` was not found on PATH", probe.binary_name)),
                detected_subcommands: Vec::new(),
                detected_flags: Vec::new(),
                inspected_at,
                notes: probe.notes.iter().map(|value| value.to_string()).collect(),
            };
        };

        let tool_host = ToolHostService::new();
        let version_output = run_command(&executable_path, &["--version"]).ok();
        let help_output = run_command(&executable_path, &["--help"]).ok();
        let summary = help_output
            .as_deref()
            .and_then(|value| tool_host.extract_summary(value));
        let detected_subcommands = help_output
            .as_deref()
            .map(|value| tool_host.extract_subcommands(value))
            .unwrap_or_default();
        let detected_flags = help_output
            .as_deref()
            .map(|value| tool_host.extract_flags(value))
            .unwrap_or_default();
        let (auth_status, auth_method) = probe
            .auth_probe
            .map(|args| infer_auth(&probe.host, run_command(&executable_path, args).ok()))
            .unwrap_or((AgentBackendAuthStatus::NotSupported, None));

        let (readiness, readiness_reason) =
            classify_readiness(&probe, &auth_status, help_output.as_deref());

        AgentBackendCatalogEntry {
            host: probe.host,
            binary_name: probe.binary_name.to_string(),
            detected: true,
            executable_path: Some(executable_path.display().to_string()),
            version_text: version_output
                .as_deref()
                .and_then(|value| tool_host.extract_version(value)),
            summary,
            auth_status,
            auth_method,
            model_discovery: probe.model_discovery,
            delegated_execution: probe.delegated_execution,
            policy_classification: probe.policy_classification.to_string(),
            readiness,
            readiness_reason,
            detected_subcommands,
            detected_flags,
            inspected_at,
            notes: probe.notes.iter().map(|value| value.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct BackendProbe {
    host: AiHost,
    binary_name: &'static str,
    auth_probe: Option<&'static [&'static str]>,
    model_discovery: AgentBackendCapability,
    delegated_execution: AgentBackendCapability,
    policy_classification: &'static str,
    notes: &'static [&'static str],
}

impl BackendProbe {
    fn claude_code() -> Self {
        Self {
            host: AiHost::ClaudeCode,
            binary_name: "claude",
            auth_probe: Some(&["auth", "status"]),
            model_discovery: AgentBackendCapability::Unsupported,
            delegated_execution: AgentBackendCapability::Candidate,
            policy_classification: "delegated_cli_candidate",
            notes: &[
                "Third-party products should use documented Anthropic API or cloud-provider integrations instead of rehosting claude.ai login.",
            ],
        }
    }

    fn codex() -> Self {
        Self {
            host: AiHost::Codex,
            binary_name: "codex",
            auth_probe: Some(&["login", "status"]),
            model_discovery: AgentBackendCapability::Unsupported,
            delegated_execution: AgentBackendCapability::Candidate,
            policy_classification: "delegated_cli_candidate",
            notes: &[
                "Use documented Codex CLI login or OpenAI API credentials rather than importing cached credentials.",
            ],
        }
    }

    fn gemini() -> Self {
        Self {
            host: AiHost::GeminiCli,
            binary_name: "gemini",
            auth_probe: None,
            model_discovery: AgentBackendCapability::Unsupported,
            delegated_execution: AgentBackendCapability::Candidate,
            policy_classification: "delegated_cli_candidate",
            notes: &[
                "Gemini CLI supports Google sign-in, Gemini API keys, and Vertex AI, and the selected auth path changes terms and pricing.",
            ],
        }
    }

    fn cursor() -> Self {
        Self {
            host: AiHost::Cursor,
            binary_name: "cursor",
            auth_probe: None,
            model_discovery: AgentBackendCapability::Unknown,
            delegated_execution: AgentBackendCapability::Unsupported,
            policy_classification: "integration_only",
            notes: &[
                "Treat Cursor conservatively until a documented programmable execution surface is confirmed.",
            ],
        }
    }
}

fn resolve_on_path(name: &str) -> Option<PathBuf> {
    let path_env = env::var_os("PATH")?;
    for directory in env::split_paths(&path_env) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn run_command(executable: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Ok(stdout);
        }
        return Ok(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(if !stderr.is_empty() { stderr } else { stdout })
}

fn infer_auth(host: &AiHost, output: Option<String>) -> (AgentBackendAuthStatus, Option<String>) {
    let Some(output) = output else {
        return (AgentBackendAuthStatus::Unknown, None);
    };
    match host {
        AiHost::ClaudeCode => parse_claude_auth(&output),
        AiHost::Codex => parse_codex_auth(&output),
        _ => (AgentBackendAuthStatus::Unknown, None),
    }
}

fn parse_claude_auth(output: &str) -> (AgentBackendAuthStatus, Option<String>) {
    let Ok(payload) = serde_json::from_str::<Value>(output) else {
        return (AgentBackendAuthStatus::Unknown, None);
    };
    let logged_in = payload
        .get("loggedIn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let method = payload
        .get("authMethod")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    (
        if logged_in {
            AgentBackendAuthStatus::LoggedIn
        } else {
            AgentBackendAuthStatus::LoggedOut
        },
        method,
    )
}

fn parse_codex_auth(output: &str) -> (AgentBackendAuthStatus, Option<String>) {
    let trimmed = output.trim();
    if let Some(method) = trimmed.strip_prefix("Logged in using ") {
        return (
            AgentBackendAuthStatus::LoggedIn,
            Some(method.trim().to_lowercase().replace(' ', "_")),
        );
    }
    if trimmed.to_ascii_lowercase().contains("not logged in") {
        return (AgentBackendAuthStatus::LoggedOut, None);
    }
    (AgentBackendAuthStatus::Unknown, None)
}

fn classify_readiness(
    probe: &BackendProbe,
    auth_status: &AgentBackendAuthStatus,
    help_output: Option<&str>,
) -> (AgentBackendReadiness, Option<String>) {
    if probe.policy_classification == "integration_only" {
        return (
            AgentBackendReadiness::DetectionOnly,
            Some("Detected integration surface, but no documented delegated model backend is confirmed yet.".to_string()),
        );
    }
    if help_output.is_none() {
        return (
            AgentBackendReadiness::Unavailable,
            Some("The local binary was found, but `--help` probing failed.".to_string()),
        );
    }
    match auth_status {
        AgentBackendAuthStatus::LoggedIn => (AgentBackendReadiness::Ready, None),
        AgentBackendAuthStatus::LoggedOut => (
            AgentBackendReadiness::Candidate,
            Some("Binary is installed, but the current login status is not ready for delegated execution.".to_string()),
        ),
        AgentBackendAuthStatus::Unknown => (
            AgentBackendReadiness::Candidate,
            Some("Binary is installed, but the CLI does not expose a trustworthy non-interactive auth status probe.".to_string()),
        ),
        AgentBackendAuthStatus::NotSupported => (
            AgentBackendReadiness::Candidate,
            Some("Binary is installed; auth will need to be confirmed through a vendor-specific flow.".to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    fn write_fake_executable(dir: &Path, name: &str, body: &str) {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn discover_local_agent_backends_reports_capabilities_truthfully() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_fake_executable(
            temp_dir.path(),
            "claude",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'claude 1.0.0'; exit 0; fi\nif [ \"$1\" = \"--help\" ]; then cat <<'EOF'\nClaude Code - starts an interactive session by default\nCommands:\n  auth\nEOF\nexit 0; fi\nif [ \"$1\" = \"auth\" ] && [ \"$2\" = \"status\" ]; then echo '{\"loggedIn\":true,\"authMethod\":\"claude.ai\"}'; exit 0; fi\nexit 1\n",
        );
        write_fake_executable(
            temp_dir.path(),
            "codex",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex 0.1.0'; exit 0; fi\nif [ \"$1\" = \"--help\" ]; then cat <<'EOF'\nCodex CLI\nCommands:\n  login\nEOF\nexit 0; fi\nif [ \"$1\" = \"login\" ] && [ \"$2\" = \"status\" ]; then echo 'Logged in using ChatGPT'; exit 0; fi\nexit 1\n",
        );
        write_fake_executable(
            temp_dir.path(),
            "gemini",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'gemini 0.1.0'; exit 0; fi\nif [ \"$1\" = \"--help\" ]; then cat <<'EOF'\nGemini CLI\nOptions:\n  --model\nEOF\nexit 0; fi\nexit 1\n",
        );
        write_fake_executable(
            temp_dir.path(),
            "cursor",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'Cursor 3.0.9'; exit 0; fi\nif [ \"$1\" = \"--help\" ]; then cat <<'EOF'\nCursor\nOptions:\n  --chat\nEOF\nexit 0; fi\nexit 1\n",
        );

        let original_path = env::var_os("PATH");
        // Test-only PATH override to keep discovery deterministic for the fake binaries.
        unsafe {
            env::set_var("PATH", temp_dir.path());
        }

        let catalog = AgentBackendCatalogService::new().discover();

        let claude = catalog
            .iter()
            .find(|entry| entry.host == AiHost::ClaudeCode)
            .unwrap();
        assert!(claude.detected);
        assert_eq!(claude.auth_status, AgentBackendAuthStatus::LoggedIn);
        assert_eq!(claude.auth_method.as_deref(), Some("claude.ai"));
        assert_eq!(claude.readiness, AgentBackendReadiness::Ready);

        let codex = catalog
            .iter()
            .find(|entry| entry.host == AiHost::Codex)
            .unwrap();
        assert_eq!(codex.auth_status, AgentBackendAuthStatus::LoggedIn);
        assert_eq!(codex.auth_method.as_deref(), Some("chatgpt"));

        let gemini = catalog
            .iter()
            .find(|entry| entry.host == AiHost::GeminiCli)
            .unwrap();
        assert_eq!(gemini.auth_status, AgentBackendAuthStatus::NotSupported);
        assert_eq!(gemini.readiness, AgentBackendReadiness::Candidate);

        let cursor = catalog
            .iter()
            .find(|entry| entry.host == AiHost::Cursor)
            .unwrap();
        assert_eq!(cursor.readiness, AgentBackendReadiness::DetectionOnly);
        assert_eq!(cursor.policy_classification, "integration_only");

        match original_path {
            Some(value) => unsafe {
                env::set_var("PATH", value);
            },
            None => unsafe {
                env::remove_var("PATH");
            },
        }
    }
}
