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
