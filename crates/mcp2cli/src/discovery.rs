//! On-demand tool discovery with minimal token cost
//!
//! This module provides efficient tool discovery that dramatically reduces
//! token usage compared to native MCP:
//!
//! - List all tools: ~16 tokens per tool (vs 300-800 native)
//! - Get help for a tool: ~80-200 tokens (vs full JSON schema)

use crate::adapters::ToolSource;
use crate::cache::ToolCache;
use crate::error::Result;
use crate::{DEFAULT_CACHE_TTL, DEFAULT_HELP_TOKEN_COST_MAX, DEFAULT_HELP_TOKEN_COST_MIN, DEFAULT_LIST_TOKEN_COST};
use serde::{Deserialize, Serialize};

use std::time::Duration;

/// On-demand tool discovery with caching
pub struct ToolDiscovery {
    cache: ToolCache,
}

impl ToolDiscovery {
    /// Create a new ToolDiscovery with default cache TTL
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_CACHE_TTL)
    }

    /// Create a new ToolDiscovery with custom cache TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: ToolCache::new(ttl),
        }
    }

    /// List all available tools (~16 tokens/tool)
    ///
    /// This method returns a compact summary of all available tools,
    /// suitable for presenting to an LLM as a tool catalog.
    pub async fn list_tools(&self, source: &ToolSource) -> Result<Vec<ToolSummary>> {
        let cache_key = format!("list:{}", source.cache_key());
        
        self.cache
            .get_or_insert(&cache_key, || async {
                let adapter = source.create_adapter().await?;
                adapter.list_tools().await
            })
            .await
            .map(|cached| cached.tools)
    }

    /// Get detailed help for a specific tool (~80-200 tokens)
    ///
    /// This method returns detailed information about a single tool,
    /// including parameter descriptions and usage examples.
    pub async fn get_help(&self, source: &ToolSource, tool_name: &str) -> Result<ToolHelp> {
        let cache_key = format!("help:{}:{}", source.cache_key(), tool_name);
        
        self.cache
            .get_or_insert_tool_help(&cache_key, || async {
                let adapter = source.create_adapter().await?;
                adapter.get_tool_help(tool_name).await
            })
            .await
    }

    /// Execute a tool with the given arguments
    pub async fn execute(&self, source: &ToolSource, tool_name: &str, args: serde_json::Value) -> Result<String> {
        let adapter = source.create_adapter().await?;
        adapter.execute_tool(tool_name, args).await
    }

    /// Clear cache for a specific source
    pub fn clear_cache(&self, source: &ToolSource) {
        let prefix = format!("{}:", source.cache_key());
        self.cache.invalidate_prefix(&prefix);
    }

    /// Clear all cached data
    pub fn clear_all_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
}

impl Default for ToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Compact tool summary (~16 tokens)
///
/// This is the minimal information needed to present a tool catalog
/// to an LLM without overwhelming the context window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSummary {
    /// Tool name
    pub name: String,
    /// Brief description (1-2 sentences)
    pub description: String,
    /// Estimated token cost of this summary
    #[serde(default = "default_list_token_cost")]
    pub token_cost: usize,
}

fn default_list_token_cost() -> usize {
    DEFAULT_LIST_TOKEN_COST
}

impl ToolSummary {
    /// Create a new tool summary
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let description = description.into();
        // Estimate token cost: ~4 chars per token on average
        let token_cost = (name.len() + description.len()) / 4;
        
        Self {
            name,
            description,
            token_cost: token_cost.max(DEFAULT_LIST_TOKEN_COST),
        }
    }

    /// Render as compact string for LLM consumption
    pub fn to_compact_string(&self) -> String {
        format!("- {}: {}", self.name, self.description)
    }
}

/// Detailed tool help (~80-200 tokens)
///
/// This provides comprehensive information about a tool when the LLM
/// needs to understand how to use it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolHelp {
    /// Tool name
    pub name: String,
    /// Full description
    pub description: String,
    /// Usage example
    pub usage: String,
    /// Parameter details
    pub parameters: Vec<ParamHelp>,
    /// Estimated token cost of this help
    #[serde(default = "default_help_token_cost")]
    pub token_cost: usize,
}

fn default_help_token_cost() -> usize {
    (DEFAULT_HELP_TOKEN_COST_MIN + DEFAULT_HELP_TOKEN_COST_MAX) / 2
}

impl ToolHelp {
    /// Create new tool help
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        usage: impl Into<String>,
        parameters: Vec<ParamHelp>,
    ) -> Self {
        let name = name.into();
        let description = description.into();
        let usage = usage.into();
        
        // Estimate token cost
        let param_tokens: usize = parameters.iter().map(|p| {
            (p.name.len() + p.description.len() + p.type_name.len()) / 4
        }).sum();
        let token_cost = ((name.len() + description.len() + usage.len()) / 4 + param_tokens)
            .max(DEFAULT_HELP_TOKEN_COST_MIN)
            .min(DEFAULT_HELP_TOKEN_COST_MAX);
        
        Self {
            name,
            description,
            usage,
            parameters,
            token_cost,
        }
    }

    /// Render as compact string for LLM consumption
    pub fn to_compact_string(&self) -> String {
        let params = if self.parameters.is_empty() {
            "None".to_string()
        } else {
            self.parameters
                .iter()
                .map(|p| format!("{} ({}){}", p.name, p.type_name, 
                    if p.required { "" } else { " [optional]" }))
                .collect::<Vec<_>>()
                .join(", ")
        };
        
        format!(
            "Tool: {}\nDescription: {}\nUsage: {}\nParameters: {}",
            self.name, self.description, self.usage, params
        )
    }
}

/// Parameter help for a tool parameter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamHelp {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Type name (e.g., "string", "number", "boolean")
    pub type_name: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value if optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

impl ParamHelp {
    /// Create a new parameter help
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        type_name: impl Into<String>,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            type_name: type_name.into(),
            required,
            default: None,
            example: None,
        }
    }

    /// Set default value
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Set example value
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }
}

// Re-export CacheStats from cache module
pub use crate::cache::CacheStats;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_summary() {
        let summary = ToolSummary::new("search", "Search for documents");
        assert_eq!(summary.name, "search");
        assert_eq!(summary.description, "Search for documents");
        assert!(summary.token_cost >= DEFAULT_LIST_TOKEN_COST);
        
        let compact = summary.to_compact_string();
        assert!(compact.contains("search"));
        assert!(compact.contains("Search for documents"));
    }

    #[test]
    fn test_param_help() {
        let param = ParamHelp::new("query", "Search query", "string", true)
            .with_example("rust programming");
        
        assert_eq!(param.name, "query");
        assert_eq!(param.example, Some("rust programming".to_string()));
        assert!(param.required);
    }

    #[test]
    fn test_tool_help() {
        let params = vec![
            ParamHelp::new("query", "Search query", "string", true),
            ParamHelp::new("limit", "Max results", "number", false)
                .with_default(serde_json::json!(10)),
        ];
        
        let help = ToolHelp::new(
            "search",
            "Search for documents",
            "search --query <query> [--limit <n>]",
            params,
        );
        
        assert_eq!(help.name, "search");
        assert_eq!(help.parameters.len(), 2);
        
        let compact = help.to_compact_string();
        assert!(compact.contains("search"));
        assert!(compact.contains("query"));
    }
}
