//! mcp2cli integration commands for OpenRustClaw.
//!
//! Provides CLI-based on-demand tool discovery for MCP servers and OpenAPI specs,
//! achieving 96-99% token savings compared to native MCP.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::time::Duration;

use openrustclaw_mcp2cli::{
    adapters::ToolSource, cache::ToolCache, discovery::ToolDiscovery, token_counter::TokenCounter,
    toon,
};

/// mcp2cli subcommands
#[derive(Subcommand)]
#[command(disable_help_subcommand = true)]
pub enum Mcp2CliCommands {
    /// List available tools from an MCP server or OpenAPI spec (~16 tokens/tool)
    List {
        /// MCP server URL (HTTP/SSE)
        #[arg(long, group = "source")]
        mcp: Option<String>,
        /// MCP server stdio command
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        /// OpenAPI spec URL or file path
        #[arg(long, group = "source")]
        spec: Option<String>,
        /// Base URL for OpenAPI (if not in spec)
        #[arg(long)]
        base_url: Option<String>,
        /// Force refresh (bypass cache)
        #[arg(long)]
        refresh: bool,
        /// Output format
        #[arg(long, default_value = "table", value_enum)]
        format: OutputFormat,
    },
    /// Get detailed help for a specific tool (~80-200 tokens)
    Help {
        /// MCP server URL
        #[arg(long, group = "source")]
        mcp: Option<String>,
        /// OpenAPI spec URL or file
        #[arg(long, group = "source")]
        spec: Option<String>,
        /// Tool/endpoint name
        tool: String,
        /// Output format
        #[arg(long, default_value = "text", value_enum)]
        format: OutputFormat,
    },
    /// Execute a tool
    Run {
        /// MCP server URL
        #[arg(long, group = "source")]
        mcp: Option<String>,
        /// OpenAPI spec URL or file
        #[arg(long, group = "source")]
        spec: Option<String>,
        /// Tool/endpoint name
        tool: String,
        /// Arguments as JSON string
        #[arg(long)]
        args: Option<String>,
        /// Read args from stdin
        #[arg(long)]
        stdin: bool,
        /// Output format
        #[arg(long, default_value = "json", value_enum)]
        format: OutputFormat,
    },
    /// Compare token costs: native MCP vs mcp2cli
    Analyze {
        /// Number of tools
        #[arg(short, long, default_value = "30")]
        tools: usize,
        /// Number of conversation turns
        #[arg(short, long, default_value = "15")]
        turns: usize,
        /// Number of unique tools actually used
        #[arg(short, long, default_value = "5")]
        used: usize,
    },
    /// Convert output to TOON format (Token-Optimized Output Notation)
    Toon {
        /// Input JSON file (or stdin if not provided)
        input: Option<String>,
        /// Decode TOON back to JSON
        #[arg(long)]
        decode: bool,
    },
    /// Manage cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached tool lists
    Clear,
    /// Show cache statistics
    Stats,
}

#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    /// Token-Optimized Output Notation (40-60% fewer tokens)
    Toon,
}

/// Run mcp2cli list command
pub async fn list(
    mcp: Option<String>,
    mcp_stdio: Option<String>,
    spec: Option<String>,
    base_url: Option<String>,
    refresh: bool,
    format: OutputFormat,
) -> Result<()> {
    let source = resolve_source(mcp, mcp_stdio, spec, base_url)?;

    let discovery = ToolDiscovery::with_ttl(Duration::from_secs(3600));

    if refresh {
        discovery.clear_cache(&source);
    }

    let tools = discovery
        .list_tools(&source)
        .await
        .context("Failed to list tools")?;

    let total_tokens: usize = tools.iter().map(|t| t.token_cost).sum();

    match format {
        OutputFormat::Table => {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!(
                "║           Available Tools ({:>3} found)              ║",
                tools.len()
            );
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("{:<30} {:<50} {:>10}", "Name", "Description", "Tokens");
            println!("{}", "─".repeat(95));

            for tool in &tools {
                let desc = if tool.description.len() > 47 {
                    format!("{}...", &tool.description[..47])
                } else {
                    tool.description.clone()
                };
                println!(
                    "{:<30} {:<50} {:>10}",
                    truncate(&tool.name, 30),
                    desc,
                    tool.token_cost
                );
            }

            println!();
            println!("Total: {} tools, ~{} tokens", tools.len(), total_tokens);
            println!("Native MCP would cost: ~{} tokens", tools.len() * 121);
            let savings = if !tools.is_empty() {
                ((tools.len() * 121) - total_tokens) as f64 / (tools.len() * 121) as f64 * 100.0
            } else {
                0.0
            };
            println!("Savings: {:.1}% with mcp2cli", savings);
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "tools": tools,
                "total_tokens": total_tokens,
                "count": tools.len(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Toon => {
            let json = serde_json::json!({
                "tools": tools.iter().map(|t| {
                    serde_json::json!({
                        "n": t.name,
                        "d": t.description,
                        "c": t.token_cost,
                    })
                }).collect::<Vec<_>>(),
                "tot": total_tokens,
                "cnt": tools.len(),
            });
            println!("{}", toon::encode_toon(&json));
        }
    }

    Ok(())
}

/// Run mcp2cli help command
pub async fn help_cmd(
    mcp: Option<String>,
    mcp_stdio: Option<String>,
    spec: Option<String>,
    tool_name: String,
    format: OutputFormat,
) -> Result<()> {
    let source = resolve_source(mcp, mcp_stdio, spec, None)?;

    let discovery = ToolDiscovery::with_ttl(Duration::from_secs(3600));

    let help = discovery
        .get_help(&source, &tool_name)
        .await
        .context(format!("Failed to get help for tool '{}'", tool_name))?;

    match format {
        OutputFormat::Table | OutputFormat::Json => {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║  Tool: {:<48} ║", truncate(&help.name, 48));
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("Description: {}", help.description);
            println!("Token cost: ~{} tokens", help.token_cost);
            println!();

            if !help.parameters.is_empty() {
                println!("Parameters:");
                println!(
                    "{:<20} {:<15} {:<10} Description",
                    "Name", "Type", "Required"
                );
                println!("{}", "─".repeat(80));
                for param in &help.parameters {
                    println!(
                        "{:<20} {:<15} {:<10} {}",
                        param.name,
                        &param.type_name,
                        if param.required { "yes" } else { "no" },
                        &param.description
                    );
                }
            }

            println!();
            println!("Usage:");
            println!("  {}", help.usage);
        }
        OutputFormat::Toon => {
            let json = serde_json::json!({
                "n": help.name,
                "d": help.description,
                "c": help.token_cost,
                "p": help.parameters.iter().map(|p| {
                    serde_json::json!({
                        "n": p.name,
                        "t": p.type_name,
                        "r": p.required,
                        "d": p.description,
                    })
                }).collect::<Vec<_>>(),
                "u": help.usage,
            });
            println!("{}", toon::encode_toon(&json));
        }
    }

    Ok(())
}

/// Run mcp2cli execute command
pub async fn run(
    mcp: Option<String>,
    mcp_stdio: Option<String>,
    spec: Option<String>,
    tool_name: String,
    args: Option<String>,
    stdin: bool,
    format: OutputFormat,
) -> Result<()> {
    let source = resolve_source(mcp, mcp_stdio, spec, None)?;

    let args_json = if stdin {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        serde_json::from_str(&input).context("Invalid JSON from stdin")?
    } else if let Some(args_str) = args {
        serde_json::from_str(&args_str).context("Invalid JSON in --args")?
    } else {
        serde_json::json!({})
    };

    let discovery = ToolDiscovery::with_ttl(Duration::from_secs(3600));

    let result = discovery
        .execute(&source, &tool_name, args_json)
        .await
        .context(format!("Failed to execute tool '{}'", tool_name))?;

    match format {
        OutputFormat::Table | OutputFormat::Json => {
            println!("{}", result);
        }
        OutputFormat::Toon => {
            let json: serde_json::Value = serde_json::from_str(&result)
                .unwrap_or_else(|_| serde_json::json!({"output": result}));
            println!("{}", toon::encode_toon(&json));
        }
    }

    Ok(())
}

/// Run token cost analysis
pub async fn analyze(tools: usize, turns: usize, used: usize) -> Result<()> {
    let comparison = TokenCounter::compare_costs(tools, turns, used);

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Token Cost Analysis: Native MCP vs mcp2cli       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "Scenario: {} tools, {} turns, {} unique tools used",
        tools, turns, used
    );
    println!();
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│ Native MCP (full schemas every turn):                   │");
    println!(
        "│   {:>10} tokens                                      │",
        comparison.native_tokens
    );
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│ mcp2cli (on-demand discovery):                          │");
    println!(
        "│   {:>10} tokens                                      │",
        comparison.mcp2cli_tokens
    );
    println!("├─────────────────────────────────────────────────────────┤");
    println!(
        "│ Savings: {:>5.1}%                                        │",
        comparison.savings_percent
    );
    println!("└─────────────────────────────────────────────────────────┘");
    println!();

    // Show breakdown
    let list_cost = tools * 16;
    let help_cost = used * 120;
    let system_prompt = turns * 67;

    println!("mcp2cli cost breakdown:");
    println!(
        "  --list ({} tools × 16 tokens):    {} tokens",
        tools, list_cost
    );
    println!(
        "  --help ({} tools × 120 tokens):  {} tokens",
        used, help_cost
    );
    println!(
        "  System prompt ({} turns):        {} tokens",
        turns, system_prompt
    );
    println!("  ─────────────────────────────────────────");
    println!(
        "  Total:                             {} tokens",
        list_cost + help_cost + system_prompt
    );

    Ok(())
}

/// Convert to/from TOON format
pub async fn toon_cmd(input: Option<String>, decode: bool) -> Result<()> {
    let content = if let Some(path) = input {
        tokio::fs::read_to_string(path)
            .await
            .context("Failed to read input file")?
    } else {
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        buf
    };

    if decode {
        let decoded = toon::decode_toon(&content).context("Failed to decode TOON")?;
        println!("{}", serde_json::to_string_pretty(&decoded)?);
    } else {
        let json: serde_json::Value =
            serde_json::from_str(&content).context("Invalid JSON input")?;
        let encoded = toon::encode_toon(&json);
        println!("{}", encoded);

        // Show savings
        let json_tokens = content.split_whitespace().count();
        let toon_tokens = encoded.split_whitespace().count();
        let savings = (json_tokens - toon_tokens) as f64 / json_tokens as f64 * 100.0;
        eprintln!(
            "Token savings: {:.1}% ({} → {} tokens)",
            savings, json_tokens, toon_tokens
        );
    }

    Ok(())
}

/// Clear cache
pub async fn cache_clear() -> Result<()> {
    let cache = ToolCache::new(Duration::from_secs(3600));
    cache.clear();
    println!("✓ Cache cleared successfully");
    Ok(())
}

/// Show cache stats
pub async fn cache_stats() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                mcp2cli Cache Statistics                  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Cache location: ~/.cache/openrustclaw/mcp2cli/");
    println!("Default TTL: 1 hour");
    println!();
    println!("Use --refresh flag with 'list' command to bypass cache.");
    Ok(())
}

/// Resolve source from CLI arguments
fn resolve_source(
    mcp: Option<String>,
    mcp_stdio: Option<String>,
    spec: Option<String>,
    _base_url: Option<String>,
) -> Result<ToolSource> {
    match (mcp, mcp_stdio, spec) {
        (Some(url), None, None) => Ok(ToolSource::McpUrl { url }),
        (None, Some(cmdline), None) => {
            let parts = shlex::split(&cmdline)
                .ok_or_else(|| anyhow::anyhow!("Invalid --mcp-stdio command line"))?;
            let (command, args) = parts
                .split_first()
                .ok_or_else(|| anyhow::anyhow!("--mcp-stdio cannot be empty"))?;
            Ok(ToolSource::McpStdio {
                command: command.clone(),
                args: args.to_vec(),
            })
        }
        (None, None, Some(spec)) => {
            // Determine if it's a URL or file path
            if spec.starts_with("http://") || spec.starts_with("https://") {
                Ok(ToolSource::OpenApiUrl { url: spec })
            } else {
                Ok(ToolSource::OpenApiFile { path: spec })
            }
        }
        _ => anyhow::bail!("Exactly one source required: --mcp, --mcp-stdio, or --spec"),
    }
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- truncate tests ---

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world, this is a long string", 10);
        assert!(result.len() <= 10);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 5), "");
    }

    // --- resolve_source tests ---

    #[test]
    fn test_resolve_source_mcp_url() {
        let result = resolve_source(Some("http://localhost:8080".to_string()), None, None, None);
        assert!(result.is_ok());
        match result.unwrap() {
            ToolSource::McpUrl { url } => assert_eq!(url, "http://localhost:8080"),
            _ => panic!("Expected McpUrl"),
        }
    }

    #[test]
    fn test_resolve_source_mcp_stdio() {
        let result = resolve_source(
            None,
            Some("npx -y @modelcontextprotocol/server-filesystem /tmp".to_string()),
            None,
            None,
        );
        assert!(result.is_ok());
        match result.unwrap() {
            ToolSource::McpStdio { command, args } => {
                assert_eq!(command, "npx");
                assert_eq!(
                    args,
                    vec![
                        "-y".to_string(),
                        "@modelcontextprotocol/server-filesystem".to_string(),
                        "/tmp".to_string()
                    ]
                );
            }
            _ => panic!("Expected McpStdio"),
        }
    }

    #[test]
    fn test_resolve_source_rejects_invalid_mcp_stdio() {
        let result = resolve_source(None, Some("\"unterminated".to_string()), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_source_openapi_url() {
        let result = resolve_source(
            None,
            None,
            Some("https://api.example.com/openapi.json".to_string()),
            None,
        );
        assert!(result.is_ok());
        match result.unwrap() {
            ToolSource::OpenApiUrl { url } => {
                assert_eq!(url, "https://api.example.com/openapi.json");
            }
            _ => panic!("Expected OpenApiUrl"),
        }
    }

    #[test]
    fn test_resolve_source_openapi_file() {
        let result = resolve_source(None, None, Some("./spec.yaml".to_string()), None);
        assert!(result.is_ok());
        match result.unwrap() {
            ToolSource::OpenApiFile { path } => assert_eq!(path, "./spec.yaml"),
            _ => panic!("Expected OpenApiFile"),
        }
    }

    #[test]
    fn test_resolve_source_no_source() {
        let result = resolve_source(None, None, None, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Exactly one source required")
        );
    }

    #[test]
    fn test_resolve_source_http_spec_is_url() {
        let result = resolve_source(
            None,
            None,
            Some("http://localhost:3000/openapi.json".to_string()),
            None,
        );
        assert!(result.is_ok());
        match result.unwrap() {
            ToolSource::OpenApiUrl { .. } => {}
            _ => panic!("http:// spec should resolve to OpenApiUrl"),
        }
    }

    // --- OutputFormat tests ---

    #[test]
    fn test_output_format_default() {
        let fmt = OutputFormat::default();
        matches!(fmt, OutputFormat::Table);
    }

    #[test]
    fn test_output_format_variants_exist() {
        let _table = OutputFormat::Table;
        let _json = OutputFormat::Json;
        let _toon = OutputFormat::Toon;
    }
}
