//! Error types for the Cursor ACP integration.

use thiserror::Error;

/// Errors specific to Cursor ACP operations.
#[derive(Debug, Error)]
pub enum CursorError {
    #[error("ACP connection error: {0}")]
    Connection(String),

    #[error("ACP protocol error: {0}")]
    Protocol(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {tool}: {message}")]
    ToolExecution { tool: String, message: String },

    #[error("File operation error: {0}")]
    FileOperation(String),

    #[error("Git operation error: {0}")]
    GitOperation(String),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("Linter error: {0}")]
    Linter(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid context: {0}")]
    InvalidContext(String),

    #[error("Project root not found")]
    ProjectRootNotFound,

    #[error("Pattern error: {0}")]
    PatternError(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Result type alias for Cursor operations.
pub type Result<T> = std::result::Result<T, CursorError>;
