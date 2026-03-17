//! Example Cursor ACP Server.
//!
//! This example demonstrates how to run the Cursor ACP server
//! that can be used as an MCP server in Cursor IDE.
//!
//! # Usage
//!
//! ```bash
//! # Run with stdio transport (for MCP)
//! cargo run --example server -- --transport stdio
//!
//! # Run with HTTP transport
//! cargo run --example server -- --transport http --port 8080
//! ```

use clap::{Parser, ValueEnum};
use openrustclaw_cursor::{CursorConfig, CursorServer, CursorServerConfig, ServerTransport};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "openrustclaw-cursor-server")]
#[command(about = "Cursor ACP Server for OpenRustClaw")]
struct Cli {
    /// Transport type
    #[arg(short, long, value_enum, default_value = "stdio")]
    transport: TransportType,

    /// Port for HTTP transport
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Project root directory
    #[arg(short, long, value_name = "PATH")]
    project_root: Option<PathBuf>,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[derive(Debug)]
enum TransportType {
    /// JSON-RPC over stdio (MCP compatible)
    Stdio,
    /// HTTP server
    Http,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = match cli.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // Determine project root
    let project_root = cli
        .project_root
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    info!("Starting Cursor ACP Server");
    info!("Project root: {:?}", project_root);
    info!("Transport: {:?}", cli.transport);

    // Create Cursor config
    let cursor_config = CursorConfig {
        enabled: true,
        project_root,
        include_patterns: vec![
            "src/**/*.rs".to_string(),
            "crates/**/*.rs".to_string(),
            "*.toml".to_string(),
            "*.md".to_string(),
        ],
        exclude_patterns: vec![
            "target/**".to_string(),
            ".git/**".to_string(),
            "node_modules/**".to_string(),
        ],
        terminal_timeout: 30,
        max_file_size: 1024 * 1024, // 1MB
        auto_format: true,
        linters: vec!["clippy".to_string(), "rustfmt".to_string()],
    };

    // Create server config
    let transport = match cli.transport {
        TransportType::Stdio => ServerTransport::Stdio,
        TransportType::Http => ServerTransport::Http { port: cli.port },
    };

    let config = CursorServerConfig {
        transport,
        name: "openrustclaw-cursor".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        cursor_config,
    };

    // Create and run server
    let server = CursorServer::new(config);

    // Handle Ctrl+C for graceful shutdown
    let shutdown = tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Shutdown signal received");
    });

    // Run server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    });

    // Wait for either server completion or shutdown signal
    tokio::select! {
        _ = shutdown => {
            info!("Shutting down gracefully...");
        }
        _ = server_handle => {
            info!("Server completed");
        }
    }

    Ok(())
}
