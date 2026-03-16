//! Error types for mcp2cli

use thiserror::Error;

/// Result type alias for mcp2cli
pub type Result<T> = std::result::Result<T, Mcp2CliError>;

/// Errors that can occur in mcp2cli
#[derive(Error, Debug)]
pub enum Mcp2CliError {
    /// Error from the core crate
    #[error("core error: {0}")]
    Core(#[from] openrustclaw_core::error::Error),

    /// Error from the MCP crate
    #[error("MCP error: {0}")]
    Mcp(String),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// OpenAPI parsing error
    #[error("OpenAPI error: {0}")]
    OpenApi(String),

    /// Authentication error
    #[error("authentication error: {0}")]
    Auth(String),

    /// Cache error
    #[error("cache error: {0}")]
    Cache(String),

    /// Tool not found
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// Endpoint not found
    #[error("endpoint not found: {0}")]
    EndpointNotFound(String),

    /// Invalid TOON format
    #[error("invalid TOON format: {0}")]
    InvalidToon(String),

    /// CLI generation error
    #[error("CLI generation error: {0}")]
    CliGeneration(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// General error
    #[error("{0}")]
    Other(String),
}

impl Mcp2CliError {
    /// Create a new MCP error
    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    /// Create a new authentication error
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Create a new OpenAPI error
    pub fn openapi(msg: impl Into<String>) -> Self {
        Self::OpenApi(msg.into())
    }

    /// Create a new cache error
    pub fn cache(msg: impl Into<String>) -> Self {
        Self::Cache(msg.into())
    }

    /// Create a new CLI generation error
    pub fn cli(msg: impl Into<String>) -> Self {
        Self::CliGeneration(msg.into())
    }

    /// Create a new TOON error
    pub fn toon(msg: impl Into<String>) -> Self {
        Self::InvalidToon(msg.into())
    }

    /// Create a new other error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Create a tool not found error
    pub fn tool_not_found(name: impl Into<String>) -> Self {
        Self::ToolNotFound(name.into())
    }
}


