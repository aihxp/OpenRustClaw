//! Agent Context Protocol (ACP) implementation.
//!
//! ACP is a protocol for rich IDE-agent communication, providing context about:
//! - Open files and cursor positions
//! - Terminal state
//! - Git status
//! - Diagnostics
//! - Selection and active editor

use crate::error::{CursorError, Result};
use crate::types::{AcpRequest, AcpResponse, CursorConfig, IdeState, ToolContext};
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Protocol version for ACP.
pub const ACP_PROTOCOL_VERSION: &str = "2024-12-01";

/// ACP message envelope for transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpMessage {
    /// Protocol version.
    pub version: String,
    /// Message ID for request/response correlation.
    pub id: String,
    /// The message payload.
    #[serde(flatten)]
    pub payload: AcpPayload,
}

/// ACP message payload types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AcpPayload {
    /// A request from the agent to the IDE.
    Request(AcpRequest),
    /// A response from the IDE to the agent.
    Response(AcpResponse),
    /// A notification (one-way message).
    Notification(AcpNotification),
}

/// One-way notifications from IDE to agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AcpNotification {
    /// File was modified.
    FileModified { path: std::path::PathBuf },
    /// Terminal output updated.
    TerminalOutput { terminal_id: String, output: String },
    /// Diagnostic updated.
    DiagnosticUpdated { file_path: std::path::PathBuf },
    /// Git state changed.
    GitStateChanged,
    /// Selection changed.
    SelectionChanged,
}

/// ACP server capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpCapabilities {
    /// Supports real-time file watching.
    pub file_watching: bool,
    /// Supports terminal integration.
    pub terminal_integration: bool,
    /// Supports git operations.
    pub git_operations: bool,
    /// Supports diagnostics reporting.
    pub diagnostics: bool,
    /// Supports code actions.
    pub code_actions: bool,
    /// Supported linter tools.
    pub linters: Vec<String>,
}

impl Default for AcpCapabilities {
    fn default() -> Self {
        Self {
            file_watching: true,
            terminal_integration: true,
            git_operations: true,
            diagnostics: true,
            code_actions: true,
            linters: vec!["clippy".to_string(), "rustfmt".to_string()],
        }
    }
}

/// Server information for ACP initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpServerInfo {
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
}

/// ACP initialization request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpInitializeRequest {
    /// Protocol version requested by client.
    pub protocol_version: String,
    /// Client capabilities.
    pub capabilities: AcpCapabilities,
    /// Client info.
    pub client_info: AcpServerInfo,
}

/// ACP initialization response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpInitializeResponse {
    /// Protocol version.
    pub protocol_version: String,
    /// Server capabilities.
    pub capabilities: AcpCapabilities,
    /// Server info.
    pub server_info: AcpServerInfo,
}

/// The ACP protocol handler.
pub struct AcpProtocol {
    config: Arc<RwLock<CursorConfig>>,
    state: Arc<RwLock<Option<IdeState>>>,
    capabilities: AcpCapabilities,
    server_info: AcpServerInfo,
}

impl AcpProtocol {
    /// Create a new ACP protocol handler.
    pub fn new(config: CursorConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(None)),
            capabilities: AcpCapabilities::default(),
            server_info: AcpServerInfo {
                name: "openrustclaw-cursor".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Initialize the protocol.
    pub async fn initialize(&self, request: AcpInitializeRequest) -> Result<AcpInitializeResponse> {
        info!(
            "Initializing ACP protocol with client: {} v{}",
            request.client_info.name, request.client_info.version
        );

        // Validate protocol version
        if request.protocol_version != ACP_PROTOCOL_VERSION {
            warn!(
                "Protocol version mismatch: client={}, server={}",
                request.protocol_version, ACP_PROTOCOL_VERSION
            );
        }

        Ok(AcpInitializeResponse {
            protocol_version: ACP_PROTOCOL_VERSION.to_string(),
            capabilities: self.capabilities.clone(),
            server_info: self.server_info.clone(),
        })
    }

    /// Handle an incoming ACP message.
    pub async fn handle_message(&self, message: AcpMessage) -> Result<Option<AcpMessage>> {
        debug!("Handling ACP message: id={}", message.id);

        match message.payload {
            AcpPayload::Request(request) => {
                let response = self.handle_request(request).await?;
                Ok(Some(AcpMessage {
                    version: ACP_PROTOCOL_VERSION.to_string(),
                    id: message.id,
                    payload: AcpPayload::Response(response),
                }))
            }
            AcpPayload::Response(_) => {
                // Server doesn't typically receive responses
                warn!("Received unexpected response message");
                Ok(None)
            }
            AcpPayload::Notification(notification) => {
                self.handle_notification(notification).await?;
                Ok(None)
            }
        }
    }

    /// Handle a request.
    async fn handle_request(&self, request: AcpRequest) -> Result<AcpResponse> {
        match request {
            AcpRequest::GetState => self.handle_get_state().await,
            AcpRequest::ExecuteCommand {
                command,
                terminal_id,
            } => self.handle_execute_command(command, terminal_id).await,
            AcpRequest::ReadFile { path } => self.handle_read_file(path).await,
            AcpRequest::WriteFile { path, content } => {
                self.handle_write_file(path, content).await
            }
            AcpRequest::EditFile {
                path,
                old_text,
                new_text,
            } => self.handle_edit_file(path, old_text, new_text).await,
            AcpRequest::SearchCode {
                query,
                path_pattern,
                max_results,
            } => self.handle_search_code(query, path_pattern, max_results).await,
            AcpRequest::ListFiles { path, recursive } => {
                self.handle_list_files(path, recursive).await
            }
            AcpRequest::GitStatus => self.handle_git_status().await,
            AcpRequest::GitDiff { staged } => self.handle_git_diff(staged).await,
            AcpRequest::GitCommand { args } => self.handle_git_command(args).await,
            AcpRequest::RunLinter { tool, path } => self.handle_run_linter(tool, path).await,
        }
    }

    /// Handle a notification.
    async fn handle_notification(&self, notification: AcpNotification) -> Result<()> {
        match notification {
            AcpNotification::FileModified { path } => {
                debug!("File modified notification: {:?}", path);
                // Update cached state if needed
            }
            AcpNotification::TerminalOutput {
                terminal_id,
                output,
            } => {
                debug!(
                    "Terminal output notification: terminal_id={}, output_len={}",
                    terminal_id,
                    output.len()
                );
            }
            AcpNotification::DiagnosticUpdated { file_path } => {
                debug!("Diagnostic updated notification: {:?}", file_path);
            }
            AcpNotification::GitStateChanged => {
                debug!("Git state changed notification");
            }
            AcpNotification::SelectionChanged => {
                debug!("Selection changed notification");
            }
        }
        Ok(())
    }

    /// Handle GetState request.
    async fn handle_get_state(&self) -> Result<AcpResponse> {
        let state = self.state.read().await;
        match state.as_ref() {
            Some(ide_state) => Ok(AcpResponse::State(ide_state.clone())),
            None => Err(CursorError::InvalidContext("No IDE state available".to_string())),
        }
    }

    /// Handle ExecuteCommand request.
    async fn handle_execute_command(
        &self,
        command: String,
        _terminal_id: Option<String>,
    ) -> Result<AcpResponse> {
        let config = self.config.read().await;
        
        // Execute command with timeout
        let output = tokio::time::timeout(
            tokio::time::Duration::from_secs(config.terminal_timeout),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .current_dir(&config.project_root)
                .output(),
        )
        .await
        .map_err(|_| CursorError::Timeout("Command execution timed out".to_string()))?
        .map_err(|e| CursorError::Terminal(e.to_string()))?;

        Ok(AcpResponse::CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Handle ReadFile request.
    async fn handle_read_file(&self, path: std::path::PathBuf) -> Result<AcpResponse> {
        let config = self.config.read().await;
        let full_path = if path.is_absolute() {
            path
        } else {
            config.project_root.join(path)
        };

        // Check file size
        let metadata = tokio::fs::metadata(&full_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;
        
        if metadata.len() > config.max_file_size as u64 {
            return Err(CursorError::FileOperation(format!(
                "File too large: {} bytes (max: {})",
                metadata.len(),
                config.max_file_size
            )));
        }

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        Ok(AcpResponse::FileContent {
            path: full_path,
            content,
        })
    }

    /// Handle WriteFile request.
    async fn handle_write_file(
        &self,
        path: std::path::PathBuf,
        content: String,
    ) -> Result<AcpResponse> {
        let config = self.config.read().await;
        let full_path = if path.is_absolute() {
            path
        } else {
            config.project_root.join(path)
        };

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| CursorError::FileOperation(e.to_string()))?;
        }

        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        Ok(AcpResponse::FileOperationSuccess {
            path: full_path,
            operation: "write".to_string(),
        })
    }

    /// Handle EditFile request.
    async fn handle_edit_file(
        &self,
        path: std::path::PathBuf,
        old_text: String,
        new_text: String,
    ) -> Result<AcpResponse> {
        let config = self.config.read().await;
        let full_path = if path.is_absolute() {
            path
        } else {
            config.project_root.join(path)
        };

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        let new_content = content.replace(&old_text, &new_text);
        
        if new_content == content {
            return Err(CursorError::FileOperation(
                "Old text not found in file".to_string(),
            ));
        }

        tokio::fs::write(&full_path, new_content)
            .await
            .map_err(|e| CursorError::FileOperation(e.to_string()))?;

        Ok(AcpResponse::FileOperationSuccess {
            path: full_path,
            operation: "edit".to_string(),
        })
    }

    /// Handle SearchCode request.
    async fn handle_search_code(
        &self,
        query: String,
        path_pattern: Option<String>,
        max_results: Option<usize>,
    ) -> Result<AcpResponse> {
        use crate::types::SearchMatch;

        let config = self.config.read().await;
        let max = max_results.unwrap_or(50);
        let mut matches = Vec::new();

        // Use grep-like search with regex
        let pattern = regex::Regex::new(&query)
            .map_err(|e| CursorError::PatternError(e.to_string()))?;

        // Walk directory and search files
        let walker = walkdir::WalkDir::new(&config.project_root)
            .follow_links(false)
            .max_depth(10);

        for entry in walker {
            let entry = entry.map_err(|e| CursorError::FileOperation(e.to_string()))?;
            
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            
            // Check include/exclude patterns
            let path_str = path.to_string_lossy();
            if !self.should_include_file(&path_str, &config) {
                continue;
            }

            // Check path pattern if specified
            if let Some(ref pat) = path_pattern {
                if !path_str.contains(pat) {
                    continue;
                }
            }

            // Search file content
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                for (line_num, line) in content.lines().enumerate() {
                    if pattern.is_match(line) {
                        matches.push(SearchMatch {
                            file_path: path.to_path_buf(),
                            line: line_num,
                            column: line.find(&query).unwrap_or(0),
                            line_content: line.to_string(),
                            context_before: vec![], // Simplified for now
                            context_after: vec![],
                        });

                        if matches.len() >= max {
                            break;
                        }
                    }
                }
            }

            if matches.len() >= max {
                break;
            }
        }

        let total = matches.len();
        Ok(AcpResponse::SearchResults { matches, total })
    }

    /// Handle ListFiles request.
    async fn handle_list_files(
        &self,
        path: std::path::PathBuf,
        recursive: bool,
    ) -> Result<AcpResponse> {
        use crate::types::DirEntry;

        let config = self.config.read().await;
        let full_path = if path.is_absolute() {
            path
        } else {
            config.project_root.join(path)
        };

        let mut entries = Vec::new();
        let max_depth = if recursive { 100 } else { 1 };

        let walker = walkdir::WalkDir::new(&full_path)
            .follow_links(false)
            .max_depth(max_depth);

        for entry in walker {
            let entry = entry.map_err(|e| CursorError::FileOperation(e.to_string()))?;
            
            if entry.path() == full_path {
                continue;
            }

            let metadata = entry.metadata().ok();
            
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path().to_path_buf(),
                is_directory: entry.file_type().is_dir(),
                size: metadata.as_ref().map(|m| m.len()),
                modified_at: metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::from(std::time::SystemTime::from(t))),
            });
        }

        Ok(AcpResponse::DirectoryListing {
            path: full_path,
            entries,
        })
    }

    /// Handle GitStatus request.
    async fn handle_git_status(&self) -> Result<AcpResponse> {
        let config = self.config.read().await;
        
        let output = tokio::process::Command::new("git")
            .args(&["status", "--porcelain", "-b"])
            .current_dir(&config.project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        if !output.status.success() {
            return Err(CursorError::GitOperation(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status = crate::types::GitStatus {
            branch: "main".to_string(),
            modified: vec![],
            staged: vec![],
            untracked: vec![],
            renamed: vec![],
            deleted: vec![],
            ahead: 0,
            behind: 0,
        };

        for line in stdout.lines() {
            if line.starts_with("##") {
                // Parse branch info
                if let Some(branch_part) = line.strip_prefix("## ") {
                    if let Some(branch) = branch_part.split("...").next() {
                        status.branch = branch.to_string();
                    }
                }
            } else if line.len() >= 3 {
                let index_status = &line[0..1];
                let worktree_status = &line[1..2];
                let path_str = &line[3..];
                let path = std::path::PathBuf::from(path_str);

                match (index_status, worktree_status) {
                    ("M", " ") | ("A", " ") => status.staged.push(path),
                    (" ", "M") => status.modified.push(path),
                    (" ", "?") => status.untracked.push(path),
                    ("D", " ") | (" ", "D") => status.deleted.push(path),
                    ("R", " ") => status.renamed.push((path.clone(), path)),
                    _ => {}
                }
            }
        }

        Ok(AcpResponse::GitStatus(status))
    }

    /// Handle GitDiff request.
    async fn handle_git_diff(&self, staged: bool) -> Result<AcpResponse> {
        let config = self.config.read().await;
        
        let mut args = vec!["diff"];
        if staged {
            args.push("--staged");
        }

        let output = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&config.project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        Ok(AcpResponse::GitDiff(String::from_utf8_lossy(&output.stdout).to_string()))
    }

    /// Handle GitCommand request.
    async fn handle_git_command(&self, args: Vec<String>) -> Result<AcpResponse> {
        let config = self.config.read().await;
        
        let output = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&config.project_root)
            .output()
            .await
            .map_err(|e| CursorError::GitOperation(e.to_string()))?;

        Ok(AcpResponse::CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Handle RunLinter request.
    async fn handle_run_linter(
        &self,
        tool: String,
        _path: Option<std::path::PathBuf>,
    ) -> Result<AcpResponse> {
        let config = self.config.read().await;
        
        let (command, args) = match tool.as_str() {
            "clippy" => ("cargo", vec!["clippy", "--all-targets", "--", "-D", "warnings"]),
            "rustfmt" => ("cargo", vec!["fmt", "--check"]),
            "cargo-check" => ("cargo", vec!["check"]),
            "cargo-test" => ("cargo", vec!["test"]),
            _ => return Err(CursorError::Linter(format!("Unknown linter: {}", tool))),
        };

        let output = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(&config.project_root)
            .output()
            .await
            .map_err(|e| CursorError::Linter(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}\n{}", stdout, stderr);

        // Parse diagnostics from output (simplified)
        let diagnostics = self.parse_diagnostics(&combined, &config.project_root);

        Ok(AcpResponse::LinterOutput {
            tool,
            output: combined,
            diagnostics,
        })
    }

    /// Update the IDE state.
    pub async fn update_state(&self, state: IdeState) {
        let mut guard = self.state.write().await;
        *guard = Some(state);
    }

    /// Get the current IDE state.
    pub async fn get_state(&self) -> Option<IdeState> {
        self.state.read().await.clone()
    }

    /// Check if a file should be included based on patterns.
    fn should_include_file(&self, path: &str, config: &CursorConfig) -> bool {
        // Check exclude patterns first
        for pattern in &config.exclude_patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(path))
                .unwrap_or(false)
            {
                return false;
            }
        }

        // Check include patterns
        for pattern in &config.include_patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(path))
                .unwrap_or(false)
            {
                return true;
            }
        }

        // Default: include if no patterns matched
        config.include_patterns.is_empty()
    }

    /// Parse diagnostics from linter output (simplified implementation).
    fn parse_diagnostics(
        &self,
        output: &str,
        project_root: &std::path::Path,
    ) -> Vec<crate::types::Diagnostic> {
        let mut diagnostics = Vec::new();

        // Simple regex-based parsing for rustc/clippy style errors
        // Format: file.rs:line:col: severity: message
        for line in output.lines() {
            if let Some(caps) = regex::Regex::new(r"^(.+):(\d+):(\d+):\s*(error|warning|info):\s*(.+)$")
                .ok()
                .and_then(|re| re.captures(line))
            {
                let file_path = project_root.join(&caps[1]);
                let line_num: usize = caps[2].parse().unwrap_or(0);
                let col: usize = caps[3].parse().unwrap_or(0);
                let severity = match &caps[4] {
                    "error" => crate::types::Severity::Error,
                    "warning" => crate::types::Severity::Warning,
                    _ => crate::types::Severity::Information,
                };
                let message = caps[5].to_string();

                diagnostics.push(crate::types::Diagnostic {
                    file_path,
                    severity,
                    message,
                    source: "linter".to_string(),
                    line: line_num.saturating_sub(1), // Convert to 0-indexed
                    column: col.saturating_sub(1),
                    code: None,
                });
            }
        }

        diagnostics
    }

    /// Create a tool context.
    pub async fn create_tool_context(&self) -> ToolContext {
        let config = self.config.read().await.clone();
        let state = self.state.read().await.clone();
        
        let mut ctx = ToolContext::new(config.project_root.clone(), config);
        if let Some(s) = state {
            ctx = ctx.with_ide_state(s);
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_message_serialization() {
        let msg = AcpMessage {
            version: ACP_PROTOCOL_VERSION.to_string(),
            id: "test-123".to_string(),
            payload: AcpPayload::Request(AcpRequest::GetState),
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("get_state"));
    }

    #[test]
    fn acp_capabilities_default() {
        let caps = AcpCapabilities::default();
        assert!(caps.file_watching);
        assert!(caps.terminal_integration);
    }
}
