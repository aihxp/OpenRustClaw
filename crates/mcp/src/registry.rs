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

    /// Get the number of connected clients.
    pub fn connected_count(&self) -> usize {
        self.clients.len()
    }

    /// Get the list of configured server entries.
    pub fn configs(&self) -> &[McpServerEntry] {
        &self.configs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_configs() -> Vec<McpServerEntry> {
        vec![
            McpServerEntry {
                name: "server-a".to_string(),
                command: "npx".to_string(),
                args: vec!["@mcp/server-a".to_string()],
                enabled: true,
            },
            McpServerEntry {
                name: "server-b".to_string(),
                command: "python3".to_string(),
                args: vec!["server_b.py".to_string()],
                enabled: true,
            },
            McpServerEntry {
                name: "disabled-server".to_string(),
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
                enabled: false,
            },
        ]
    }

    // --- construction ---

    #[test]
    fn registry_new_stores_configs() {
        let configs = sample_configs();
        let registry = McpRegistry::new(configs.clone());
        assert_eq!(registry.configs().len(), 3);
    }

    #[test]
    fn registry_new_starts_with_no_clients() {
        let registry = McpRegistry::new(sample_configs());
        assert_eq!(registry.connected_count(), 0);
    }

    #[test]
    fn registry_new_empty_configs() {
        let registry = McpRegistry::new(vec![]);
        assert_eq!(registry.configs().len(), 0);
        assert_eq!(registry.connected_count(), 0);
    }

    // --- get_client_mut ---

    #[test]
    fn get_client_mut_returns_none_when_no_clients() {
        let mut registry = McpRegistry::new(sample_configs());
        assert!(registry.get_client_mut("server-a").is_none());
    }

    #[test]
    fn get_client_mut_returns_none_for_unknown_server() {
        let mut registry = McpRegistry::new(sample_configs());
        assert!(registry.get_client_mut("nonexistent").is_none());
    }

    // --- McpServerEntry ---

    #[test]
    fn server_entry_serialization_roundtrip() {
        let entry = McpServerEntry {
            name: "test-server".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@mcp/test".to_string()],
            enabled: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: McpServerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.command, "npx");
        assert_eq!(deserialized.args.len(), 2);
        assert!(deserialized.enabled);
    }

    #[test]
    fn server_entry_disabled_serializes_correctly() {
        let entry = McpServerEntry {
            name: "off".to_string(),
            command: "node".to_string(),
            args: vec![],
            enabled: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: McpServerEntry = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.enabled);
    }

    #[test]
    fn server_entry_clone_is_independent() {
        let entry = McpServerEntry {
            name: "clone-test".to_string(),
            command: "npx".to_string(),
            args: vec!["arg1".to_string()],
            enabled: true,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.name, "clone-test");
        assert_eq!(cloned.command, "npx");
        assert_eq!(cloned.args, vec!["arg1".to_string()]);
    }

    #[test]
    fn server_entry_debug_format() {
        let entry = McpServerEntry {
            name: "debug-test".to_string(),
            command: "python3".to_string(),
            args: vec![],
            enabled: true,
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("python3"));
    }

    // --- connect_all behavior with disabled servers ---

    #[tokio::test]
    async fn connect_all_skips_disabled_servers() {
        // All servers will fail to connect (commands won't resolve to real MCP servers),
        // but the disabled one should be skipped entirely.
        // The enabled ones will fail gracefully (logged as warnings).
        let configs = vec![McpServerEntry {
            name: "disabled".to_string(),
            command: "npx".to_string(),
            args: vec!["nonexistent-package".to_string()],
            enabled: false,
        }];
        let mut registry = McpRegistry::new(configs);
        let result = registry.connect_all().await;
        // connect_all should succeed (errors are logged, not propagated)
        assert!(result.is_ok());
        // No clients should be connected since the only server was disabled
        assert_eq!(registry.connected_count(), 0);
    }

    #[tokio::test]
    async fn shutdown_all_on_empty_registry() {
        let mut registry = McpRegistry::new(vec![]);
        let result = registry.shutdown_all().await;
        assert!(result.is_ok());
        assert_eq!(registry.connected_count(), 0);
    }
}
