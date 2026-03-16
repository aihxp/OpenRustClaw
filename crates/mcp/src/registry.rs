//! MCP server discovery and connection management.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::info;

use openrustclaw_core::error::Result;

use crate::client::{McpClient, McpToolDef};

/// Configuration for connecting to an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
}

/// Registry of MCP server connections.
pub struct McpRegistry {
    configs: Vec<McpServerEntry>,
    clients: HashMap<String, McpClient>,
}

impl McpRegistry {
    pub fn new(configs: Vec<McpServerEntry>) -> Self {
        Self {
            configs,
            clients: HashMap::new(),
        }
    }

    /// Connect to all configured MCP servers.
    pub async fn connect_all(&mut self) -> Result<()> {
        for config in &self.configs {
            if !config.enabled {
                continue;
            }
            let args: Vec<&str> = config.args.iter().map(|s| s.as_str()).collect();
            match McpClient::connect(&config.name, &config.command, &args).await {
                Ok(client) => {
                    info!(server = %config.name, "Connected to MCP server");
                    self.clients.insert(config.name.clone(), client);
                }
                Err(e) => {
                    tracing::warn!(
                        server = %config.name,
                        error = %e,
                        "Failed to connect to MCP server"
                    );
                }
            }
        }
        Ok(())
    }

    /// Discover tools from all connected servers.
    pub async fn discover_all_tools(&mut self) -> Result<Vec<McpToolDef>> {
        let mut all_tools = Vec::new();
        let names: Vec<String> = self.clients.keys().cloned().collect();
        for name in names {
            if let Some(client) = self.clients.get_mut(&name) {
                match client.discover_tools().await {
                    Ok(tools) => all_tools.extend(tools),
                    Err(e) => {
                        tracing::warn!(
                            server = %name,
                            error = %e,
                            "Failed to discover tools"
                        );
                    }
                }
            }
        }
        Ok(all_tools)
    }

    /// Get a mutable reference to a client by server name.
    pub fn get_client_mut(&mut self, name: &str) -> Option<&mut McpClient> {
        self.clients.get_mut(name)
    }

    /// Shut down all connections.
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (_, client) in self.clients.iter_mut() {
            let _ = client.shutdown().await;
        }
        self.clients.clear();
        Ok(())
    }
}
