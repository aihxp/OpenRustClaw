//! Adapter for MCP servers
//!
//! This adapter connects to MCP servers via HTTP/SSE or stdio and provides
//! a unified interface for tool discovery and execution.

use crate::adapters::ToolSourceAdapter;
use crate::discovery::{ParamHelp, ToolHelp, ToolSummary};
use crate::error::{Mcp2CliError, Result};
use async_trait::async_trait;
use openrustclaw_mcp::McpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Adapter for MCP servers
pub struct McpAdapter {
    client: Mutex<McpClient>,
}

impl McpAdapter {
    /// Connect to an MCP server via HTTP/SSE URL
    pub async fn from_url(url: &str) -> Result<Self> {
        debug!("Connecting to MCP server at {}", url);

        Err(Mcp2CliError::mcp(format!(
            "Remote MCP over HTTP/SSE is not implemented for mcp2-cli in this repo: {}. Use --mcp-stdio or --spec instead.",
            url
        )))
    }

    /// Connect to an MCP server via stdio
    pub async fn from_stdio(command: &str, args: Vec<String>) -> Result<Self> {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        info!("Starting MCP server via {} {:?}", command, args);

        let client = McpClient::connect("stdio_server", command, &args_refs)
            .await
            .map_err(|e| Mcp2CliError::mcp(format!("Failed to start MCP server: {}", e)))?;

        info!("MCP server started successfully");

        Ok(Self {
            client: Mutex::new(client),
        })
    }

    async fn load_tools(&self) -> Result<Vec<McpToolDef>> {
        let mut client = self.client.lock().await;
        let tools = client
            .discover_tools()
            .await
            .map_err(|e| Mcp2CliError::mcp(format!("Failed to discover MCP tools: {}", e)))?;

        Ok(tools
            .into_iter()
            .map(|tool| McpToolDef {
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
            })
            .collect())
    }

    /// Convert MCP tool definition to ToolSummary
    #[allow(dead_code)]
    fn mcp_tool_to_summary(tool: &McpToolDef) -> ToolSummary {
        // Create a compact description (first sentence only)
        let description = tool
            .description
            .split('.')
            .next()
            .unwrap_or(&tool.description)
            .trim()
            .to_string();

        ToolSummary::new(&tool.name, description)
    }

    /// Convert MCP tool definition to ToolHelp
    #[allow(dead_code)]
    fn mcp_tool_to_help(tool: &McpToolDef) -> ToolHelp {
        let parameters = Self::extract_parameters(&tool.input_schema);

        // Generate usage string
        let usage = if parameters.is_empty() {
            tool.name.to_string()
        } else {
            let required_params: Vec<_> = parameters
                .iter()
                .filter(|p| p.required)
                .map(|p| format!("--{} <{}>", p.name, p.type_name))
                .collect();

            let optional_params: Vec<_> = parameters
                .iter()
                .filter(|p| !p.required)
                .map(|p| format!("[--{} <{}>]", p.name, p.type_name))
                .collect();

            let mut parts = vec![tool.name.clone()];
            parts.extend(required_params);
            parts.extend(optional_params);
            parts.join(" ")
        };

        ToolHelp::new(&tool.name, &tool.description, usage, parameters)
    }

    /// Extract parameter help from JSON schema
    #[allow(dead_code)]
    fn extract_parameters(schema: &Value) -> Vec<ParamHelp> {
        let mut params = Vec::new();

        let properties = match schema.get("properties") {
            Some(Value::Object(props)) => props,
            _ => return params,
        };

        let required: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for (name, prop) in properties {
            let description = prop
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let type_name = Self::json_schema_type(prop);
            let is_required = required.contains(name);

            let default = prop.get("default").cloned();
            let example = prop.get("example").map(|e| match e {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });

            let mut param = ParamHelp::new(name, description, type_name, is_required);

            if let Some(d) = default {
                param = param.with_default(d);
            }
            if let Some(e) = example {
                param = param.with_example(e);
            }

            params.push(param);
        }

        params
    }

    /// Get JSON schema type name
    #[allow(dead_code)]
    fn json_schema_type(prop: &Value) -> String {
        prop.get("type")
            .and_then(|t| t.as_str())
            .map(String::from)
            .or_else(|| {
                // Handle enum types
                if prop.get("enum").is_some() {
                    Some("enum".to_string())
                } else if prop.get("items").is_some() {
                    Some("array".to_string())
                } else {
                    Some("any".to_string())
                }
            })
            .unwrap_or_else(|| "any".to_string())
    }

    /// Convert CLI-style arguments to JSON
    #[allow(dead_code)]
    fn args_to_json(args: &Value, params: &[ParamHelp]) -> Value {
        // If args is already an object, return it
        if let Value::Object(_) = args {
            return args.clone();
        }

        // If args is a string, try to parse it as CLI arguments
        if let Some(arg_str) = args.as_str() {
            let mut result = serde_json::Map::new();
            let tokens: Vec<&str> = arg_str.split_whitespace().collect();
            let mut i = 0;

            while i < tokens.len() {
                let token = tokens[i];

                // Check if it's a flag (--param-name)
                if let Some(param_name) = token.strip_prefix("--") {
                    // Find the parameter definition
                    if let Some(param_def) = params.iter().find(|p| p.name == param_name) {
                        if param_def.type_name == "boolean" {
                            // Boolean flags don't need a value
                            result.insert(param_name.to_string(), Value::Bool(true));
                        } else if i + 1 < tokens.len() {
                            // Take the next token as value
                            let value = Self::parse_value(tokens[i + 1], &param_def.type_name);
                            result.insert(param_name.to_string(), value);
                            i += 1; // Skip the value token
                        }
                    }
                }

                i += 1;
            }

            return Value::Object(result);
        }

        args.clone()
    }

    /// Parse a string value to appropriate JSON type
    #[allow(dead_code)]
    fn parse_value(s: &str, type_name: &str) -> Value {
        match type_name {
            "number" | "integer" => {
                if let Ok(n) = s.parse::<i64>() {
                    Value::Number(n.into())
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::String(s.to_string()))
                } else {
                    Value::String(s.to_string())
                }
            }
            "boolean" => s
                .parse::<bool>()
                .map(Value::Bool)
                .unwrap_or_else(|_| Value::String(s.to_string())),
            _ => Value::String(s.to_string()),
        }
    }
}

#[async_trait]
impl ToolSourceAdapter for McpAdapter {
    async fn list_tools(&self) -> Result<Vec<ToolSummary>> {
        let tools = self.load_tools().await?;
        Ok(tools.iter().map(Self::mcp_tool_to_summary).collect())
    }

    async fn get_tool_help(&self, tool_name: &str) -> Result<ToolHelp> {
        let tools = self.load_tools().await?;
        let tool = tools
            .iter()
            .find(|tool| tool.name == tool_name)
            .ok_or_else(|| Mcp2CliError::tool_not_found(tool_name))?;

        Ok(Self::mcp_tool_to_help(tool))
    }

    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<String> {
        info!(tool_name = %tool_name, "Executing MCP tool");
        let mut client = self.client.lock().await;
        let output = client
            .call_tool(tool_name, args)
            .await
            .map_err(|e| Mcp2CliError::mcp(format!("Failed to execute MCP tool: {}", e)))?;
        Ok(output.content)
    }
}

/// MCP tool definition (mirrors the MCP spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_parameters() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results",
                    "default": 10
                }
            },
            "required": ["query"]
        });

        let params = McpAdapter::extract_parameters(&schema);
        assert_eq!(params.len(), 2);

        let query_param = params.iter().find(|p| p.name == "query").unwrap();
        assert!(query_param.required);
        assert_eq!(query_param.type_name, "string");

        let limit_param = params.iter().find(|p| p.name == "limit").unwrap();
        assert!(!limit_param.required);
        assert_eq!(limit_param.default, Some(serde_json::json!(10)));
    }

    #[test]
    fn test_args_to_json() {
        let params = vec![
            ParamHelp::new("query", "Search", "string", true),
            ParamHelp::new("limit", "Limit", "number", false),
        ];

        let args = Value::String("--query rust --limit 10".to_string());
        let result = McpAdapter::args_to_json(&args, &params);

        assert_eq!(result["query"], "rust");
        assert_eq!(result["limit"], 10.0);
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(McpAdapter::parse_value("123", "number"), 123.0);
        assert_eq!(McpAdapter::parse_value("true", "boolean"), true);
        assert_eq!(McpAdapter::parse_value("hello", "string"), "hello");
    }

    #[tokio::test]
    async fn from_url_returns_honest_unsupported_error() {
        let err = match McpAdapter::from_url("https://mcp.example.com/sse").await {
            Err(err) => err,
            Ok(_) => panic!("expected unsupported URL error"),
        };
        assert!(err.to_string().contains("not implemented"));
        assert!(err.to_string().contains("--mcp-stdio"));
    }
}
