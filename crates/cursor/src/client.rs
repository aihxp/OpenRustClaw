//! Cursor ACP Client for connecting to Cursor IDE.
//!
//! The client connects to a Cursor ACP server (running within the IDE or externally)
//! and provides a convenient API for making requests and receiving responses.

use crate::acp::{AcpMessage, AcpPayload, ACP_PROTOCOL_VERSION};
use crate::types::{AcpRequest, AcpResponse};
use crate::error::{CursorError, Result};
use crate::types::{CursorConfig, Diagnostic, GitStatus, IdeState};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Connection type for the ACP client.
#[derive(Debug, Clone)]
pub enum ClientConnection {
    /// Connect via stdio to a subprocess.
    Stdio {
        /// Command to spawn.
        command: String,
        /// Arguments for the command.
        args: Vec<String>,
    },
    /// Connect via TCP to a running server.
    Tcp { host: String, port: u16 },
    /// Connect via WebSocket.
    WebSocket { url: String },
}

impl Default for ClientConnection {
    fn default() -> Self {
        ClientConnection::Stdio {
            command: "cargo".to_string(),
            args: vec![
                "run".to_string(),
                "--bin".to_string(),
                "openrustclaw".to_string(),
                "--".to_string(),
                "cursor".to_string(),
                "serve".to_string(),
            ],
        }
    }
}

/// Cursor ACP Client.
pub struct CursorClient {
    connection: ClientConnection,
    config: CursorConfig,
    request_timeout: Duration,
}

/// Active connection handle.
enum ConnectionHandle {
    Stdio {
        child: Child,
        request_tx: mpsc::Sender<AcpMessage>,
        response_rx: mpsc::Receiver<Result<AcpResponse>>,
    },
    Tcp {
        stream: TcpStream,
        request_tx: mpsc::Sender<AcpMessage>,
        response_rx: mpsc::Receiver<Result<AcpResponse>>,
    },
}

impl CursorClient {
    /// Create a new Cursor client.
    pub fn new(connection: ClientConnection, config: CursorConfig) -> Self {
        Self {
            connection,
            config,
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Create a client with default connection.
    pub fn with_defaults() -> Self {
        Self::new(ClientConnection::default(), CursorConfig::default())
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.request_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Connect to the server.
    pub async fn connect(&self) -> Result<CursorConnection> {
        match &self.connection {
            ClientConnection::Stdio { command, args } => {
                self.connect_stdio(command, args).await
            }
            ClientConnection::Tcp { host, port } => {
                self.connect_tcp(host, *port).await
            }
            ClientConnection::WebSocket { url: _ } => {
                // WebSocket support would require the tokio-tungstenite crate
                Err(CursorError::Connection(
                    "WebSocket not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Connect via stdio.
    async fn connect_stdio(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<CursorConnection> {
        info!("Connecting to Cursor server via stdio: {} {:?}", command, args);

        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn Cursor server: {}", e);
                CursorError::Connection(e.to_string())
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CursorError::Connection("No stdin handle".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CursorError::Connection("No stdout handle".to_string()))?;

        // Create channels for request/response
        let (request_tx, mut request_rx) = mpsc::channel::<AcpMessage>(100);
        let (response_tx, response_rx) = mpsc::channel::<Result<AcpResponse>>(100);

        // Spawn writer task
        let response_tx_clone = response_tx.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(message) = request_rx.recv().await {
                let msg_str = match serde_json::to_string(&message) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = response_tx_clone.send(Err(CursorError::Protocol(e.to_string()))).await;
                        continue;
                    }
                };

                if let Err(e) = stdin.write_all(msg_str.as_bytes()).await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("Server closed stdout");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<AcpMessage>(trimmed) {
                            Ok(message) => {
                                if let AcpPayload::Response(response) = message.payload {
                                    let _ = response_tx.send(Ok(response)).await;
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(CursorError::Connection(e.to_string()))).await;
                        break;
                    }
                }
            }
        });

        Ok(CursorConnection {
            request_tx,
            response_rx,
            timeout: self.request_timeout,
        })
    }

    /// Connect via TCP.
    async fn connect_tcp(&self, host: &str, port: u16) -> Result<CursorConnection> {
        info!("Connecting to Cursor server at {}:{}", host, port);

        let stream = TcpStream::connect(format!("{}:{}", host, port))
            .await
            .map_err(|e| CursorError::Connection(e.to_string()))?;

        let (request_tx, mut request_rx) = mpsc::channel::<AcpMessage>(100);
        let (response_tx, response_rx) = mpsc::channel::<Result<AcpResponse>>(100);

        // Split stream for reading/writing
        let (reader, mut writer) = stream.into_split();

        // Spawn writer task
        let response_tx_clone = response_tx.clone();
        tokio::spawn(async move {
            while let Some(message) = request_rx.recv().await {
                let msg_str = match serde_json::to_string(&message) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = response_tx_clone.send(Err(CursorError::Protocol(e.to_string()))).await;
                        continue;
                    }
                };

                if let Err(e) = writer.write_all(msg_str.as_bytes()).await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
                if let Err(e) = writer.write_all(b"\n").await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
                if let Err(e) = writer.flush().await {
                    let _ = response_tx_clone.send(Err(CursorError::Connection(e.to_string()))).await;
                    break;
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<AcpMessage>(trimmed) {
                            Ok(message) => {
                                if let AcpPayload::Response(response) = message.payload {
                                    let _ = response_tx.send(Ok(response)).await;
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(CursorError::Connection(e.to_string()))).await;
                        break;
                    }
                }
            }
        });

        Ok(CursorConnection {
            request_tx,
            response_rx,
            timeout: self.request_timeout,
        })
    }
}

/// Active connection for making requests.
pub struct CursorConnection {
    request_tx: mpsc::Sender<AcpMessage>,
    response_rx: mpsc::Receiver<Result<AcpResponse>>,
    timeout: Duration,
}

impl CursorConnection {
    /// Send a request and wait for response.
    async fn request(&mut self, request: AcpRequest) -> Result<AcpResponse> {
        let message = AcpMessage {
            version: ACP_PROTOCOL_VERSION.to_string(),
            id: Uuid::new_v4().to_string(),
            payload: AcpPayload::Request(request),
        };

        // Send request
        self.request_tx
            .send(message)
            .await
            .map_err(|_| CursorError::Connection("Request channel closed".to_string()))?;

        // Wait for response with timeout
        match timeout(self.timeout, self.response_rx.recv()).await {
            Ok(Some(Ok(response))) => Ok(response),
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(CursorError::Connection("Response channel closed".to_string())),
            Err(_) => Err(CursorError::Timeout(
                "Request timed out".to_string(),
            )),
        }
    }

    /// Get the current IDE state.
    pub async fn get_state(&mut self) -> Result<IdeState> {
        match self.request(AcpRequest::GetState).await? {
            AcpResponse::State(state) => Ok(state),
            AcpResponse::Error { message, .. } => Err(CursorError::InvalidContext(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Execute a command in the terminal.
    pub async fn execute_command(
        &mut self,
        command: impl Into<String>,
        terminal_id: Option<impl Into<String>>,
    ) -> Result<(String, String, i32)> {
        let request = AcpRequest::ExecuteCommand {
            command: command.into(),
            terminal_id: terminal_id.map(|s| s.into()),
        };

        match self.request(request).await? {
            AcpResponse::CommandResult {
                stdout,
                stderr,
                exit_code,
            } => Ok((stdout, stderr, exit_code)),
            AcpResponse::Error { message, .. } => Err(CursorError::Terminal(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Read a file.
    pub async fn read_file(&mut self, path: impl Into<PathBuf>) -> Result<String> {
        let request = AcpRequest::ReadFile { path: path.into() };

        match self.request(request).await? {
            AcpResponse::FileContent { content, .. } => Ok(content),
            AcpResponse::Error { message, .. } => Err(CursorError::FileOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Write a file.
    pub async fn write_file(
        &mut self,
        path: impl Into<PathBuf>,
        content: impl Into<String>,
    ) -> Result<()> {
        let request = AcpRequest::WriteFile {
            path: path.into(),
            content: content.into(),
        };

        match self.request(request).await? {
            AcpResponse::FileOperationSuccess { .. } => Ok(()),
            AcpResponse::Error { message, .. } => Err(CursorError::FileOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Edit a file.
    pub async fn edit_file(
        &mut self,
        path: impl Into<PathBuf>,
        old_text: impl Into<String>,
        new_text: impl Into<String>,
    ) -> Result<()> {
        let request = AcpRequest::EditFile {
            path: path.into(),
            old_text: old_text.into(),
            new_text: new_text.into(),
        };

        match self.request(request).await? {
            AcpResponse::FileOperationSuccess { .. } => Ok(()),
            AcpResponse::Error { message, .. } => Err(CursorError::FileOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Search code.
    pub async fn search_code(
        &mut self,
        query: impl Into<String>,
    ) -> Result<Vec<crate::types::SearchMatch>> {
        let request = AcpRequest::SearchCode {
            query: query.into(),
            path_pattern: None,
            max_results: Some(50),
        };

        match self.request(request).await? {
            AcpResponse::SearchResults { matches, .. } => Ok(matches),
            AcpResponse::Error { message, .. } => Err(CursorError::PatternError(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// List files.
    pub async fn list_files(
        &mut self,
        path: impl Into<PathBuf>,
        recursive: bool,
    ) -> Result<Vec<crate::types::DirEntry>> {
        let request = AcpRequest::ListFiles {
            path: path.into(),
            recursive,
        };

        match self.request(request).await? {
            AcpResponse::DirectoryListing { entries, .. } => Ok(entries),
            AcpResponse::Error { message, .. } => Err(CursorError::FileOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Get git status.
    pub async fn git_status(&mut self) -> Result<GitStatus> {
        match self.request(AcpRequest::GitStatus).await? {
            AcpResponse::GitStatus(status) => Ok(status),
            AcpResponse::Error { message, .. } => Err(CursorError::GitOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Get git diff.
    pub async fn git_diff(&mut self, staged: bool) -> Result<String> {
        let request = AcpRequest::GitDiff { staged };

        match self.request(request).await? {
            AcpResponse::GitDiff(diff) => Ok(diff),
            AcpResponse::Error { message, .. } => Err(CursorError::GitOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Run a git command.
    pub async fn git_command(&mut self, args: Vec<String>) -> Result<(String, String, i32)> {
        let request = AcpRequest::GitCommand { args };

        match self.request(request).await? {
            AcpResponse::CommandResult {
                stdout,
                stderr,
                exit_code,
            } => Ok((stdout, stderr, exit_code)),
            AcpResponse::Error { message, .. } => Err(CursorError::GitOperation(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Run linter.
    pub async fn run_linter(
        &mut self,
        tool: impl Into<String>,
        path: Option<impl Into<PathBuf>>,
    ) -> Result<(String, Vec<Diagnostic>)> {
        let request = AcpRequest::RunLinter {
            tool: tool.into(),
            path: path.map(|p| p.into()),
        };

        match self.request(request).await? {
            AcpResponse::LinterOutput {
                output,
                diagnostics,
                ..
            } => Ok((output, diagnostics)),
            AcpResponse::Error { message, .. } => Err(CursorError::Linter(message)),
            _ => Err(CursorError::Protocol("Unexpected response type".to_string())),
        }
    }

    /// Close the connection.
    pub async fn close(self) {
        // Channels will be dropped, causing the tasks to exit
        drop(self.request_tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CursorClient::with_defaults();
        assert!(matches!(client.connection, ClientConnection::Stdio { .. }));
    }

    #[test]
    fn test_client_with_timeout() {
        let client = CursorClient::with_defaults().with_timeout(60);
        assert_eq!(client.request_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_tcp_connection_config() {
        let conn = ClientConnection::Tcp {
            host: "localhost".to_string(),
            port: 8080,
        };
        
        if let ClientConnection::Tcp { host, port } = conn {
            assert_eq!(host, "localhost");
            assert_eq!(port, 8080);
        } else {
            panic!("Expected TCP connection");
        }
    }
}
