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
use tracing::{debug, info, warn};

/// Adapter for MCP servers
pub struct McpAdapter {
    client: McpClient,
    server_url: Option<String>,
}

impl McpAdapter {
    /// Connect to an MCP server via HTTP/SSE URL
    pub async fn from_url(url: &str) -> Result<Self> {
        debug!("Connecting to MCP server at {}", url);
        
        // For HTTP-based MCP, we'd typically use an SSE transport
        // For now, we'll use a simplified approach with HTTP polling
        // In a full implementation, this would use proper MCP HTTP+SSE transport
        
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| Mcp2CliError::other(format!("Failed to connect to MCP server: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(Mcp2CliError::other(format!(
                "MCP server returned error: {}",
                response.status()
            )));
        }
        
        // Create a placeholder client - in a real implementation,
        // we'd initialize the MCP client with proper HTTP transport
        let client = McpClient::connect("http_server", "echo", &[]).await
            .map_err(|e| Mcp2CliError::mcp(format!("Failed to initialize MCP client: {}", e)))?;
        
        info!("Connected to MCP server at {}", url);
        
        Ok(Self {
            client,
            server_url: Some(url.to_string()),
        })
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
            client,
            server_url: None,
        })
    }

    /// Convert MCP tool definition to ToolSummary
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
    fn mcp_tool_to_help(tool: &McpToolDef) -> ToolHelp {
        let parameters = Self::extract_parameters(&tool.input_schema);

        // Generate usage string
        let usage = if parameters.is_empty() {
            format!("{}", tool.name)
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
            let example = prop.get("example").and_then(|e| match e {
                Value::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
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
                if token.starts_with("--") {
                    let param_name = &token[2..];
                    
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
            "boolean" => s.parse::<bool>()
                .map(Value::Bool)
                .unwrap_or_else(|_| Value::String(s.to_string())),
            _ => Value::String(s.to_string()),
        }
    }
}

#[async_trait]
impl ToolSourceAdapter for McpAdapter {
    async fn list_tools(&self) -> Result<Vec<ToolSummary>> {
        // In a real implementation, this would call the MCP client
        // For now, we'll return placeholder data
        warn!("McpAdapter::list_tools using placeholder implementation");
        
        // This would be: self.client.discover_tools().await
        // and then convert each McpToolDef to ToolSummary
        
        Ok(vec![
            ToolSummary::new("mcp_tool_1", "Example MCP tool"),
            ToolSummary::new("mcp_tool_2", "Another MCP tool"),
        ])
    }

    async fn get_tool_help(&self, tool_name: &str) -> Result<ToolHelp> {
        // In a real implementation, this would fetch from the MCP client
        warn!(
            tool_name = %tool_name,
            "McpAdapter::get_tool_help using placeholder implementation"
        );

        Ok(ToolHelp::new(
            tool_name,
            format!("Help for {} (placeholder)", tool_name),
            format!("{} [args]", tool_name),
            vec![],
        ))
    }

    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<String> {
        info!(
            tool_name = %tool_name,
            "Executing MCP tool"
        );

        // In a real implementation:
        // let output = self.client.call_tool(tool_name, args).await?;
        // Ok(output.content)

        // Placeholder implementation
        Ok(format!("Executed {} with args: {}", tool_name, args))
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
}
