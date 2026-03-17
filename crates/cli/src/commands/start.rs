//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore, init_pool, run_migrations};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::{LangBridgeClient, sidecar::SidecarManager};
use openrustclaw_mcp::server::{McpServer, McpServerConfig, McpServerTool};
use openrustclaw_scheduler::{SchedulerWorker, worker::SchedulerConfig as WorkerSchedulerConfig};
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
                ChannelType::Matrix => {
                    config.channels.matrix.enabled = true;
                    info!("Matrix channel enabled");
                }
                ChannelType::IMessage => {
                    config.channels.imessage.enabled = true;
                    info!("iMessage channel enabled");
                }
                ChannelType::Line => {
                    config.channels.line.enabled = true;
                    info!("LINE channel enabled");
                }
                ChannelType::Viber => {
                    config.channels.viber.enabled = true;
                    info!("Viber channel enabled");
                }
                ChannelType::WeChat => {
                    config.channels.wechat.enabled = true;
                    info!("WeChat channel enabled");
                }
                ChannelType::Messenger | ChannelType::Instagram => {
                    config.channels.meta.enabled = true;
                    info!("Meta channel enabled");
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

    let internal_api_addr = internal_api_addr(&config.gateway.host, config.gateway.port);
    let internal_api_token = uuid::Uuid::new_v4().to_string();

    // Start Python sidecar if auto_start is enabled
    let mut sidecar: Option<SidecarManager> = None;
    if config.sidecar.auto_start {
        let mut manager = SidecarManager::new(
            config.sidecar.python_path.clone(),
            config.sidecar.grpc_port,
        )
        .with_env("OPENRUSTCLAW_INTERNAL_API_URL", format!("{}/internal", internal_api_addr))
        .with_env("OPENRUSTCLAW_INTERNAL_API_TOKEN", internal_api_token.clone());

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
    let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
    let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));

    // Create origin validator
    let origin_validator = Arc::new(OriginValidator::new(config.gateway.allowed_origins.clone()));

    // Build gateway state
    let gateway_state = GatewayState {
        session_manager,
        origin_validator,
        require_auth: config.security.require_auth,
        internal_api_token: Some(Arc::new(internal_api_token)),
        memory_store: Some(memory_store),
        core_memory_store: Some(core_memory_store),
    };

    // Create and start gateway server
    let gateway = GatewayServer::new(config.gateway.host.clone(), config.gateway.port);

    let app = gateway.router(gateway_state);
    let addr = gateway.addr();

    info!(addr = %addr, "Starting gateway server");

    // Initialize enabled channels
    let mut channel_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let sidecar_addr = format!("http://127.0.0.1:{}", config.sidecar.grpc_port);
    let scheduler_task = spawn_scheduler_task(
        pool.clone(),
        sidecar_addr.clone(),
        config.scheduler.clone(),
    );

    let enabled_channels = ChannelFactory::create_channels(&config.channels);
    let enabled_platforms: Vec<_> = enabled_channels
        .iter()
        .map(|channel| channel.platform())
        .collect();

    for channel in enabled_channels {
        channel_tasks.push(spawn_channel_task(channel));
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
        info!("Sidecar gRPC: {}", sidecar_addr);
    }
    info!(
        poll_interval_ms = config.scheduler.poll_interval_ms,
        "Scheduler worker enabled"
    );
    for platform in enabled_platforms {
        info!(platform = ?platform, "Channel enabled");
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
    scheduler_task.abort();

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
pub async fn run_mcp_server(transport: &str) -> Result<()> {
    if transport != "stdio" {
        return Err(anyhow::anyhow!(
            "Unsupported MCP transport '{}'; only 'stdio' is currently implemented",
            transport
        ));
    }

    info!("Starting MCP server (stdio transport)");
    let workspace_root =
        std::env::current_dir().context("Failed to determine current directory")?;
    let server = build_mcp_server(workspace_root.clone());

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();
    let mut writer = stdout;

    while let Some(line) = reader
        .next_line()
        .await
        .context("Failed to read MCP stdin")?
    {
        if line.trim().is_empty() {
            continue;
        }

        let request: serde_json::Value = serde_json::from_str(&line)
            .with_context(|| format!("Invalid MCP JSON request: {}", line))?;
        let response = server.handle_request(&request);
        let response_line =
            serde_json::to_string(&response).context("Failed to serialize MCP response")?;
        writer
            .write_all(response_line.as_bytes())
            .await
            .context("Failed to write MCP response")?;
        writer
            .write_all(b"\n")
            .await
            .context("Failed to write MCP newline")?;
        writer
            .flush()
            .await
            .context("Failed to flush MCP response")?;
    }

    Ok(())
}

fn spawn_scheduler_task(
    pool: sqlx::SqlitePool,
    sidecar_addr: String,
    scheduler_config: openrustclaw_core::config::SchedulerConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = SchedulerWorker::new(WorkerSchedulerConfig {
            poll_interval: tokio::time::Duration::from_millis(scheduler_config.poll_interval_ms),
            lease_duration: tokio::time::Duration::from_secs(
                scheduler_config.lease_duration_secs,
            ),
            base_retry_delay_secs: scheduler_config.base_retry_delay_secs,
            max_retry_delay_secs: scheduler_config.max_retry_delay_secs,
        });

        loop {
            match LangBridgeClient::connect(&sidecar_addr).await {
                Ok(mut client) => match worker.run_due_jobs_once(&pool, &mut client, 32).await {
                    Ok(runs) if !runs.is_empty() => {
                        info!(count = runs.len(), "Scheduler worker processed due jobs");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!(error = %e, "Scheduler worker failed to process jobs");
                    }
                },
                Err(e) => {
                    warn!(
                        error = %e,
                        addr = %sidecar_addr,
                        "Scheduler worker could not reach sidecar"
                    );
                }
            }

            tokio::time::sleep(worker.poll_interval()).await;
        }
    })
}

fn internal_api_addr(host: &str, port: u16) -> String {
    let loopback_host = match host {
        "0.0.0.0" | "::" => "127.0.0.1",
        other => other,
    };
    format!("http://{}:{}", loopback_host, port)
}

fn spawn_channel_task(mut channel: Box<dyn Channel>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let platform = channel.platform();
        info!(platform = ?platform, "Starting channel");
        match channel.connect().await {
            Ok(()) => {
                info!(platform = ?platform, "Channel connected");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                }
            }
            Err(e) => {
                error!(platform = ?platform, error = %e, "Failed to connect channel");
            }
        }
    })
}

fn build_mcp_server(workspace_root: PathBuf) -> McpServer {
    let mut server = McpServer::new(McpServerConfig {
        name: "openrustclaw".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tools: vec![
            McpServerTool {
                name: "health".to_string(),
                description: "Return basic OpenRustClaw workspace health".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "list_files".to_string(),
                description: "List files under the current workspace root".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "recursive": {"type": "boolean"}
                    }
                }),
            },
            McpServerTool {
                name: "read_file".to_string(),
                description: "Read a UTF-8 file from the current workspace root".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
            },
        ],
    });

    let root_for_health = workspace_root.clone();
    server.register_handler("health", move |_| {
        Ok(serde_json::json!({
            "status": "healthy",
            "workspace_root": root_for_health,
        }))
    });

    let root_for_list = workspace_root.clone();
    server.register_handler("list_files", move |args| {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let resolved = resolve_workspace_path(&root_for_list, path)?;
        let max_depth = if recursive { usize::MAX } else { 1 };
        let entries: Vec<_> = walkdir::WalkDir::new(&resolved)
            .max_depth(max_depth)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path() != resolved)
            .map(|entry| {
                serde_json::json!({
                    "path": entry.path(),
                    "is_directory": entry.file_type().is_dir(),
                })
            })
            .collect();
        Ok(serde_json::json!({
            "path": resolved,
            "entries": entries,
        }))
    });

    let root_for_read = workspace_root;
    server.register_handler("read_file", move |args| {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            openrustclaw_core::error::Error::Mcp(openrustclaw_core::error::McpError::ToolExecution(
                "Missing 'path' parameter".to_string(),
            ))
        })?;
        let resolved = resolve_workspace_path(&root_for_read, path)?;
        let content = std::fs::read_to_string(&resolved).map_err(|e| {
            openrustclaw_core::error::Error::Mcp(openrustclaw_core::error::McpError::ToolExecution(
                e.to_string(),
            ))
        })?;
        Ok(serde_json::json!({
            "path": resolved,
            "content": content,
        }))
    });

    server
}

fn resolve_workspace_path(
    workspace_root: &Path,
    path: &str,
) -> openrustclaw_core::error::Result<PathBuf> {
    let root = workspace_root.canonicalize().map_err(|e| {
        openrustclaw_core::error::Error::Mcp(openrustclaw_core::error::McpError::ToolExecution(
            e.to_string(),
        ))
    })?;
    let requested = PathBuf::from(path);
    let candidate = if requested.is_absolute() {
        requested
    } else {
        root.join(requested)
    };

    let canonical = candidate.canonicalize().map_err(|e| {
        openrustclaw_core::error::Error::Mcp(openrustclaw_core::error::McpError::ToolExecution(
            e.to_string(),
        ))
    })?;

    if canonical.starts_with(&root) {
        Ok(canonical)
    } else {
        Err(openrustclaw_core::error::Error::Mcp(
            openrustclaw_core::error::McpError::ToolExecution(
                "Path is outside workspace root".to_string(),
            ),
        ))
    }
}
