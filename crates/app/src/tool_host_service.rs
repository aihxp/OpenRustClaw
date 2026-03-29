use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn display_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Codex => "Codex",
            Self::GeminiCli => "Gemini CLI",
            Self::GithubCopilot => "GitHub Copilot",
        }
    }

    pub fn startup_hint(self) -> &'static str {
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

    pub fn from_id(value: &str) -> Option<Self> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolProfileSnapshot {
    pub name: String,
    pub executable: String,
    pub executable_path: String,
    pub version_text: Option<String>,
    pub summary: Option<String>,
    pub detected_subcommands: Vec<String>,
    pub detected_flags: Vec<String>,
    pub hosts: Vec<String>,
    pub inspected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolProfileFingerprint {
    pub executable_sha256: String,
    pub help_digest: String,
    pub version_text: Option<String>,
    pub detected_subcommands: Vec<String>,
    pub detected_flags: Vec<String>,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolHostService;

impl ToolHostService {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_version(&self, output: &str) -> Option<String> {
        output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    }

    pub fn extract_summary(&self, help_output: &str) -> Option<String> {
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

    pub fn extract_subcommands(&self, help_output: &str) -> Vec<String> {
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

    pub fn extract_flags(&self, help_output: &str) -> Vec<String> {
        let mut values = BTreeSet::new();
        for token in help_output
            .split(|ch: char| ch.is_whitespace() || [',', ';', '(', ')', '[', ']'].contains(&ch))
        {
            if token.starts_with("--")
                && token.len() > 2
                && token
                    .chars()
                    .skip(2)
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            {
                values.insert(token.to_string());
            }
        }
        values.into_iter().collect()
    }

    pub fn normalized_hosts(
        &self,
        existing: Option<&[String]>,
        requested: &[AiHost],
    ) -> Vec<AiHost> {
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

    pub fn profile_changed(
        &self,
        left: &ToolProfileFingerprint,
        right: &ToolProfileFingerprint,
    ) -> bool {
        left.executable_sha256 != right.executable_sha256
            || left.help_digest != right.help_digest
            || left.version_text != right.version_text
            || left.detected_subcommands != right.detected_subcommands
            || left.detected_flags != right.detected_flags
            || left.hosts != right.hosts
    }

    pub fn render_host_markdown(&self, host: AiHost, profile: &ToolProfileSnapshot) -> String {
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

    pub fn render_host_index(&self, host: AiHost, profiles: &[ToolProfileSnapshot]) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_summary_and_flags_from_help() {
        let service = ToolHostService::new();
        let help = "Fake tool summary\n\nUsage: fake-tool [OPTIONS] <COMMAND>\n\nCommands:\n  sync      Sync state\n\nOptions:\n  --help     Show help\n  --json     JSON output\n";
        assert_eq!(
            service.extract_summary(help).as_deref(),
            Some("Fake tool summary")
        );
        assert!(
            service
                .extract_subcommands(help)
                .contains(&"sync".to_string())
        );
        assert!(service.extract_flags(help).contains(&"--json".to_string()));
    }
}
