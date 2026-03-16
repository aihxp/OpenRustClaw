//! MCP server: expose OpenRustClaw tools as MCP endpoints.
//!
//! External MCP clients (Claude Desktop, Code, Cursor) can connect
//! and use OpenRustClaw's tools.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use openrustclaw_core::error::{Error, McpError, Result};

/// Configuration for the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub tools: Vec<McpServerTool>,
}

/// A tool exposed by the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// The MCP server that handles JSON-RPC requests from stdin/stdout.
pub struct McpServer {
    config: McpServerConfig,
    handlers: HashMap<String, Box<dyn Fn(Value) -> Result<Value> + Send + Sync>>,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            handlers: HashMap::new(),
        }
    }

    /// Register a tool handler.
    pub fn register_handler<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    /// Handle a JSON-RPC request.
    pub fn handle_request(&self, request: &Value) -> Value {
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let params = request
            .get("params")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        let result = match method {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&params),
            _ => Err(Error::Mcp(McpError::ToolNotFound {
                server: self.config.name.clone(),
                tool: method.to_string(),
            })),
        };

        match result {
            Ok(res) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": res,
            }),
            Err(e) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": e.to_string(),
                },
            }),
        }
    }

    fn handle_initialize(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.config.name,
                "version": self.config.version,
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value> {
        let tools: Vec<Value> = self
            .config
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();

        Ok(serde_json::json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, params: &Value) -> Result<Value> {
        let tool_name = params
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        if let Some(handler) = self.handlers.get(tool_name) {
            let result = handler(arguments)?;
            Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": result.to_string(),
                }]
            }))
        } else {
            Err(Error::Mcp(McpError::ToolNotFound {
                server: self.config.name.clone(),
                tool: tool_name.to_string(),
            }))
        }
    }
}
