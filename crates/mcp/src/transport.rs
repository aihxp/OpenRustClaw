//! MCP transport layer (stdio subprocess management).

use openrustclaw_core::error::{Error, McpError, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::debug;

/// A stdio transport connection to an MCP server subprocess.
pub struct StdioTransport {
    child: Child,
    // We read from stdout and write to stdin.
}

impl StdioTransport {
    /// Spawn an MCP server as a subprocess.
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        let child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                Error::Mcp(McpError::Transport(format!(
                    "Failed to spawn MCP server: {}",
                    e
                )))
            })?;

        Ok(Self { child })
    }

    /// Send a JSON-RPC request and read the response.
    pub async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Mcp(McpError::Transport("No stdin handle".to_string())))?;

        let request_str = serde_json::to_string(&request)
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        stdin
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;
        stdin
            .flush()
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        debug!(method = %method, "Sent MCP request");

        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| Error::Mcp(McpError::Transport("No stdout handle".to_string())))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| Error::Mcp(McpError::Transport(e.to_string())))?;

        let response: Value = serde_json::from_str(&line).map_err(|e| {
            Error::Mcp(McpError::Transport(format!(
                "Invalid JSON response: {}",
                e
            )))
        })?;

        if let Some(error) = response.get("error") {
            return Err(Error::Mcp(McpError::ToolExecution(
                error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
            )));
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Kill the subprocess.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }
}
