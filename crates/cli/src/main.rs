//! OpenRustClaw CLI entry point.

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        /// Channels to enable (comma-separated, e.g. webchat,telegram,discord,slack,teams)
        #[arg(short = 'C', long, value_name = "CHANNELS")]
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
    /// Manage autonomous optimization targets and candidates
    Optimize {
        #[command(subcommand)]
        action: OptimizeAction,
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
    /// Interactive onboarding wizard
    Onboard,
    /// Set up Cursor IDE integration
    #[cfg(feature = "cursor")]
    Cursor {
        #[command(subcommand)]
        action: CursorAction,
    },
    /// Start MCP server for external clients
    McpServer {
        /// Transport type (stdio or sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Config file path for database-backed MCP tools
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// mcp2cli - Token-efficient MCP tool discovery (96-99% savings)
    Mcp2Cli {
        #[command(subcommand)]
        action: Mcp2CliAction,
    },
    /// Talk Mode - continuous voice conversation
    #[cfg(feature = "voice")]
    Talk {
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(short, long, default_value = "anthropic")]
        provider: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Wake word to activate listening
        #[arg(short, long, default_value = "Hey Assistant")]
        wake_word: String,
        /// Silence timeout in seconds (stop listening after silence)
        #[arg(long, default_value = "3")]
        silence_timeout: u64,
        /// Maximum utterance duration in seconds
        #[arg(long, default_value = "30")]
        max_utterance: u64,
        /// Enable barge-in (interrupt TTS with wake word)
        #[arg(long, default_value = "true")]
        barge_in: bool,
    },
    /// Manage webhooks for external integrations
    Webhooks {
        #[command(subcommand)]
        action: WebhooksAction,
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
    /// Search for skills in the registry
    Search {
        query: String,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        /// Sort by: relevance, downloads, rating, recent
        #[arg(short, long, default_value = "relevance")]
        sort: String,
    },
    /// Install a skill from the workspace or ClawHub registry
    Install { name: String },
    /// Update an installed skill
    Update { name: String },
    /// Uninstall a skill
    Uninstall { name: String },
    /// Verify skill signatures
    Verify { name: String },
    /// Show popular skills
    Popular {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show trending skills
    Trending {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
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
        #[arg(short, long)]
        description: Option<String>,
        /// Run every N seconds
        #[arg(long)]
        every_seconds: Option<u64>,
        /// Run once at an RFC3339 timestamp
        #[arg(long)]
        at: Option<String>,
        /// JSON workflow payload passed to the sidecar
        #[arg(long)]
        payload: Option<String>,
    },
    /// Pause a job
    Pause { id: String },
    /// Resume a job
    Resume { id: String },
    /// List recent job attempts
    Runs {
        #[arg(long)]
        job: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// List dead-letter entries
    DeadLetters {
        #[arg(long)]
        job: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Replay a dead-letter entry by id
    ReplayDeadLetter { id: String },
    /// List recent runtime events
    Events {
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum OptimizeAction {
    /// List registered optimization targets
    ListTargets,
    /// Show one optimization target
    ShowTarget { id: String },
    /// Register a new optimization target
    RegisterTarget {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        tier: String,
        #[arg(long, default_value = "safe_config")]
        risk_class: String,
        #[arg(long, default_value = "experimental")]
        ship_status: String,
        #[arg(long, default_value = ".")]
        workspace_root: String,
        #[arg(long = "allowed-path")]
        allowed_paths: Vec<String>,
        #[arg(long = "forbidden-path")]
        forbidden_paths: Vec<String>,
        #[arg(long = "allowed-field")]
        allowed_fields: Vec<String>,
        #[arg(long, default_value_t = 8)]
        max_changed_files: usize,
        #[arg(long, default_value_t = 32768)]
        max_total_bytes: usize,
        #[arg(long, default_value_t = 400)]
        max_diff_lines: usize,
        #[arg(long = "required-test")]
        required_tests: Vec<String>,
        #[arg(long = "mandatory-eval")]
        mandatory_evals: Vec<String>,
        #[arg(long = "eval")]
        evals: Vec<String>,
        #[arg(long)]
        metadata: Option<String>,
    },
    /// List optimization candidates
    ListCandidates {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show an optimization candidate and its history
    ShowCandidate { id: String },
    /// Submit a new optimization candidate from a JSON change set
    SubmitCandidate {
        #[arg(long)]
        target: String,
        #[arg(long)]
        hypothesis: String,
        #[arg(long, default_value = "operator")]
        proposed_by: String,
        #[arg(long)]
        change_set: String,
        #[arg(long)]
        trace_id: Option<String>,
    },
    /// Run the bounded experiment loop for one candidate
    RunCandidate { id: String },
    /// Approve a candidate without promoting it
    ApproveCandidate {
        id: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Reject a candidate
    RejectCandidate {
        id: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Record a promotion decision
    PromoteCandidate {
        id: String,
        #[arg(long)]
        decision: String,
        #[arg(long, default_value = "operator")]
        decided_by: String,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        rollback_reference: Option<String>,
    },
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

#[cfg(feature = "cursor")]
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
        mcp_stdio: Option<String>,
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
        mcp_stdio: Option<String>,
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

#[derive(Subcommand)]
enum WebhooksAction {
    /// List configured webhooks
    List,
    /// Create a new webhook
    Create {
        /// Webhook path (e.g., "github", "stripe")
        path: String,
    },
    /// Delete a webhook
    Delete {
        /// Webhook path to delete
        path: String,
    },
    /// Enable a webhook
    Enable {
        /// Webhook path to enable
        path: String,
    },
    /// Disable a webhook
    Disable {
        /// Webhook path to disable
        path: String,
    },
    /// Show webhook details
    Info {
        /// Webhook path
        path: String,
    },
    /// Test a webhook by sending a sample request
    Test {
        /// Webhook path to test
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, channels } => {
            commands::start::run(&config, channels.as_deref()).await
        }
        Commands::Chat { provider, model } => {
            commands::chat::run(&provider, model.as_deref()).await
        }
        Commands::Models { action } => match action {
            ModelsAction::List => commands::models::list().await,
            ModelsAction::Info { name } => commands::models::info(&name).await,
        },
        Commands::Skills { action } => match action {
            SkillsAction::List => commands::skills::list().await,
            SkillsAction::Search {
                query,
                category,
                sort,
            } => commands::skills::search(&query, category.as_deref(), &sort).await,
            SkillsAction::Install { name } => commands::skills::install(&name).await,
            SkillsAction::Update { name } => commands::skills::update(&name).await,
            SkillsAction::Uninstall { name } => commands::skills::uninstall(&name).await,
            SkillsAction::Verify { name } => commands::skills::verify(&name).await,
            SkillsAction::Popular { limit } => commands::skills::popular(limit).await,
            SkillsAction::Trending { limit } => commands::skills::trending(limit).await,
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::List => commands::schedule::list().await,
            ScheduleAction::Create {
                name,
                workflow,
                description,
                every_seconds,
                at,
                payload,
            } => {
                commands::schedule::create(
                    &name,
                    &workflow,
                    description.as_deref(),
                    every_seconds,
                    at.as_deref(),
                    payload.as_deref(),
                )
                .await
            }
            ScheduleAction::Pause { id } => commands::schedule::pause(&id).await,
            ScheduleAction::Resume { id } => commands::schedule::resume(&id).await,
            ScheduleAction::Runs { job, limit } => {
                commands::schedule::runs(job.as_deref(), limit).await
            }
            ScheduleAction::DeadLetters { job, limit } => {
                commands::schedule::dead_letters(job.as_deref(), limit).await
            }
            ScheduleAction::ReplayDeadLetter { id } => {
                commands::schedule::replay_dead_letter(&id).await
            }
            ScheduleAction::Events { name, limit } => {
                commands::schedule::events(name.as_deref(), limit).await
            }
        },
        Commands::Optimize { action } => match action {
            OptimizeAction::ListTargets => commands::optimize::list_targets().await,
            OptimizeAction::ShowTarget { id } => commands::optimize::show_target(&id).await,
            OptimizeAction::RegisterTarget {
                name,
                description,
                kind,
                tier,
                risk_class,
                ship_status,
                workspace_root,
                allowed_paths,
                forbidden_paths,
                allowed_fields,
                max_changed_files,
                max_total_bytes,
                max_diff_lines,
                required_tests,
                mandatory_evals,
                evals,
                metadata,
            } => {
                commands::optimize::register_target(
                    &name,
                    description.as_deref(),
                    &kind,
                    &tier,
                    &risk_class,
                    &ship_status,
                    &workspace_root,
                    &allowed_paths,
                    &forbidden_paths,
                    &allowed_fields,
                    max_changed_files,
                    max_total_bytes,
                    max_diff_lines,
                    &required_tests,
                    &mandatory_evals,
                    &evals,
                    metadata.as_deref(),
                )
                .await
            }
            OptimizeAction::ListCandidates {
                target,
                status,
                limit,
            } => {
                commands::optimize::list_candidates(target.as_deref(), status.as_deref(), limit)
                    .await
            }
            OptimizeAction::ShowCandidate { id } => commands::optimize::show_candidate(&id).await,
            OptimizeAction::SubmitCandidate {
                target,
                hypothesis,
                proposed_by,
                change_set,
                trace_id,
            } => {
                commands::optimize::submit_candidate(
                    &target,
                    &hypothesis,
                    &proposed_by,
                    &change_set,
                    trace_id.as_deref(),
                )
                .await
            }
            OptimizeAction::RunCandidate { id } => commands::optimize::run_candidate(&id).await,
            OptimizeAction::ApproveCandidate {
                id,
                decided_by,
                notes,
            } => commands::optimize::approve_candidate(&id, &decided_by, notes.as_deref()).await,
            OptimizeAction::RejectCandidate {
                id,
                decided_by,
                notes,
            } => commands::optimize::reject_candidate(&id, &decided_by, notes.as_deref()).await,
            OptimizeAction::PromoteCandidate {
                id,
                decision,
                decided_by,
                notes,
                rollback_reference,
            } => {
                commands::optimize::promote_candidate(
                    &id,
                    &decision,
                    &decided_by,
                    notes.as_deref(),
                    rollback_reference.as_deref(),
                )
                .await
            }
        },
        Commands::Security { action } => match action {
            SecurityAction::Audit => commands::security::audit().await,
            SecurityAction::GenerateKeys => commands::security::generate_keys().await,
        },
        Commands::Memory { action } => match action {
            MemoryAction::Export { output, user_id } => {
                commands::memory::export(&output, user_id.as_deref()).await
            }
            MemoryAction::Import { file, user_id } => {
                commands::memory::import(&file, &user_id).await
            }
            MemoryAction::Stats => commands::memory::stats().await,
        },
        Commands::Doctor => commands::doctor::run().await,
        Commands::Onboard => commands::onboard::run().await,
        #[cfg(feature = "cursor")]
        Commands::Cursor { action } => match action {
            CursorAction::Setup => commands::cursor::setup().await,
            CursorAction::Start { transport, port } => {
                commands::cursor::start(&transport, port).await
            }
            CursorAction::Status => commands::cursor::status().await,
        },
        Commands::McpServer { transport, config } => {
            commands::start::run_mcp_server(&transport, &config).await
        }
        Commands::Mcp2Cli { action } => match action {
            Mcp2CliAction::List {
                mcp,
                mcp_stdio,
                spec,
                base_url,
                refresh,
                format,
            } => {
                commands::mcp2cli::list(
                    mcp,
                    mcp_stdio,
                    spec,
                    base_url,
                    refresh,
                    parse_format(&format),
                )
                .await
            }
            Mcp2CliAction::Help {
                mcp,
                mcp_stdio,
                spec,
                tool,
                format,
            } => {
                commands::mcp2cli::help_cmd(mcp, mcp_stdio, spec, tool, parse_format(&format)).await
            }
            Mcp2CliAction::Run {
                mcp,
                mcp_stdio,
                spec,
                tool,
                args,
                stdin,
                format,
            } => {
                commands::mcp2cli::run(
                    mcp,
                    mcp_stdio,
                    spec,
                    tool,
                    args,
                    stdin,
                    parse_format(&format),
                )
                .await
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
        #[cfg(feature = "voice")]
        Commands::Talk {
            provider,
            model,
            wake_word,
            silence_timeout,
            max_utterance,
            barge_in,
        } => {
            commands::talk::run(
                &provider,
                model.as_deref(),
                &wake_word,
                silence_timeout,
                max_utterance,
                barge_in,
            )
            .await
        }
        Commands::Webhooks { action } => match action {
            WebhooksAction::List => commands::webhooks::list().await,
            WebhooksAction::Create { path } => commands::webhooks::create(&path).await,
            WebhooksAction::Delete { path } => commands::webhooks::delete(&path).await,
            WebhooksAction::Enable { path } => commands::webhooks::enable(&path).await,
            WebhooksAction::Disable { path } => commands::webhooks::disable(&path).await,
            WebhooksAction::Info { path } => commands::webhooks::info(&path).await,
            WebhooksAction::Test { path } => commands::webhooks::test(&path).await,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // --- parse_format tests ---

    #[test]
    fn test_parse_format_json() {
        matches!(parse_format("json"), commands::mcp2cli::OutputFormat::Json);
    }

    #[test]
    fn test_parse_format_json_uppercase() {
        matches!(parse_format("JSON"), commands::mcp2cli::OutputFormat::Json);
    }

    #[test]
    fn test_parse_format_toon() {
        matches!(parse_format("toon"), commands::mcp2cli::OutputFormat::Toon);
    }

    #[test]
    fn test_parse_format_table() {
        matches!(
            parse_format("table"),
            commands::mcp2cli::OutputFormat::Table
        );
    }

    #[test]
    fn test_parse_format_unknown_defaults_to_table() {
        matches!(parse_format("xyz"), commands::mcp2cli::OutputFormat::Table);
    }

    #[test]
    fn test_parse_format_empty_defaults_to_table() {
        matches!(parse_format(""), commands::mcp2cli::OutputFormat::Table);
    }

    // --- CLI argument parsing tests ---

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::try_parse_from(["openrustclaw", "doctor"]);
        assert!(cli.is_ok());
        matches!(cli.unwrap().command, Commands::Doctor);
    }

    #[test]
    fn test_cli_parse_onboard() {
        let cli = Cli::try_parse_from(["openrustclaw", "onboard"]);
        assert!(cli.is_ok());
        matches!(cli.unwrap().command, Commands::Onboard);
    }

    #[test]
    fn test_cli_parse_start_defaults() {
        let cli = Cli::try_parse_from(["openrustclaw", "start"]).unwrap();
        match cli.command {
            Commands::Start { config, channels } => {
                assert_eq!(config, "config/default.toml");
                assert!(channels.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_with_config() {
        let cli =
            Cli::try_parse_from(["openrustclaw", "start", "--config", "my_config.toml"]).unwrap();
        match cli.command {
            Commands::Start { config, channels } => {
                assert_eq!(config, "my_config.toml");
                assert!(channels.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_start_with_channels() {
        let cli = Cli::try_parse_from(["openrustclaw", "start", "-C", "telegram,discord"]).unwrap();
        match cli.command {
            Commands::Start {
                config: _,
                channels,
            } => {
                assert_eq!(channels.as_deref(), Some("telegram,discord"));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_defaults() {
        let cli = Cli::try_parse_from(["openrustclaw", "chat"]).unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "anthropic");
                assert!(model.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_with_provider() {
        let cli = Cli::try_parse_from(["openrustclaw", "chat", "--provider", "openai"]).unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "openai");
                assert!(model.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_chat_with_model() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "chat",
            "--provider",
            "ollama",
            "--model",
            "llama3.2",
        ])
        .unwrap();
        match cli.command {
            Commands::Chat { provider, model } => {
                assert_eq!(provider, "ollama");
                assert_eq!(model.as_deref(), Some("llama3.2"));
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cli_parse_models_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "models", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Models {
                action: ModelsAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_models_info() {
        let cli = Cli::try_parse_from(["openrustclaw", "models", "info", "gpt-4o"]).unwrap();
        match cli.command {
            Commands::Models {
                action: ModelsAction::Info { name },
            } => {
                assert_eq!(name, "gpt-4o");
            }
            _ => panic!("Expected Models Info command"),
        }
    }

    #[test]
    fn test_cli_parse_security_audit() {
        let cli = Cli::try_parse_from(["openrustclaw", "security", "audit"]).unwrap();
        matches!(
            cli.command,
            Commands::Security {
                action: SecurityAction::Audit
            }
        );
    }

    #[test]
    fn test_cli_parse_security_generate_keys() {
        let cli = Cli::try_parse_from(["openrustclaw", "security", "generate-keys"]).unwrap();
        matches!(
            cli.command,
            Commands::Security {
                action: SecurityAction::GenerateKeys
            }
        );
    }

    #[test]
    fn test_cli_parse_memory_stats() {
        let cli = Cli::try_parse_from(["openrustclaw", "memory", "stats"]).unwrap();
        matches!(
            cli.command,
            Commands::Memory {
                action: MemoryAction::Stats
            }
        );
    }

    #[test]
    fn test_cli_parse_memory_export() {
        let cli = Cli::try_parse_from(["openrustclaw", "memory", "export", "--output", "dump.md"])
            .unwrap();
        match cli.command {
            Commands::Memory {
                action: MemoryAction::Export { output, user_id },
            } => {
                assert_eq!(output, "dump.md");
                assert!(user_id.is_none());
            }
            _ => panic!("Expected Memory Export command"),
        }
    }

    #[test]
    fn test_cli_parse_memory_import() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "memory",
            "import",
            "--file",
            "MEMORY.md",
            "--user-id",
            "user1",
        ])
        .unwrap();
        match cli.command {
            Commands::Memory {
                action: MemoryAction::Import { file, user_id },
            } => {
                assert_eq!(file, "MEMORY.md");
                assert_eq!(user_id, "user1");
            }
            _ => panic!("Expected Memory Import command"),
        }
    }

    #[test]
    fn test_cli_parse_schedule_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "schedule", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Schedule {
                action: ScheduleAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_schedule_create() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "schedule",
            "create",
            "--name",
            "daily-check",
            "--workflow",
            "health_check",
        ])
        .unwrap();
        match cli.command {
            Commands::Schedule {
                action:
                    ScheduleAction::Create {
                        name,
                        workflow,
                        description,
                        every_seconds,
                        at,
                        payload,
                    },
            } => {
                assert_eq!(name, "daily-check");
                assert_eq!(workflow, "health_check");
                assert!(description.is_none());
                assert!(every_seconds.is_none());
                assert!(at.is_none());
                assert!(payload.is_none());
            }
            _ => panic!("Expected Schedule Create command"),
        }
    }

    #[test]
    fn test_cli_parse_optimize_register_target() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "optimize",
            "register-target",
            "--name",
            "skills.instructions",
            "--kind",
            "skill",
            "--tier",
            "rust_native",
            "--allowed-path",
            "skills/",
            "--eval",
            "smoke=bash -lc 'echo ok'",
        ])
        .unwrap();
        match cli.command {
            Commands::Optimize {
                action:
                    OptimizeAction::RegisterTarget {
                        name,
                        kind,
                        tier,
                        allowed_paths,
                        evals,
                        ..
                    },
            } => {
                assert_eq!(name, "skills.instructions");
                assert_eq!(kind, "skill");
                assert_eq!(tier, "rust_native");
                assert_eq!(allowed_paths, vec!["skills/"]);
                assert_eq!(evals, vec!["smoke=bash -lc 'echo ok'"]);
            }
            _ => panic!("Expected Optimize RegisterTarget command"),
        }
    }

    #[test]
    fn test_cli_parse_optimize_submit_candidate() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "optimize",
            "submit-candidate",
            "--target",
            "skills.instructions",
            "--hypothesis",
            "reduce noise",
            "--change-set",
            "changes.json",
        ])
        .unwrap();
        match cli.command {
            Commands::Optimize {
                action:
                    OptimizeAction::SubmitCandidate {
                        target,
                        hypothesis,
                        change_set,
                        ..
                    },
            } => {
                assert_eq!(target, "skills.instructions");
                assert_eq!(hypothesis, "reduce noise");
                assert_eq!(change_set, "changes.json");
            }
            _ => panic!("Expected Optimize SubmitCandidate command"),
        }
    }

    #[test]
    fn test_cli_parse_skills_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Skills {
                action: SkillsAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_skills_search() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "search", "web"]).unwrap();
        match cli.command {
            Commands::Skills {
                action:
                    SkillsAction::Search {
                        query,
                        category,
                        sort,
                    },
            } => {
                assert_eq!(query, "web");
                assert!(category.is_none());
                assert_eq!(sort, "relevance");
            }
            _ => panic!("Expected Skills Search command"),
        }
    }

    #[test]
    fn test_cli_parse_skills_install() {
        let cli = Cli::try_parse_from(["openrustclaw", "skills", "install", "web_search"]).unwrap();
        match cli.command {
            Commands::Skills {
                action: SkillsAction::Install { name },
            } => {
                assert_eq!(name, "web_search");
            }
            _ => panic!("Expected Skills Install command"),
        }
    }

    #[test]
    fn test_cli_parse_webhooks_list() {
        let cli = Cli::try_parse_from(["openrustclaw", "webhooks", "list"]).unwrap();
        matches!(
            cli.command,
            Commands::Webhooks {
                action: WebhooksAction::List
            }
        );
    }

    #[test]
    fn test_cli_parse_webhooks_create() {
        let cli = Cli::try_parse_from(["openrustclaw", "webhooks", "create", "github"]).unwrap();
        match cli.command {
            Commands::Webhooks {
                action: WebhooksAction::Create { path },
            } => {
                assert_eq!(path, "github");
            }
            _ => panic!("Expected Webhooks Create command"),
        }
    }

    #[test]
    fn test_cli_parse_mcp_server_default() {
        let cli = Cli::try_parse_from(["openrustclaw", "mcp-server"]).unwrap();
        match cli.command {
            Commands::McpServer { transport, config } => {
                assert_eq!(transport, "stdio");
                assert_eq!(config, "config/default.toml");
            }
            _ => panic!("Expected McpServer command"),
        }
    }

    #[test]
    fn test_cli_parse_mcp_server_with_config() {
        let cli = Cli::try_parse_from([
            "openrustclaw",
            "mcp-server",
            "--transport",
            "stdio",
            "--config",
            "config/dev.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::McpServer { transport, config } => {
                assert_eq!(transport, "stdio");
                assert_eq!(config, "config/dev.toml");
            }
            _ => panic!("Expected McpServer command"),
        }
    }

    #[test]
    fn test_cli_parse_invalid_command() {
        let cli = Cli::try_parse_from(["openrustclaw", "nonexistent"]);
        assert!(cli.is_err());
    }

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::try_parse_from(["openrustclaw"]);
        assert!(cli.is_err());
    }
}
