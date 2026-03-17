//! Terminal integration tools for running commands and reading output.

use crate::error::{CursorError, Result};
use crate::types::{CursorTool, TerminalState};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Shared state for terminal sessions.
pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<String, TerminalSession>>>,
    default_timeout: Duration,
}

/// A terminal session.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct TerminalSession {
    pub id: String,
    pub cwd: std::path::PathBuf,
    pub last_command: Option<String>,
    pub recent_output: String,
    pub is_running: bool,
    pub last_exit_code: Option<i32>,
    pub history: Vec<CommandHistoryEntry>,
}

/// A command history entry.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct CommandHistoryEntry {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
}

impl TerminalManager {
    /// Create a new terminal manager.
    pub fn new(default_timeout_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_timeout: Duration::from_secs(default_timeout_secs),
        }
    }

    /// Create a new terminal session.
    pub async fn create_session(&self, cwd: Option<std::path::PathBuf>) -> String {
        let id = Uuid::new_v4().to_string();
        let cwd = cwd.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

        let session = TerminalSession {
            id: id.clone(),
            cwd,
            last_command: None,
            recent_output: String::new(),
            is_running: false,
            last_exit_code: None,
            history: Vec::new(),
        };

        self.sessions.write().await.insert(id.clone(), session);
        info!("Created terminal session: {}", id);
        id
    }

    /// Get a terminal session.
    pub async fn get_session(&self, id: &str) -> Option<TerminalSession> {
        self.sessions.read().await.get(id).cloned()
    }

    /// Execute a command in a terminal session.
    pub async fn execute_command(
        &self,
        session_id: Option<&str>,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<CommandResult> {
        let timeout_duration = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Get or create session
        let session_id = if let Some(id) = session_id {
            id.to_string()
        } else {
            self.create_session(None).await
        };

        // Update session state
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.is_running = true;
                session.last_command = Some(command.to_string());
            }
        }

        let start_time = std::time::Instant::now();

        // Execute command
        let result = self.run_command(command, timeout_duration).await;

        // Update session state with results
        let duration_ms = start_time.elapsed().as_millis() as u64;

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.is_running = false;

                match &result {
                    Ok(cmd_result) => {
                        session.last_exit_code = Some(cmd_result.exit_code);
                        session.recent_output = cmd_result.stdout.clone();
                        session.history.push(CommandHistoryEntry {
                            command: command.to_string(),
                            stdout: cmd_result.stdout.clone(),
                            stderr: cmd_result.stderr.clone(),
                            exit_code: cmd_result.exit_code,
                            executed_at: chrono::Utc::now(),
                            duration_ms,
                        });

                        // Limit history size
                        if session.history.len() > 100 {
                            session.history.remove(0);
                        }
                    }
                    Err(e) => {
                        session.last_exit_code = Some(-1);
                        session.recent_output = e.to_string();
                    }
                }
            }
        }

        result
    }

    /// Run a command with timeout.
    async fn run_command(
        &self,
        command: &str,
        timeout_duration: Duration,
    ) -> Result<CommandResult> {
        debug!(
            "Executing command: {} (timeout: {:?})",
            command, timeout_duration
        );

        let output = timeout(
            timeout_duration,
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| {
            error!("Command timed out: {}", command);
            CursorError::Timeout(format!("Command timed out after {:?}", timeout_duration))
        })?
        .map_err(|e| {
            error!("Failed to execute command: {}", e);
            CursorError::Terminal(e.to_string())
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        info!(
            "Command completed: {} (exit_code: {}, stdout: {} bytes, stderr: {} bytes)",
            command,
            exit_code,
            stdout.len(),
            stderr.len()
        );

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Get recent output from a terminal session.
    pub async fn get_recent_output(
        &self,
        session_id: &str,
        lines: Option<usize>,
    ) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|session| {
            if let Some(n) = lines {
                let all_lines: Vec<_> = session.recent_output.lines().collect();
                let start = all_lines.len().saturating_sub(n);
                all_lines[start..].join("\n")
            } else {
                session.recent_output.clone()
            }
        })
    }

    /// Get terminal state.
    pub async fn get_state(&self, session_id: &str) -> Option<TerminalState> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|session| TerminalState {
            id: session.id.clone(),
            cwd: session.cwd.clone(),
            last_command: session.last_command.clone(),
            recent_output: session.recent_output.clone(),
            is_running: session.is_running,
            last_exit_code: session.last_exit_code,
        })
    }

    /// List all session IDs.
    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Kill a terminal session.
    pub async fn kill_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
    }
}

/// Result of executing a command.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run a command in the terminal.
pub struct RunCommandTool {
    terminal_manager: Arc<TerminalManager>,
}

impl RunCommandTool {
    /// Create a new run command tool.
    pub fn new(terminal_manager: Arc<TerminalManager>) -> Self {
        Self { terminal_manager }
    }
}

impl Default for RunCommandTool {
    fn default() -> Self {
        Self::new(Arc::new(TerminalManager::new(30)))
    }
}

#[async_trait]
impl CursorTool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the terminal. Commands are run in a subshell with sh -c."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "terminal_id": {
                    "type": "string",
                    "description": "Optional terminal session ID (creates new if not provided)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30)",
                    "default": 30
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let command = params
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| input_validation_error(self.name(), "Missing 'command' parameter"))?;

        let terminal_id = params.get("terminal_id").and_then(|t| t.as_str());
        let timeout = params.get("timeout").and_then(|t| t.as_u64());

        let result = self
            .terminal_manager
            .execute_command(terminal_id, command, timeout)
            .await?;

        Ok(json!({
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "success": result.exit_code == 0,
            "terminal_id": terminal_id,
        }))
    }
}

/// Read terminal output.
pub struct ReadTerminalTool {
    terminal_manager: Arc<TerminalManager>,
}

impl ReadTerminalTool {
    /// Create a new read terminal tool.
    pub fn new(terminal_manager: Arc<TerminalManager>) -> Self {
        Self { terminal_manager }
    }
}

impl Default for ReadTerminalTool {
    fn default() -> Self {
        Self::new(Arc::new(TerminalManager::new(30)))
    }
}

#[async_trait]
impl CursorTool for ReadTerminalTool {
    fn name(&self) -> &str {
        "read_terminal"
    }

    fn description(&self) -> &str {
        "Read recent output from a terminal session."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "terminal_id": {
                    "type": "string",
                    "description": "Terminal session ID"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of recent lines to return (default: all)",
                },
                "include_history": {
                    "type": "boolean",
                    "description": "Include command history",
                    "default": false
                }
            },
            "required": ["terminal_id"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let terminal_id = params
            .get("terminal_id")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                input_validation_error(self.name(), "Missing 'terminal_id' parameter")
            })?;

        let lines = params
            .get("lines")
            .and_then(|l| l.as_u64())
            .map(|l| l as usize);
        let include_history = params
            .get("include_history")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);

        let state = self
            .terminal_manager
            .get_state(terminal_id)
            .await
            .ok_or_else(|| {
                CursorError::Terminal(format!("Terminal session not found: {}", terminal_id))
            })?;

        let output = self
            .terminal_manager
            .get_recent_output(terminal_id, lines)
            .await
            .unwrap_or_default();

        let mut result = json!({
            "terminal_id": terminal_id,
            "cwd": state.cwd,
            "is_running": state.is_running,
            "last_command": state.last_command,
            "last_exit_code": state.last_exit_code,
            "output": output,
        });

        if include_history {
            // This would require exposing history from TerminalSession
            // For now, we skip this
            result["history"] = json!([]);
        }

        Ok(result)
    }
}

// Helper function to create input validation errors
fn input_validation_error(tool: impl Into<String>, message: impl Into<String>) -> CursorError {
    CursorError::ToolExecution {
        tool: tool.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_manager() {
        let manager = TerminalManager::new(5);

        // Create session
        let session_id = manager.create_session(None).await;
        assert!(!session_id.is_empty());

        // Execute command
        let result = manager
            .execute_command(Some(&session_id), "echo 'Hello, World!'", None)
            .await
            .unwrap();

        assert!(result.stdout.contains("Hello, World!"));
        assert_eq!(result.exit_code, 0);

        // Get state
        let state = manager.get_state(&session_id).await.unwrap();
        assert_eq!(state.last_command, Some("echo 'Hello, World!'".to_string()));
        assert_eq!(state.last_exit_code, Some(0));

        // List sessions
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);

        // Kill session
        assert!(manager.kill_session(&session_id).await);
        assert!(!manager.kill_session(&session_id).await);
    }

    #[tokio::test]
    async fn test_run_command_tool() {
        let tool = RunCommandTool::default();

        let result = tool
            .execute(json!({"command": "echo 'test output'"}))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("test output"));
        assert_eq!(result["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_command_timeout() {
        let manager = TerminalManager::new(1); // 1 second timeout

        let result = manager.execute_command(None, "sleep 5", None).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CursorError::Timeout(_)));
    }
}
