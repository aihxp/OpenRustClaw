//! Agent Context Protocol (ACP) types for Cursor IDE integration.
//!
//! ACP provides rich context about the IDE state to the agent, enabling
//! more intelligent and context-aware assistance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ──────────────────────────────────────────────
// Core ACP Types
// ──────────────────────────────────────────────

/// The current state of the Cursor IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeState {
    /// The IDE's unique session identifier.
    pub session_id: String,
    /// Currently open files.
    pub open_files: Vec<OpenFile>,
    /// The active file (currently focused editor).
    pub active_file: Option<OpenFile>,
    /// Terminal sessions.
    pub terminals: Vec<TerminalState>,
    /// Selection information.
    pub selection: Option<Selection>,
    /// Git repository state.
    pub git_state: Option<GitState>,
    /// Diagnostics (errors/warnings) across the workspace.
    pub diagnostics: Vec<Diagnostic>,
    /// When this state was captured.
    pub timestamp: DateTime<Utc>,
}

/// Information about an open file in the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFile {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// File content at the time of capture.
    pub content: String,
    /// Cursor position in the file.
    pub cursor: CursorPosition,
    /// Language ID (e.g., "rust", "typescript").
    pub language_id: String,
    /// Whether the file has unsaved changes.
    pub is_dirty: bool,
    /// When the file was last modified.
    pub modified_at: DateTime<Utc>,
}

/// Position of the cursor in a file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CursorPosition {
    /// Line number (0-indexed).
    pub line: usize,
    /// Column number (0-indexed).
    pub column: usize,
}

impl CursorPosition {
    /// Create a new cursor position.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Convert to a human-readable string (1-indexed).
    pub fn to_display(&self) -> String {
        format!("{}:{}", self.line + 1, self.column + 1)
    }
}

/// A selection range in a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Selection {
    /// File path where the selection exists.
    pub file_path: PathBuf,
    /// Start position of the selection.
    pub start: CursorPosition,
    /// End position of the selection.
    pub end: CursorPosition,
    /// The selected text content.
    pub text: String,
}

impl Selection {
    /// Check if the selection is empty (cursor only).
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the range as line numbers.
    pub fn line_range(&self) -> (usize, usize) {
        (self.start.line, self.end.line)
    }
}

/// State of a terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalState {
    /// Terminal session identifier.
    pub id: String,
    /// Current working directory.
    pub cwd: PathBuf,
    /// Last command executed.
    pub last_command: Option<String>,
    /// Recent terminal output.
    pub recent_output: String,
    /// Whether the terminal is currently running a command.
    pub is_running: bool,
    /// Exit code of the last command (if completed).
    pub last_exit_code: Option<i32>,
}

/// Git repository state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    /// Current branch name.
    pub branch: String,
    /// Whether there are uncommitted changes.
    pub has_changes: bool,
    /// Number of modified files.
    pub modified_files: usize,
    /// Number of staged files.
    pub staged_files: usize,
    /// Number of untracked files.
    pub untracked_files: usize,
    /// Commit hash of HEAD.
    pub head_commit: String,
    /// Number of commits ahead of upstream.
    pub commits_ahead: usize,
    /// Number of commits behind upstream.
    pub commits_behind: usize,
}

/// A diagnostic message (error, warning, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// File path where the diagnostic applies.
    pub file_path: PathBuf,
    /// Severity level.
    pub severity: Severity,
    /// Diagnostic message.
    pub message: String,
    /// Source of the diagnostic (e.g., "rustc", "clippy").
    pub source: String,
    /// Line number (0-indexed).
    pub line: usize,
    /// Column number (0-indexed).
    pub column: usize,
    /// Error code if available.
    pub code: Option<String>,
}

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Information => write!(f, "info"),
            Severity::Hint => write!(f, "hint"),
        }
    }
}

// ──────────────────────────────────────────────
// ACP Request/Response Types
// ──────────────────────────────────────────────

/// A request sent from the agent to the Cursor IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[allow(missing_docs)]
pub enum AcpRequest {
    /// Get the current IDE state.
    GetState,
    /// Execute a command in the terminal.
    ExecuteCommand { command: String, terminal_id: Option<String> },
    /// Read a file's content.
    ReadFile { path: PathBuf },
    /// Write content to a file.
    WriteFile { path: PathBuf, content: String },
    /// Edit a file with a replacement.
    EditFile {
        path: PathBuf,
        old_text: String,
        new_text: String,
    },
    /// Search for text across files.
    SearchCode {
        query: String,
        path_pattern: Option<String>,
        max_results: Option<usize>,
    },
    /// List directory contents.
    ListFiles { path: PathBuf, recursive: bool },
    /// Get git status.
    GitStatus,
    /// Get git diff.
    GitDiff { staged: bool },
    /// Run a git command.
    GitCommand { args: Vec<String> },
    /// Run a linter/formatter.
    RunLinter { tool: String, path: Option<PathBuf> },
}

/// A response from the Cursor IDE to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[allow(missing_docs)]
pub enum AcpResponse {
    /// Current IDE state.
    State(IdeState),
    /// Command execution result.
    CommandResult {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    /// File content.
    FileContent {
        path: PathBuf,
        content: String,
    },
    /// File operation success.
    FileOperationSuccess {
        path: PathBuf,
        operation: String,
    },
    /// Search results.
    SearchResults {
        matches: Vec<SearchMatch>,
        total: usize,
    },
    /// Directory listing.
    DirectoryListing {
        path: PathBuf,
        entries: Vec<DirEntry>,
    },
    /// Git status.
    GitStatus(GitStatus),
    /// Git diff.
    GitDiff(String),
    /// Linter output.
    LinterOutput {
        tool: String,
        output: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// Error response.
    Error {
        message: String,
        code: Option<String>,
    },
}

/// A match from a code search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// File path where the match was found.
    pub file_path: PathBuf,
    /// Line number (0-indexed).
    pub line: usize,
    /// Column number (0-indexed).
    pub column: usize,
    /// The matching line content.
    pub line_content: String,
    /// Context lines before the match.
    pub context_before: Vec<String>,
    /// Context lines after the match.
    pub context_after: Vec<String>,
}

/// A directory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// Entry name.
    pub name: String,
    /// Full path.
    pub path: PathBuf,
    /// Whether this is a directory.
    pub is_directory: bool,
    /// File size in bytes (None for directories).
    pub size: Option<u64>,
    /// Last modification time.
    pub modified_at: Option<DateTime<Utc>>,
}

/// Extended git status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct GitStatus {
    pub branch: String,
    pub modified: Vec<PathBuf>,
    pub staged: Vec<PathBuf>,
    pub untracked: Vec<PathBuf>,
    pub renamed: Vec<(PathBuf, PathBuf)>,
    pub deleted: Vec<PathBuf>,
    pub ahead: usize,
    pub behind: usize,
}

// ──────────────────────────────────────────────
// Configuration Types
// ──────────────────────────────────────────────

/// Configuration for Cursor ACP integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorConfig {
    /// Whether Cursor integration is enabled.
    pub enabled: bool,
    /// Project root directory.
    pub project_root: PathBuf,
    /// File patterns to include in indexing.
    pub include_patterns: Vec<String>,
    /// File patterns to exclude from indexing.
    pub exclude_patterns: Vec<String>,
    /// Terminal command timeout in seconds.
    pub terminal_timeout: u64,
    /// Maximum file size to read (in bytes).
    pub max_file_size: usize,
    /// Whether to enable auto-format on save.
    pub auto_format: bool,
    /// Linter tools to enable.
    pub linters: Vec<String>,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            include_patterns: vec![
                "src/**/*.rs".to_string(),
                "*.toml".to_string(),
                "*.md".to_string(),
            ],
            exclude_patterns: vec![
                "target/**".to_string(),
                ".git/**".to_string(),
                "node_modules/**".to_string(),
            ],
            terminal_timeout: 30,
            max_file_size: 1024 * 1024, // 1MB
            auto_format: true,
            linters: vec!["clippy".to_string(), "rustfmt".to_string()],
        }
    }
}

// ──────────────────────────────────────────────
// Tool Types
// ──────────────────────────────────────────────

/// Trait for Cursor-specific tools.
#[async_trait::async_trait]
pub trait CursorTool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;
    /// Get the tool description.
    fn description(&self) -> &str;
    /// Get the JSON schema for tool parameters.
    fn parameters_schema(&self) -> serde_json::Value;
    /// Execute the tool with the given parameters.
    async fn execute(&self, params: serde_json::Value) -> crate::error::Result<serde_json::Value>;
}

/// Tool execution context passed to tools.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// The project root directory.
    pub project_root: PathBuf,
    /// Current IDE state (if available).
    pub ide_state: Option<IdeState>,
    /// Configuration.
    pub config: CursorConfig,
}

impl ToolContext {
    /// Create a new tool context.
    pub fn new(project_root: PathBuf, config: CursorConfig) -> Self {
        Self {
            project_root,
            ide_state: None,
            config,
        }
    }

    /// Update the IDE state.
    pub fn with_ide_state(mut self, state: IdeState) -> Self {
        self.ide_state = Some(state);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_position_display() {
        let pos = CursorPosition::new(10, 5);
        assert_eq!(pos.to_display(), "11:6");
    }

    #[test]
    fn selection_is_empty() {
        let sel = Selection {
            file_path: PathBuf::from("test.rs"),
            start: CursorPosition::new(0, 0),
            end: CursorPosition::new(0, 0),
            text: String::new(),
        };
        assert!(sel.is_empty());
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
    }

    #[test]
    fn default_config() {
        let config = CursorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.terminal_timeout, 30);
        assert!(!config.include_patterns.is_empty());
    }
}
