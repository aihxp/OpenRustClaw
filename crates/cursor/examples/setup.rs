//! Example: Setup Cursor Integration.
//!
//! This example demonstrates how to setup Cursor IDE integration
//! for an OpenRustClaw project.
//!
//! # Usage
//!
//! ```bash
//! # Setup in current directory
//! cargo run --example setup
//!
//! # Setup in specific directory
//! cargo run --example setup -- --path /path/to/project
//!
//! # Check setup status
//! cargo run --example setup -- --check
//! ```

use clap::Parser;
use openrustclaw_cursor::{check_cursor_setup, setup_cursor_integration, generate_mcp_config};
use std::path::PathBuf;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "openrustclaw-cursor-setup")]
#[command(about = "Setup Cursor IDE Integration for OpenRustClaw")]
struct Cli {
    /// Project path (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    path: Option<PathBuf>,

    /// Only check status, don't setup
    #[arg(short, long)]
    check: bool,

    /// Force re-setup even if already configured
    #[arg(short, long)]
    force: bool,

    /// Generate MCP config to stdout
    #[arg(short, long)]
    mcp_config: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Determine project root
    let project_root = cli
        .path
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    info!("OpenRustClaw Cursor Integration Setup");
    info!("Project root: {:?}", project_root);

    // Generate MCP config only
    if cli.mcp_config {
        let config = generate_mcp_config(project_root, vec!["*"]);
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }

    // Check current status
    let status = check_cursor_setup(&project_root).await;
    info!("\nCurrent Status:\n{}", status);

    if cli.check {
        // Just check and exit
        if status.all_ready {
            info!("✓ Cursor integration is properly configured!");
            std::process::exit(0);
        } else {
            warn!("✗ Cursor integration is not fully configured.");
            std::process::exit(1);
        }
    }

    // Check if already setup and not forced
    if status.all_ready && !cli.force {
        info!("\nCursor integration is already configured.");
        info!("Use --force to reconfigure.");
        return Ok(());
    }

    // Perform setup
    info!("\nSetting up Cursor integration...");
    
    match setup_cursor_integration(project_root.clone()).await {
        Ok(_) => {
            info!("✓ Setup complete!");
            info!("\nNext steps:");
            info!("1. Open Cursor IDE");
            info!("2. Go to Settings > MCP");
            info!("3. Verify 'openrustclaw' and 'openrustclaw-cursor' servers are enabled");
            info!("4. Restart Cursor if needed");
            
            // Check status again
            let new_status = check_cursor_setup(&project_root).await;
            if new_status.all_ready {
                info!("\n✓ All configuration files are in place!");
            } else {
                warn!("\n⚠ Some configuration files may be missing.");
            }
        }
        Err(e) => {
            error!("✗ Setup failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
