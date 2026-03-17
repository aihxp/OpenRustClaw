//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use openrustclaw_core::error::{Error as CoreError, McpError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::future::Future;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::{
    CoreMemoryStore as CoreMemoryStoreTrait, MemoryStore as MemoryStoreTrait,
};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{MemoryEntry, MemoryQuery, MemoryType, SourceType};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore, init_pool, run_migrations};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::{LangBridgeClient, sidecar::SidecarManager};
use openrustclaw_memory::MemoryPolicies;
use openrustclaw_mcp::server::{McpServer, McpServerConfig, McpServerTool};
use openrustclaw_scheduler::{SchedulerWorker, worker::SchedulerConfig as WorkerSchedulerConfig};
use openrustclaw_security::OriginValidator;
use sqlx::Row;
use uuid::Uuid;

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
pub async fn run_mcp_server(transport: &str, config_path: &str) -> Result<()> {
    if transport != "stdio" {
        return Err(anyhow::anyhow!(
            "Unsupported MCP transport '{}'; only 'stdio' is currently implemented",
            transport
        ));
    }

    info!("Starting MCP server (stdio transport)");
    let workspace_root =
        std::env::current_dir().context("Failed to determine current directory")?;
    let config = AppConfig::load_from(config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;
    let pool = init_pool(&config.database.url, 4)
        .await
        .context("Failed to initialize database for MCP server")?;
    run_migrations(&pool)
        .await
        .context("Failed to run database migrations for MCP server")?;
    let server = build_mcp_server(workspace_root.clone(), pool);

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

fn build_mcp_server(workspace_root: PathBuf, pool: sqlx::SqlitePool) -> McpServer {
    let memory_store = SqliteMemoryStore::new(pool.clone());
    let core_memory_store = SqliteCoreMemoryStore::new(pool.clone());
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
            McpServerTool {
                name: "search_memory".to_string(),
                description: "Search persisted recall memory for a user namespace.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "string"},
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "required": ["user_id", "query"]
                }),
            },
            McpServerTool {
                name: "store_memory".to_string(),
                description: "Persist a recall memory entry for a user namespace.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "string"},
                        "content": {"type": "string"},
                        "category": {"type": "string", "enum": ["semantic", "episodic", "procedural"]},
                        "importance": {"type": "number"},
                        "session_id": {"type": "string"},
                        "source": {"type": "string"}
                    },
                    "required": ["user_id", "content"]
                }),
            },
            McpServerTool {
                name: "render_core_memory".to_string(),
                description: "Render the formatted core memory block for a user.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "string"}
                    },
                    "required": ["user_id"]
                }),
            },
            McpServerTool {
                name: "set_core_memory".to_string(),
                description: "Set a named core-memory entry for a user.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "string"},
                        "key": {"type": "string"},
                        "value": {"type": "string"},
                        "importance": {"type": "number"}
                    },
                    "required": ["user_id", "key", "value"]
                }),
            },
            McpServerTool {
                name: "list_scheduled_jobs".to_string(),
                description: "List scheduled jobs persisted in SQLite.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "state": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "create_scheduled_job".to_string(),
                description: "Create a persisted scheduled job for a workflow.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "workflow_id": {"type": "string"},
                        "description": {"type": "string"},
                        "interval_seconds": {"type": "integer", "minimum": 1},
                        "run_at": {"type": "string", "description": "RFC3339 timestamp"},
                        "payload": {"type": "object"},
                        "workflow_metadata": {"type": "object"},
                        "timezone": {"type": "string"},
                        "max_retries": {"type": "integer", "minimum": 0}
                    },
                    "required": ["name", "workflow_id"]
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

    let memory_store_for_search = memory_store.clone();
    server.register_handler("search_memory", move |args| {
        let request: McpMemorySearchArgs = parse_tool_args(args)?;
        let memory_store = memory_store_for_search.clone();
        block_on_tool(async move {
            let query = MemoryQuery {
                text: request.query,
                memory_types: vec![],
                source_types: vec![],
                namespace: Some(request.user_id),
                limit: request.limit.unwrap_or(5).max(1),
                min_confidence: 0.0,
                recency_weight: 0.0,
            };
            let results = memory_store.search(&query).await?;
            Ok(serde_json::json!({
                "memories": results.into_iter().map(|scored| serde_json::json!({
                    "id": scored.entry.id,
                    "content": scored.entry.content,
                    "score": scored.score,
                    "importance": scored.entry.importance,
                    "created_at": scored.entry.created_at,
                })).collect::<Vec<_>>()
            }))
        })
    });

    let memory_store_for_store = memory_store.clone();
    server.register_handler("store_memory", move |args| {
        let request: McpStoreMemoryArgs = parse_tool_args(args)?;
        let memory_store = memory_store_for_store.clone();
        block_on_tool(async move {
            let category = request
                .category
                .unwrap_or_else(|| "semantic".to_string())
                .to_lowercase();
            let memory_type = match category.as_str() {
                "episodic" => MemoryType::Episodic,
                "procedural" => MemoryType::Procedural,
                _ => MemoryType::Semantic,
            };

            let entry = MemoryEntry {
                id: Uuid::new_v4(),
                memory_type,
                content_hash: MemoryPolicies::content_hash(&request.content),
                content: request.content,
                source: request
                    .source
                    .or_else(|| Some("mcp_server".to_string())),
                source_type: Some(SourceType::Conversation),
                session_id: request
                    .session_id
                    .as_deref()
                    .and_then(|value| Uuid::parse_str(value).ok()),
                user_id: Some(request.user_id.clone()),
                namespace: request.user_id,
                importance: request.importance.unwrap_or(0.7).clamp(0.0, 1.0),
                confidence: 1.0,
                access_count: 0,
                last_accessed: None,
                created_at: Utc::now(),
                expires_at: None,
                metadata: serde_json::json!({
                    "source": "mcp_server",
                    "category": category,
                }),
            };
            let id = entry.id;
            memory_store.store(entry).await?;
            Ok(serde_json::json!({
                "stored": true,
                "id": id,
            }))
        })
    });

    let core_memory_for_render = core_memory_store.clone();
    server.register_handler("render_core_memory", move |args| {
        let request: McpRenderCoreMemoryArgs = parse_tool_args(args)?;
        let core_memory_store = core_memory_for_render.clone();
        block_on_tool(async move {
            let content = core_memory_store.render(&request.user_id).await?;
            Ok(serde_json::json!({ "content": content }))
        })
    });

    let core_memory_for_set = core_memory_store.clone();
    server.register_handler("set_core_memory", move |args| {
        let request: McpSetCoreMemoryArgs = parse_tool_args(args)?;
        let core_memory_store = core_memory_for_set.clone();
        block_on_tool(async move {
            let entry = openrustclaw_db::CoreEntryBuilder::new(&request.key, &request.value)
                .importance(request.importance.unwrap_or(0.8).clamp(0.0, 1.0))
                .build();
            core_memory_store.set(&request.user_id, entry).await?;
            Ok(serde_json::json!({
                "stored": true,
                "user_id": request.user_id,
                "key": request.key,
            }))
        })
    });

    let pool_for_list_jobs = pool.clone();
    server.register_handler("list_scheduled_jobs", move |args| {
        let request: McpListScheduledJobsArgs = parse_tool_args(args)?;
        let pool = pool_for_list_jobs.clone();
        block_on_tool(async move {
            let mut sql = String::from(
                r#"
                SELECT id, name, description, workflow_id, trigger_type, trigger_config,
                       state, timezone, max_retries, next_run_at, last_run_at,
                       run_count, consecutive_failures, metadata, created_at
                FROM scheduled_jobs
                "#,
            );
            let state = request.state.unwrap_or_default();
            if state.is_empty() {
                sql.push_str(" ORDER BY next_run_at ASC, created_at DESC LIMIT ?");
            } else {
                sql.push_str(" WHERE state = ? ORDER BY next_run_at ASC, created_at DESC LIMIT ?");
            }

            let limit = request.limit.unwrap_or(20).max(1) as i64;
            let mut query = sqlx::query(&sql);
            if !state.is_empty() {
                query = query.bind(state);
            }
            query = query.bind(limit);

            let rows = query.fetch_all(&pool).await.map_err(|e| {
                CoreError::Mcp(McpError::ToolExecution(format!(
                    "Failed to list scheduled jobs: {}",
                    e
                )))
            })?;

            let jobs = rows
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "id": row.get::<String, _>("id"),
                        "name": row.get::<String, _>("name"),
                        "description": row.get::<Option<String>, _>("description"),
                        "workflow_id": row.get::<String, _>("workflow_id"),
                        "trigger_type": row.get::<String, _>("trigger_type"),
                        "trigger_config": parse_json_column(row.get::<String, _>("trigger_config")),
                        "state": row.get::<String, _>("state"),
                        "timezone": row.get::<String, _>("timezone"),
                        "max_retries": row.get::<i64, _>("max_retries"),
                        "next_run_at": row.get::<Option<String>, _>("next_run_at"),
                        "last_run_at": row.get::<Option<String>, _>("last_run_at"),
                        "run_count": row.get::<i64, _>("run_count"),
                        "consecutive_failures": row.get::<i64, _>("consecutive_failures"),
                        "metadata": parse_json_column(row.get::<String, _>("metadata")),
                        "created_at": row.get::<String, _>("created_at"),
                    })
                })
                .collect::<Vec<_>>();

            Ok(serde_json::json!({ "jobs": jobs }))
        })
    });

    let pool_for_create_jobs = pool;
    server.register_handler("create_scheduled_job", move |args| {
        let request: McpCreateScheduledJobArgs = parse_tool_args(args)?;
        let pool = pool_for_create_jobs.clone();
        block_on_tool(async move {
            if request.interval_seconds.is_some() && request.run_at.is_some() {
                return Err(mcp_tool_error(
                    "Use either interval_seconds or run_at, not both",
                ));
            }

            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM scheduled_jobs WHERE name = ?")
                    .bind(&request.name)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| {
                        CoreError::Mcp(McpError::ToolExecution(format!(
                            "Failed to check for existing job: {}",
                            e
                        )))
                    })?;

            if existing.is_some() {
                return Err(mcp_tool_error(format!(
                    "A job named '{}' already exists",
                    request.name
                )));
            }

            let id = Uuid::new_v4().to_string();
            let idempotency_key = format!("{}:{}", id, Uuid::new_v4());
            let timezone = request.timezone.unwrap_or_else(|| "UTC".to_string());
            let payload = request.payload.unwrap_or_else(|| serde_json::json!({}));
            let workflow_metadata = request
                .workflow_metadata
                .unwrap_or_else(|| serde_json::json!({}));

            let (trigger_type, trigger_config, next_run_at) = if let Some(run_at) = request.run_at {
                let parsed = parse_mcp_timestamp(&run_at)?;
                (
                    "absolute",
                    serde_json::json!({
                        "type": "absolute",
                        "run_at": parsed.to_rfc3339(),
                    }),
                    parsed,
                )
            } else {
                let interval_seconds = request.interval_seconds.unwrap_or(3600).max(1);
                (
                    "interval",
                    serde_json::json!({
                        "type": "interval",
                        "interval_secs": interval_seconds,
                    }),
                    Utc::now() + chrono::Duration::seconds(interval_seconds as i64),
                )
            };

            let metadata = serde_json::json!({
                "input": payload,
                "workflow_metadata": workflow_metadata,
            });

            sqlx::query(
                r#"
                INSERT INTO scheduled_jobs (
                    id, name, description, workflow_id, trigger_type, trigger_config,
                    idempotency_key, state, timezone, max_retries, next_run_at,
                    run_count, metadata, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&id)
            .bind(&request.name)
            .bind(
                request.description.unwrap_or_else(|| {
                    format!("Auto-created job for workflow {}", request.workflow_id)
                }),
            )
            .bind(&request.workflow_id)
            .bind(trigger_type)
            .bind(trigger_config.to_string())
            .bind(&idempotency_key)
            .bind("active")
            .bind(&timezone)
            .bind(request.max_retries.unwrap_or(3) as i64)
            .bind(next_run_at.to_rfc3339())
            .bind(0i64)
            .bind(metadata.to_string())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .map_err(|e| {
                CoreError::Mcp(McpError::ToolExecution(format!(
                    "Failed to create scheduled job: {}",
                    e
                )))
            })?;

            Ok(serde_json::json!({
                "created": true,
                "job": {
                    "id": id,
                    "name": request.name,
                    "workflow_id": request.workflow_id,
                    "trigger_type": trigger_type,
                    "next_run_at": next_run_at.to_rfc3339(),
                    "timezone": timezone,
                }
            }))
        })
    });

    server
}

fn block_on_tool<F>(future: F) -> openrustclaw_core::error::Result<serde_json::Value>
where
    F: Future<Output = openrustclaw_core::error::Result<serde_json::Value>> + Send + 'static,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn mcp_tool_error(message: impl Into<String>) -> CoreError {
    CoreError::Mcp(McpError::ToolExecution(message.into()))
}

fn parse_tool_args<T: serde::de::DeserializeOwned>(
    args: serde_json::Value,
) -> openrustclaw_core::error::Result<T> {
    serde_json::from_value(args).map_err(|e| mcp_tool_error(format!("Invalid tool arguments: {}", e)))
}

fn parse_json_column(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!(raw))
}

fn parse_mcp_timestamp(raw: &str) -> openrustclaw_core::error::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| mcp_tool_error(format!("Invalid RFC3339 timestamp '{}': {}", raw, e)))
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

#[derive(serde::Deserialize)]
struct McpMemorySearchArgs {
    user_id: String,
    query: String,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpStoreMemoryArgs {
    user_id: String,
    content: String,
    category: Option<String>,
    importance: Option<f32>,
    session_id: Option<String>,
    source: Option<String>,
}

#[derive(serde::Deserialize)]
struct McpRenderCoreMemoryArgs {
    user_id: String,
}

#[derive(serde::Deserialize)]
struct McpSetCoreMemoryArgs {
    user_id: String,
    key: String,
    value: String,
    importance: Option<f32>,
}

#[derive(serde::Deserialize, Default)]
struct McpListScheduledJobsArgs {
    state: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpCreateScheduledJobArgs {
    name: String,
    workflow_id: String,
    description: Option<String>,
    interval_seconds: Option<u64>,
    run_at: Option<String>,
    payload: Option<serde_json::Value>,
    workflow_metadata: Option<serde_json::Value>,
    timezone: Option<String>,
    max_retries: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_memory_tools_persist_and_render() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-memory.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(workspace.path().to_path_buf(), pool);

        let store_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "store_memory",
                "arguments": {
                    "user_id": "user-1",
                    "content": "prefers Rust over Python",
                    "importance": 0.9
                }
            }
        });
        let store_resp = server.handle_request(&store_req);
        let store_text = store_resp["result"]["content"][0]["text"].as_str().unwrap();
        let store_payload: serde_json::Value = serde_json::from_str(store_text).unwrap();
        assert_eq!(store_payload["stored"], true);

        let search_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "search_memory",
                "arguments": {
                    "user_id": "user-1",
                    "query": "Rust",
                    "limit": 3
                }
            }
        });
        let search_resp = server.handle_request(&search_req);
        let search_text = search_resp["result"]["content"][0]["text"].as_str().unwrap();
        let search_payload: serde_json::Value = serde_json::from_str(search_text).unwrap();
        assert_eq!(search_payload["memories"].as_array().unwrap().len(), 1);

        let set_core_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "set_core_memory",
                "arguments": {
                    "user_id": "user-1",
                    "key": "language_preference",
                    "value": "Rust",
                    "importance": 0.9
                }
            }
        });
        let set_core_resp = server.handle_request(&set_core_req);
        let set_core_text = set_core_resp["result"]["content"][0]["text"].as_str().unwrap();
        let set_core_payload: serde_json::Value = serde_json::from_str(set_core_text).unwrap();
        assert_eq!(set_core_payload["stored"], true);

        let render_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "render_core_memory",
                "arguments": {
                    "user_id": "user-1"
                }
            }
        });
        let render_resp = server.handle_request(&render_req);
        let render_text = render_resp["result"]["content"][0]["text"].as_str().unwrap();
        let render_payload: serde_json::Value = serde_json::from_str(render_text).unwrap();
        assert!(render_payload["content"]
            .as_str()
            .unwrap()
            .contains("language_preference"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_scheduler_tools_create_and_list_jobs() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-scheduler.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(workspace.path().to_path_buf(), pool);

        let create_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "create_scheduled_job",
                "arguments": {
                    "name": "nightly-summary",
                    "workflow_id": "scheduler",
                    "interval_seconds": 600,
                    "payload": {"thread_id": "abc"},
                    "workflow_metadata": {"purpose": "test"}
                }
            }
        });
        let create_resp = server.handle_request(&create_req);
        let create_text = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let create_payload: serde_json::Value = serde_json::from_str(create_text).unwrap();
        assert_eq!(create_payload["created"], true);

        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "list_scheduled_jobs",
                "arguments": {
                    "limit": 5
                }
            }
        });
        let list_resp = server.handle_request(&list_req);
        let list_text = list_resp["result"]["content"][0]["text"].as_str().unwrap();
        let list_payload: serde_json::Value = serde_json::from_str(list_text).unwrap();
        assert_eq!(list_payload["jobs"].as_array().unwrap().len(), 1);
        assert_eq!(list_payload["jobs"][0]["name"], "nightly-summary");
        assert_eq!(list_payload["jobs"][0]["metadata"]["workflow_metadata"]["purpose"], "test");
    }
}
