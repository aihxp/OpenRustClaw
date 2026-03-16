//! Adapters for different tool sources (MCP servers, OpenAPI specs)

pub mod mcp_adapter;
pub mod openapi_adapter;

use crate::discovery::{ToolHelp, ToolSummary};
use crate::error::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// A source of tools (MCP server, OpenAPI spec, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolSource {
    /// MCP server via HTTP/SSE
    McpUrl { url: String },
    /// MCP server via stdio
    McpStdio { command: String, args: Vec<String> },
    /// OpenAPI spec via URL
    OpenApiUrl { url: String },
    /// OpenAPI spec from file
    OpenApiFile { path: String },
}

impl ToolSource {
    /// Create a new MCP URL source
    pub fn mcp_url(url: impl Into<String>) -> Self {
        Self::McpUrl { url: url.into() }
    }

    /// Create a new MCP stdio source
    pub fn mcp_stdio(command: impl Into<String>) -> Self {
        Self::McpStdio {
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Create a new MCP stdio source with args
    pub fn mcp_stdio_with_args(command: impl Into<String>, args: Vec<String>) -> Self {
        Self::McpStdio {
            command: command.into(),
            args,
        }
    }

    /// Create a new OpenAPI URL source
    pub fn openapi_url(url: impl Into<String>) -> Self {
        Self::OpenApiUrl { url: url.into() }
    }

    /// Create a new OpenAPI file source
    pub fn openapi_file(path: impl Into<String>) -> Self {
        Self::OpenApiFile { path: path.into() }
    }

    /// Get cache key for this source
    pub fn cache_key(&self) -> String {
        match self {
            ToolSource::McpUrl { url } => format!("mcp_url:{}", url),
            ToolSource::McpStdio { command, args } => {
                format!("mcp_stdio:{}:{}", command, args.join(","))
            }
            ToolSource::OpenApiUrl { url } => format!("openapi_url:{}", url),
            ToolSource::OpenApiFile { path } => format!("openapi_file:{}", path),
        }
    }

    /// Create an adapter for this source
    pub async fn create_adapter(&self) -> Result<Arc<dyn ToolSourceAdapter>> {
        match self {
            ToolSource::McpUrl { url } => {
                let adapter = mcp_adapter::McpAdapter::from_url(url).await?;
                Ok(Arc::new(adapter) as Arc<dyn ToolSourceAdapter>)
            }
            ToolSource::McpStdio { command, args } => {
                let adapter = mcp_adapter::McpAdapter::from_stdio(command, args.clone()).await?;
                Ok(Arc::new(adapter) as Arc<dyn ToolSourceAdapter>)
            }
            ToolSource::OpenApiUrl { url } => {
                let adapter = openapi_adapter::OpenApiAdapter::from_url(url).await?;
                Ok(Arc::new(adapter) as Arc<dyn ToolSourceAdapter>)
            }
            ToolSource::OpenApiFile { path } => {
                let adapter = openapi_adapter::OpenApiAdapter::from_file(path).await?;
                Ok(Arc::new(adapter) as Arc<dyn ToolSourceAdapter>)
            }
        }
    }

    /// Get a human-readable description of this source
    pub fn description(&self) -> String {
        match self {
            ToolSource::McpUrl { url } => format!("MCP server at {}", url),
            ToolSource::McpStdio { command, .. } => format!("MCP server via {}", command),
            ToolSource::OpenApiUrl { url } => format!("OpenAPI spec at {}", url),
            ToolSource::OpenApiFile { path } => format!("OpenAPI spec at {}", path),
        }
    }
}

/// Trait for tool source adapters
#[async_trait]
pub trait ToolSourceAdapter: Send + Sync {
    /// List all available tools
    async fn list_tools(&self) -> Result<Vec<ToolSummary>>;

    /// Get detailed help for a specific tool
    async fn get_tool_help(&self, tool_name: &str) -> Result<ToolHelp>;

    /// Execute a tool with the given arguments
    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_source_mcp_url() {
        let source = ToolSource::mcp_url("http://localhost:3000/sse");
        assert!(matches!(source, ToolSource::McpUrl { .. }));
        assert!(source.cache_key().contains("mcp_url"));
    }

    #[test]
    fn test_tool_source_mcp_stdio() {
        let source = ToolSource::mcp_stdio("npx");
        assert!(matches!(source, ToolSource::McpStdio { .. }));
        
        let source = ToolSource::mcp_stdio_with_args("npx", vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]);
        assert!(source.cache_key().contains("npx"));
    }

    #[test]
    fn test_tool_source_openapi() {
        let url_source = ToolSource::openapi_url("https://api.example.com/openapi.json");
        assert!(matches!(url_source, ToolSource::OpenApiUrl { .. }));

        let file_source = ToolSource::openapi_file("./spec.yaml");
        assert!(matches!(file_source, ToolSource::OpenApiFile { .. }));
    }
}
