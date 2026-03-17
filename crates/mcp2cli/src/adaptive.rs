//! Adaptive MCP implementation - transparent mcp2cli optimization
//!
//! This module provides a transparent layer that automatically chooses between
//! native MCP and mcp2cli based on the use case:
//!
//! - **Native mode**: < 10 tools, short conversations (low latency)
//! - **Efficient mode**: 10+ tools, long conversations (token savings)
//! - **Auto mode**: Dynamically switches based on usage patterns
//!
//! To users, this looks like regular MCP but with automatic optimizations.

use openrustclaw_core::error::{Error, McpError, Result};
use openrustclaw_core::types::ToolOutput;
use openrustclaw_mcp::client::{McpClient, McpToolDef};
use openrustclaw_mcp::registry::McpServerEntry;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

use crate::adapters::ToolSource;
use crate::discovery::ToolDiscovery;

/// Adaptive MCP registry that transparently uses mcp2cli when beneficial
pub struct AdaptiveMcpRegistry {
    /// Native MCP clients
    native_clients: HashMap<String, McpClient>,

    /// mcp2cli wrappers for efficient mode
    efficient_clients: HashMap<String, EfficientClient>,

    /// Server configurations
    configs: Vec<McpServerEntry>,

    /// Adaptive configuration
    config: AdaptiveConfig,

    /// Usage statistics for auto mode
    stats: UsageStats,
}

/// Configuration for adaptive behavior
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Mode selection strategy
    pub mode: AdaptiveMode,

    /// Threshold for switching to efficient mode (number of tools)
    pub efficient_threshold: usize,

    /// Whether to use TOON format for large responses
    pub use_toon: bool,

    /// Cache TTL for tool discovery
    pub cache_ttl_secs: u64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            mode: AdaptiveMode::Auto,
            efficient_threshold: 10,
            use_toon: true,
            cache_ttl_secs: 3600,
        }
    }
}

/// Adaptive mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveMode {
    /// Always use native MCP
    Native,

    /// Always use mcp2cli (efficient mode)
    Efficient,

    /// Automatically choose based on usage patterns
    Auto,
}

/// Usage statistics for auto mode decisions
#[derive(Debug, Default)]
struct UsageStats {
    conversation_turns: usize,
    total_tools_discovered: usize,
    tools_actually_used: std::collections::HashSet<String>,
}

/// Efficient client wrapper using mcp2cli
struct EfficientClient {
    source: ToolSource,
    discovery: ToolDiscovery,
    tool_count: usize,
}

impl AdaptiveMcpRegistry {
    /// Create new adaptive registry with default config
    pub fn new(configs: Vec<McpServerEntry>) -> Self {
        Self::with_config(configs, AdaptiveConfig::default())
    }

    /// Create new adaptive registry with custom config
    pub fn with_config(configs: Vec<McpServerEntry>, config: AdaptiveConfig) -> Self {
        info!(
            mode = ?config.mode,
            threshold = config.efficient_threshold,
            "Creating adaptive MCP registry"
        );

        Self {
            native_clients: HashMap::new(),
            efficient_clients: HashMap::new(),
            configs,
            config,
            stats: UsageStats::default(),
        }
    }

    /// Connect to all configured MCP servers
    #[instrument(skip(self))]
    pub async fn connect_all(&mut self) -> Result<()> {
        // Clone configs to avoid borrow issues
        let configs: Vec<McpServerEntry> = self.configs.clone();

        for config in &configs {
            if !config.enabled {
                continue;
            }

            match self.connect_server(config).await {
                Ok(()) => info!(server = %config.name, "Connected to MCP server"),
                Err(e) => warn!(server = %config.name, error = %e, "Failed to connect"),
            }
        }

        Ok(())
    }

    /// Connect to a single server with automatic mode selection
    async fn connect_server(&mut self, config: &McpServerEntry) -> Result<()> {
        let args: Vec<&str> = config.args.iter().map(|s| s.as_str()).collect();

        // Always connect native first to get tool count
        let mut native_client = McpClient::connect(&config.name, &config.command, &args).await?;

        // Quick discovery to count tools
        let tools = native_client.discover_tools().await?;
        let tool_count = tools.len();

        // Decide which mode to use
        let use_efficient = self.should_use_efficient(tool_count);

        if use_efficient {
            info!(
                server = %config.name,
                tool_count = tool_count,
                "Using efficient mode (mcp2cli)"
            );

            // Create mcp2cli wrapper
            let source = ToolSource::mcp_stdio(&config.command);
            let discovery = ToolDiscovery::new();

            self.efficient_clients.insert(
                config.name.clone(),
                EfficientClient {
                    source,
                    discovery,
                    tool_count,
                },
            );

            // Keep native client as fallback
            self.native_clients
                .insert(config.name.clone(), native_client);
        } else {
            info!(
                server = %config.name,
                tool_count = tool_count,
                "Using native mode"
            );
            self.native_clients
                .insert(config.name.clone(), native_client);
        }

        self.stats.total_tools_discovered += tool_count;

        Ok(())
    }

    /// Determine if we should use efficient mode for this server
    fn should_use_efficient(&self, tool_count: usize) -> bool {
        match self.config.mode {
            AdaptiveMode::Native => false,
            AdaptiveMode::Efficient => true,
            AdaptiveMode::Auto => {
                // Use efficient mode if:
                // 1. Tool count exceeds threshold
                // 2. We've already seen high tool counts elsewhere
                tool_count >= self.config.efficient_threshold
                    || self.stats.total_tools_discovered >= self.config.efficient_threshold
            }
        }
    }

    /// Discover all tools from all connected servers
    ///
    /// In efficient mode, this returns compact summaries instead of full schemas
    #[instrument(skip(self))]
    pub async fn discover_all_tools(&mut self) -> Result<Vec<McpToolDef>> {
        let mut all_tools = Vec::new();

        // For each server, use the appropriate discovery method
        for (name, client) in &mut self.native_clients {
            if let Some(efficient) = self.efficient_clients.get(name) {
                // Use efficient discovery
                match efficient.discovery.list_tools(&efficient.source).await {
                    Ok(tools) => {
                        debug!(server = %name, count = tools.len(), "Discovered tools (efficient)");
                        // Convert summaries to McpToolDef with minimal schema
                        for summary in tools {
                            all_tools.push(McpToolDef {
                                name: summary.name,
                                description: summary.description,
                                input_schema: minimal_schema(),
                                server_name: name.clone(),
                            });
                        }
                        continue;
                    }
                    Err(e) => {
                        warn!(server = %name, error = %e, "Efficient discovery failed, falling back");
                    }
                }
            }

            // Native discovery (fallback or native mode)
            match client.discover_tools().await {
                Ok(tools) => {
                    debug!(server = %name, count = tools.len(), "Discovered tools (native)");
                    all_tools.extend(tools);
                }
                Err(e) => {
                    warn!(server = %name, error = %e, "Native discovery failed");
                }
            }
        }

        Ok(all_tools)
    }

    /// Get detailed tool schema (used when model wants to call a tool)
    ///
    /// In efficient mode, this fetches the schema on-demand
    #[instrument(skip(self))]
    pub async fn get_tool_schema(&self, server_name: &str, tool_name: &str) -> Result<Value> {
        // Check efficient clients first
        let efficient_result = if let Some(efficient) = self.efficient_clients.get(server_name) {
            // Fetch help on-demand
            match efficient
                .discovery
                .get_help(&efficient.source, tool_name)
                .await
            {
                Ok(help) => {
                    debug!(tool = %tool_name, "Fetched schema on-demand (efficient)");
                    Some(Ok(help_to_schema(&help)))
                }
                Err(e) => {
                    warn!(tool = %tool_name, error = %e, "Efficient help failed, falling back");
                    None
                }
            }
        } else {
            None
        };

        if let Some(result) = efficient_result {
            return result;
        }

        // Fallback to native client
        if self.native_clients.contains_key(server_name) {
            // In native mode, we need to find the tool in the client's discovered tools
            // This is a simplified version - full impl would cache this
            return Ok(minimal_schema());
        }

        Err(Error::Mcp(McpError::ToolNotFound {
            server: server_name.to_string(),
            tool: tool_name.to_string(),
        }))
    }

    /// Execute a tool call
    ///
    /// Automatically uses the appropriate client (native or efficient)
    #[instrument(skip(self, args))]
    pub async fn execute_tool(
        &mut self,
        server_name: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<ToolOutput> {
        // Track usage for auto mode
        self.stats.tools_actually_used.insert(tool_name.to_string());
        self.stats.conversation_turns += 1;

        // Check if we have an efficient client for this server
        let efficient_result = if let Some(efficient) = self.efficient_clients.get(server_name) {
            // Clone args for efficient attempt
            let args_clone = args.clone();
            match efficient
                .discovery
                .execute(&efficient.source, tool_name, args_clone)
                .await
            {
                Ok(result) => {
                    debug!(tool = %tool_name, "Executed via efficient mode");

                    // Optionally convert to TOON
                    let content = if self.config.use_toon {
                        if let Ok(json) = serde_json::from_str::<Value>(&result) {
                            crate::toon::encode_toon(&json)
                        } else {
                            result
                        }
                    } else {
                        result
                    };

                    Some(Ok(ToolOutput {
                        tool_call_id: format!("{}:{}", server_name, tool_name),
                        content,
                        is_error: false,
                    }))
                }
                Err(e) => {
                    warn!(tool = %tool_name, error = %e, "Efficient execution failed, falling back");
                    None // Will try native fallback
                }
            }
        } else {
            None
        };

        // Return efficient result if successful
        if let Some(result) = efficient_result {
            return result;
        }

        // Fallback to native
        if let Some(client) = self.native_clients.get_mut(server_name) {
            debug!(tool = %tool_name, "Executing via native mode");
            client.call_tool(tool_name, args).await
        } else {
            Err(Error::Mcp(McpError::ToolNotFound {
                server: server_name.to_string(),
                tool: tool_name.to_string(),
            }))
        }
    }

    /// Get current mode statistics
    pub fn stats(&self) -> AdaptiveStats {
        AdaptiveStats {
            native_servers: self.native_clients.len() - self.efficient_clients.len(),
            efficient_servers: self.efficient_clients.len(),
            total_tools: self.stats.total_tools_discovered,
            tools_used: self.stats.tools_actually_used.len(),
            conversation_turns: self.stats.conversation_turns,
            estimated_tokens_saved: self.estimate_token_savings(),
        }
    }

    /// Estimate tokens saved vs native MCP
    fn estimate_token_savings(&self) -> usize {
        let efficient_count = self
            .efficient_clients
            .values()
            .map(|c| c.tool_count)
            .sum::<usize>();

        // Native: ~121 tokens per tool per turn
        // Efficient: ~16 tokens per tool (list) + ~120 per used tool (help)
        let native_cost = efficient_count * 121 * self.stats.conversation_turns.max(1);
        let efficient_cost = efficient_count * 16
            + self.stats.tools_actually_used.len() * 120
            + self.stats.conversation_turns * 67; // system prompt

        native_cost.saturating_sub(efficient_cost)
    }

    /// Shut down all connections
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (_, client) in self.native_clients.iter_mut() {
            let _ = client.shutdown().await;
        }
        self.native_clients.clear();
        self.efficient_clients.clear();

        Ok(())
    }
}

/// Statistics about adaptive behavior
#[derive(Debug)]
pub struct AdaptiveStats {
    pub native_servers: usize,
    pub efficient_servers: usize,
    pub total_tools: usize,
    pub tools_used: usize,
    pub conversation_turns: usize,
    pub estimated_tokens_saved: usize,
}

/// Create minimal JSON schema placeholder
fn minimal_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "description": "Schema available on-demand via get_tool_schema()"
    })
}

fn help_to_schema(help: &crate::discovery::ToolHelp) -> Value {
    use std::collections::HashMap;

    let properties: HashMap<String, Value> = help
        .parameters
        .iter()
        .map(|p| {
            let schema = serde_json::json!({
                "type": &p.type_name,
                "description": &p.description,
            });
            (p.name.clone(), schema)
        })
        .collect();

    let required: Vec<String> = help
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| p.name.clone())
        .collect();

    serde_json::json!({
        "type": "object",
        "description": &help.description,
        "properties": properties,
        "required": required,
    })
}

/// Builder for adaptive configuration
pub struct AdaptiveConfigBuilder {
    config: AdaptiveConfig,
}

impl AdaptiveConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AdaptiveConfig::default(),
        }
    }

    pub fn mode(mut self, mode: AdaptiveMode) -> Self {
        self.config.mode = mode;
        self
    }

    pub fn efficient_threshold(mut self, threshold: usize) -> Self {
        self.config.efficient_threshold = threshold;
        self
    }

    pub fn use_toon(mut self, use_toon: bool) -> Self {
        self.config.use_toon = use_toon;
        self
    }

    pub fn build(self) -> AdaptiveConfig {
        self.config
    }
}

impl Default for AdaptiveConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
