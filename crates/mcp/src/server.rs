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
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
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
        let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> McpServerConfig {
        McpServerConfig {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            tools: vec![
                McpServerTool {
                    name: "echo".to_string(),
                    description: "Echoes input back".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" }
                        },
                        "required": ["message"]
                    }),
                },
                McpServerTool {
                    name: "add".to_string(),
                    description: "Adds two numbers".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "a": { "type": "number" },
                            "b": { "type": "number" }
                        },
                        "required": ["a", "b"]
                    }),
                },
            ],
        }
    }

    fn test_server() -> McpServer {
        let mut server = McpServer::new(test_config());
        server.register_handler("echo", |args| {
            let msg = args
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("no message");
            Ok(serde_json::json!({ "echoed": msg }))
        });
        server.register_handler("add", |args| {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(serde_json::json!({ "sum": a + b }))
        });
        server
    }

    // --- initialize ---

    #[test]
    fn handle_initialize_returns_protocol_version() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let resp = server.handle_request(&req);
        let result = resp.get("result").unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn handle_initialize_returns_server_info() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let resp = server.handle_request(&req);
        let info = &resp["result"]["serverInfo"];
        assert_eq!(info["name"], "test-server");
        assert_eq!(info["version"], "1.0.0");
    }

    #[test]
    fn handle_initialize_returns_capabilities() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "initialize",
            "params": {}
        });
        let resp = server.handle_request(&req);
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn handle_initialize_preserves_request_id() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "initialize",
            "params": {}
        });
        let resp = server.handle_request(&req);
        assert_eq!(resp["id"], 99);
        assert_eq!(resp["jsonrpc"], "2.0");
    }

    // --- tools/list ---

    #[test]
    fn handle_tools_list_returns_all_registered_tools() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        let resp = server.handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn handle_tools_list_includes_tool_names() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let resp = server.handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"add"));
    }

    #[test]
    fn handle_tools_list_includes_descriptions() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let resp = server.handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        let echo_tool = tools.iter().find(|t| t["name"] == "echo").unwrap();
        assert_eq!(echo_tool["description"], "Echoes input back");
    }

    #[test]
    fn handle_tools_list_includes_input_schema() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let resp = server.handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        let echo_tool = tools.iter().find(|t| t["name"] == "echo").unwrap();
        assert_eq!(echo_tool["inputSchema"]["type"], "object");
        assert!(echo_tool["inputSchema"]["properties"]["message"].is_object());
    }

    #[test]
    fn handle_tools_list_empty_when_no_tools_configured() {
        let config = McpServerConfig {
            name: "empty".to_string(),
            version: "1.0.0".to_string(),
            tools: vec![],
        };
        let server = McpServer::new(config);
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });
        let resp = server.handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert!(tools.is_empty());
    }

    // --- tools/call ---

    #[test]
    fn handle_tools_call_invokes_echo_handler() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "message": "hello world" }
            }
        });
        let resp = server.handle_request(&req);
        let content = resp["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("hello world"));
    }

    #[test]
    fn handle_tools_call_invokes_add_handler() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "add",
                "arguments": { "a": 3, "b": 7 }
            }
        });
        let resp = server.handle_request(&req);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("10"));
    }

    #[test]
    fn handle_tools_call_unknown_tool_returns_error() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "nonexistent",
                "arguments": {}
            }
        });
        let resp = server.handle_request(&req);
        assert!(resp.get("error").is_some());
        assert!(resp.get("result").is_none());
    }

    #[test]
    fn handle_tools_call_missing_arguments_uses_empty_object() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "echo"
            }
        });
        let resp = server.handle_request(&req);
        // Should succeed since echo handles missing message gracefully
        assert!(resp.get("result").is_some());
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("no message"));
    }

    #[test]
    fn handle_tools_call_preserves_request_id() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 777,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "message": "test" }
            }
        });
        let resp = server.handle_request(&req);
        assert_eq!(resp["id"], 777);
    }

    // --- unknown method ---

    #[test]
    fn handle_unknown_method_returns_error() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "resources/list",
            "params": {}
        });
        let resp = server.handle_request(&req);
        assert!(resp.get("error").is_some());
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn handle_prompts_list_returns_error() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "prompts/list",
            "params": {}
        });
        let resp = server.handle_request(&req);
        assert!(resp.get("error").is_some());
    }

    #[test]
    fn handle_completely_unknown_method_returns_error() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "foo/bar/baz"
        });
        let resp = server.handle_request(&req);
        assert!(resp.get("error").is_some());
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 12);
    }

    // --- malformed requests ---

    #[test]
    fn handle_request_missing_method_returns_error() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 20
        });
        let resp = server.handle_request(&req);
        // Missing method defaults to "" which is unknown
        assert!(resp.get("error").is_some());
    }

    #[test]
    fn handle_request_missing_id_uses_null_id() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize"
        });
        let resp = server.handle_request(&req);
        // Should still succeed; id defaults to null
        assert!(resp.get("result").is_some());
        assert!(resp["id"].is_null());
    }

    #[test]
    fn handle_request_string_id_preserved() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "req-abc-123",
            "method": "initialize"
        });
        let resp = server.handle_request(&req);
        assert_eq!(resp["id"], "req-abc-123");
    }

    #[test]
    fn handle_request_empty_object() {
        let server = test_server();
        let req = serde_json::json!({});
        let resp = server.handle_request(&req);
        // Method is "" which is unknown, should return error
        assert!(resp.get("error").is_some());
    }

    // --- error response format ---

    #[test]
    fn error_response_has_correct_jsonrpc_structure() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "unknown_method"
        });
        let resp = server.handle_request(&req);
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 30);
        assert!(resp["error"]["code"].is_number());
        assert!(resp["error"]["message"].is_string());
    }

    #[test]
    fn error_response_message_contains_useful_info() {
        let server = test_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": { "name": "does_not_exist" }
        });
        let resp = server.handle_request(&req);
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(msg.contains("does_not_exist") || msg.contains("not found"));
    }

    // --- handler registration ---

    #[test]
    fn register_handler_overwrites_existing() {
        let mut server = McpServer::new(test_config());
        server.register_handler("echo", |_args| Ok(serde_json::json!("first")));
        server.register_handler("echo", |_args| Ok(serde_json::json!("second")));

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "echo", "arguments": {} }
        });
        let resp = server.handle_request(&req);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("second"));
    }

    #[test]
    fn handler_returning_error_produces_error_response() {
        let mut server = McpServer::new(test_config());
        server.register_handler("echo", |_args| {
            Err(Error::Mcp(McpError::ToolExecution(
                "handler failed".to_string(),
            )))
        });

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "echo", "arguments": {} }
        });
        let resp = server.handle_request(&req);
        assert!(resp.get("error").is_some());
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(msg.contains("handler failed"));
    }

    // --- McpServerConfig serialization ---

    #[test]
    fn server_config_serializes_and_deserializes() {
        let config = test_config();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.tools.len(), 2);
    }

    #[test]
    fn server_tool_serializes_and_deserializes() {
        let tool = McpServerTool {
            name: "my_tool".to_string(),
            description: "A tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpServerTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "my_tool");
        assert_eq!(deserialized.description, "A tool");
    }
}
