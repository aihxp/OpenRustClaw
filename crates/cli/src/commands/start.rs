//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_db::{init_pool, run_migrations};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::sidecar::SidecarManager;
use openrustclaw_security::OriginValidator;

/// Run the start command - load config, init DB, start sidecar, start gateway.
pub async fn run(config_path: &str, channels: Option<&str>) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting OpenRustClaw...");

    // Load configuration
    let mut config = AppConfig::load_from(config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    info!(config_path = %config_path, "Configuration loaded");

    // Parse and enable channels from CLI argument
    if let Some(channels_str) = channels {
        let channel_types = parse_channels_list(channels_str)
            .map_err(|e| anyhow::anyhow!("Invalid channels argument: {}", e))?;

        info!(channels = %channels_str, "Enabling channels from CLI");

        // Enable specified channels in config
        for channel_type in &channel_types {
            match channel_type {
                ChannelType::Telegram => {
                    config.channels.telegram.enabled = true;
                    info!("Telegram channel enabled");
                }
                ChannelType::Discord => {
                    config.channels.discord.enabled = true;
                    info!("Discord channel enabled");
                }
                ChannelType::Slack => {
                    config.channels.slack.enabled = true;
                    info!("Slack channel enabled");
                }
                ChannelType::WebChat => {
                    // WebChat is always enabled via gateway
                    info!("WebChat is always enabled via gateway");
                }
                ChannelType::Teams => {
                    config.channels.teams.enabled = true;
                    info!("Teams channel enabled");
                }
                ChannelType::GoogleChat => {
                    config.channels.google_chat.enabled = true;
                    info!("Google Chat channel enabled");
                }
                ChannelType::WhatsApp => {
                    config.channels.whatsapp.enabled = true;
                    info!("WhatsApp channel enabled");
                }
                ChannelType::Gmail => {
                    config.channels.gmail_pubsub.enabled = true;
                    info!("Gmail Pub/Sub channel enabled");
                }
                _ => {
                    // Other channels not yet fully implemented in CLI
                    info!(
                        "Channel {:?} not yet fully implemented in CLI",
                        channel_type
                    );
                }
            }
        }
    }

    // Ensure data directory exists
    let db_path = config.database.url.replace("sqlite://", "");
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create data directory: {:?}", parent))?;
    }

    // Initialize database pool
    let pool = init_pool(&config.database.url, config.database.max_connections)
        .await
        .context("Failed to initialize database pool")?;

    info!(url = %config.database.url, "Database pool initialized");

    // Run migrations
    run_migrations(&pool)
        .await
        .context("Failed to run database migrations")?;

    // Start Python sidecar if auto_start is enabled
    let mut sidecar: Option<SidecarManager> = None;
    if config.sidecar.auto_start {
        let mut manager =
            SidecarManager::new(config.sidecar.python_path.clone(), config.sidecar.grpc_port);

        match manager.start().await {
            Ok(()) => {
                info!(
                    grpc_port = config.sidecar.grpc_port,
                    "Python sidecar started"
                );
                sidecar = Some(manager);
            }
            Err(e) => {
                warn!(error = %e, "Failed to start Python sidecar - continuing without it");
                if !config.sidecar.restart_on_crash {
                    return Err(e.into());
                }
            }
        }
    }

    // Create session manager
    let session_manager = Arc::new(SessionManager::new());

    // Create origin validator
    let origin_validator = Arc::new(OriginValidator::new(config.gateway.allowed_origins.clone()));

    // Build gateway state
    let gateway_state = GatewayState {
        session_manager,
        origin_validator,
        require_auth: config.security.require_auth,
    };

    // Create and start gateway server
    let gateway = GatewayServer::new(config.gateway.host.clone(), config.gateway.port);

    let app = gateway.router(gateway_state);
    let addr = gateway.addr();

    info!(addr = %addr, "Starting gateway server");

    // Initialize enabled channels
    let mut channel_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // Start Telegram if enabled
    if config.channels.telegram.enabled {
        if config.channels.telegram.token.is_empty() {
            warn!("Telegram channel enabled but no token provided");
        } else {
            info!("Starting Telegram channel...");
            let telegram_config = config.channels.telegram.clone();
            let handle = tokio::spawn(async move {
                match ChannelFactory::create_telegram(telegram_config) {
                    Ok(mut channel) => {
                        if let Err(e) = channel.connect().await {
                            error!(error = %e, "Failed to connect Telegram channel");
                        } else {
                            info!("Telegram channel connected");
                            // Keep the channel alive
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to create Telegram channel");
                    }
                }
            });
            channel_tasks.push(handle);
        }
    }

    // Start Discord if enabled
    if config.channels.discord.enabled {
        if config.channels.discord.token.is_empty() {
            warn!("Discord channel enabled but no token provided");
        } else {
            info!("Starting Discord channel...");
            let discord_config = config.channels.discord.clone();
            let handle = tokio::spawn(async move {
                match ChannelFactory::create_discord(discord_config) {
                    Ok(mut channel) => {
                        if let Err(e) = channel.connect().await {
                            error!(error = %e, "Failed to connect Discord channel");
                        } else {
                            info!("Discord channel connected");
                            // Keep the channel alive
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to create Discord channel");
                    }
                }
            });
            channel_tasks.push(handle);
        }
    }

    // Start Slack if enabled
    if config.channels.slack.enabled {
        if config.channels.slack.token.is_empty() {
            warn!("Slack channel enabled but no token provided");
        } else {
            info!("Starting Slack channel...");
            let slack_config = config.channels.slack.clone();
            let handle = tokio::spawn(async move {
                match ChannelFactory::create_slack(slack_config) {
                    Ok(mut channel) => {
                        if let Err(e) = channel.connect().await {
                            error!(error = %e, "Failed to connect Slack channel");
                        } else {
                            info!("Slack channel connected");
                            // Keep the channel alive
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to create Slack channel");
                    }
                }
            });
            channel_tasks.push(handle);
        }
    }

    // Create shutdown signal handler
    let shutdown = async {
        let mut sigterm = match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(sig) => sig,
            Err(e) => {
                error!(error = %e, "Failed to create SIGTERM handler");
                return;
            }
        };
        let mut sigint = match signal::unix::signal(signal::unix::SignalKind::interrupt()) {
            Ok(sig) => sig,
            Err(e) => {
                error!(error = %e, "Failed to create SIGINT handler");
                return;
            }
        };

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM, shutting down..."),
            _ = sigint.recv() => info!("Received SIGINT, shutting down..."),
        }
    };

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("OpenRustClaw is ready!");
    info!("Gateway: http://{}", addr);
    info!("WebSocket: ws://{}/ws", addr);
    if sidecar.is_some() {
        info!(
            "Sidecar gRPC: http://127.0.0.1:{}",
            config.sidecar.grpc_port
        );
    }
    if config.channels.telegram.enabled {
        info!("Telegram: enabled");
    }
    if config.channels.discord.enabled {
        info!("Discord: enabled");
    }
    if config.channels.slack.enabled {
        info!("Slack: enabled");
    }

    // Run server with graceful shutdown
    tokio::select! {
        result = axum::serve(listener, app) => {
            result.context("Server error")?;
        }
        _ = shutdown => {
            info!("Shutdown signal received, stopping server...");
        }
    }

    // Cleanup channel tasks
    info!("Stopping channel tasks...");
    for task in channel_tasks {
        task.abort();
    }

    // Cleanup
    if let Some(mut sidecar) = sidecar {
        info!("Stopping Python sidecar...");
        if let Err(e) = sidecar.stop().await {
            error!(error = %e, "Error stopping sidecar");
        }
    }

    info!("OpenRustClaw shutdown complete");
    Ok(())
}

/// Run MCP server (stdio transport).
pub async fn run_mcp_server(_transport: &str) -> Result<()> {
    info!("Starting MCP server (stdio transport)");

    // For now, we just print a message that MCP server is available
    // Full implementation would set up stdio transport and handle JSON-RPC
    println!("MCP server is starting...");
    println!("Note: Full MCP server implementation requires integration with the McpServer");

    // TODO: Implement full MCP server with stdio transport
    // The transport needs to be adapted from the client-side StdioTransport

    Ok(())
}
