//! mcp2cli: MCP-to-CLI adapter with on-demand discovery for 96-99% token savings
//!
//! This crate solves the MCP token bloat problem by converting MCP servers and
//! OpenAPI specs into CLI tools with on-demand discovery:
//!
//! - `--list`: ~16 tokens per tool (vs 300-800 tokens native)
//! - `--help`: ~80-200 tokens per tool (vs full schema)
//!
//! Key features:
//! - On-demand tool discovery
//! - TOON (Token-Optimized Output Notation) format
//! - Caching with configurable TTL
//! - OAuth support with PKCE
//! - Unified interface for MCP and OpenAPI
//! - Zero-codegen runtime CLI generation

pub mod adapters;
pub mod auth;
pub mod cache;
pub mod cli_generator;
pub mod discovery;
pub mod error;
pub mod integration;
pub mod token_counter;
pub mod toon;
pub mod adaptive;

// Re-export mcp for unified API
pub use openrustclaw_mcp as mcp;

// Core mcp2cli exports
pub use adapters::{mcp_adapter::McpAdapter, openapi_adapter::OpenApiAdapter, ToolSource};
pub use auth::{AuthManager, OAuthConfig, Token};
pub use cache::{ToolCache, CachedToolList};
pub use cli_generator::CliGenerator;
pub use discovery::{ToolDiscovery, ToolSummary, ToolHelp, ParamHelp};
pub use error::{Mcp2CliError, Result};
pub use integration::{Mcp2CliTool, Mcp2CliFactory};
pub use token_counter::{TokenCounter, CostComparison};
pub use toon::{encode_toon, decode_toon, calculate_savings};

// Unified adaptive MCP exports - transparent mcp2cli optimization
pub use adaptive::{
    AdaptiveConfig, AdaptiveConfigBuilder, AdaptiveMcpRegistry, 
    AdaptiveMode, AdaptiveStats,
};

use std::time::Duration;

/// Default cache TTL (1 hour)
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Default token cost for tool list entry (~16 tokens)
pub const DEFAULT_LIST_TOKEN_COST: usize = 16;

/// Default token cost for tool help (~80-200 tokens)
pub const DEFAULT_HELP_TOKEN_COST_MIN: usize = 80;
pub const DEFAULT_HELP_TOKEN_COST_MAX: usize = 200;

/// Version of the mcp2cli crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the mcp2cli subsystem
pub fn init() {
    tracing::info!("mcp2cli v{} initialized", VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_CACHE_TTL.as_secs(), 3600);
        assert_eq!(DEFAULT_LIST_TOKEN_COST, 16);
        assert!(!VERSION.is_empty());
    }
}
