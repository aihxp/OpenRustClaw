//! Cursor ACP Server for handling IDE-agent communication.
//!
//! The server runs as a JSON-RPC over stdio or HTTP/WebSocket server,
//! processing ACP requests from Cursor IDE and responding with results.

use crate::acp::{
    ACP_PROTOCOL_VERSION, AcpCapabilities, AcpInitializeRequest, AcpMessage, AcpPayload,
    AcpProtocol, AcpServerInfo,
};
use crate::error::{CursorError, Result};
use crate::tools::ToolRegistry;
use crate::types::{CursorConfig, IdeState, ToolContext};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Transport type for the ACP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerTransport {
    /// JSON-RPC over stdio (for MCP compatibility).
    Stdio,
    /// HTTP/WebSocket server.
    Http {
        /// Port to listen on.
        port: u16,
    },
}

/// Configuration for the Cursor ACP server.
#[derive(Debug, Clone)]
pub struct CursorServerConfig {
    /// Server transport type.
    pub transport: ServerTransport,
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Cursor configuration.
    pub cursor_config: CursorConfig,
}

impl Default for CursorServerConfig {
    fn default() -> Self {
        Self {
            transport: ServerTransport::Stdio,
            name: "openrustclaw-cursor".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            cursor_config: CursorConfig::default(),
        }
    }
}

/// The Cursor ACP server.
#[allow(missing_docs)]
pub struct CursorServer {
    config: CursorServerConfig,
    protocol: Arc<AcpProtocol>,
    tool_registry: Arc<ToolRegistry>,
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
}

/// State of a client connection.
#[derive(Debug)]
#[allow(dead_code)]
struct ConnectionState {
    id: String,
    initialized: bool,
    capabilities: AcpCapabilities,
}

impl CursorServer {
    /// Create a new Cursor ACP server.
    pub fn new(config: CursorServerConfig) -> Self {
        let protocol = Arc::new(AcpProtocol::new(config.cursor_config.clone()));
        let tool_context = ToolContext::new(
            config.cursor_config.project_root.clone(),
            config.cursor_config.clone(),
        );
        let tool_registry = Arc::new(ToolRegistry::new(tool_context));

        Self {
            config,
            protocol,
            tool_registry,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Run the server.
    pub async fn run(&self) -> Result<()> {
        match self.config.transport {
            ServerTransport::Stdio => self.run_stdio().await,
            ServerTransport::Http { port } => self.run_http(port).await,
        }
    }

    /// Run as stdio server (for MCP compatibility).
    async fn run_stdio(&self) -> Result<()> {
        info!("Starting Cursor ACP server on stdio");

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        self.handle_stdio(stdin, stdout).await
    }

    /// Handle stdio communication.
    async fn handle_stdio(&self, stdin: Stdin, mut stdout: Stdout) -> Result<()> {
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("EOF reached, shutting down");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    debug!("Received: {}", trimmed);

                    match self.process_message(trimmed).await {
                        Ok(Some(response)) => {
                            let response_str = serde_json::to_string(&response)
                                .map_err(|e| CursorError::Protocol(e.to_string()))?;
                            stdout
                                .write_all(response_str.as_bytes())
                                .await
                                .map_err(|e| CursorError::Connection(e.to_string()))?;
                            stdout
                                .write_all(b"\n")
                                .await
                                .map_err(|e| CursorError::Connection(e.to_string()))?;
                            stdout
                                .flush()
                                .await
                                .map_err(|e| CursorError::Connection(e.to_string()))?;
                            debug!("Sent: {}", response_str);
                        }
                        Ok(None) => {
                            // No response needed (notification)
                        }
                        Err(e) => {
                            error!("Error processing message: {}", e);
                            // Send error response
                            let error_response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32603,
                                    "message": e.to_string(),
                                }
                            });
                            let error_str = error_response.to_string();
                            let _ = stdout.write_all(error_str.as_bytes()).await;
                            let _ = stdout.write_all(b"\n").await;
                            let _ = stdout.flush().await;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    return Err(CursorError::Connection(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Run as HTTP/WebSocket server.
    async fn run_http(&self, port: u16) -> Result<()> {
        info!("Starting Cursor ACP server on port {}", port);

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| CursorError::Connection(e.to_string()))?;

        info!("Server listening on http://127.0.0.1:{}", port);

        loop {
            let (stream, addr) = listener
                .accept()
                .await
                .map_err(|e| CursorError::Connection(e.to_string()))?;

            debug!("New connection from: {}", addr);

            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_http_connection(stream).await {
                    error!("Error handling HTTP connection: {}", e);
                }
            });
        }
    }

    /// Handle an HTTP connection.
    async fn handle_http_connection(&self, stream: TcpStream) -> Result<()> {
        // For simplicity, we'll handle basic HTTP requests
        // In production, you'd want to use a proper HTTP framework like axum

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut request_line = String::new();

        // Read request line
        reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| CursorError::Connection(e.to_string()))?;

        debug!("HTTP request: {}", request_line.trim());

        // Parse request
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(CursorError::Protocol("Invalid HTTP request".to_string()));
        }

        let method = parts[0];
        let path = parts[1];

        // Simple routing
        let response = match (method, path) {
            ("GET", "/health") => {
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}"
                    .to_string()
            }
            ("GET", "/tools") => {
                let tools = self.tool_registry.get_tool_definitions();
                let body = serde_json::json!({ "tools": tools }).to_string();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                    body
                )
            }
            ("POST", "/invoke") => {
                // Read headers to get content length
                let mut content_length = 0;
                let mut headers_done = false;
                let mut header_line = String::new();

                while !headers_done {
                    header_line.clear();
                    reader
                        .read_line(&mut header_line)
                        .await
                        .map_err(|e| CursorError::Connection(e.to_string()))?;
                    if header_line.trim().is_empty() {
                        headers_done = true;
                    } else if header_line.to_lowercase().starts_with("content-length:") {
                        content_length = header_line
                            .split(':')
                            .nth(1)
                            .and_then(|s| s.trim().parse().ok())
                            .unwrap_or(0);
                    }
                }

                // Read body
                let mut body = vec![0u8; content_length];
                if content_length > 0 {
                    reader
                        .read_exact(&mut body)
                        .await
                        .map_err(|e| CursorError::Connection(e.to_string()))?;
                }

                let body_str = String::from_utf8_lossy(&body);
                let result = self.process_http_invoke(&body_str).await;

                match result {
                    Ok(response_body) => format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                        response_body
                    ),
                    Err(e) => format!(
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\r\n{{\"error\":\"{}\"}}",
                        e
                    ),
                }
            }
            _ => "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
        };

        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| CursorError::Connection(e.to_string()))?;

        Ok(())
    }

    /// Process HTTP invoke request.
    async fn process_http_invoke(&self, body: &str) -> Result<String> {
        let request: Value =
            serde_json::from_str(body).map_err(|e| CursorError::Protocol(e.to_string()))?;

        let tool_name = request
            .get("tool")
            .and_then(|t| t.as_str())
            .ok_or_else(|| CursorError::Protocol("Missing 'tool' field".to_string()))?;

        let params = request.get("params").cloned().unwrap_or(Value::Null);

        let result = self.tool_registry.execute(tool_name, params).await?;

        Ok(result.to_string())
    }

    /// Process an incoming message.
    async fn process_message(&self, message: &str) -> Result<Option<AcpMessage>> {
        // Try to parse as ACP message
        match serde_json::from_str::<AcpMessage>(message) {
            Ok(acp_message) => self.protocol.handle_message(acp_message).await,
            Err(_) => {
                // Try to parse as JSON-RPC (for MCP compatibility)
                self.process_jsonrpc(message).await
            }
        }
    }

    /// Process JSON-RPC message (MCP compatibility layer).
    async fn process_jsonrpc(&self, message: &str) -> Result<Option<AcpMessage>> {
        let request: Value =
            serde_json::from_str(message).map_err(|e| CursorError::Protocol(e.to_string()))?;

        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = request.get("id").cloned().unwrap_or(Value::Null);

        let result = match method {
            "initialize" => {
                // MCP initialize
                let init_request = AcpInitializeRequest {
                    protocol_version: request
                        .get("params")
                        .and_then(|p| p.get("protocolVersion"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(ACP_PROTOCOL_VERSION)
                        .to_string(),
                    capabilities: AcpCapabilities::default(),
                    client_info: AcpServerInfo {
                        name: "cursor".to_string(),
                        version: "1.0.0".to_string(),
                    },
                };

                let response = self.protocol.initialize(init_request).await?;
                serde_json::to_value(response).map_err(|e| CursorError::Protocol(e.to_string()))?
            }
            "tools/list" => {
                let tools = self.tool_registry.get_tool_definitions();
                serde_json::json!({ "tools": tools })
            }
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let tool_params = params.get("arguments").cloned().unwrap_or(Value::Null);

                match self.tool_registry.execute(tool_name, tool_params).await {
                    Ok(result) => serde_json::json!({
                        "content": [{"type": "text", "text": result.to_string()}],
                        "isError": false,
                    }),
                    Err(e) => serde_json::json!({
                        "content": [{"type": "text", "text": e.to_string()}],
                        "isError": true,
                    }),
                }
            }
            _ => {
                return Err(CursorError::ToolNotFound(method.to_string()));
            }
        };

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });

        // Convert to ACP message for uniform handling
        Ok(Some(AcpMessage {
            version: ACP_PROTOCOL_VERSION.to_string(),
            id: Uuid::new_v4().to_string(),
            payload: AcpPayload::Response(Box::new(crate::types::AcpResponse::CommandResult {
                stdout: response.to_string(),
                stderr: String::new(),
                exit_code: 0,
            })),
        }))
    }

    /// Update the IDE state.
    pub async fn update_ide_state(&self, state: IdeState) {
        self.protocol.update_state(state).await;
    }

    /// Get the current IDE state.
    pub async fn get_ide_state(&self) -> Option<IdeState> {
        self.protocol.get_state().await
    }

    /// Execute a tool directly.
    pub async fn execute_tool(&self, name: &str, params: Value) -> Result<Value> {
        self.tool_registry.execute(name, params).await
    }

    /// Get the tool registry.
    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        self.tool_registry.clone()
    }

    /// Get the ACP protocol.
    pub fn protocol(&self) -> Arc<AcpProtocol> {
        self.protocol.clone()
    }

    /// Create an MCP server configuration.
    pub fn create_mcp_config(&self) -> Value {
        let tools = self.tool_registry.get_tool_definitions();

        serde_json::json!({
            "mcpServers": {
                "openrustclaw-cursor": {
                    "command": "cargo",
                    "args": ["run", "--bin", "openrustclaw", "--", "cursor", "serve", "--transport", "stdio"],
                    "env": {
                        "RUST_LOG": "info"
                    },
                    "description": "OpenRustClaw Cursor ACP Server",
                    "enabled": true,
                    "tools": tools
                }
            }
        })
    }
}

impl Clone for CursorServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol: self.protocol.clone(),
            tool_registry: self.tool_registry.clone(),
            connections: self.connections.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_server_config_default() {
        let config = CursorServerConfig::default();
        assert_eq!(config.name, "openrustclaw-cursor");
        assert!(matches!(config.transport, ServerTransport::Stdio));
    }

    #[test]
    fn test_create_mcp_config() {
        let config = CursorServerConfig::default();
        let server = CursorServer::new(config);
        let mcp_config = server.create_mcp_config();

        assert!(mcp_config.get("mcpServers").is_some());
        assert!(
            mcp_config["mcpServers"]
                .get("openrustclaw-cursor")
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_server_execute_tool() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = CursorServerConfig::default();
        config.cursor_config.project_root = temp_dir.path().to_path_buf();

        let server = CursorServer::new(config);

        // Test list_files tool
        let result = server
            .execute_tool("list_files", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.get("entries").is_some());
    }
}
