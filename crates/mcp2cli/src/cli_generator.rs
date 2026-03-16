//! Dynamic CLI generation from tool definitions
//!
//! This module generates clap::Command structures from tool definitions,
//! enabling runtime CLI generation without code generation.

use crate::discovery::{ToolHelp, ToolSummary};
use crate::error::{Mcp2CliError, Result};
use clap::{Arg, ArgAction, Command};
use serde_json::Value;

/// CLI generator for tool definitions
pub struct CliGenerator;

impl CliGenerator {
    /// Generate a clap::Command from tool summaries
    ///
    /// Creates a CLI with subcommands for each tool.
    pub fn generate_command(name: &'static str, tools: &[ToolSummary]) -> Command {
        let mut cmd = Command::new(name)
            .about("MCP-to-CLI adapter with on-demand discovery")
            .version(crate::VERSION);

        // Add global flags
        cmd = cmd.arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .action(ArgAction::SetTrue)
                .help("List all available tools (~16 tokens each)"),
        );

        cmd = cmd.arg(
            Arg::new("help-tool")
                .long("help-tool")
                .value_name("TOOL")
                .help("Get detailed help for a specific tool (~80-200 tokens)"),
        );

        cmd = cmd.arg(
            Arg::new("toon")
                .long("toon")
                .action(ArgAction::SetTrue)
                .help("Use TOON format for output (40-60% fewer tokens)"),
        );

        cmd = cmd.arg(
            Arg::new("cache-ttl")
                .long("cache-ttl")
                .value_name("SECONDS")
                .default_value("3600")
                .help("Cache TTL in seconds (default: 1 hour)"),
        );

        cmd = cmd.arg(
            Arg::new("clear-cache")
                .long("clear-cache")
                .action(ArgAction::SetTrue)
                .help("Clear the tool cache"),
        );

        // Add subcommands for each tool
        for tool in tools {
            let subcmd = Self::generate_subcommand_from_summary(tool);
            cmd = cmd.subcommand(subcmd);
        }

        cmd
    }

    /// Generate a subcommand from a ToolSummary
    fn generate_subcommand_from_summary(tool: &ToolSummary) -> Command {
        // Leak the string to get 'static lifetime for clap
        let name: &'static str = Box::leak(tool.name.clone().into_boxed_str());
        Command::new(name)
            .about(tool.description.clone())
            // Add help flag specific to this tool
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::SetTrue)
                    .help("Show help for this tool"),
            )
    }

    /// Generate a subcommand from detailed ToolHelp
    ///
    /// This creates a fully-featured subcommand with arguments for each parameter.
    pub fn generate_subcommand(tool: &ToolHelp) -> Command {
        // Leak the string to get 'static lifetime for clap
        let name: &'static str = Box::leak(tool.name.clone().into_boxed_str());
        let mut cmd = Command::new(name)
            .about(tool.description.clone())
            .long_about(format!("{}\n\nUsage: {}", tool.description, tool.usage));

        // Add arguments for each parameter
        for param in &tool.parameters {
            let arg = Self::generate_argument(param);
            cmd = cmd.arg(arg);
        }

        // Add common flags
        cmd = cmd.arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output raw JSON instead of TOON"),
        );

        cmd = cmd.arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Show what would be executed without running"),
        );

        cmd
    }

    /// Generate an argument from a parameter definition
    fn generate_argument(param: &crate::discovery::ParamHelp) -> Arg {
        // Leak the name to get 'static lifetime for clap
        let name: &'static str = Box::leak(param.name.clone().into_boxed_str());
        let help_text = if let Some(ref example) = param.example {
            format!("{} (e.g., {})", param.description, example)
        } else {
            param.description.clone()
        };

        let mut arg = Arg::new(name)
            .long(name)
            .help(help_text);

        // Set required status
        if param.required {
            arg = arg.required(true);
        } else {
            arg = arg.required(false);
        }

        // Set value parser based on type
        arg = match param.type_name.as_str() {
            "number" | "integer" | "float" | "double" => {
                arg.value_parser(clap::value_parser!(f64))
            }
            "boolean" | "bool" => arg
                .value_parser(["true", "false"])
                .num_args(0..=1)
                .default_value("true"),
            _ => arg.value_parser(clap::value_parser!(String)),
        };

        // Set default value if present
        if let Some(ref default) = param.default {
            if let Some(s) = default.as_str() {
                // Leak the default value to get 'static lifetime
                let default_static: &'static str = Box::leak(s.to_string().into_boxed_str());
                arg = arg.default_value(default_static);
            }
        }

        arg
    }

    /// Parse CLI arguments and convert to JSON
    ///
    /// # Arguments
    ///
    /// * `matches` - The parsed clap matches
    ///
    /// # Returns
    ///
    /// A JSON object with the parsed arguments
    pub fn parse_args(matches: &clap::ArgMatches) -> Result<Value> {
        let mut result = serde_json::Map::new();

        // Collect all argument values
        for arg_id in matches.ids() {
            let key = arg_id.as_str();

            // Skip internal flags
            if key == "json" || key == "dry-run" || key == "help" {
                continue;
            }

            if let Ok(value) = matches.try_get_one::<String>(key) {
                if let Some(v) = value {
                    result.insert(key.to_string(), Value::String(v.clone()));
                }
            } else if let Ok(value) = matches.try_get_one::<f64>(key) {
                if let Some(v) = value {
                    result.insert(key.to_string(), serde_json::json!(v));
                }
            } else if let Ok(value) = matches.try_get_one::<bool>(key) {
                if let Some(v) = value {
                    result.insert(key.to_string(), Value::Bool(*v));
                }
            } else if let Ok(values) = matches.try_get_many::<String>(key) {
                let arr: Vec<Value> = values
                    .into_iter()
                    .flatten()
                    .map(|v| Value::String(v.clone()))
                    .collect();
                if !arr.is_empty() {
                    result.insert(key.to_string(), Value::Array(arr));
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// Parse arguments for a specific subcommand
    pub fn parse_subcommand_args(cmd: &Command, args: &[String]) -> Result<(String, Value)> {
        let cmd = cmd.clone();
        let matches = cmd.try_get_matches_from(args)
            .map_err(|e| Mcp2CliError::cli(format!("Failed to parse arguments: {}", e)))?;

        let (subcommand_name, sub_matches) = matches
            .subcommand()
            .ok_or_else(|| Mcp2CliError::cli("No subcommand specified"))?;

        let args = Self::parse_args(sub_matches)?;

        Ok((subcommand_name.to_string(), args))
    }

    /// Generate completion script
    #[cfg(feature = "completions")]
    pub fn generate_completions(shell: clap_complete::Shell, cmd: &mut Command) -> String {
        use clap_complete::generate;
        use std::io::Cursor;

        let mut buf = Cursor::new(Vec::new());
        generate(shell, cmd, cmd.get_name().to_string(), &mut buf);

        String::from_utf8(buf.into_inner()).unwrap_or_default()
    }
}

/// Generate a simple CLI from tool summaries for quick listing
pub fn generate_list_cli(tools: &[ToolSummary]) -> String {
    let mut output = String::new();
    output.push_str("Available tools:\n\n");

    for tool in tools {
        output.push_str(&format!("  {:<30} {}\n", tool.name, tool.description));
    }

    output.push_str(&format!("\nTotal: {} tools\n", tools.len()));
    output.push_str("\nUse --help-tool <TOOL> for detailed help\n");

    output
}

/// Format tool help for display
pub fn format_tool_help(tool: &ToolHelp) -> String {
    let mut output = String::new();

    output.push_str(&format!("Tool: {}\n", tool.name));
    output.push_str(&format!("Description: {}\n\n", tool.description));
    output.push_str(&format!("Usage: {}\n\n", tool.usage));

    if !tool.parameters.is_empty() {
        output.push_str("Parameters:\n");
        for param in &tool.parameters {
            let required_flag = if param.required { " (required)" } else { "" };
            output.push_str(&format!(
                "  --{} <{}>{}\n    {}\n",
                param.name, param.type_name, required_flag, param.description
            ));

            if let Some(ref default) = param.default {
                output.push_str(&format!("    Default: {}\n", default));
            }
            if let Some(ref example) = param.example {
                output.push_str(&format!("    Example: {}\n", example));
            }
            output.push('\n');
        }
    }

    output.push_str(&format!("\nEstimated token cost: {} tokens\n", tool.token_cost));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::ParamHelp;

    #[test]
    fn test_generate_command() {
        let tools = vec![
            ToolSummary::new("search", "Search for documents"),
            ToolSummary::new("create", "Create a new item"),
        ];

        let cmd = CliGenerator::generate_command("test-cli", &tools);
        
        assert_eq!(cmd.get_name(), "test-cli");
        // Check that subcommands were added
        let subcommands: Vec<_> = cmd.get_subcommands().collect();
        assert_eq!(subcommands.len(), 2);
    }

    #[test]
    fn test_generate_subcommand() {
        let params = vec![
            ParamHelp::new("query", "Search query", "string", true),
            ParamHelp::new("limit", "Max results", "number", false)
                .with_default(serde_json::json!(10)),
        ];

        let tool_help = ToolHelp::new(
            "search",
            "Search for documents",
            "search --query <query> [--limit <n>]",
            params,
        );

        let cmd = CliGenerator::generate_subcommand(&tool_help);
        
        assert_eq!(cmd.get_name(), "search");
        // Check that arguments were added
        let args: Vec<_> = cmd.get_arguments().collect();
        assert!(args.len() >= 2); // At least query and limit
    }

    #[test]
    fn test_generate_list_cli() {
        let tools = vec![
            ToolSummary::new("search", "Search for documents"),
            ToolSummary::new("create", "Create a new item"),
        ];

        let output = generate_list_cli(&tools);
        
        assert!(output.contains("Available tools"));
        assert!(output.contains("search"));
        assert!(output.contains("create"));
        assert!(output.contains("Total: 2 tools"));
    }

    #[test]
    fn test_format_tool_help() {
        let params = vec![
            ParamHelp::new("query", "Search query", "string", true)
                .with_example("rust programming"),
        ];

        let tool_help = ToolHelp::new(
            "search",
            "Search for documents",
            "search --query <query>",
            params,
        );

        let output = format_tool_help(&tool_help);
        
        assert!(output.contains("Tool: search"));
        assert!(output.contains("Parameters:"));
        assert!(output.contains("--query"));
        assert!(output.contains("rust programming"));
    }
}
