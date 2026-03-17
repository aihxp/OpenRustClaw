//! Example Cursor ACP Client.
//!
//! This example demonstrates how to connect to a Cursor ACP server
//! and use the available tools.
//!
//! # Usage
//!
//! ```bash
//! # Connect via TCP to a running server
//! cargo run --example client -- --connection tcp --host localhost --port 8080
//!
//! # The client will demonstrate various operations
//! ```

use clap::Parser;
use openrustclaw_cursor::{ClientConnection, CursorClient, CursorConfig};
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "openrustclaw-cursor-client")]
#[command(about = "Cursor ACP Client Example for OpenRustClaw")]
struct Cli {
    /// Connection type
    #[arg(short, long, default_value = "tcp")]
    connection: String,

    /// Host for TCP connection
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    /// Port for TCP connection
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Cursor ACP Client Example");
    info!("Connection: {} ({}:{})", cli.connection, cli.host, cli.port);

    // Create connection config
    let connection = match cli.connection.as_str() {
        "tcp" => ClientConnection::Tcp {
            host: cli.host.clone(),
            port: cli.port,
        },
        _ => {
            warn!("Unknown connection type: {}, using TCP", cli.connection);
            ClientConnection::Tcp {
                host: cli.host.clone(),
                port: cli.port,
            }
        }
    };

    // Create client
    let config = CursorConfig::default();
    let client = CursorClient::new(connection, config).with_timeout(cli.timeout);

    // Connect to server
    info!("Connecting to server...");
    let mut conn = client.connect().await?;
    info!("Connected!");

    // Demonstrate operations
    info!("\n=== Getting IDE State ===");
    match conn.get_state().await {
        Ok(state) => {
            info!("Session ID: {}", state.session_id);
            info!("Open files: {}", state.open_files.len());
            info!("Terminals: {}", state.terminals.len());
            if let Some(ref active) = state.active_file {
                info!("Active file: {:?}", active.path);
            }
        }
        Err(e) => {
            warn!("Failed to get IDE state: {}", e);
        }
    }

    info!("\n=== Running Command ===");
    match conn
        .execute_command("echo 'Hello from OpenRustClaw!'", None::<&str>)
        .await
    {
        Ok((stdout, stderr, exit_code)) => {
            info!("Exit code: {}", exit_code);
            info!("Stdout: {}", stdout.trim());
            if !stderr.is_empty() {
                warn!("Stderr: {}", stderr);
            }
        }
        Err(e) => {
            warn!("Command failed: {}", e);
        }
    }

    info!("\n=== Checking Git Status ===");
    match conn.git_status().await {
        Ok(status) => {
            info!("Branch: {}", status.branch);
            info!("Modified: {}", status.modified.len());
            info!("Staged: {}", status.staged.len());
            info!("Untracked: {}", status.untracked.len());
        }
        Err(e) => {
            warn!("Git status failed: {}", e);
        }
    }

    info!("\n=== Listing Files ===");
    match conn.list_files(".", false).await {
        Ok(entries) => {
            info!("Found {} entries", entries.len());
            for entry in entries.iter().take(10) {
                let entry_type = if entry.is_directory {
                    "[DIR]"
                } else {
                    "[FILE]"
                };
                info!("  {} {}", entry_type, entry.name);
            }
            if entries.len() > 10 {
                info!("  ... and {} more", entries.len() - 10);
            }
        }
        Err(e) => {
            warn!("List files failed: {}", e);
        }
    }

    info!("\n=== Searching Code ===");
    match conn.search_code("fn main").await {
        Ok(matches) => {
            info!("Found {} matches", matches.len());
            for m in matches.iter().take(5) {
                info!(
                    "  {:?}:{} - {}",
                    m.file_path,
                    m.line + 1,
                    m.line_content.trim()
                );
            }
        }
        Err(e) => {
            warn!("Search failed: {}", e);
        }
    }

    info!("\n=== Running Linter ===");
    match conn.run_linter("cargo-check", None::<&str>).await {
        Ok((output, diagnostics)) => {
            info!("Linter completed with {} diagnostics", diagnostics.len());
            if !diagnostics.is_empty() {
                for diag in diagnostics.iter().take(5) {
                    info!(
                        "  [{:?}] {:?}:{} - {}",
                        diag.severity,
                        diag.file_path,
                        diag.line + 1,
                        diag.message
                    );
                }
            }
        }
        Err(e) => {
            warn!("Linter failed: {}", e);
        }
    }

    // Close connection
    info!("\n=== Closing Connection ===");
    conn.close().await;
    info!("Done!");

    Ok(())
}
