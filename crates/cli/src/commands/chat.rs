//! Interactive chat REPL command.

use anyhow::{Context, Result};
use std::io::{self, Write};
use std::sync::Arc;

use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_agent::tools::ToolRegistry;
use openrustclaw_core::types::{Message, Platform, Session};

use super::runtime;

/// Run the interactive chat REPL.
pub async fn run(provider: &str, model: Option<&str>) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           OpenRustClaw Interactive Chat                  ║");
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
    println!("  /help, /h     - Show this help");
    println!();

    // Initialize provider based on CLI args
    let provider = create_provider(provider, model)
        .await
        .context("Failed to initialize provider")?;

    // Create tool registry
    let tool_registry = Arc::new(ToolRegistry::new());

    // Create agent runtime
    let runtime = AgentRuntime::new(provider, tool_registry.clone(), "OpenRustClaw".to_string());

    // Create a session
    let user_id = std::env::var("USER").unwrap_or_else(|_| "cli_user".to_string());
    let session = Session::new_dm(&user_id, Platform::Cli);
    let session_id = session.id.to_string();

    println!("Session started: {}", session_id);
    println!("Type your message and press Enter (or /quit to exit)");
    println!();

    // Conversation history
    let mut messages: Vec<Message> = vec![];
    let core_memory = vec![];

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
        match handle_command(input, &tool_registry, &messages).await? {
            CommandResult::Continue => continue,
            CommandResult::Quit => {
                println!("Goodbye!");
                break;
            }
            CommandResult::Proceed => {}
        }

        // Add user message to history
        messages.push(Message::user(input));

        // Send to agent runtime and stream response
        print!("\x1b[1;34mAgent:\x1b[0m ");
        io::stdout().flush()?;

        match runtime
            .process(&messages, &core_memory, &session_id, &user_id)
            .await
        {
            Ok(response) => {
                println!("{}", response.message.content);

                if response.tool_calls_made > 0 {
                    println!("\x1b[90m[Used {} tool(s)]\x1b[0m", response.tool_calls_made);
                }

                // Add assistant response to history
                messages.push(response.message);

                // Keep conversation size manageable
                if messages.len() > 20 {
                    // Keep system context (first message if system) and last 10 exchanges
                    let drain_count = messages.len() - 20;
                    messages.drain(0..drain_count);
                }
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

/// Handle special commands starting with /.
async fn handle_command(
    input: &str,
    tool_registry: &ToolRegistry,
    messages: &[Message],
) -> Result<CommandResult> {
    match input {
        "/quit" | "/q" | "exit" => Ok(CommandResult::Quit),
        "/help" | "/h" => {
            print_help();
            Ok(CommandResult::Continue)
        }
        "/memory" | "/m" => {
            show_memory(messages);
            Ok(CommandResult::Continue)
        }
        "/tools" | "/t" => {
            show_tools(tool_registry).await;
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

/// Create an LLM provider based on the provider name.
async fn create_provider(
    provider_name: &str,
    model: Option<&str>,
) -> Result<Arc<dyn openrustclaw_core::traits::LlmProvider>> {
    let workspace_root = std::env::current_dir()?;
    let mut config =
        runtime::load_effective_config("config/default.toml", &workspace_root).unwrap_or_default();
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
    runtime::create_provider_from_config(provider_name, &config)
}
