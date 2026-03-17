//! MCP client: connect to external MCP servers, discover tools, invoke them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use openrustclaw_core::error::Result;
use openrustclaw_core::types::ToolOutput;

use crate::transport::StdioTransport;

/// An MCP tool definition discovered from a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(alias = "inputSchema")]
    pub input_schema: Value,
    pub server_name: String,
}

/// MCP client connected to a single server.
pub struct McpClient {
    transport: StdioTransport,
    server_name: String,
    tools: Vec<McpToolDef>,
}

impl McpClient {
    /// Connect to an MCP server via stdio.
    pub async fn connect(server_name: &str, command: &str, args: &[&str]) -> Result<Self> {
        let mut transport = StdioTransport::spawn(command, args).await?;

        // Initialize the connection per the MCP specification.
        let _init_result = transport
            .request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "openrustclaw",
                        "version": "0.1.0"
                    }
                })),
            )
            .await?;

        info!(server = %server_name, "MCP client connected");

        Ok(Self {
            transport,
            server_name: server_name.to_string(),
            tools: Vec::new(),
        })
    }

    /// Discover available tools from the server.
    pub async fn discover_tools(&mut self) -> Result<Vec<McpToolDef>> {
        let result = self.transport.request("tools/list", None).await?;

        let tools_array = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        self.tools = tools_array
            .iter()
            .filter_map(|t| {
                Some(McpToolDef {
                    name: t.get("name")?.as_str()?.to_string(),
                    description: t
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: t
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new())),
                    server_name: self.server_name.clone(),
                })
            })
            .collect();

        info!(
            server = %self.server_name,
            count = self.tools.len(),
            "Discovered MCP tools"
        );
        Ok(self.tools.clone())
    }

    /// Invoke a tool on the connected server.
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<ToolOutput> {
        let result = self
            .transport
            .request(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments,
                })),
            )
            .await?;

        // MCP returns a content array.
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| result.to_string());

        let is_error = result
            .get("isError")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);

        Ok(ToolOutput {
            tool_call_id: String::new(), // Will be set by caller
            content,
            is_error,
        })
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get cached tool list.
    pub fn tools(&self) -> &[McpToolDef] {
        &self.tools
    }

    /// Shut down the connection.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.transport.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- McpToolDef ---

    #[test]
    fn mcp_tool_def_serialization_roundtrip() {
        let tool = McpToolDef {
            name: "read_file".to_string(),
            description: "Read a file from disk".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            server_name: "filesystem".to_string(),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpToolDef = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "read_file");
        assert_eq!(deserialized.description, "Read a file from disk");
        assert_eq!(deserialized.server_name, "filesystem");
        assert_eq!(deserialized.input_schema["type"], "object");
    }

    #[test]
    fn mcp_tool_def_clone_is_independent() {
        let tool = McpToolDef {
            name: "tool".to_string(),
            description: "desc".to_string(),
            input_schema: serde_json::json!({}),
            server_name: "srv".to_string(),
        };
        let cloned = tool.clone();
        assert_eq!(cloned.name, tool.name);
        assert_eq!(cloned.server_name, tool.server_name);
    }

    #[test]
    fn mcp_tool_def_debug_format() {
        let tool = McpToolDef {
            name: "test".to_string(),
            description: "test desc".to_string(),
            input_schema: serde_json::json!({}),
            server_name: "srv".to_string(),
        };
        let debug = format!("{:?}", tool);
        assert!(debug.contains("test"));
        assert!(debug.contains("srv"));
    }

    #[test]
    fn mcp_tool_def_with_complex_schema() {
        let tool = McpToolDef {
            name: "query".to_string(),
            description: "Query database".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string" },
                    "params": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "timeout": { "type": "integer" },
                            "readonly": { "type": "boolean" }
                        }
                    }
                },
                "required": ["sql"]
            }),
            server_name: "db-server".to_string(),
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(
            json["input_schema"]["properties"]["params"]["type"],
            "array"
        );
        assert_eq!(
            json["input_schema"]["properties"]["options"]["properties"]["timeout"]["type"],
            "integer"
        );
    }

    // --- connect rejects disallowed commands ---

    #[tokio::test]
    async fn connect_rejects_disallowed_command() {
        let result = McpClient::connect("test", "bash", &["-c", "echo"]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn connect_rejects_shell_injection() {
        let result = McpClient::connect("test", "node; rm -rf /", &[]).await;
        assert!(result.is_err());
    }
}
