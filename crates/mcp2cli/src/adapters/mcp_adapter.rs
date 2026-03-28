//! Adapter for MCP servers.
//!
//! Supports MCP servers over stdio and remote legacy HTTP/SSE transports.

use std::time::Duration;

use crate::adapters::ToolSourceAdapter;
use crate::discovery::{ParamHelp, ToolHelp, ToolSummary};
use crate::error::{Mcp2CliError, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use openrustclaw_core::types::ToolOutput;
use openrustclaw_mcp::McpClient;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use url::Url;

/// Adapter for MCP servers.
pub struct McpAdapter {
    client: Mutex<McpClientHandle>,
}

enum McpClientHandle {
    Local(McpClient),
    Remote(RemoteSseClient),
}

struct RemoteSseClient {
    http: reqwest::Client,
    post_url: Url,
    events: mpsc::UnboundedReceiver<RemoteEvent>,
    next_id: u64,
    _reader_task: JoinHandle<()>,
}

enum RemoteEvent {
    Endpoint(String),
    Message(Value),
    Closed,
}

impl McpAdapter {
    /// Connect to an MCP server via HTTP/SSE URL.
    pub async fn from_url(url: &str) -> Result<Self> {
        debug!("Connecting to MCP server at {}", url);
        let client = RemoteSseClient::connect(url).await?;
        info!("Connected to remote MCP server over SSE");
        Ok(Self {
            client: Mutex::new(McpClientHandle::Remote(client)),
        })
    }

    /// Connect to an MCP server via stdio.
    pub async fn from_stdio(command: &str, args: Vec<String>) -> Result<Self> {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        info!("Starting MCP server via {} {:?}", command, args);

        let client = McpClient::connect("stdio_server", command, &args_refs)
            .await
            .map_err(|e| Mcp2CliError::mcp(format!("Failed to start MCP server: {}", e)))?;

        info!("MCP server started successfully");

        Ok(Self {
            client: Mutex::new(McpClientHandle::Local(client)),
        })
    }

    async fn load_tools(&self) -> Result<Vec<McpToolDef>> {
        let mut client = self.client.lock().await;
        let tools = match &mut *client {
            McpClientHandle::Local(client) => client
                .discover_tools()
                .await
                .map_err(|e| Mcp2CliError::mcp(format!("Failed to discover MCP tools: {}", e)))?,
            McpClientHandle::Remote(client) => client.discover_tools().await?,
        };

        Ok(tools
            .into_iter()
            .map(|tool| McpToolDef {
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
            })
            .collect())
    }

    async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<ToolOutput> {
        let mut client = self.client.lock().await;
        match &mut *client {
            McpClientHandle::Local(client) => client
                .call_tool(tool_name, arguments)
                .await
                .map_err(|e| Mcp2CliError::mcp(format!("Failed to execute MCP tool: {}", e))),
            McpClientHandle::Remote(client) => client.call_tool(tool_name, arguments).await,
        }
    }

    /// Convert MCP tool definition to ToolSummary.
    #[allow(dead_code)]
    fn mcp_tool_to_summary(tool: &McpToolDef) -> ToolSummary {
        let description = tool
            .description
            .split('.')
            .next()
            .unwrap_or(&tool.description)
            .trim()
            .to_string();

        ToolSummary::new(&tool.name, description)
    }

    /// Convert MCP tool definition to ToolHelp.
    #[allow(dead_code)]
    fn mcp_tool_to_help(tool: &McpToolDef) -> ToolHelp {
        let parameters = Self::extract_parameters(&tool.input_schema);

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

    /// Extract parameter help from JSON schema.
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

    /// Get JSON schema type name.
    #[allow(dead_code)]
    fn json_schema_type(prop: &Value) -> String {
        prop.get("type")
            .and_then(|t| t.as_str())
            .map(String::from)
            .or_else(|| {
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

    /// Convert CLI-style arguments to JSON.
    #[allow(dead_code)]
    fn args_to_json(args: &Value, params: &[ParamHelp]) -> Value {
        if let Value::Object(_) = args {
            return args.clone();
        }

        if let Some(arg_str) = args.as_str() {
            let mut result = serde_json::Map::new();
            let tokens: Vec<&str> = arg_str.split_whitespace().collect();
            let mut i = 0;

            while i < tokens.len() {
                let token = tokens[i];

                if let Some(param_name) = token.strip_prefix("--")
                    && let Some(param_def) = params.iter().find(|p| p.name == param_name)
                {
                    if param_def.type_name == "boolean" {
                        result.insert(param_name.to_string(), Value::Bool(true));
                    } else if i + 1 < tokens.len() {
                        let value = Self::parse_value(tokens[i + 1], &param_def.type_name);
                        result.insert(param_name.to_string(), value);
                        i += 1;
                    }
                }

                i += 1;
            }

            return Value::Object(result);
        }

        args.clone()
    }

    /// Parse a string value to appropriate JSON type.
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

impl RemoteSseClient {
    async fn connect(url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let sse_url = Url::parse(url)
            .map_err(|error| Mcp2CliError::mcp(format!("Invalid MCP URL '{}': {}", url, error)))?;

        let response = http
            .get(sse_url.clone())
            .header(ACCEPT, "text/event-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Mcp2CliError::mcp(format!(
                "Failed to connect to remote MCP SSE endpoint {}: HTTP {} {}",
                sse_url, status, body
            )));
        }

        let (tx, mut rx_init) = mpsc::unbounded_channel();
        let stream_base_url = response.url().clone();
        let reader_task = tokio::spawn(async move {
            let mut stream = response.bytes_stream().eventsource();
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        let event_name = event.event.as_str();
                        if event_name == "endpoint" {
                            let _ = tx.send(RemoteEvent::Endpoint(event.data.clone()));
                            continue;
                        }
                        if event.data == "[DONE]" {
                            continue;
                        }
                        match serde_json::from_str::<Value>(&event.data) {
                            Ok(payload) => {
                                let _ = tx.send(RemoteEvent::Message(payload));
                            }
                            Err(error) => {
                                warn!(
                                    error = %error,
                                    event = event_name,
                                    "Ignoring invalid MCP SSE payload"
                                );
                            }
                        }
                    }
                    Err(error) => {
                        warn!(error = %error, "Remote MCP SSE stream ended with error");
                        break;
                    }
                }
            }
            let _ = tx.send(RemoteEvent::Closed);
        });

        let endpoint = timeout(Duration::from_secs(5), async {
            loop {
                match rx_init.recv().await {
                    Some(RemoteEvent::Endpoint(endpoint)) => break Ok(endpoint),
                    Some(RemoteEvent::Message(_)) => continue,
                    Some(RemoteEvent::Closed) | None => {
                        break Err(Mcp2CliError::mcp(
                            "Remote MCP SSE stream closed before it announced a message endpoint",
                        ));
                    }
                }
            }
        })
        .await
        .map_err(|_| {
            Mcp2CliError::mcp("Timed out waiting for remote MCP SSE endpoint announcement")
        })??;

        let post_url = stream_base_url.join(&endpoint).map_err(|error| {
            Mcp2CliError::mcp(format!(
                "Invalid remote MCP message endpoint '{}': {}",
                endpoint, error
            ))
        })?;

        let mut client = Self {
            http,
            post_url,
            events: rx_init,
            next_id: 1,
            _reader_task: reader_task,
        };

        let _ = client
            .request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "openrustclaw-mcp2cli",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
            )
            .await?;

        Ok(client)
    }

    async fn discover_tools(&mut self) -> Result<Vec<openrustclaw_mcp::client::McpToolDef>> {
        let result = self.request("tools/list", None).await?;
        let tools_array = result
            .get("tools")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(tools_array
            .iter()
            .filter_map(|tool| {
                Some(openrustclaw_mcp::client::McpToolDef {
                    name: tool.get("name")?.as_str()?.to_string(),
                    description: tool
                        .get("description")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: tool
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new())),
                    server_name: self.post_url.to_string(),
                })
            })
            .collect())
    }

    async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<ToolOutput> {
        let result = self
            .request(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments,
                })),
            )
            .await?;

        let content = result
            .get("content")
            .and_then(|value| value.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| block.get("text").and_then(|text| text.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| result.to_string());

        let is_error = result
            .get("isError")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content,
            is_error,
        })
    }

    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let response = self
            .http
            .post(self.post_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 202 {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Mcp2CliError::mcp(format!(
                "Remote MCP request '{}' failed with HTTP {} {}",
                method, status, body
            )));
        }

        let response_text = response.text().await.unwrap_or_default();
        if !response_text.trim().is_empty()
            && let Ok(payload) = serde_json::from_str::<Value>(&response_text)
        {
            return parse_jsonrpc_response(payload, id);
        }

        timeout(Duration::from_secs(30), async {
            loop {
                match self.events.recv().await {
                    Some(RemoteEvent::Message(payload)) => {
                        if payload.get("id").and_then(|value| value.as_u64()) == Some(id) {
                            break parse_jsonrpc_response(payload, id);
                        }
                    }
                    Some(RemoteEvent::Endpoint(_)) => continue,
                    Some(RemoteEvent::Closed) | None => {
                        break Err(Mcp2CliError::mcp(format!(
                            "Remote MCP SSE stream closed while waiting for response to '{}'",
                            method
                        )));
                    }
                }
            }
        })
        .await
        .map_err(|_| {
            Mcp2CliError::mcp(format!(
                "Timed out waiting for remote MCP response to '{}'",
                method
            ))
        })?
    }
}

fn parse_jsonrpc_response(payload: Value, expected_id: u64) -> Result<Value> {
    if let Some(error) = payload.get("error") {
        return Err(Mcp2CliError::mcp(
            error
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("Unknown MCP error"),
        ));
    }

    if let Some(id) = payload.get("id").and_then(|value| value.as_u64())
        && id != expected_id
    {
        return Err(Mcp2CliError::mcp(format!(
            "MCP response id mismatch: expected {}, got {}",
            expected_id, id
        )));
    }

    Ok(payload.get("result").cloned().unwrap_or(Value::Null))
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
        let output = self.call_tool(tool_name, args).await?;
        Ok(output.content)
    }
}

/// MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
    async fn from_url_supports_legacy_sse_transport() {
        let server = MockServer::start().await;
        let sse_body = r#"event: endpoint
data: /messages?sessionId=test

event: message
data: {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"mock","version":"1.0"}}}

event: message
data: {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"ping","description":"Ping tool","inputSchema":{"type":"object","properties":{"value":{"type":"string","description":"Value"}},"required":["value"]}}]}}

event: message
data: {"jsonrpc":"2.0","id":3,"result":{"tools":[{"name":"ping","description":"Ping tool","inputSchema":{"type":"object","properties":{"value":{"type":"string","description":"Value"}},"required":["value"]}}]}}

event: message
data: {"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"pong"}]}}

"#;

        Mock::given(method("GET"))
            .and(path("/sse"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(202))
            .mount(&server)
            .await;

        let adapter = McpAdapter::from_url(&format!("{}/sse", server.uri()))
            .await
            .unwrap();

        let tools = adapter.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "ping");

        let help = adapter.get_tool_help("ping").await.unwrap();
        assert_eq!(help.name, "ping");
        assert_eq!(help.parameters.len(), 1);

        let output = adapter
            .execute_tool("ping", json!({"value": "hello"}))
            .await
            .unwrap();
        assert_eq!(output, "pong");
    }
}
