//! Integration with OpenRustClaw agent runtime
//!
//! This module provides integration between mcp2cli and the OpenRustClaw
//! agent runtime, allowing mcp2cli sources to be used as standard tools.

use crate::adapters::ToolSource;
use crate::discovery::ToolDiscovery;
use crate::error::Result as Mcp2CliResult;
use crate::toon::encode_toon;
use async_trait::async_trait;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{SkillCapability, ToolOutput};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// mcp2cli tool wrapper for the agent runtime
///
/// This wraps a ToolSource (MCP server or OpenAPI spec) as a Tool
/// that can be registered with the agent runtime.
pub struct Mcp2CliTool {
    discovery: ToolDiscovery,
    source: ToolSource,
    name: String,
    description: String,
}

impl Mcp2CliTool {
    /// Create a new mcp2cli tool wrapper
    pub fn new(discovery: ToolDiscovery, source: ToolSource, name: String, description: String) -> Self {
        Self {
            discovery,
            source,
            name,
            description,
        }
    }

    /// Get the underlying source
    pub fn source(&self) -> &ToolSource {
        &self.source
    }

    /// Get the discovery instance
    pub fn discovery(&self) -> &ToolDiscovery {
        &self.discovery
    }

    /// List available tools from this source
    pub async fn list_available_tools(&self) -> Mcp2CliResult<Vec<crate::discovery::ToolSummary>> {
        self.discovery.list_tools(&self.source).await
    }

    /// Get help for a specific tool
    pub async fn get_tool_help(&self, tool_name: &str) -> Mcp2CliResult<crate::discovery::ToolHelp> {
        self.discovery.get_help(&self.source, tool_name).await
    }

    /// Clear the cache for this source
    pub fn clear_cache(&self) {
        self.discovery.clear_cache(&self.source);
    }
}

#[async_trait]
impl Tool for Mcp2CliTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        // Return a schema that accepts:
        // - action: "list" | "help" | "execute"
        // - tool_name: string (for help and execute)
        // - args: object (for execute)
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "help", "execute"],
                    "description": "Action to perform: list tools, get help, or execute a tool"
                },
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool (required for help and execute actions)"
                },
                "args": {
                    "type": "object",
                    "description": "Arguments for tool execution (required for execute action)"
                },
                "use_toon": {
                    "type": "boolean",
                    "description": "Use TOON format for output (40-60% fewer tokens)",
                    "default": true
                }
            },
            "required": ["action"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        // Requires network access for MCP/OpenAPI sources
        vec![SkillCapability::NetworkAccess]
    }

    #[instrument(skip(self, input, ctx), fields(tool_name = %self.name))]
    async fn execute(&self, input: Value, ctx: &ToolContext) -> openrustclaw_core::error::Result<ToolOutput> {
        use openrustclaw_core::error::ToolError;

        let action = input
            .get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InputValidation { 
                tool: self.name.clone(), 
                message: "Missing 'action' field".into() 
            })?;

        let use_toon = input
            .get("use_toon")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        match action {
            "list" => {
                debug!("Listing tools from source");
                let tools = self.discovery.list_tools(&self.source).await
                    .map_err(|e| ToolError::ExecutionFailed { 
                        tool: self.name.clone(), 
                        message: e.to_string() 
                    })?;
                
                let output = tools
                    .iter()
                    .map(|t| t.to_compact_string())
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(ToolOutput {
                    tool_call_id: ctx.session_id.clone(),
                    content: output,
                    is_error: false,
                })
            }
            "help" => {
                let tool_name = input
                    .get("tool_name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| ToolError::InputValidation { 
                        tool: self.name.clone(), 
                        message: "Missing 'tool_name' for help action".into() 
                    })?;

                debug!(tool_name = %tool_name, "Getting tool help");
                let help = self.discovery.get_help(&self.source, tool_name).await
                    .map_err(|e| ToolError::ExecutionFailed { 
                        tool: self.name.clone(), 
                        message: e.to_string() 
                    })?;

                let content = if use_toon {
                    let help_json = serde_json::json!({
                        "name": help.name,
                        "description": help.description,
                        "usage": help.usage,
                        "parameters": help.parameters.iter().map(|p| {
                            serde_json::json!({
                                "name": p.name,
                                "type": p.type_name,
                                "required": p.required
                            })
                        }).collect::<Vec<_>>()
                    });
                    encode_toon(&help_json)
                } else {
                    help.to_compact_string()
                };

                Ok(ToolOutput {
                    tool_call_id: ctx.session_id.clone(),
                    content,
                    is_error: false,
                })
            }
            "execute" => {
                let tool_name = input
                    .get("tool_name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| ToolError::InputValidation { 
                        tool: self.name.clone(), 
                        message: "Missing 'tool_name' for execute action".into() 
                    })?;

                let args = input
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                info!(
                    tool_name = %tool_name,
                    session_id = %ctx.session_id,
                    "Executing tool via mcp2cli"
                );

                let result = self.discovery.execute(&self.source, tool_name, args).await
                    .map_err(|e| ToolError::ExecutionFailed { 
                        tool: tool_name.to_string(), 
                        message: e.to_string() 
                    })?;

                // Try to parse as JSON and convert to TOON if requested
                let content = if use_toon {
                    if let Ok(json) = serde_json::from_str::<Value>(&result) {
                        encode_toon(&json)
                    } else {
                        result
                    }
                } else {
                    result
                };

                Ok(ToolOutput {
                    tool_call_id: ctx.session_id.clone(),
                    content,
                    is_error: false,
                })
            }
            _ => Err(ToolError::InputValidation { 
                tool: self.name.clone(), 
                message: format!("Unknown action: {}. Use 'list', 'help', or 'execute'", action)
            }.into()),
        }
    }
}

/// Factory for creating mcp2cli tools
pub struct Mcp2CliFactory;

impl Mcp2CliFactory {
    /// Create a tool from an MCP server URL
    pub async fn from_mcp_url(url: &str) -> Mcp2CliResult<Mcp2CliTool> {
        let source = ToolSource::mcp_url(url);
        let discovery = ToolDiscovery::new();

        // Try to list tools to validate connection
        let tools = discovery.list_tools(&source).await?;
        let tool_count = tools.len();

        info!(url = %url, tool_count = %tool_count, "Created mcp2cli tool from MCP URL");

        Ok(Mcp2CliTool::new(
            discovery,
            source,
            format!("mcp_{}", sanitize_name(url)),
            format!("MCP server at {} ({} tools)", url, tool_count),
        ))
    }

    /// Create a tool from an MCP server command (stdio)
    pub async fn from_mcp_stdio(command: &str, args: Vec<String>) -> Mcp2CliResult<Mcp2CliTool> {
        let source = ToolSource::mcp_stdio_with_args(command, args.clone());
        let discovery = ToolDiscovery::new();

        let tools = discovery.list_tools(&source).await?;
        let tool_count = tools.len();

        info!(
            command = %command,
            tool_count = %tool_count,
            "Created mcp2cli tool from MCP stdio"
        );

        Ok(Mcp2CliTool::new(
            discovery,
            source,
            format!("mcp_{}", sanitize_name(command)),
            format!("MCP server via {} ({} tools)", command, tool_count),
        ))
    }

    /// Create a tool from an OpenAPI spec URL
    pub async fn from_openapi_url(url: &str) -> Mcp2CliResult<Mcp2CliTool> {
        let source = ToolSource::openapi_url(url);
        let discovery = ToolDiscovery::new();

        let tools = discovery.list_tools(&source).await?;
        let tool_count = tools.len();

        info!(
            url = %url,
            tool_count = %tool_count,
            "Created mcp2cli tool from OpenAPI URL"
        );

        Ok(Mcp2CliTool::new(
            discovery,
            source,
            format!("openapi_{}", sanitize_name(url)),
            format!("OpenAPI spec at {} ({} endpoints)", url, tool_count),
        ))
    }

    /// Create a tool from an OpenAPI spec file
    pub async fn from_openapi_file(path: &str) -> Mcp2CliResult<Mcp2CliTool> {
        let source = ToolSource::openapi_file(path);
        let discovery = ToolDiscovery::new();

        let tools = discovery.list_tools(&source).await?;
        let tool_count = tools.len();

        info!(
            path = %path,
            tool_count = %tool_count,
            "Created mcp2cli tool from OpenAPI file"
        );

        Ok(Mcp2CliTool::new(
            discovery,
            source,
            format!("openapi_{}", sanitize_name(path)),
            format!("OpenAPI spec at {} ({} endpoints)", path, tool_count),
        ))
    }

    /// Create a tool with custom name and description
    pub fn with_name(
        source: ToolSource,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Mcp2CliTool {
        Mcp2CliTool::new(ToolDiscovery::new(), source, name.into(), description.into())
    }
}

/// Sanitize a name for use as a tool identifier
fn sanitize_name(input: &str) -> String {
    input
        .replace("https://", "")
        .replace("http://", "")
        .replace('/', "_")
        .replace('.', "_")
        .replace(':', "_")
        .replace('-', "_")
        .replace(' ', "_")
        .to_lowercase()
}

/// Registry for managing multiple mcp2cli sources
pub struct Mcp2CliRegistry {
    sources: DashMap<String, Arc<Mcp2CliTool>>,
}

use dashmap::DashMap;

impl Mcp2CliRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            sources: DashMap::new(),
        }
    }

    /// Register a new source
    pub fn register(&self, name: impl Into<String>, tool: Mcp2CliTool) {
        let name = name.into();
        self.sources.insert(name, Arc::new(tool));
    }

    /// Get a registered source
    pub fn get(&self, name: &str) -> Option<Arc<Mcp2CliTool>> {
        self.sources.get(name).map(|t| t.clone())
    }

    /// List all registered sources
    pub fn list(&self) -> Vec<(String, String)> {
        self.sources
            .iter()
            .map(|e| (e.key().clone(), e.description().to_string()))
            .collect()
    }

    /// Remove a source
    pub fn remove(&self, name: &str) -> Option<Arc<Mcp2CliTool>> {
        self.sources.remove(name).map(|(_, v)| v)
    }

    /// Clear all sources
    pub fn clear(&self) {
        self.sources.clear();
    }

    /// Get all sources as Tools
    pub fn get_all_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.sources
            .iter()
            .map(|e| e.clone() as Arc<dyn Tool>)
            .collect()
    }
}

impl Default for Mcp2CliRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("https://api.example.com"), "api_example_com");
        assert_eq!(sanitize_name("my-command"), "my_command");
        assert_eq!(sanitize_name("test:8080/path"), "test_8080_path");
    }

    #[test]
    fn test_mcp2cli_tool_schema() {
        let tool = Mcp2CliFactory::with_name(
            ToolSource::mcp_url("http://test"),
            "test_tool",
            "Test tool",
        );

        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["tool_name"].is_object());
        assert!(schema["properties"]["args"].is_object());
        assert!(schema["properties"]["use_toon"].is_object());
    }

    #[test]
    fn test_registry() {
        let registry = Mcp2CliRegistry::new();
        
        let tool = Mcp2CliFactory::with_name(
            ToolSource::mcp_url("http://test"),
            "test_source",
            "Test source for registry",
        );
        
        registry.register("test", tool);
        
        assert!(registry.get("test").is_some());
        assert_eq!(registry.list().len(), 1);
        
        registry.remove("test");
        assert!(registry.get("test").is_none());
    }
}
