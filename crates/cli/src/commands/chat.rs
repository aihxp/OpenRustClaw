//! Interactive chat REPL command.

use anyhow::{Context, Result};
use openrustclaw_core::traits::CoreMemoryStore;
use std::io::{self, Write};
use std::sync::Arc;

use openrustclaw_agent::tools::ToolRegistry;
use openrustclaw_core::types::{Message, Platform, Session};
use openrustclaw_db::{
    SessionStatus, SqliteCoreMemoryStore, SqliteMemoryStore, SqliteSessionStore,
};

use super::assistant;
use super::{runtime, session};

const CHAT_HISTORY_WINDOW: usize = 128;

/// Run the interactive chat REPL.
pub async fn run(provider: &str, model: Option<&str>) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║        OpenRustClaw Assistant Chat (Persisted)           ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Provider: {}", provider);
    if let Some(m) = model {
        println!("Model: {}", m);
    }
    println!();
    println!("Commands:");
    println!("  /quit, /q     - Exit the chat");
    println!("  /memory, /m   - Show current memory/context");
    println!("  /tools, /t    - List available tools");
    println!("  /session, /s  - Show the persisted session details");
    println!("  /help, /h     - Show this help");
    println!();

    let workspace_root = std::env::current_dir()?;
    let user_id = std::env::var("USER").unwrap_or_else(|_| "cli_user".to_string());
    let config = load_chat_config(provider, model)
        .await
        .context("Failed to load chat configuration")?;
    let pool = session::open_pool(&config).await?;
    let session_store = SqliteSessionStore::new(pool.clone());
    let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
    let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool));

    // Initialize provider chain based on CLI args and effective config
    let provider = session::build_provider(&config).context("Failed to initialize provider")?;

    // Create agent runtime
    let runtime = assistant::build_runtime(
        provider,
        memory_store,
        core_memory_store.clone(),
        workspace_root.clone(),
    );
    let tool_registry = runtime.tool_registry().clone();

    let mut chat_state =
        load_or_create_chat_session(&session_store, &user_id, &workspace_root).await?;

    if chat_state.resumed_existing {
        println!(
            "Resumed session: {} ({} message(s) loaded)",
            chat_state.session_id,
            chat_state.messages.len()
        );
    } else {
        println!("Started new session: {}", chat_state.session_id);
    }
    println!("Type your message and press Enter (or /quit to exit)");
    println!();

    // REPL loop
    loop {
        // Print prompt
        print!("\x1b[1;32mYou:\x1b[0m ");
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Handle special commands
        match handle_command(input, &tool_registry, &chat_state).await? {
            CommandResult::Continue => continue,
            CommandResult::Quit => {
                println!("Goodbye!");
                break;
            }
            CommandResult::Proceed => {}
        }

        // Add user message to persisted and in-memory history
        let user_message = Message::user(input);
        session_store
            .append_message(&chat_state.session_id, &user_message)
            .await?;
        chat_state.messages.push(user_message);
        trim_messages(&mut chat_state.messages);

        // Send to agent runtime and stream response
        print!("\x1b[1;34mAgent:\x1b[0m ");
        io::stdout().flush()?;

        let core_memory = core_memory_store.get_all(&user_id).await?;
        match runtime
            .process(
                &chat_state.messages,
                &core_memory,
                &chat_state.session_id,
                &user_id,
            )
            .await
        {
            Ok(response) => {
                println!("{}", response.message.content);

                if response.tool_calls_made > 0 {
                    println!("\x1b[90m[Used {} tool(s)]\x1b[0m", response.tool_calls_made);
                }

                session_store
                    .append_message(&chat_state.session_id, &response.message)
                    .await?;
                chat_state.messages.push(response.message);
                trim_messages(&mut chat_state.messages);
            }
            Err(e) => {
                eprintln!("\x1b[1;31mError: {}\x1b[0m", e);
            }
        }

        println!();
    }

    Ok(())
}

/// Result of handling a special command.
enum CommandResult {
    /// Continue to next iteration (command handled).
    Continue,
    /// Quit the REPL.
    Quit,
    /// Proceed with normal message processing.
    Proceed,
}

struct ChatSessionState {
    session_id: String,
    route_key: String,
    messages: Vec<Message>,
    resumed_existing: bool,
}

/// Handle special commands starting with /.
async fn handle_command(
    input: &str,
    tool_registry: &ToolRegistry,
    chat_state: &ChatSessionState,
) -> Result<CommandResult> {
    match input {
        "/quit" | "/q" | "exit" => Ok(CommandResult::Quit),
        "/help" | "/h" => {
            print_help();
            Ok(CommandResult::Continue)
        }
        "/memory" | "/m" => {
            show_memory(&chat_state.messages);
            Ok(CommandResult::Continue)
        }
        "/tools" | "/t" => {
            show_tools(tool_registry).await;
            Ok(CommandResult::Continue)
        }
        "/session" | "/s" => {
            show_session(chat_state);
            Ok(CommandResult::Continue)
        }
        _ if input.starts_with('/') => {
            println!("Unknown command: {}", input);
            println!("Type /help for available commands");
            Ok(CommandResult::Continue)
        }
        _ => Ok(CommandResult::Proceed),
    }
}

/// Print help message.
fn print_help() {
    println!();
    println!("Available commands:");
    println!("  /quit, /q     - Exit the chat");
    println!("  /memory, /m   - Show current conversation context");
    println!("  /tools, /t    - List available tools");
    println!("  /session, /s  - Show the persisted session id and route key");
    println!("  /help, /h     - Show this help");
    println!();
}

/// Show current memory/conversation context.
fn show_memory(messages: &[Message]) {
    println!();
    println!("═══ Current Conversation Context ═══");
    if messages.is_empty() {
        println!("  (No messages yet)");
    } else {
        for (i, msg) in messages.iter().enumerate() {
            let role_color = match msg.role {
                openrustclaw_core::types::Role::User => "\x1b[32m",
                openrustclaw_core::types::Role::Assistant => "\x1b[34m",
                openrustclaw_core::types::Role::System => "\x1b[90m",
                openrustclaw_core::types::Role::Tool => "\x1b[33m",
            };
            let preview: String = msg.content.chars().take(60).collect();
            let ellipsis = if msg.content.len() > 60 { "..." } else { "" };
            println!(
                "  {}[{}] {}\x1b[0m: {}{}\x1b[0m",
                role_color, i, msg.role, preview, ellipsis
            );
        }
    }
    println!("═══ {} messages in context ═══", messages.len());
    println!();
}

/// Show available tools.
async fn show_tools(tool_registry: &ToolRegistry) {
    println!();
    println!("═══ Available Tools ═══");
    let definitions = tool_registry.definitions();
    if definitions.is_empty() {
        println!("  (No tools registered)");
    } else {
        for def in &definitions {
            println!("  • {} - {}", def.name, def.description);
        }
    }
    println!("═══ {} tool(s) available ═══", definitions.len());
    println!();
}

fn show_session(chat_state: &ChatSessionState) {
    println!();
    println!("═══ Current Session ═══");
    println!("  id: {}", chat_state.session_id);
    println!("  route: {}", chat_state.route_key);
    println!(
        "  mode: {}",
        if chat_state.resumed_existing {
            "resumed"
        } else {
            "new"
        }
    );
    println!("  loaded messages: {}", chat_state.messages.len());
    println!();
}

/// Create an LLM provider based on the provider name.
async fn load_chat_config(
    provider_name: &str,
    model: Option<&str>,
) -> Result<openrustclaw_core::config::AppConfig> {
    let mut config =
        runtime::load_effective_config("config/default.toml", &std::env::current_dir()?)
            .unwrap_or_default();
    config.providers.default_provider = provider_name.to_string();
    match provider_name.to_lowercase().as_str() {
        "anthropic" => {
            if let Some(model) = model {
                config.providers.anthropic.model = model.to_string();
            }
        }
        "openai" => {
            if let Some(model) = model {
                config.providers.openai.model = model.to_string();
            }
        }
        "openrouter" => {
            if let Some(model) = model {
                config.providers.openrouter.model = model.to_string();
            }
        }
        "ollama" => {
            if let Some(model) = model {
                config.providers.ollama.model = model.to_string();
            }
        }
        _ => anyhow::bail!(
            "Unknown provider: {}. Available: anthropic, openai, openrouter, ollama",
            provider_name
        ),
    }
    Ok(config)
}

async fn load_or_create_chat_session(
    store: &SqliteSessionStore,
    user_id: &str,
    workspace_root: &std::path::Path,
) -> Result<ChatSessionState> {
    let route_key = assistant::cli_route_key(user_id, workspace_root);
    if let Some(existing) = store.find_active_by_route_key(&route_key).await? {
        let messages = store
            .list_history(&existing.session.id.to_string(), CHAT_HISTORY_WINDOW)
            .await?;
        return Ok(ChatSessionState {
            session_id: existing.session.id.to_string(),
            route_key,
            messages,
            resumed_existing: true,
        });
    }

    let mut session = Session::new_dm(user_id, Platform::Cli);
    session.metadata = assistant::session_metadata(
        "cli",
        Some(&route_key),
        Some(workspace_root),
        serde_json::json!({
            "assistant_mode": "chat",
        }),
    );
    store
        .create_or_update(&session, Some(&route_key), SessionStatus::Active)
        .await?;
    Ok(ChatSessionState {
        session_id: session.id.to_string(),
        route_key,
        messages: Vec::new(),
        resumed_existing: false,
    })
}

fn trim_messages(messages: &mut Vec<Message>) {
    if messages.len() > CHAT_HISTORY_WINDOW {
        let drain_count = messages.len() - CHAT_HISTORY_WINDOW;
        messages.drain(0..drain_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_db::run_migrations;
    use tempfile::tempdir;

    #[tokio::test]
    async fn chat_session_creates_new_persisted_cli_session() -> Result<()> {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
        run_migrations(&pool).await?;
        let store = SqliteSessionStore::new(pool);
        let workspace = tempdir()?;

        let state = load_or_create_chat_session(&store, "alice", workspace.path()).await?;

        assert!(!state.resumed_existing);
        assert!(state.messages.is_empty());
        assert!(state.route_key.starts_with("cli:assistant:alice:"));
        let loaded = store.get_session(&state.session_id).await?;
        let loaded = loaded.expect("persisted session");
        assert_eq!(loaded.session.metadata["assistant_identity"], "primary");
        assert_eq!(loaded.session.metadata["assistant_surface"], "cli");
        Ok(())
    }

    #[tokio::test]
    async fn chat_session_resumes_existing_history_for_same_workspace_user() -> Result<()> {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
        run_migrations(&pool).await?;
        let store = SqliteSessionStore::new(pool);
        let workspace = tempdir()?;

        let first = load_or_create_chat_session(&store, "alice", workspace.path()).await?;
        store
            .append_message(
                &first.session_id,
                &Message::user("hello from the first run"),
            )
            .await?;

        let resumed = load_or_create_chat_session(&store, "alice", workspace.path()).await?;

        assert!(resumed.resumed_existing);
        assert_eq!(resumed.session_id, first.session_id);
        assert_eq!(resumed.messages.len(), 1);
        assert_eq!(resumed.messages[0].content, "hello from the first run");
        Ok(())
    }
}
