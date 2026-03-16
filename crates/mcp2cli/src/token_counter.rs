//! Token counting for cost analysis
//!
//! This module provides token counting using tiktoken-rs and calculates
//! cost comparisons between native MCP and mcp2cli.

use crate::toon::encode_toon;
use serde_json::Value;
use tiktoken_rs::cl100k_base;

/// Token counter using tiktoken
pub struct TokenCounter;

impl TokenCounter {
    /// Count tokens in text using tiktoken (cl100k_base)
    ///
    /// This is the tokenizer used by GPT-4, GPT-3.5, and Claude.
    pub fn count_tokens(text: &str) -> usize {
        let bpe = cl100k_base().unwrap();
        bpe.encode_with_special_tokens(text).len()
    }

    /// Count tokens in a JSON value
    pub fn count_json_tokens(value: &Value) -> usize {
        Self::count_tokens(&value.to_string())
    }

    /// Count tokens in TOON format
    pub fn count_toon_tokens(value: &Value) -> usize {
        let toon = encode_toon(value);
        Self::count_tokens(&toon)
    }

    /// Compare costs between native MCP and mcp2cli
    ///
    /// # Arguments
    ///
    /// * `tool_count` - Number of available tools
    /// * `turns` - Number of conversation turns
    /// * `tools_used` - Number of tools actually used in the conversation
    ///
    /// # Returns
    ///
    /// A cost comparison showing token savings
    pub fn compare_costs(
        tool_count: usize,
        turns: usize,
        tools_used: usize,
    ) -> CostComparison {
        // Native MCP: All tool schemas are included in every prompt
        // Average tool schema is ~500 tokens
        let native_tokens_per_turn = tool_count * 500;
        let native_total = native_tokens_per_turn * turns;

        // mcp2cli: Only tool summaries (~16 tokens each) + help for used tools (~150 tokens each)
        let mcp2cli_list_cost = tool_count * 16; // Tool list
        let mcp2cli_help_cost = tools_used * 150; // Help for tools that get used
        let _mcp2cli_total = mcp2cli_list_cost + mcp2cli_help_cost;

        // Account for multiple turns - only pay help cost once per unique tool
        let unique_tools_used = tools_used.min(tool_count);
        let mcp2cli_per_turn = tool_count * 16; // Always pay list cost
        let mcp2cli_total = mcp2cli_per_turn + (unique_tools_used * 150);

        let savings = if native_total > 0 {
            (native_total - mcp2cli_total) as f64 / native_total as f64
        } else {
            0.0
        };

        CostComparison {
            native_tokens: native_total,
            mcp2cli_tokens: mcp2cli_total,
            savings_percent: savings.clamp(0.0, 1.0),
        }
    }

    /// Calculate detailed cost breakdown for a session
    pub fn session_cost_breakdown(
        tool_count: usize,
        tools_discovered: usize,
        tools_used_with_help: usize,
        turns: usize,
    ) -> SessionCostBreakdown {
        // Discovery phase
        let discovery_list_cost = tool_count * 16;
        let discovery_detail_cost = tools_discovered * 150;

        // Execution phase (per turn)
        let per_turn_list_cost = tool_count * 16;
        let help_lookup_cost = tools_used_with_help * 150;

        let total_discovered = discovery_list_cost + discovery_detail_cost;
        let total_per_turn = per_turn_list_cost + help_lookup_cost;
        let total_for_session = total_discovered + (total_per_turn * turns);

        // Compare to native
        let native_cost = tool_count * 500 * turns;
        let savings = if native_cost > 0 {
            (native_cost - total_for_session) as f64 / native_cost as f64
        } else {
            0.0
        };

        SessionCostBreakdown {
            discovery_list_cost,
            discovery_detail_cost,
            per_turn_list_cost,
            help_lookup_cost,
            total_discovered,
            total_per_turn,
            total_for_session,
            native_equivalent: native_cost,
            savings_percent: savings.clamp(0.0, 1.0),
        }
    }

    /// Estimate token cost for a tool schema
    pub fn estimate_schema_tokens(schema: &Value) -> usize {
        let schema_str = schema.to_string();
        Self::count_tokens(&schema_str)
    }

    /// Estimate token cost for a tool summary
    pub fn estimate_summary_tokens(name: &str, description: &str) -> usize {
        // Compact format: "name: description"
        let summary = format!("{}: {}", name, description);
        Self::count_tokens(&summary)
    }

    /// Estimate token cost for tool help
    pub fn estimate_help_tokens(
        name: &str,
        description: &str,
        usage: &str,
        param_count: usize,
    ) -> usize {
        // Help format includes name, description, usage, and params
        let base = format!("{} {} {}", name, description, usage);
        let base_tokens = Self::count_tokens(&base);
        let param_tokens = param_count * 20; // ~20 tokens per param
        base_tokens + param_tokens
    }
}

/// Cost comparison between native MCP and mcp2cli
#[derive(Debug, Clone, Copy)]
pub struct CostComparison {
    /// Token cost using native MCP (all schemas every turn)
    pub native_tokens: usize,
    /// Token cost using mcp2cli (on-demand discovery)
    pub mcp2cli_tokens: usize,
    /// Savings percentage (0.0 to 1.0, where 0.96 = 96% savings)
    pub savings_percent: f64,
}

impl CostComparison {
    /// Get the actual tokens saved
    pub fn tokens_saved(&self) -> usize {
        self.native_tokens.saturating_sub(self.mcp2cli_tokens)
    }

    /// Format as a human-readable string
    pub fn format(&self) -> String {
        format!(
            "Native MCP: {} tokens\nmcp2cli: {} tokens\nSavings: {:.1}% ({} tokens)",
            self.native_tokens,
            self.mcp2cli_tokens,
            self.savings_percent * 100.0,
            self.tokens_saved()
        )
    }
}

/// Detailed cost breakdown for a session
#[derive(Debug, Clone, Copy)]
pub struct SessionCostBreakdown {
    /// Cost to list all tools (discovery)
    pub discovery_list_cost: usize,
    /// Cost to get detailed help for discovered tools
    pub discovery_detail_cost: usize,
    /// Cost to maintain tool list per turn
    pub per_turn_list_cost: usize,
    /// Cost to look up help for tools being used
    pub help_lookup_cost: usize,
    /// Total cost during discovery phase
    pub total_discovered: usize,
    /// Total cost per turn
    pub total_per_turn: usize,
    /// Total cost for entire session
    pub total_for_session: usize,
    /// What native MCP would have cost
    pub native_equivalent: usize,
    /// Savings percentage
    pub savings_percent: f64,
}

impl SessionCostBreakdown {
    /// Format as a detailed report
    pub fn format_report(&self) -> String {
        format!(
            r#"Token Cost Breakdown
====================

Discovery Phase:
  - List {} tools: {} tokens
  - Get help for {} tools: {} tokens
  - Subtotal: {} tokens

Execution Phase (per turn):
  - Tool list: {} tokens
  - Help lookups: {} tokens
  - Subtotal: {} tokens

Session Total ({} turns): {} tokens
Native MCP equivalent: {} tokens

SAVINGS: {:.1}% ({} tokens)
"#,
            self.discovery_list_cost / 16,
            self.discovery_list_cost,
            self.discovery_detail_cost / 150,
            self.discovery_detail_cost,
            self.total_discovered,
            self.per_turn_list_cost,
            self.help_lookup_cost,
            self.total_per_turn,
            (self.total_for_session - self.total_discovered) / self.total_per_turn.max(1) + 1,
            self.total_for_session,
            self.native_equivalent,
            self.savings_percent * 100.0,
            self.native_equivalent.saturating_sub(self.total_for_session)
        )
    }
}

/// Quick calculation for common scenarios
pub fn quick_savings_estimate(tool_count: usize) -> String {
    let scenarios = vec![
        ("Single turn, 1 tool used", 1, 1),
        ("5 turns, 3 tools used", 5, 3),
        ("10 turns, 5 tools used", 10, 5),
        ("20 turns, 10 tools used", 20, 10),
    ];

    let mut output = format!(
        "Token Savings Estimate ({} tools available)\n",
        tool_count
    );
    output.push_str("=" .repeat(50).as_str());
    output.push('\n');

    for (desc, turns, tools_used) in scenarios {
        let comparison = TokenCounter::compare_costs(tool_count, turns, tools_used);
        output.push_str(&format!(
            "\n{}:\n  Native: {} tokens\n  mcp2cli: {} tokens\n  Savings: {:.1}%\n",
            desc,
            comparison.native_tokens,
            comparison.mcp2cli_tokens,
            comparison.savings_percent * 100.0
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens() {
        let text = "Hello, world!";
        let count = TokenCounter::count_tokens(text);
        assert!(count > 0);
        
        // Should be consistent
        assert_eq!(TokenCounter::count_tokens(text), count);
    }

    #[test]
    fn test_compare_costs() {
        // 100 tools, 10 turns, 5 tools actually used
        let comparison = TokenCounter::compare_costs(100, 10, 5);
        
        // Native: 100 * 500 * 10 = 500,000 tokens
        // mcp2cli: 100 * 16 + 5 * 150 = 2,350 tokens
        // Savings should be very high (>95%)
        
        assert!(comparison.native_tokens > comparison.mcp2cli_tokens);
        assert!(comparison.savings_percent > 0.90); // >90% savings
    }

    #[test]
    fn test_cost_comparison_format() {
        let comparison = CostComparison {
            native_tokens: 500000,
            mcp2cli_tokens: 2350,
            savings_percent: 0.9953,
        };
        
        let formatted = comparison.format();
        assert!(formatted.contains("500000"));
        assert!(formatted.contains("2350"));
        assert!(formatted.contains("99.5"));
    }

    #[test]
    fn test_session_cost_breakdown() {
        let breakdown = TokenCounter::session_cost_breakdown(50, 10, 3, 5);
        
        assert!(breakdown.total_for_session < breakdown.native_equivalent);
        assert!(breakdown.savings_percent > 0.0);
        
        let report = breakdown.format_report();
        assert!(report.contains("Token Cost Breakdown"));
        assert!(report.contains("SAVINGS"));
    }

    #[test]
    fn test_estimate_schema_tokens() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" }
            }
        });
        
        let tokens = TokenCounter::estimate_schema_tokens(&schema);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_summary_tokens() {
        let tokens = TokenCounter::estimate_summary_tokens("search", "Search for documents");
        assert!(tokens > 0);
        assert!(tokens < 50); // Should be much less than a full schema
    }

    #[test]
    fn test_quick_savings_estimate() {
        let estimate = quick_savings_estimate(100);
        assert!(estimate.contains("100 tools"));
        assert!(estimate.contains("Savings"));
    }
}
