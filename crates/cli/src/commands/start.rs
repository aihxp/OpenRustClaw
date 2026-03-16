//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn, error};

use openrustclaw_core::config::AppConfig;
use openrustclaw_db::{init_pool, run_migrations};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::sidecar::SidecarManager;
use openrustclaw_security::OriginValidator;

/// Run the start command - load config, init DB, start sidecar, start gateway.
pub async fn run(config_path: &str) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting OpenRustClaw...");
    
    // Load configuration
    let config = AppConfig::load_from(config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;
    
    info!(config_path = %config_path, "Configuration loaded");
    
    // Ensure data directory exists
    let db_path = config.database.url.replace("sqlite://", "");
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        tokio::fs::create_dir_all(parent).await
            .with_context(|| format!("Failed to create data directory: {:?}", parent))?;
    }
    
    // Initialize database pool
    let pool = init_pool(&config.database.url, config.database.max_connections)
        .await
        .context("Failed to initialize database pool")?;
    
    info!(url = %config.database.url, "Database pool initialized");
    
    // Run migrations
    run_migrations(&pool).await.context("Failed to run database migrations")?;
    
    // Start Python sidecar if auto_start is enabled
    let mut sidecar: Option<SidecarManager> = None;
    if config.sidecar.auto_start {
        let mut manager = SidecarManager::new(
            config.sidecar.python_path.clone(),
            config.sidecar.grpc_port,
        );
        
        match manager.start().await {
            Ok(()) => {
                info!(grpc_port = config.sidecar.grpc_port, "Python sidecar started");
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
    let origin_validator = Arc::new(OriginValidator::new(
        config.gateway.allowed_origins.clone(),
    ));
    
    // Build gateway state
    let gateway_state = GatewayState {
        session_manager,
        origin_validator,
        require_auth: config.security.require_auth,
    };
    
    // Create and start gateway server
    let gateway = GatewayServer::new(
        config.gateway.host.clone(),
        config.gateway.port,
    );
    
    let app = gateway.router(gateway_state);
    let addr = gateway.addr();
    
    info!(addr = %addr, "Starting gateway server");
    
    // Create shutdown signal handler
    let shutdown = async {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM, shutting down..."),
            _ = sigint.recv() => info!("Received SIGINT, shutting down..."),
        }
    };
    
    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr).await
        .with_context(|| format!("Failed to bind to {}", addr))?;
    
    info!("OpenRustClaw is ready!");
    info!("Gateway: http://{}", addr);
    info!("WebSocket: ws://{}/ws", addr);
    if sidecar.is_some() {
        info!("Sidecar gRPC: http://127.0.0.1:{}", config.sidecar.grpc_port);
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
