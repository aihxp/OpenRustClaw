//! MCP transport layer (stdio subprocess management).
//!
//! Commands are validated against an allowlist before spawning to prevent
//! execution of arbitrary binaries.

use openrustclaw_core::error::{Error, McpError, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, warn};

/// Commands allowed to be spawned as MCP server subprocesses.
const ALLOWED_COMMANDS: &[&str] = &[
    "npx", "uvx", "node", "python3", "python", "docker", "deno", "bun", "cargo", "go",
];

/// A stdio transport connection to an MCP server subprocess.
pub struct StdioTransport {
    child: Child,
    // We read from stdout and write to stdin.
}

impl StdioTransport {
    /// Validate that a command is safe to execute.
    fn validate_command(command: &str) -> Result<()> {
        // Extract the base command name (strip path if present)
        let base_name = std::path::Path::new(command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(command);

        // Check against allowlist
        if !ALLOWED_COMMANDS.contains(&base_name) {
            warn!(command = %command, "MCP server command rejected: not in allowlist");
            return Err(Error::Mcp(McpError::Transport(format!(
                "Command '{}' is not in the allowed MCP server commands list. Allowed: {:?}",
                command, ALLOWED_COMMANDS
            ))));
        }

        // Reject commands with shell metacharacters
        let shell_chars = [
            '|', '&', ';', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\n',
        ];
        if command.chars().any(|c| shell_chars.contains(&c)) {
            warn!(command = %command, "MCP server command rejected: contains shell metacharacters");
            return Err(Error::Mcp(McpError::Transport(format!(
                "Command '{}' contains disallowed shell metacharacters",
                command
            ))));
        }

        Ok(())
    }

    /// Spawn an MCP server as a subprocess.
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        Self::validate_command(command)?;

        let child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                Error::Mcp(McpError::Transport(format!(
                    "Failed to spawn MCP server: {}",
                    e
                )))
            })?;

        Ok(Self { child })
    }

    /// Send a JSON-RPC request and read the response.
    pub async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Mcp(McpError::Transport("No stdin handle".to_string())))?;

        let request_str = serde_json::to_string(&request)
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        stdin
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;
        stdin
            .flush()
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        debug!(method = %method, "Sent MCP request");

        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| Error::Mcp(McpError::Transport("No stdout handle".to_string())))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        let response: Value = serde_json::from_str(&line).map_err(|e| {
            Error::Mcp(McpError::Transport(format!("Invalid JSON response: {}", e)))
        })?;

        if let Some(error) = response.get("error") {
            return Err(Error::Mcp(McpError::ToolExecution(
                error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
            )));
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Kill the subprocess.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_allows_permitted_commands() {
        assert!(StdioTransport::validate_command("npx").is_ok());
        assert!(StdioTransport::validate_command("node").is_ok());
        assert!(StdioTransport::validate_command("python3").is_ok());
        assert!(StdioTransport::validate_command("docker").is_ok());
        assert!(StdioTransport::validate_command("uvx").is_ok());
    }

    #[test]
    fn validate_command_rejects_arbitrary_binaries() {
        assert!(StdioTransport::validate_command("bash").is_err());
        assert!(StdioTransport::validate_command("sh").is_err());
        assert!(StdioTransport::validate_command("curl").is_err());
        assert!(StdioTransport::validate_command("rm").is_err());
    }

    #[test]
    fn validate_command_extracts_basename_from_path() {
        assert!(StdioTransport::validate_command("/usr/bin/node").is_ok());
        assert!(StdioTransport::validate_command("/usr/local/bin/npx").is_ok());
        assert!(StdioTransport::validate_command("/tmp/evil").is_err());
    }

    #[test]
    fn validate_command_rejects_shell_metacharacters() {
        assert!(StdioTransport::validate_command("node; rm -rf /").is_err());
        assert!(StdioTransport::validate_command("node | cat").is_err());
        assert!(StdioTransport::validate_command("$(whoami)").is_err());
    }

    // --- additional allowed commands ---

    #[test]
    fn validate_command_allows_deno_and_bun() {
        assert!(StdioTransport::validate_command("deno").is_ok());
        assert!(StdioTransport::validate_command("bun").is_ok());
    }

    #[test]
    fn validate_command_allows_cargo_and_go() {
        assert!(StdioTransport::validate_command("cargo").is_ok());
        assert!(StdioTransport::validate_command("go").is_ok());
    }

    #[test]
    fn validate_command_allows_python_variants() {
        assert!(StdioTransport::validate_command("python").is_ok());
        assert!(StdioTransport::validate_command("python3").is_ok());
    }

    // --- additional rejections ---

    #[test]
    fn validate_command_rejects_common_dangerous_commands() {
        assert!(StdioTransport::validate_command("wget").is_err());
        assert!(StdioTransport::validate_command("nc").is_err());
        assert!(StdioTransport::validate_command("cat").is_err());
        assert!(StdioTransport::validate_command("chmod").is_err());
    }

    #[test]
    fn validate_command_rejects_empty_string() {
        assert!(StdioTransport::validate_command("").is_err());
    }

    // --- path-based commands ---

    #[test]
    fn validate_command_allows_absolute_path_to_permitted_binary() {
        assert!(StdioTransport::validate_command("/opt/homebrew/bin/npx").is_ok());
        assert!(StdioTransport::validate_command("/usr/local/bin/python3").is_ok());
    }

    #[test]
    fn validate_command_rejects_absolute_path_to_forbidden_binary() {
        assert!(StdioTransport::validate_command("/usr/bin/bash").is_err());
        assert!(StdioTransport::validate_command("/bin/sh").is_err());
    }

    // --- shell metacharacters ---

    #[test]
    fn validate_command_rejects_backtick_injection() {
        assert!(StdioTransport::validate_command("node`whoami`").is_err());
    }

    #[test]
    fn validate_command_rejects_redirect_characters() {
        assert!(StdioTransport::validate_command("node > /tmp/out").is_err());
        assert!(StdioTransport::validate_command("node < /etc/passwd").is_err());
    }

    #[test]
    fn validate_command_rejects_ampersand() {
        assert!(StdioTransport::validate_command("node & sleep 10").is_err());
    }

    #[test]
    fn validate_command_rejects_newline() {
        assert!(StdioTransport::validate_command("node\nrm -rf /").is_err());
    }

    #[test]
    fn validate_command_rejects_exclamation() {
        assert!(StdioTransport::validate_command("node!").is_err());
    }

    #[test]
    fn validate_command_rejects_curly_braces() {
        assert!(StdioTransport::validate_command("node{1,2}").is_err());
    }

    #[test]
    fn validate_command_rejects_parentheses() {
        assert!(StdioTransport::validate_command("node(test)").is_err());
    }

    // --- error message quality ---

    #[test]
    fn rejected_command_error_mentions_allowlist() {
        let result = StdioTransport::validate_command("evil-binary");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not in the allowed"));
    }

    #[test]
    fn shell_metachar_error_mentions_metacharacters() {
        // Use a path where the basename is in the allowlist but the full path
        // contains shell metacharacters (semicolon in a directory name).
        let result = StdioTransport::validate_command("/tmp/evil;dir/node");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("metacharacters"));
    }
}
