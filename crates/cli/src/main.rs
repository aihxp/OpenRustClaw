//! OpenRustClaw CLI entry point.

use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(name = "openrustclaw")]
#[command(about = "OpenRustClaw - AI Agent Framework", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server and Python sidecar
    Start {
        /// Config file path
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        /// Channels to enable (comma-separated: telegram,discord,slack)
        #[arg(short, long, value_name = "CHANNELS")]
        channels: Option<String>,
    },
    /// Interactive chat with the agent
    Chat {
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(short, long, default_value = "anthropic")]
        provider: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Manage LLM models
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },
    /// Manage skills
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Manage scheduled jobs
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
    /// Security audit and management
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
    /// Memory management
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Run diagnostics
    Doctor,
    /// Set up Cursor IDE integration
    Cursor {
        #[command(subcommand)]
        action: CursorAction,
    },
    /// Start MCP server for external clients
    McpServer {
        /// Transport type (stdio or sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
    },
    /// mcp2cli - Token-efficient MCP tool discovery (96-99% savings)
    Mcp2Cli {
        #[command(subcommand)]
        action: Mcp2CliAction,
    },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// List available models
    List,
    /// Show model details
    Info { name: String },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// List installed skills
    List,
    /// Install a skill from marketplace
    Install { name: String },
    /// Verify skill signatures
    Verify { name: String },
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// List scheduled jobs
    List,
    /// Create a new job
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        workflow: String,
    },
    /// Pause a job
    Pause { id: String },
    /// Resume a job
    Resume { id: String },
}

#[derive(Subcommand)]
enum SecurityAction {
    /// Run security audit
    Audit,
    /// Generate Ed25519 keypair for skill signing
    GenerateKeys,
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Export memory to markdown for inspection
    Export {
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        user_id: Option<String>,
    },
    /// Import legacy OpenClaw MEMORY.md
    Import {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        user_id: String,
    },
    /// Show memory statistics
    Stats,
}

#[derive(Subcommand)]
enum CursorAction {
    /// Generate .cursor/mcp.json and .cursor/rules/
    Setup,
    /// Start the Cursor ACP server
    Start {
        /// Transport type (stdio or tcp)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Port for TCP transport
        #[arg(short, long, default_value = "9000")]
        port: u16,
    },
    /// Check Cursor IDE integration status
    Status,
}

#[derive(Subcommand)]
#[clap(rename_all = "kebab-case")]
enum Mcp2CliAction {
    /// List available tools (~16 tokens/tool)
    List {
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        mcp_stdio: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        refresh: bool,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Get tool help (~80-200 tokens)
    Help {
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        tool: String,
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Execute a tool
    Run {
        #[arg(long, group = "source")]
        mcp: Option<String>,
        #[arg(long, group = "source")]
        spec: Option<String>,
        tool: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long)]
        stdin: bool,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Analyze token costs
    Analyze {
        #[arg(short, long, default_value = "30")]
        tools: usize,
        #[arg(short, long, default_value = "15")]
        turns: usize,
        #[arg(short, long, default_value = "5")]
        used: usize,
    },
    /// Convert to/from TOON format
    Toon {
        input: Option<String>,
        #[arg(long)]
        decode: bool,
    },
    /// Cache management
    Cache {
        #[command(subcommand)]
        action: Mcp2CliCacheAction,
    },
}

#[derive(Subcommand)]
enum Mcp2CliCacheAction {
    /// Clear all cached data
    Clear,
    /// Show cache statistics
    Stats,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, channels } => commands::start::run(&config, channels.as_deref()).await,
        Commands::Chat { provider, model } => commands::chat::run(&provider, model.as_deref()).await,
        Commands::Models { action } => match action {
            ModelsAction::List => commands::models::list().await,
            ModelsAction::Info { name } => commands::models::info(&name).await,
        },
        Commands::Skills { action } => match action {
            SkillsAction::List => commands::skills::list().await,
            SkillsAction::Install { name } => commands::skills::install(&name).await,
            SkillsAction::Verify { name } => commands::skills::verify(&name).await,
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::List => commands::schedule::list().await,
            ScheduleAction::Create { name, workflow } => commands::schedule::create(&name, &workflow).await,
            ScheduleAction::Pause { id } => commands::schedule::pause(&id).await,
            ScheduleAction::Resume { id } => commands::schedule::resume(&id).await,
        },
        Commands::Security { action } => match action {
            SecurityAction::Audit => commands::security::audit().await,
            SecurityAction::GenerateKeys => commands::security::generate_keys().await,
        },
        Commands::Memory { action } => match action {
            MemoryAction::Export { output, user_id } => commands::memory::export(&output, user_id.as_deref()).await,
            MemoryAction::Import { file, user_id } => commands::memory::import(&file, &user_id).await,
            MemoryAction::Stats => commands::memory::stats().await,
        },
        Commands::Doctor => commands::doctor::run().await,
        Commands::Cursor { action } => match action {
            CursorAction::Setup => commands::cursor::setup().await,
            CursorAction::Start { transport, port } => commands::cursor::start(&transport, port).await,
            CursorAction::Status => commands::cursor::status().await,
        },
        Commands::McpServer { transport } => commands::start::run_mcp_server(&transport).await,
        Commands::Mcp2Cli { action } => match action {
            Mcp2CliAction::List { mcp, mcp_stdio, spec, base_url, refresh, format } => {
                commands::mcp2cli::list(mcp, mcp_stdio, spec, base_url, refresh, parse_format(&format)).await
            }
            Mcp2CliAction::Help { mcp, spec, tool, format } => {
                commands::mcp2cli::help_cmd(mcp, spec, tool, parse_format(&format)).await
            }
            Mcp2CliAction::Run { mcp, spec, tool, args, stdin, format } => {
                commands::mcp2cli::run(mcp, spec, tool, args, stdin, parse_format(&format)).await
            }
            Mcp2CliAction::Analyze { tools, turns, used } => {
                commands::mcp2cli::analyze(tools, turns, used).await
            }
            Mcp2CliAction::Toon { input, decode } => {
                commands::mcp2cli::toon_cmd(input, decode).await
            }
            Mcp2CliAction::Cache { action } => match action {
                Mcp2CliCacheAction::Clear => commands::mcp2cli::cache_clear().await,
                Mcp2CliCacheAction::Stats => commands::mcp2cli::cache_stats().await,
            },
        },
    }
}

fn parse_format(s: &str) -> commands::mcp2cli::OutputFormat {
    match s.to_lowercase().as_str() {
        "json" => commands::mcp2cli::OutputFormat::Json,
        "toon" => commands::mcp2cli::OutputFormat::Toon,
        _ => commands::mcp2cli::OutputFormat::Table,
    }
}
