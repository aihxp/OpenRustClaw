//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    Router,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use chrono::{DateTime, Utc};
use futures::Stream;
use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_channels::discord::DiscordInteractionsHandler;
use openrustclaw_channels::imessage::{BlueBubblesMessage, IMessageWebhookHandler};
use openrustclaw_channels::google_chat::GoogleChatWebhookHandler;
use openrustclaw_channels::slack::SlackEventHandler;
use openrustclaw_channels::teams::TeamsWebhookHandler;
use openrustclaw_core::error::{ChannelError as CoreChannelError, Error as CoreError, McpError};
use openrustclaw_core::traits::{Channel, LlmProvider};
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, Event, MemoryEntry, MemoryQuery, MemorySource,
    MemoryType, Message, OutgoingMessage, Platform, SessionType, SourceType, StreamChunk,
    ToolFormat,
};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::{AppConfig, SessionRoutingConfig, SlackMode};
use openrustclaw_core::traits::{
    CoreMemoryStore as CoreMemoryStoreTrait, MemoryStore as MemoryStoreTrait,
};
use openrustclaw_db::{
    SqliteCoreMemoryStore, SqliteMemoryStore, SqliteRagStore, SqliteSessionStore, init_pool,
    run_migrations,
};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::sidecar::SidecarManager;
use openrustclaw_mcp::server::{McpServer, McpServerConfig, McpServerTool};
use openrustclaw_memory::{MemoryPolicies, WorkspaceArtifactRegistry};
use openrustclaw_observability::LangSmithClient;
use openrustclaw_observability::langsmith::RunType;
use openrustclaw_optimization::{
    CandidateChange, CandidateRunner, CandidateRunnerConfig, EvaluationSpec, MutationPolicy,
    OptimizationStore, PromotionPolicy, TargetRegistration,
};
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider, ProviderChain,
    openrouter::RouteStrategy,
};
use openrustclaw_scheduler::{DurableEventBus, ReminderSender, RustWorkflowDispatcher};
use openrustclaw_scheduler::{SchedulerWorker, worker::SchedulerConfig as WorkerSchedulerConfig};
use openrustclaw_security::OriginValidator;
use sqlx::Row;
use uuid::Uuid;

use super::channels::{
    ChannelBindingSpec, ChannelRegistry, ChannelSendPolicy, ensure_account_manifest,
    identity_from_message, load_registry, message_bot_mentioned, resolve_root,
};
use super::control;

/// Run the start command - load config, optionally start the compatibility/experimental sidecar, and start the gateway.
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
                ChannelType::WhatsApp => {
                    config.channels.whatsapp.enabled = true;
                    info!("WhatsApp channel enabled");
                }
                ChannelType::IMessage => {
                    config.channels.imessage.enabled = true;
                    info!("iMessage channel enabled");
                }
                ChannelType::WebChat => {
                    // WebChat is always enabled via gateway
                    info!("WebChat is always enabled via gateway");
                }
                unsupported => {
                    warn!(
                        channel = %unsupported,
                        "Channel is not part of the current shipped runtime and will be ignored by `openrustclaw start`"
                    );
                }
            }
        }
    }

    gate_nonshipping_channels(&mut config.channels);

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

    // Start the optional Python sidecar only when explicitly enabled for a non-disabled role.
    let mut sidecar: Option<SidecarManager> = None;
    if config.sidecar.auto_start && !config.sidecar.is_disabled() {
        let mut manager =
            SidecarManager::new(config.sidecar.python_path.clone(), config.sidecar.grpc_port)
                .with_env(
                    "OPENRUSTCLAW_INTERNAL_API_URL",
                    format!("{}/internal", internal_api_addr),
                )
                .with_env(
                    "OPENRUSTCLAW_INTERNAL_API_TOKEN",
                    internal_api_token.clone(),
                );

        match manager.start().await {
            Ok(()) => {
                info!(
                    role = ?config.sidecar.role,
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
    } else if config.sidecar.auto_start && config.sidecar.is_disabled() {
        warn!("Python sidecar auto_start is set, but sidecar role is disabled; skipping startup");
    } else if config.sidecar.supports_experimental_lane() {
        info!(
            "Python sidecar is configured as an experimental LangGraph lane and will not be used for production compatibility dispatch unless explicitly started"
        );
    }

    // Create session manager
    let session_store = Arc::new(SqliteSessionStore::new(pool.clone()));
    let session_manager = Arc::new(SessionManager::with_store(session_store.clone()));
    let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
    let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));
    let rag_store = Arc::new(SqliteRagStore::new(pool.clone()));
    let event_bus = DurableEventBus::new(pool.clone(), 1024);

    // Create origin validator
    let origin_validator = Arc::new(OriginValidator::new(config.gateway.allowed_origins.clone()));

    // Build gateway state
    let gateway_state = GatewayState {
        session_manager: session_manager.clone(),
        origin_validator,
        require_auth: config.security.require_auth,
        internal_api_token: Some(Arc::new(internal_api_token)),
        memory_store: Some(memory_store.clone()),
        core_memory_store: Some(core_memory_store.clone()),
        rag_store: Some(rag_store),
        langsmith: gateway_langsmith_client(&config),
    };

    // Create and start gateway server
    let gateway = GatewayServer::new(config.gateway.host.clone(), config.gateway.port);

    let mut app = gateway.router(gateway_state);
    let addr = gateway.addr();

    info!(addr = %addr, "Starting gateway server");

    // Initialize enabled channels
    let mut channel_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let sidecar_addr = format!("http://127.0.0.1:{}", config.sidecar.grpc_port);
    let mut channel_config = config.channels.clone();
    let mut discord_ingress_handler = None;
    let mut slack_ingress_handler = None;
    let mut teams_ingress_handler = None;
    let mut google_chat_ingress_handler = None;
    let mut imessage_ingress_handler = None;
    let mut enabled_channels = Vec::new();
    let channel_registry = Arc::new(tokio::sync::RwLock::new(
        load_registry(resolve_root(None)?).unwrap_or_else(|error| {
            warn!(error = %error, "Failed to load file-backed channel registry; continuing with defaults");
            ChannelRegistry::default()
        }),
    ));

    if channel_config.discord.enabled {
        match ChannelFactory::create_discord(channel_config.discord.clone()) {
            Ok(discord_channel) => {
                match discord_channel.interactions_handler() {
                    Ok(handler) => {
                        discord_ingress_handler = Some(handler);
                    }
                    Err(error) => {
                        warn!(
                            error = %error,
                            "Discord Interactions ingress not enabled; channel remains outbound-only"
                        );
                    }
                }
                enabled_channels.push(Box::new(discord_channel) as Box<dyn Channel>);
                channel_config.discord.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Discord channel");
                channel_config.discord.enabled = false;
            }
        }
    }

    if channel_config.teams.enabled {
        match ChannelFactory::create_teams(channel_config.teams.clone()) {
            Ok(teams_channel) => {
                teams_ingress_handler = Some((
                    channel_config.teams.webhook_path.clone(),
                    teams_channel.webhook_handler(),
                ));
                enabled_channels.push(Box::new(teams_channel) as Box<dyn Channel>);
                channel_config.teams.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Teams channel");
                channel_config.teams.enabled = false;
            }
        }
    }

    if channel_config.slack.enabled {
        match ChannelFactory::create_slack(channel_config.slack.clone()) {
            Ok(slack_channel) => {
                if channel_config.slack.mode == SlackMode::Http {
                    slack_ingress_handler = Some(slack_channel.event_handler());
                }
                enabled_channels.push(Box::new(slack_channel) as Box<dyn Channel>);
                channel_config.slack.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Slack channel");
                channel_config.slack.enabled = false;
            }
        }
    }

    if channel_config.google_chat.enabled {
        match ChannelFactory::create_google_chat(channel_config.google_chat.clone()) {
            Ok(google_chat_channel) => {
                google_chat_ingress_handler = Some(google_chat_channel.event_handler());
                enabled_channels.push(Box::new(google_chat_channel) as Box<dyn Channel>);
                channel_config.google_chat.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Google Chat channel");
                channel_config.google_chat.enabled = false;
            }
        }
    }

    if channel_config.imessage.enabled {
        match ChannelFactory::create_imessage(channel_config.imessage.clone()) {
            Ok(imessage_channel) => {
                imessage_ingress_handler = Some(imessage_channel.webhook_handler());
                enabled_channels.push(Box::new(imessage_channel) as Box<dyn Channel>);
                channel_config.imessage.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create iMessage channel");
                channel_config.imessage.enabled = false;
            }
        }
    }

    enabled_channels.extend(ChannelFactory::create_channels(&channel_config));
    let enabled_platforms: Vec<_> = enabled_channels
        .iter()
        .map(|channel| channel.platform())
        .collect();
    let channel_agent = if enabled_channels.is_empty() {
        None
    } else {
        Some(Arc::new(build_channel_agent(
            &config,
            memory_store.clone(),
            core_memory_store.clone(),
            session_manager.clone(),
            channel_langsmith_client(&config),
            event_bus.clone(),
            channel_registry.clone(),
        )?))
    };

    let mut delivery_router = ChannelDeliveryRouter::default();
    for mut channel in enabled_channels {
        let platform = channel.platform();
        match channel.connect().await {
            Ok(()) => {
                info!(platform = ?platform, "Channel connected");
                let channel: SharedChannel = Arc::from(channel);
                delivery_router.insert(platform, channel.clone());
                channel_tasks.push(spawn_channel_task(channel, channel_agent.clone()));
            }
            Err(error) => {
                error!(platform = ?platform, error = %error, "Failed to connect channel");
            }
        }
    }

    let reminder_sender = (!delivery_router.is_empty())
        .then_some(Arc::new(delivery_router) as Arc<dyn ReminderSender>);
    let scheduler_task = spawn_scheduler_task(
        pool.clone(),
        config.clone(),
        sidecar_addr.clone(),
        config.scheduler.clone(),
        scheduler_langsmith_client(&config),
        event_bus.clone(),
        reminder_sender,
    );

    if let Some(handler) = slack_ingress_handler {
        app = app.merge(slack_ingress_router(
            handler,
            channel_langsmith_client(&config),
        ));
        info!("Slack HTTP ingress enabled at /webhooks/slack/events");
    }
    if let Some(handler) = discord_ingress_handler {
        app = app.merge(discord_ingress_router(
            handler,
            channel_langsmith_client(&config),
        ));
        info!("Discord Interactions ingress enabled at /webhooks/discord/interactions");
    }
    if let Some((webhook_path, handler)) = teams_ingress_handler {
        app = app.merge(teams_ingress_router(webhook_path.as_str(), handler));
        info!(path = %webhook_path, "Teams ingress enabled");
    }
    if let Some(handler) = google_chat_ingress_handler {
        app = app.merge(google_chat_ingress_router(handler));
        info!("Google Chat ingress enabled at /webhooks/google-chat/events");
    }
    if let Some(handler) = imessage_ingress_handler {
        app = app.merge(imessage_ingress_router(handler));
        info!("iMessage BlueBubbles ingress enabled at /webhooks/imessage/bluebubbles");
    }
    app = app.merge(channel_registry_router(channel_registry));

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
        info!(role = ?config.sidecar.role, "Sidecar gRPC: {}", sidecar_addr);
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

fn gate_nonshipping_channels(config: &mut openrustclaw_core::config::ChannelsConfig) {
    if config.teams.enabled {
        warn!("Teams is on a partial shipped path; deeper parity is still incomplete");
    }
    if config.gmail_pubsub.enabled {
        warn!("Gmail Pub/Sub is currently gated and will not be started by `openrustclaw start`");
        config.gmail_pubsub.enabled = false;
    }
    if config.matrix.enabled {
        warn!("Matrix is on a partial shipped path; deeper parity is still incomplete");
    }
    if config.line.enabled {
        warn!("LINE is currently gated and will not be started by `openrustclaw start`");
        config.line.enabled = false;
    }
    if config.viber.enabled {
        warn!("Viber is currently gated and will not be started by `openrustclaw start`");
        config.viber.enabled = false;
    }
    if config.wechat.enabled {
        warn!("WeChat is currently gated and will not be started by `openrustclaw start`");
        config.wechat.enabled = false;
    }
    if config.meta.enabled {
        warn!("Meta channels are currently gated and will not be started by `openrustclaw start`");
        config.meta.enabled = false;
    }
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
    let server = build_mcp_server(
        workspace_root.clone(),
        pool,
        config.clone(),
        mcp_langsmith_client(&config),
    );

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
    app_config: AppConfig,
    sidecar_addr: String,
    scheduler_config: openrustclaw_core::config::SchedulerConfig,
    langsmith_client: Option<LangSmithClient>,
    event_bus: DurableEventBus,
    reminder_sender: Option<Arc<dyn ReminderSender>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut worker = SchedulerWorker::new(WorkerSchedulerConfig {
            poll_interval: tokio::time::Duration::from_millis(scheduler_config.poll_interval_ms),
            lease_duration: tokio::time::Duration::from_secs(scheduler_config.lease_duration_secs),
            base_retry_delay_secs: scheduler_config.base_retry_delay_secs,
            max_retry_delay_secs: scheduler_config.max_retry_delay_secs,
        });
        if let Some(client) = langsmith_client {
            worker = worker.with_langsmith(client);
        }
        worker = worker.with_event_bus(event_bus.clone());

        let compat_sidecar_addr = app_config
            .sidecar
            .supports_compat_dispatch()
            .then_some(sidecar_addr);
        let mut dispatcher = match RustWorkflowDispatcher::from_config(
            pool.clone(),
            &app_config,
            compat_sidecar_addr,
            reminder_sender,
            Some(event_bus.clone()),
        ) {
            Ok(dispatcher) => dispatcher,
            Err(error) => {
                error!(error = %error, "Failed to initialize Rust workflow dispatcher");
                return;
            }
        };

        loop {
            match worker.run_due_jobs_once(&pool, &mut dispatcher, 32).await {
                Ok(runs) if !runs.is_empty() => {
                    info!(count = runs.len(), "Scheduler worker processed due jobs");
                }
                Ok(_) => {}
                Err(e) => {
                    error!(error = %e, "Scheduler worker failed to process timed jobs");
                }
            }

            match worker
                .run_due_event_dispatches_once(&pool, &mut dispatcher, 32)
                .await
            {
                Ok(runs) if !runs.is_empty() => {
                    info!(
                        count = runs.len(),
                        "Scheduler worker processed queued event dispatches"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    error!(error = %e, "Scheduler worker failed to process queued events");
                }
            }

            tokio::time::sleep(worker.poll_interval()).await;
        }
    })
}

fn scheduler_langsmith_client(config: &AppConfig) -> Option<LangSmithClient> {
    if !config.observability.langsmith_enabled {
        return None;
    }

    let client = LangSmithClient::from_env(Some("openrustclaw-scheduler".to_string()));
    if client.is_enabled() {
        Some(client)
    } else {
        warn!(
            "LangSmith tracing is enabled in config, but no LANGSMITH_API_KEY/LANGCHAIN_API_KEY was found for scheduler tracing"
        );
        None
    }
}

fn channel_langsmith_client(config: &AppConfig) -> Option<LangSmithClient> {
    if !config.observability.langsmith_enabled {
        return None;
    }

    let client = LangSmithClient::from_env(Some("openrustclaw-channels".to_string()));
    if client.is_enabled() {
        Some(client)
    } else {
        warn!(
            "LangSmith tracing is enabled in config, but no LANGSMITH_API_KEY/LANGCHAIN_API_KEY was found for channel tracing"
        );
        None
    }
}

fn gateway_langsmith_client(config: &AppConfig) -> Option<LangSmithClient> {
    if !config.observability.langsmith_enabled {
        return None;
    }

    let client = LangSmithClient::from_env(Some("openrustclaw-gateway".to_string()));
    if client.is_enabled() {
        Some(client)
    } else {
        warn!(
            "LangSmith tracing is enabled in config, but no LANGSMITH_API_KEY/LANGCHAIN_API_KEY was found for gateway tracing"
        );
        None
    }
}

fn mcp_langsmith_client(config: &AppConfig) -> Option<LangSmithClient> {
    if !config.observability.langsmith_enabled {
        return None;
    }

    let client = LangSmithClient::from_env(Some("openrustclaw-mcp".to_string()));
    if client.is_enabled() {
        Some(client)
    } else {
        warn!(
            "LangSmith tracing is enabled in config, but no LANGSMITH_API_KEY/LANGCHAIN_API_KEY was found for MCP tracing"
        );
        None
    }
}

fn internal_api_addr(host: &str, port: u16) -> String {
    let loopback_host = match host {
        "0.0.0.0" | "::" => "127.0.0.1",
        other => other,
    };
    format!("http://{}:{}", loopback_host, port)
}

fn spawn_channel_task(
    channel: SharedChannel,
    channel_agent: Option<Arc<ChannelAgent>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let platform = channel.platform();
        let mut route_sessions = HashMap::new();
        info!(platform = ?platform, "Starting channel receive loop");
        loop {
            match channel.receive().await {
                Ok(message) => {
                    info!(
                        platform = ?platform,
                        user_id = %message.user_id,
                        session_id = %message.session_id,
                        content = %message.content,
                        "Channel received incoming message"
                    );
                    if let Some(agent) = channel_agent.as_ref() {
                        match agent
                            .handle_incoming_message(&mut route_sessions, message)
                            .await
                        {
                            Ok(Some(reply)) => {
                                for outbound in expand_outgoing_message(reply) {
                                    if let Err(e) = channel.send(outbound.clone()).await {
                                        error!(
                                            platform = ?platform,
                                            error = %e,
                                            "Failed to send channel reply"
                                        );
                                        break;
                                    }
                                    let delay_ms = outbound
                                        .metadata
                                        .get("claw_send_policy")
                                        .and_then(|value| value.get("chunk_delay_ms"))
                                        .and_then(|value| value.as_u64())
                                        .unwrap_or(0);
                                    if delay_ms > 0 {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            delay_ms,
                                        ))
                                        .await;
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                error!(
                                    platform = ?platform,
                                    error = %e,
                                    "Failed to process inbound channel message"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(platform = ?platform, error = %e, "Channel receive failed");
                    break;
                }
            }
        }

        if let Some(agent) = channel_agent.as_ref() {
            agent
                .close_route_sessions(&mut route_sessions, platform, "channel_receive_ended")
                .await;
        }
    })
}

#[derive(Clone)]
struct ChannelAgent {
    runtime: Arc<AgentRuntime>,
    session_manager: Arc<SessionManager>,
    core_memory_store: Option<Arc<dyn CoreMemoryStoreTrait>>,
    max_history_messages: usize,
    session_routing: SessionRoutingConfig,
    channel_registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
    langsmith: Option<LangSmithClient>,
    event_bus: DurableEventBus,
}

struct ChannelConversationState {
    session_id: Uuid,
    history: Vec<Message>,
    reply_metadata: serde_json::Value,
    workspace_id: Option<String>,
    agent_id: Option<String>,
    send_policy: ChannelSendPolicy,
}

#[derive(Debug, Clone)]
struct ChannelRouteDecision {
    route_key: String,
    session_type: SessionType,
    workspace_id: Option<String>,
    agent_id: Option<String>,
    activation_mode: String,
    send_policy: ChannelSendPolicy,
    account_id: String,
    binding_id: Option<String>,
}

type SharedChannel = Arc<dyn Channel>;

#[derive(Clone, Default)]
struct ChannelDeliveryRouter {
    channels: HashMap<Platform, SharedChannel>,
}

impl ChannelDeliveryRouter {
    fn insert(&mut self, platform: Platform, channel: SharedChannel) {
        self.channels.insert(platform, channel);
    }

    fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

#[async_trait]
impl ReminderSender for ChannelDeliveryRouter {
    async fn send(
        &self,
        platform: Platform,
        message: OutgoingMessage,
    ) -> openrustclaw_scheduler::Result<()> {
        let Some(channel) = self.channels.get(&platform) else {
            return Err(openrustclaw_core::error::SchedulerError::WorkflowFailed(
                format!(
                    "channel '{}' is not connected for reminder delivery",
                    platform
                ),
            ));
        };

        channel.send(message).await.map_err(|error| {
            openrustclaw_core::error::SchedulerError::WorkflowFailed(error.to_string())
        })
    }

    fn available_platforms(&self) -> Vec<Platform> {
        self.channels.keys().copied().collect()
    }
}

impl ChannelAgent {
    async fn close_route_sessions(
        &self,
        route_sessions: &mut HashMap<String, ChannelConversationState>,
        platform: Platform,
        reason: &str,
    ) {
        for (route_key, state) in route_sessions.drain() {
            if let Err(error) = self
                .event_bus
                .publish(Event::SessionLifecycle {
                    session_id: state.session_id,
                    hook: "session.end".to_string(),
                    metadata: serde_json::json!({
                        "platform": platform.to_string(),
                        "route_key": route_key,
                        "reason": reason,
                        "history_len": state.history.len(),
                    }),
                })
                .await
            {
                warn!(error = %error, session_id = %state.session_id, "Failed to publish session.end event");
            }
            if let Err(error) = self
                .event_bus
                .publish(Event::SessionClosed {
                    session_id: state.session_id,
                })
                .await
            {
                warn!(error = %error, session_id = %state.session_id, "Failed to publish session.closed event");
            }
            if let Err(error) = self
                .session_manager
                .remove_session(&state.session_id.to_string())
                .await
            {
                warn!(error = %error, session_id = %state.session_id, "Failed to remove channel session");
            }
        }
    }

    async fn handle_incoming_message(
        &self,
        route_sessions: &mut HashMap<String, ChannelConversationState>,
        incoming: openrustclaw_core::types::IncomingMessage,
    ) -> Result<Option<OutgoingMessage>> {
        let trimmed_content = incoming.content.trim();
        if trimmed_content.is_empty() {
            return Ok(None);
        }

        let Some(decision) = self.resolve_route_decision(&incoming).await? else {
            return Ok(None);
        };
        if message_is_group_candidate(&incoming)
            && decision.activation_mode == "mention"
            && !message_bot_mentioned(&incoming)
        {
            return Ok(None);
        }

        let route_key = decision.route_key.clone();
        if !route_sessions.contains_key(&route_key) {
            let session = self
                .session_manager
                .restore_or_create_session_with_context(
                    &incoming.user_id,
                    decision.session_type,
                    incoming.platform,
                    Some(&route_key),
                    decision.workspace_id.as_deref(),
                    Some(serde_json::json!({
                        "route_key": route_key,
                        "channel_account_id": decision.account_id,
                        "binding_id": decision.binding_id,
                        "agent_id": decision.agent_id,
                    })),
                )
                .await?;
            let restored_history = self
                .session_manager
                .list_history(&session.id.to_string(), self.max_history_messages)
                .await
                .unwrap_or_default();
            if let Err(error) = self
                .event_bus
                .publish(Event::SessionCreated {
                    session: session.clone(),
                })
                .await
            {
                warn!(error = %error, "Failed to publish session.created event");
            }
            if let Err(error) = self
                .event_bus
                .publish(Event::SessionLifecycle {
                    session_id: session.id,
                    hook: "session.start".to_string(),
                    metadata: serde_json::json!({
                        "platform": incoming.platform.to_string(),
                        "user_id": incoming.user_id,
                        "route_key": route_key,
                    }),
                })
                .await
            {
                warn!(error = %error, "Failed to publish session.start event");
            }
            route_sessions.insert(
                route_key.clone(),
                ChannelConversationState {
                    session_id: session.id,
                    history: restored_history,
                    reply_metadata: incoming.metadata.clone(),
                    workspace_id: decision.workspace_id.clone(),
                    agent_id: decision.agent_id.clone(),
                    send_policy: decision.send_policy.clone(),
                },
            );
        }

        let Some(route_state) = route_sessions.get_mut(&route_key) else {
            return Ok(None);
        };

        route_state.reply_metadata = incoming.metadata.clone();
        route_state.workspace_id = decision.workspace_id.clone();
        route_state.agent_id = decision.agent_id.clone();
        route_state.send_policy = decision.send_policy.clone();
        let inbound_message = Message::user(trimmed_content);
        if let Err(error) = self
            .event_bus
            .publish(Event::MessageReceived {
                session_id: route_state.session_id,
                message: inbound_message.clone(),
            })
            .await
        {
            warn!(error = %error, "Failed to publish message.received event");
        }
        if let Err(error) = self
            .session_manager
            .append_message(&route_state.session_id.to_string(), &inbound_message)
            .await
        {
            warn!(error = %error, "Failed to persist inbound session message");
        }
        route_state.history.push(inbound_message);
        let drained_before = trim_history(&mut route_state.history, self.max_history_messages);
        if drained_before > 0 {
            let _ = self
                .event_bus
                .publish(Event::SessionLifecycle {
                    session_id: route_state.session_id,
                    hook: "session.pre_compaction".to_string(),
                    metadata: serde_json::json!({
                        "route_key": route_key,
                        "evicted_messages": drained_before,
                    }),
                })
                .await;
            self.persist_compaction_summary(
                &incoming.user_id,
                route_state.session_id,
                &route_key,
                &route_state.history,
                drained_before,
            )
            .await;
        }
        let mut trace = self.channel_trace(&incoming, &route_state.session_id, trimmed_content);
        if let (Some(client), Some(run)) = (&self.langsmith, trace.as_ref()) {
            if let Err(error) = client.trace_run(run).await {
                warn!(
                    error = %error,
                    platform = ?incoming.platform,
                    user_id = %incoming.user_id,
                    "Failed to create LangSmith channel trace"
                );
            }
        }

        let core_memory = if let Some(store) = self.core_memory_store.as_ref() {
            match store.get_all(&incoming.user_id).await {
                Ok(entries) => entries,
                Err(error) => {
                    warn!(
                        platform = ?incoming.platform,
                        user_id = %incoming.user_id,
                        error = %error,
                        "Failed to load core memory for inbound channel message"
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let response = match self
            .runtime
            .process(
                &route_state.history,
                &core_memory,
                &route_state.session_id.to_string(),
                &incoming.user_id,
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                if let (Some(client), Some(run)) = (&self.langsmith, trace.as_mut()) {
                    run.error = Some(error.to_string());
                    run.end_time = Some(chrono::Utc::now());
                    run.extra = Some(serde_json::json!({
                        "platform": incoming.platform.to_string(),
                        "user_id": incoming.user_id,
                        "session_id": route_state.session_id,
                        "route_key": route_key,
                        "history_len": route_state.history.len(),
                        "core_memory_entries": core_memory.len(),
                        "workspace_id": route_state.workspace_id,
                        "agent_id": route_state.agent_id,
                    }));
                    if let Err(trace_error) = client.update_run(run).await {
                        warn!(error = %trace_error, "Failed to update LangSmith channel trace");
                    }
                }
                return Err(error.into());
            }
        };

        route_state.history.push(response.message.clone());
        if let Err(error) = self
            .session_manager
            .append_message(&route_state.session_id.to_string(), &response.message)
            .await
        {
            warn!(error = %error, "Failed to persist outbound session message");
        }
        let drained_after = trim_history(&mut route_state.history, self.max_history_messages);
        if drained_after > 0 {
            let _ = self
                .event_bus
                .publish(Event::SessionLifecycle {
                    session_id: route_state.session_id,
                    hook: "session.pre_compaction".to_string(),
                    metadata: serde_json::json!({
                        "route_key": route_key,
                        "evicted_messages": drained_after,
                    }),
                })
                .await;
            self.persist_compaction_summary(
                &incoming.user_id,
                route_state.session_id,
                &route_key,
                &route_state.history,
                drained_after,
            )
            .await;
        }

        if response.message.content.trim().is_empty() {
            info!(
                platform = ?incoming.platform,
                user_id = %incoming.user_id,
                session_id = %route_state.session_id,
                "Agent returned empty channel response; skipping outbound send"
            );
            if let (Some(client), Some(run)) = (&self.langsmith, trace.as_mut()) {
                run.outputs = Some(serde_json::json!({"reply": ""}));
                run.end_time = Some(chrono::Utc::now());
                run.extra = Some(serde_json::json!({
                    "platform": incoming.platform.to_string(),
                    "user_id": incoming.user_id,
                    "session_id": route_state.session_id,
                    "route_key": route_key,
                    "history_len": route_state.history.len(),
                    "core_memory_entries": core_memory.len(),
                    "empty_reply": true,
                    "workspace_id": route_state.workspace_id,
                    "agent_id": route_state.agent_id,
                }));
                if let Err(trace_error) = client.update_run(run).await {
                    warn!(error = %trace_error, "Failed to update LangSmith channel trace");
                }
            }
            return Ok(None);
        }

        if let Err(error) = self
            .event_bus
            .publish(Event::SessionLifecycle {
                session_id: route_state.session_id,
                hook: "session.post_turn".to_string(),
                metadata: serde_json::json!({
                    "platform": incoming.platform.to_string(),
                    "user_id": incoming.user_id,
                    "route_key": route_key,
                    "history_len": route_state.history.len(),
                    "reply_length": response.message.content.len(),
                    "workspace_id": route_state.workspace_id,
                    "agent_id": route_state.agent_id,
                }),
            })
            .await
        {
            warn!(error = %error, "Failed to publish session.post_turn event");
        }

        if let Err(error) = self
            .event_bus
            .publish(Event::MessageSent {
                session_id: route_state.session_id,
                message: response.message.clone(),
            })
            .await
        {
            warn!(error = %error, "Failed to publish message.sent event");
        }
        self.capture_turn_memory(
            &incoming.user_id,
            route_state.session_id,
            trimmed_content,
            &response.message.content,
        )
        .await;

        if let (Some(client), Some(run)) = (&self.langsmith, trace.as_mut()) {
            run.outputs = Some(serde_json::json!({"reply": response.message.content}));
            run.end_time = Some(chrono::Utc::now());
            run.extra = Some(serde_json::json!({
                "platform": incoming.platform.to_string(),
                "user_id": incoming.user_id,
                "session_id": route_state.session_id,
                "route_key": route_key,
                "history_len": route_state.history.len(),
                "core_memory_entries": core_memory.len(),
                "reply_length": response.message.content.len(),
                "workspace_id": route_state.workspace_id,
                "agent_id": route_state.agent_id,
            }));
            if let Err(trace_error) = client.update_run(run).await {
                warn!(error = %trace_error, "Failed to update LangSmith channel trace");
            }
        }

        Ok(Some(OutgoingMessage {
            session_id: route_state.session_id,
            content: response.message.content,
            metadata: augment_reply_metadata(route_state.reply_metadata.clone(), &decision),
        }))
    }

    fn channel_trace(
        &self,
        incoming: &openrustclaw_core::types::IncomingMessage,
        session_id: &Uuid,
        content: &str,
    ) -> Option<openrustclaw_observability::langsmith::TraceRun> {
        let client = self.langsmith.as_ref()?;
        let mut run = client.new_run(
            "channel_inbound_message",
            RunType::Chain,
            serde_json::json!({
                "platform": incoming.platform.to_string(),
                "user_id": incoming.user_id,
                "session_id": session_id,
                "content": content,
                "metadata": incoming.metadata,
            }),
        );
        run.tags = Some(vec![
            "channels".to_string(),
            format!("platform:{}", incoming.platform),
        ]);
        Some(run)
    }
}

impl ChannelAgent {
    async fn resolve_route_decision(
        &self,
        incoming: &openrustclaw_core::types::IncomingMessage,
    ) -> Result<Option<ChannelRouteDecision>> {
        let identity = identity_from_message(incoming);
        let registry_root = { self.channel_registry.read().await.root.clone() };
        let account = ensure_account_manifest(
            &registry_root,
            &identity,
            self.session_routing.pairing_approval_required,
        )?;
        {
            let mut registry = self.channel_registry.write().await;
            registry
                .accounts
                .insert(account.id.clone(), account.clone());
        }

        if account.blocked || !account.enabled {
            return Ok(None);
        }
        if !account.approved {
            let _ = self
                .event_bus
                .publish_named(
                    "channel.pairing_requested",
                    "channel_pairing",
                    None,
                    &serde_json::json!({
                        "platform": incoming.platform.to_string(),
                        "account_id": account.id,
                        "user_id": incoming.user_id,
                        "workspace_id": identity.workspace_id,
                    }),
                    None,
                )
                .await;
            return Ok(None);
        }

        let registry = self.channel_registry.read().await;
        let channel_scopes = channel_scope_candidates(
            &incoming.metadata,
            self.session_routing.thread_overrides_channel,
        );
        let binding = resolve_channel_binding(
            &registry,
            incoming.platform,
            identity.workspace_id.as_deref(),
            Some(account.id.as_str()),
            &channel_scopes,
        );

        let direct_strategy = account
            .direct_strategy
            .clone()
            .or_else(|| binding.and_then(|value| value.direct_strategy.clone()))
            .unwrap_or_else(|| self.session_routing.direct_strategy.clone());
        let group_strategy = account
            .group_strategy
            .clone()
            .or_else(|| binding.and_then(|value| value.group_strategy.clone()))
            .unwrap_or_else(|| self.session_routing.group_strategy.clone());
        let activation_mode = account
            .activation_mode
            .clone()
            .or_else(|| binding.and_then(|value| value.activation_mode.clone()))
            .unwrap_or_else(|| self.session_routing.default_group_activation.clone());
        let send_policy = account
            .send_policy
            .clone()
            .or_else(|| binding.and_then(|value| value.send_policy.clone()))
            .unwrap_or_else(|| default_send_policy(&self.session_routing));
        let workspace_id = account
            .workspace_target
            .clone()
            .or_else(|| binding.and_then(|value| value.workspace_target.clone()))
            .or_else(|| identity.workspace_id.clone());
        let agent_id = account
            .agent_id
            .clone()
            .or_else(|| binding.and_then(|value| value.agent_id.clone()));
        let session_type = if identity.is_group {
            SessionType::Group
        } else {
            SessionType::Dm
        };

        let route_key = channel_route_key_with_binding(
            incoming,
            &direct_strategy,
            &group_strategy,
            self.session_routing.thread_overrides_channel,
            workspace_id.as_deref(),
            agent_id.as_deref(),
            Some(account.id.as_str()),
        );

        Ok(Some(ChannelRouteDecision {
            route_key,
            session_type,
            workspace_id,
            agent_id,
            activation_mode,
            send_policy,
            account_id: account.id,
            binding_id: binding.map(|value| value.id.clone()),
        }))
    }

    async fn capture_turn_memory(
        &self,
        user_id: &str,
        session_id: Uuid,
        inbound: &str,
        outbound: &str,
    ) {
        let Some(store) = self.runtime.memory_store() else {
            return;
        };

        for candidate in derive_turn_memory_candidates(user_id, session_id, inbound, outbound) {
            if let Err(error) = store.store(candidate).await {
                warn!(error = %error, "Failed to auto-capture turn memory");
            }
        }
    }

    async fn persist_compaction_summary(
        &self,
        user_id: &str,
        session_id: Uuid,
        route_key: &str,
        history: &[Message],
        evicted_messages: usize,
    ) {
        let Some(store) = self.runtime.memory_store() else {
            return;
        };
        let summary = summarize_history_tail(history, evicted_messages);
        if summary.is_empty() {
            return;
        }

        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Episodic,
            content_hash: MemoryPolicies::content_hash(&summary),
            content: format!(
                "Compaction checkpoint for {} after evicting {} messages:\n{}",
                route_key, evicted_messages, summary
            ),
            source: Some("session_compaction".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: Some(session_id),
            user_id: Some(user_id.to_string()),
            namespace: user_id.to_string(),
            importance: 0.65,
            confidence: 0.8,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "source": "session_compaction",
                "route_key": route_key,
                "evicted_messages": evicted_messages,
            }),
        };

        if let Err(error) = store.store(entry).await {
            warn!(error = %error, "Failed to persist compaction summary");
        }
    }
}

fn build_channel_agent(
    config: &AppConfig,
    memory_store: Arc<SqliteMemoryStore>,
    core_memory_store: Arc<SqliteCoreMemoryStore>,
    session_manager: Arc<SessionManager>,
    langsmith: Option<LangSmithClient>,
    event_bus: DurableEventBus,
    channel_registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
) -> Result<ChannelAgent> {
    let workspace_root = std::env::current_dir()?;
    let provider = build_channel_provider(config)?;
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "OpenRustClaw".to_string(),
        memory_store,
        core_memory_store.clone(),
    )
    .with_workspace_path(workspace_root);

    Ok(ChannelAgent {
        runtime: Arc::new(runtime),
        session_manager,
        core_memory_store: Some(core_memory_store),
        max_history_messages: 24,
        session_routing: config.session_routing.clone(),
        channel_registry,
        langsmith,
        event_bus,
    })
}

fn build_channel_provider(config: &AppConfig) -> Result<Arc<dyn LlmProvider>> {
    let mut provider_names = Vec::new();
    provider_names.push(config.providers.default_provider.clone());
    provider_names.extend(config.providers.fallback_chain.clone());

    let mut seen = HashSet::new();
    let mut providers = Vec::new();

    for provider_name in provider_names {
        if !seen.insert(provider_name.clone()) {
            continue;
        }

        match create_provider_from_config(&provider_name, config) {
            Ok(provider) => providers.push(provider),
            Err(error) if providers.is_empty() => {
                return Err(error).with_context(|| {
                    format!("Failed to initialize default provider '{}'", provider_name)
                });
            }
            Err(error) => {
                warn!(
                    provider = %provider_name,
                    error = %error,
                    "Skipping fallback provider that could not be initialized"
                );
            }
        }
    }

    if providers.is_empty() {
        anyhow::bail!("No channel providers could be initialized from configuration");
    }

    if providers.len() == 1 {
        return Ok(providers.remove(0));
    }

    let primary = providers[0].clone();
    Ok(Arc::new(ChannelProviderChain::new(providers, primary)))
}

fn create_provider_from_config(
    provider_name: &str,
    config: &AppConfig,
) -> Result<Arc<dyn LlmProvider>> {
    match provider_name.to_lowercase().as_str() {
        "anthropic" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY environment variable not set")?;
            let provider =
                AnthropicProvider::new(api_key, config.providers.anthropic.model.clone());
            Ok(Arc::new(provider))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY environment variable not set")?;
            let provider = OpenAiProvider::new(api_key, config.providers.openai.model.clone());
            Ok(Arc::new(provider))
        }
        "openrouter" => {
            let api_key = std::env::var("OPENROUTER_API_KEY")
                .context("OPENROUTER_API_KEY environment variable not set")?;
            let strategy = match config.providers.openrouter.route_strategy.as_str() {
                "price" => RouteStrategy::Price,
                "throughput" => RouteStrategy::Throughput,
                "web_search" | "online" => RouteStrategy::WebSearch,
                _ => RouteStrategy::Quality,
            };
            let provider = OpenRouterProvider::with_strategy(
                api_key,
                "anthropic/claude-sonnet-4".to_string(),
                strategy,
            );
            Ok(Arc::new(provider))
        }
        "ollama" => {
            let provider = OllamaProvider::with_base_url(
                config.providers.ollama.model.clone(),
                config.providers.ollama.base_url.clone(),
            );
            Ok(Arc::new(provider))
        }
        _ => anyhow::bail!(
            "Unknown provider '{}'. Available: anthropic, openai, openrouter, ollama",
            provider_name
        ),
    }
}

struct ChannelProviderChain {
    chain: ProviderChain,
    primary: Arc<dyn LlmProvider>,
}

impl ChannelProviderChain {
    fn new(providers: Vec<Arc<dyn LlmProvider>>, primary: Arc<dyn LlmProvider>) -> Self {
        Self {
            chain: ProviderChain::new(providers),
            primary,
        }
    }
}

#[async_trait]
impl LlmProvider for ChannelProviderChain {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> openrustclaw_core::error::Result<CompletionResponse> {
        self.chain.complete(request).await
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> openrustclaw_core::error::Result<
        Pin<Box<dyn Stream<Item = openrustclaw_core::error::Result<StreamChunk>> + Send>>,
    > {
        Err(openrustclaw_core::error::Error::Provider(
            openrustclaw_core::error::ProviderError::StreamError {
                provider: self.primary.provider_name().to_string(),
                message: "Streaming is not implemented for provider chains".to_string(),
            },
        ))
    }

    fn model_id(&self) -> &str {
        self.primary.model_id()
    }

    fn max_tokens(&self) -> usize {
        self.primary.max_tokens()
    }

    fn provider_name(&self) -> &str {
        self.primary.provider_name()
    }

    fn supports_strict_tools(&self) -> bool {
        self.primary.supports_strict_tools()
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        self.primary.native_tool_format()
    }
}

#[allow(dead_code)]
fn infer_session_type(
    message: &openrustclaw_core::types::IncomingMessage,
    policy: &SessionRoutingConfig,
) -> SessionType {
    if message_is_group_candidate(message)
        || channel_scope_from_metadata(&message.metadata, policy.thread_overrides_channel).is_some()
    {
        SessionType::Group
    } else {
        SessionType::Dm
    }
}

fn message_is_group_candidate(message: &openrustclaw_core::types::IncomingMessage) -> bool {
    super::channels::message_is_group(message)
}

#[allow(dead_code)]
fn channel_route_key(
    message: &openrustclaw_core::types::IncomingMessage,
    policy: &SessionRoutingConfig,
) -> String {
    channel_route_key_with_binding(
        message,
        &policy.direct_strategy,
        &policy.group_strategy,
        policy.thread_overrides_channel,
        None,
        None,
        None,
    )
}

fn channel_route_key_with_binding(
    message: &openrustclaw_core::types::IncomingMessage,
    direct_strategy: &str,
    group_strategy: &str,
    thread_overrides_channel: bool,
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
    account_id: Option<&str>,
) -> String {
    let mut prefix = vec![message.platform.to_string()];
    if let Some(workspace_id) = workspace_id {
        prefix.push(format!("workspace={workspace_id}"));
    }
    if let Some(agent_id) = agent_id {
        prefix.push(format!("agent={agent_id}"));
    }
    if let Some(account_id) = account_id {
        prefix.push(format!("account={account_id}"));
    }

    if let Some(scope) = channel_scope_from_metadata(&message.metadata, thread_overrides_channel) {
        if group_strategy == "shared_channel" {
            prefix.push(scope);
            prefix.push("shared".to_string());
            return prefix.join(":");
        }
        prefix.push(scope);
        prefix.push(message.user_id.clone());
        return prefix.join(":");
    }

    match direct_strategy {
        "shared_main" => {
            prefix.push("main".to_string());
            prefix.join(":")
        }
        _ => {
            prefix.push("direct".to_string());
            prefix.push(message.user_id.clone());
            prefix.join(":")
        }
    }
}

fn resolve_channel_binding<'a>(
    registry: &'a ChannelRegistry,
    platform: Platform,
    workspace_id: Option<&str>,
    account_id: Option<&str>,
    channel_scopes: &[String],
) -> Option<&'a ChannelBindingSpec> {
    registry
        .bindings
        .iter()
        .filter(|binding| binding.enabled && binding.platform == platform.to_string())
        .filter(|binding| {
            binding
                .workspace_match
                .as_deref()
                .map(|value| workspace_id == Some(value))
                .unwrap_or(true)
        })
        .filter(|binding| {
            binding
                .account_match
                .as_deref()
                .map(|value| account_id == Some(value))
                .unwrap_or(true)
        })
        .filter(|binding| {
            binding
                .channel_match
                .as_deref()
                .map(|value| channel_scopes.iter().any(|scope| scope == value))
                .unwrap_or(true)
        })
        .max_by_key(|binding| {
            let specificity = usize::from(binding.workspace_match.is_some())
                + usize::from(binding.account_match.is_some())
                + usize::from(binding.channel_match.is_some());
            (specificity, -(binding.priority as isize))
        })
}

fn channel_scope_candidates(
    metadata: &serde_json::Value,
    thread_overrides_channel: bool,
) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(primary) = channel_scope_from_metadata(metadata, thread_overrides_channel) {
        candidates.push(primary);
    }
    if let Some(parent) = parent_channel_scope_from_metadata(metadata)
        && !candidates.iter().any(|existing| existing == &parent)
    {
        candidates.push(parent);
    }
    candidates
}

fn default_send_policy(policy: &SessionRoutingConfig) -> ChannelSendPolicy {
    ChannelSendPolicy {
        mode: policy.default_send_mode.clone(),
        max_chunk_chars: policy.default_chunk_chars,
        chunk_delay_ms: policy.default_chunk_delay_ms,
        coalesce_below_chars: Some(320),
        preview_chars: 280,
    }
}

fn augment_reply_metadata(
    mut metadata: serde_json::Value,
    decision: &ChannelRouteDecision,
) -> serde_json::Value {
    metadata["channel_account_id"] = serde_json::json!(decision.account_id);
    if let Some(binding_id) = &decision.binding_id {
        metadata["channel_binding_id"] = serde_json::json!(binding_id);
    }
    if let Some(workspace_id) = &decision.workspace_id {
        metadata["workspace_id"] = serde_json::json!(workspace_id);
    }
    if let Some(agent_id) = &decision.agent_id {
        metadata["agent_id"] = serde_json::json!(agent_id);
    }
    metadata["claw_send_policy"] = serde_json::to_value(&decision.send_policy).unwrap_or_default();
    metadata
}

fn expand_outgoing_message(message: OutgoingMessage) -> Vec<OutgoingMessage> {
    let policy: ChannelSendPolicy = message
        .metadata
        .get("claw_send_policy")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();

    if message.content.chars().count() <= policy.max_chunk_chars || policy.mode == "single" {
        return vec![message];
    }

    let preview = if policy.mode == "preview_then_blocks" {
        let preview: String = message.content.chars().take(policy.preview_chars).collect();
        Some(format!("{preview}…"))
    } else {
        None
    };
    let mut chunks = split_message_blocks(&message.content, policy.max_chunk_chars);
    if let Some(limit) = policy.coalesce_below_chars
        && chunks.len() >= 2
        && chunks[0].chars().count() + chunks[1].chars().count() <= limit
    {
        let second = chunks.remove(1);
        chunks[0] = format!("{}\n\n{}", chunks[0], second);
    }

    let total = chunks.len();
    let mut expanded: Vec<OutgoingMessage> = chunks
        .into_iter()
        .enumerate()
        .map(|(index, content)| {
            let mut metadata = message.metadata.clone();
            metadata["reply_part"] = serde_json::json!(index + 1);
            metadata["reply_parts_total"] = serde_json::json!(total);
            OutgoingMessage {
                session_id: message.session_id,
                content,
                metadata,
            }
        })
        .collect();

    if let Some(preview) = preview {
        let mut metadata = message.metadata.clone();
        metadata["reply_preview"] = serde_json::json!(true);
        expanded.insert(
            0,
            OutgoingMessage {
                session_id: message.session_id,
                content: preview,
                metadata,
            },
        );
    }

    expanded
}

fn split_message_blocks(content: &str, max_chunk_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for paragraph in content.split("\n\n") {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        let addition = if current.is_empty() {
            paragraph.to_string()
        } else {
            format!("{}\n\n{}", current, paragraph)
        };
        if addition.chars().count() <= max_chunk_chars {
            current = addition;
            continue;
        }
        if !current.is_empty() {
            chunks.push(current);
            current = String::new();
        }
        let mut buffer = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if buffer.is_empty() {
                word.to_string()
            } else {
                format!("{buffer} {word}")
            };
            if candidate.chars().count() > max_chunk_chars && !buffer.is_empty() {
                chunks.push(buffer);
                buffer = word.to_string();
            } else {
                buffer = candidate;
            }
        }
        if !buffer.is_empty() {
            current = buffer;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(content.to_string());
    }
    chunks
}

fn channel_scope_from_metadata(
    metadata: &serde_json::Value,
    thread_overrides_channel: bool,
) -> Option<String> {
    let primary_keys: &[&str] = if thread_overrides_channel {
        &[
            "slack_thread_ts",
            "slack_channel",
            "telegram_chat_id",
            "discord_thread_id",
            "discord_channel_id",
            "google_chat_thread",
            "google_chat_space",
            "teams_conversation_id",
            "matrix_room_id",
            "whatsapp_chat_id",
            "line_room_id",
            "meta_thread_id",
        ]
    } else {
        &[
            "slack_channel",
            "telegram_chat_id",
            "discord_channel_id",
            "google_chat_space",
            "teams_conversation_id",
            "matrix_room_id",
            "whatsapp_chat_id",
            "line_room_id",
            "meta_thread_id",
        ]
    };

    for key in primary_keys {
        if let Some(value) = metadata.get(*key) {
            if let Some(text) = value.as_str() {
                return Some(format!("{}={}", key, text));
            }
            if let Some(number) = value.as_i64() {
                return Some(format!("{}={}", key, number));
            }
            if let Some(number) = value.as_u64() {
                return Some(format!("{}={}", key, number));
            }
        }
    }

    None
}

fn parent_channel_scope_from_metadata(metadata: &serde_json::Value) -> Option<String> {
    for key in ["discord_parent_channel_id", "slack_channel"] {
        if let Some(value) = metadata.get(key) {
            if let Some(text) = value.as_str() {
                return Some(format!("{}={}", key, text));
            }
            if let Some(number) = value.as_i64() {
                return Some(format!("{}={}", key, number));
            }
            if let Some(number) = value.as_u64() {
                return Some(format!("{}={}", key, number));
            }
        }
    }
    None
}

fn trim_history(history: &mut Vec<Message>, max_history_messages: usize) -> usize {
    if history.len() > max_history_messages {
        let drain_count = history.len() - max_history_messages;
        history.drain(0..drain_count);
        drain_count
    } else {
        0
    }
}

fn summarize_history_tail(history: &[Message], evicted_messages: usize) -> String {
    if history.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for message in history.iter().rev().take(6).rev() {
        lines.push(format!("{}: {}", message.role, message.content.trim()));
    }
    format!(
        "Evicted {} earlier messages. Recent retained context:\n{}",
        evicted_messages,
        lines.join("\n")
    )
}

fn derive_turn_memory_candidates(
    user_id: &str,
    session_id: Uuid,
    inbound: &str,
    outbound: &str,
) -> Vec<MemoryEntry> {
    let normalized = inbound.trim();
    let mut candidates = Vec::new();

    let interesting_prefixes = [
        "remember that ",
        "my name is ",
        "i prefer ",
        "we use ",
        "the project is ",
        "always ",
    ];
    let lower = normalized.to_lowercase();
    if interesting_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
        || lower.contains("please remember")
    {
        candidates.push(MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content_hash: MemoryPolicies::content_hash(normalized),
            content: normalized.to_string(),
            source: Some("turn_memory_capture".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: Some(session_id),
            user_id: Some(user_id.to_string()),
            namespace: user_id.to_string(),
            importance: 0.9,
            confidence: 0.95,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "source": "turn_memory_capture",
                "reply_preview": outbound.chars().take(120).collect::<String>(),
            }),
        });
    }

    if lower.starts_with("error")
        || lower.contains("failed")
        || outbound.to_lowercase().contains("error")
    {
        let content = format!(
            "User turn:\n{}\n\nAssistant reply:\n{}",
            normalized, outbound
        );
        candidates.push(MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Episodic,
            content_hash: MemoryPolicies::content_hash(&content),
            content,
            source: Some("error_ledger".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: Some(session_id),
            user_id: Some(user_id.to_string()),
            namespace: format!("{}_errors", user_id),
            importance: 0.75,
            confidence: 0.75,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "source": "error_ledger",
            }),
        });
    }

    candidates
}

#[derive(Clone)]
struct SlackIngressState {
    handler: Arc<SlackEventHandler>,
    langsmith: Option<LangSmithClient>,
}

fn slack_ingress_router(handler: SlackEventHandler, langsmith: Option<LangSmithClient>) -> Router {
    Router::new()
        .route("/webhooks/slack/events", post(slack_events_handler))
        .with_state(SlackIngressState {
            handler: Arc::new(handler),
            langsmith,
        })
}

async fn slack_events_handler(
    State(state): State<SlackIngressState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let timestamp = headers
        .get("x-slack-request-timestamp")
        .and_then(|value| value.to_str().ok());
    let signature = headers
        .get("x-slack-signature")
        .and_then(|value| value.to_str().ok());
    let mut trace = ingress_trace(
        state.langsmith.as_ref(),
        "slack_ingress",
        serde_json::json!({
            "body_bytes": body.len(),
            "has_timestamp": timestamp.is_some(),
            "has_signature": signature.is_some(),
        }),
    );
    start_ingress_trace(state.langsmith.as_ref(), trace.as_ref()).await;

    match state
        .handler
        .handle_event(&body, timestamp, signature)
        .await
    {
        Ok(Some(response)) => {
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::OK.as_u16(),
                    "outcome": "url_verification",
                })),
                None,
            )
            .await;
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Ok(None) => {
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::OK.as_u16(),
                    "outcome": "accepted",
                })),
                None,
            )
            .await;
            StatusCode::OK.into_response()
        }
        Err(CoreError::Channel(CoreChannelError::AuthFailed { message, .. })) => {
            warn!(error = %message, "Rejected Slack webhook due to failed auth");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::UNAUTHORIZED.as_u16(),
                    "outcome": "auth_failed",
                })),
                Some(message.clone()),
            )
            .await;
            (StatusCode::UNAUTHORIZED, message).into_response()
        }
        Err(CoreError::Channel(CoreChannelError::PermissionDenied { message, .. })) => {
            warn!(error = %message, "Rejected Slack webhook due to permission check");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::FORBIDDEN.as_u16(),
                    "outcome": "permission_denied",
                })),
                Some(message.clone()),
            )
            .await;
            (StatusCode::FORBIDDEN, message).into_response()
        }
        Err(error) => {
            warn!(error = %error, "Failed to process Slack webhook");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::BAD_REQUEST.as_u16(),
                    "outcome": "error",
                })),
                Some(error.to_string()),
            )
            .await;
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct DiscordIngressState {
    handler: Arc<DiscordInteractionsHandler>,
    langsmith: Option<LangSmithClient>,
}

#[derive(Clone)]
struct ChannelRegistryApiState {
    registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
}

fn discord_ingress_router(
    handler: DiscordInteractionsHandler,
    langsmith: Option<LangSmithClient>,
) -> Router {
    Router::new()
        .route(
            "/webhooks/discord/interactions",
            post(discord_interactions_handler),
        )
        .with_state(DiscordIngressState {
            handler: Arc::new(handler),
            langsmith,
        })
}

fn channel_registry_router(registry: Arc<tokio::sync::RwLock<ChannelRegistry>>) -> Router {
    Router::new()
        .route("/control/channels", get(channel_registry_index_handler))
        .route(
            "/control/channels/accounts/{id}",
            get(channel_registry_account_handler),
        )
        .route(
            "/control/channels/accounts/{id}/approve",
            post(channel_registry_approve_handler),
        )
        .route(
            "/control/channels/accounts/{id}/block",
            post(channel_registry_block_handler),
        )
        .route(
            "/control/channels/accounts/{id}/activation",
            post(channel_registry_activation_handler),
        )
        .route("/control/channels/bindings", post(channel_registry_bind_handler))
        .with_state(ChannelRegistryApiState { registry })
}

async fn channel_registry_index_handler(
    State(state): State<ChannelRegistryApiState>,
) -> Json<serde_json::Value> {
    let registry = state.registry.read().await;
    let mut accounts: Vec<_> = registry.accounts.values().cloned().collect();
    accounts.sort_by(|left, right| left.id.cmp(&right.id));
    Json(serde_json::json!({
        "root": registry.root.clone(),
        "accounts": accounts,
        "bindings": registry.bindings.clone(),
    }))
}

async fn channel_registry_account_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let registry = state.registry.read().await;
    match registry.accounts.get(&id) {
        Some(account) => (StatusCode::OK, Json(serde_json::json!(account))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "account not found"})))
            .into_response(),
    }
}

async fn channel_registry_approve_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    mutate_channel_registry_account(state.registry, &id, "approve", None).await
}

async fn channel_registry_block_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    mutate_channel_registry_account(state.registry, &id, "block", None).await
}

#[derive(serde::Deserialize)]
struct ChannelActivationRequest {
    mode: String,
}

async fn channel_registry_activation_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<ChannelActivationRequest>,
) -> impl IntoResponse {
    mutate_channel_registry_account(state.registry, &id, "activation", Some(payload.mode)).await
}

#[derive(serde::Deserialize)]
struct ChannelBindingRequest {
    id: String,
    platform: String,
    workspace_match: Option<String>,
    account_match: Option<String>,
    channel_match: Option<String>,
    workspace_target: Option<String>,
    agent_id: Option<String>,
    activation_mode: Option<String>,
}

async fn channel_registry_bind_handler(
    State(state): State<ChannelRegistryApiState>,
    Json(payload): Json<ChannelBindingRequest>,
) -> impl IntoResponse {
    let root = {
        state
            .registry
            .read()
            .await
            .root
            .to_string_lossy()
            .to_string()
    };
    match super::channels::bind(
        Some(&root),
        &payload.id,
        &payload.platform,
        payload.workspace_match.as_deref(),
        payload.account_match.as_deref(),
        payload.channel_match.as_deref(),
        payload.workspace_target.as_deref(),
        payload.agent_id.as_deref(),
        payload.activation_mode.as_deref(),
    ) {
        Ok(()) => match reload_channel_registry(&state.registry).await {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "binding_id": payload.id})),
            )
                .into_response(),
            Err(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response(),
        },
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mutate_channel_registry_account(
    registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
    id: &str,
    action: &str,
    mode: Option<String>,
) -> axum::response::Response {
    let root = { registry.read().await.root.to_string_lossy().to_string() };
    let result = match action {
        "approve" => super::channels::approve(Some(&root), id),
        "block" => super::channels::block(Some(&root), id),
        "activation" => {
            super::channels::activation(Some(&root), id, mode.as_deref().unwrap_or(""))
        }
        _ => Err(anyhow::anyhow!("unsupported channel registry action")),
    };

    match result {
        Ok(()) => match reload_channel_registry(&registry).await {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "account_id": id, "action": action})),
            )
                .into_response(),
            Err(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response(),
        },
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn reload_channel_registry(
    registry: &Arc<tokio::sync::RwLock<ChannelRegistry>>,
) -> Result<()> {
    let root = { registry.read().await.root.clone() };
    let refreshed = load_registry(root)?;
    *registry.write().await = refreshed;
    Ok(())
}

#[derive(Clone)]
struct IMessageIngressState {
    handler: Arc<IMessageWebhookHandler>,
}

fn imessage_ingress_router(handler: IMessageWebhookHandler) -> Router {
    Router::new()
        .route("/webhooks/imessage/bluebubbles", post(imessage_bluebubbles_handler))
        .with_state(IMessageIngressState {
            handler: Arc::new(handler),
        })
}

async fn imessage_bluebubbles_handler(
    State(state): State<IMessageIngressState>,
    Json(payload): Json<BlueBubblesMessage>,
) -> impl IntoResponse {
    match state.handler.handle_event(payload).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Clone)]
struct TeamsIngressState {
    handler: Arc<TeamsWebhookHandler>,
}

fn teams_ingress_router(path: &str, handler: TeamsWebhookHandler) -> Router {
    Router::new()
        .route(path, post(teams_events_handler))
        .with_state(TeamsIngressState {
            handler: Arc::new(handler),
        })
}

async fn teams_events_handler(
    State(state): State<TeamsIngressState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        match state.handler.verify_token(token).await {
            Ok(true) => {}
            Ok(false) => {
                return (StatusCode::UNAUTHORIZED, "Invalid Teams auth token").into_response();
            }
            Err(error) => {
                return (StatusCode::UNAUTHORIZED, error.to_string()).into_response();
            }
        }
    }

    match state.handler.handle_request(payload).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

#[derive(Clone)]
struct GoogleChatIngressState {
    handler: Arc<GoogleChatWebhookHandler>,
}

fn google_chat_ingress_router(handler: GoogleChatWebhookHandler) -> Router {
    Router::new()
        .route("/webhooks/google-chat/events", post(google_chat_events_handler))
        .with_state(GoogleChatIngressState {
            handler: Arc::new(handler),
        })
}

async fn google_chat_events_handler(
    State(state): State<GoogleChatIngressState>,
    body: Bytes,
) -> impl IntoResponse {
    match state.handler.handle_event(&body).await {
        Ok(Some(response)) => (StatusCode::OK, Json(response)).into_response(),
        Ok(None) => StatusCode::OK.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

async fn discord_interactions_handler(
    State(state): State<DiscordIngressState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let signature = headers
        .get("x-signature-ed25519")
        .and_then(|value| value.to_str().ok());
    let timestamp = headers
        .get("x-signature-timestamp")
        .and_then(|value| value.to_str().ok());
    let mut trace = ingress_trace(
        state.langsmith.as_ref(),
        "discord_interactions_ingress",
        serde_json::json!({
            "body_bytes": body.len(),
            "has_signature": signature.is_some(),
            "has_timestamp": timestamp.is_some(),
        }),
    );
    start_ingress_trace(state.langsmith.as_ref(), trace.as_ref()).await;

    match state
        .handler
        .handle_event(&body, signature, timestamp)
        .await
    {
        Ok(response) => {
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::OK.as_u16(),
                    "outcome": "accepted",
                    "response_type": response.get("type").and_then(|value| value.as_u64()),
                })),
                None,
            )
            .await;
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Err(CoreError::Channel(CoreChannelError::AuthFailed { message, .. })) => {
            warn!(error = %message, "Rejected Discord interaction due to failed auth");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::UNAUTHORIZED.as_u16(),
                    "outcome": "auth_failed",
                })),
                Some(message.clone()),
            )
            .await;
            (StatusCode::UNAUTHORIZED, message).into_response()
        }
        Err(CoreError::Channel(CoreChannelError::PermissionDenied { message, .. })) => {
            warn!(error = %message, "Rejected Discord interaction due to permission check");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::FORBIDDEN.as_u16(),
                    "outcome": "permission_denied",
                })),
                Some(message.clone()),
            )
            .await;
            (StatusCode::FORBIDDEN, message).into_response()
        }
        Err(error) => {
            warn!(error = %error, "Failed to process Discord interaction");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::BAD_REQUEST.as_u16(),
                    "outcome": "error",
                })),
                Some(error.to_string()),
            )
            .await;
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

fn ingress_trace(
    client: Option<&LangSmithClient>,
    name: &str,
    inputs: serde_json::Value,
) -> Option<openrustclaw_observability::langsmith::TraceRun> {
    let mut run = client?.new_run(name, RunType::Tool, inputs);
    run.tags = Some(vec!["channels".to_string(), "ingress".to_string()]);
    Some(run)
}

async fn start_ingress_trace(
    client: Option<&LangSmithClient>,
    trace: Option<&openrustclaw_observability::langsmith::TraceRun>,
) {
    let (Some(client), Some(trace)) = (client, trace) else {
        return;
    };

    if let Err(trace_error) = client.trace_run(trace).await {
        warn!(error = %trace_error, trace_name = %trace.name, "Failed to create LangSmith ingress trace");
    }
}

async fn complete_ingress_trace(
    client: Option<&LangSmithClient>,
    trace: Option<&mut openrustclaw_observability::langsmith::TraceRun>,
    outputs: Option<serde_json::Value>,
    error: Option<String>,
) {
    let (Some(client), Some(trace)) = (client, trace) else {
        return;
    };

    trace.outputs = outputs;
    trace.error = error;
    trace.end_time = Some(chrono::Utc::now());
    if let Err(trace_error) = client.update_run(trace).await {
        warn!(error = %trace_error, trace_name = %trace.name, "Failed to update LangSmith ingress trace");
    }
}

fn build_mcp_server(
    workspace_root: PathBuf,
    pool: sqlx::SqlitePool,
    config: AppConfig,
    langsmith: Option<LangSmithClient>,
) -> McpServer {
    let memory_store = SqliteMemoryStore::new(pool.clone());
    let core_memory_store = SqliteCoreMemoryStore::new(pool.clone());
    let rag_store = SqliteRagStore::new(pool.clone());
    let session_store = SqliteSessionStore::new(pool.clone());
    let optimization_store = OptimizationStore::new(pool.clone());
    let event_bus = DurableEventBus::new(pool.clone(), 256);
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
                name: "get_memory".to_string(),
                description: "Fetch one persisted memory entry by id.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "list_memory_namespaces".to_string(),
                description: "List distinct recall-memory namespaces.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "memory_timeline".to_string(),
                description: "Inspect recent persisted memory entries.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "list_memory_archive".to_string(),
                description: "Inspect archive summaries for a namespace.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "list_sessions".to_string(),
                description: "List durable sessions and their statuses.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "inspect_session".to_string(),
                description: "Inspect one session plus persisted history.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "history_limit": {"type": "integer", "minimum": 1}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "spawn_session".to_string(),
                description: "Create a durable operator session.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "string"},
                        "platform": {"type": "string"},
                        "session_type": {"type": "string"},
                        "route_key": {"type": "string"},
                        "workspace_id": {"type": "string"}
                    },
                    "required": ["user_id"]
                }),
            },
            McpServerTool {
                name: "send_to_session".to_string(),
                description: "Run a message turn against a persisted session.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "content": {"type": "string"}
                    },
                    "required": ["id", "content"]
                }),
            },
            McpServerTool {
                name: "close_session".to_string(),
                description: "Close or archive a durable session.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "archive": {"type": "boolean"},
                        "reason": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "scan_workspace_artifacts".to_string(),
                description: "Scan model-aware workspace artifact files.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "render_workspace_artifacts".to_string(),
                description: "Render the merged instruction bundle for a model.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "model": {"type": "string"}
                    },
                    "required": ["model"]
                }),
            },
            McpServerTool {
                name: "sync_workspace_artifacts".to_string(),
                description: "Sync preferred model-family artifact files.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "model": {"type": "string"}
                    },
                    "required": ["model"]
                }),
            },
            McpServerTool {
                name: "list_agent_profiles".to_string(),
                description: "List file-backed agent profiles from .claw/control.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "inspect_agent_profile".to_string(),
                description: "Inspect one file-backed agent profile.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "list_model_profiles".to_string(),
                description: "List file-backed model profiles from .claw/control.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "inspect_model_profile".to_string(),
                description: "Inspect one file-backed model profile.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "list_claws".to_string(),
                description: "List configured Claws and their bindings from .claw/control."
                    .to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "inspect_claw".to_string(),
                description: "Inspect one configured Claw.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "inspect_runtime_mode".to_string(),
                description: "Inspect the current solo/task/category/orchestrated runtime mode."
                    .to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "self_describe_runtime".to_string(),
                description:
                    "Return a machine-readable self-description of the current control plane."
                        .to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
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
                        "max_retries": {"type": "integer", "minimum": 0},
                        "priority": {"type": "integer"}
                    },
                    "required": ["name", "workflow_id"]
                }),
            },
            McpServerTool {
                name: "inspect_scheduled_job".to_string(),
                description:
                    "Inspect one scheduled job/task including priority and manifest metadata."
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "reprioritize_scheduled_job".to_string(),
                description: "Change a scheduled job/task priority (lower numbers run first)."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "priority": {"type": "integer"}
                    },
                    "required": ["id", "priority"]
                }),
            },
            McpServerTool {
                name: "list_workflows".to_string(),
                description: "List the unified workflow registry and execution tiers.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "list_job_runs".to_string(),
                description: "Inspect recent workflow/job attempts persisted by the scheduler."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "job": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "list_dead_letters".to_string(),
                description: "Inspect dead-letter queue entries for failed workflow runs."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "job": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "replay_dead_letter".to_string(),
                description: "Resolve and replay a dead-letter entry by id.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
            McpServerTool {
                name: "list_runtime_events".to_string(),
                description: "Inspect durable runtime events and their processing status."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "event_name": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "list_rag_collections".to_string(),
                description: "List durable RAG collections and their stored stats.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "load_rag_chunks".to_string(),
                description: "Load stored chunks from a durable RAG collection.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "collection_name": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "required": ["collection_name"]
                }),
            },
            McpServerTool {
                name: "list_optimization_targets".to_string(),
                description: "List registered optimization targets.".to_string(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            McpServerTool {
                name: "register_optimization_target".to_string(),
                description: "Register a bounded optimization target with policy and eval suite."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "description": {"type": "string"},
                        "target_kind": {"type": "string"},
                        "execution_tier": {"type": "string"},
                        "risk_class": {"type": "string"},
                        "ship_status": {"type": "string"},
                        "workspace_root": {"type": "string"},
                        "mutation_policy": {"type": "object"},
                        "eval_suite": {"type": "array", "items": {"type": "object"}},
                        "promotion_policy": {"type": "object"},
                        "metadata": {"type": "object"}
                    },
                    "required": ["name", "target_kind", "execution_tier"]
                }),
            },
            McpServerTool {
                name: "submit_optimization_candidate".to_string(),
                description: "Submit a candidate change-set for an optimization target."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {"type": "string"},
                        "hypothesis": {"type": "string"},
                        "proposed_by": {"type": "string"},
                        "changes": {"type": "array", "items": {"type": "object"}},
                        "trace_id": {"type": "string"}
                    },
                    "required": ["target", "hypothesis", "changes"]
                }),
            },
            McpServerTool {
                name: "list_optimization_candidates".to_string(),
                description: "List optimization candidates and their statuses.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": {"type": "string"},
                        "status": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "inspect_optimization_candidate".to_string(),
                description: "Inspect one candidate plus evaluations and promotion history."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "candidate_id": {"type": "string"}
                    },
                    "required": ["candidate_id"]
                }),
            },
            McpServerTool {
                name: "run_optimization_candidate".to_string(),
                description: "Run a bounded optimization candidate in an isolated temp workspace."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "candidate_id": {"type": "string"}
                    },
                    "required": ["candidate_id"]
                }),
            },
            McpServerTool {
                name: "promote_optimization_candidate".to_string(),
                description:
                    "Record approve/reject/promote decisions for an optimization candidate."
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "candidate_id": {"type": "string"},
                        "decision": {"type": "string"},
                        "decided_by": {"type": "string"},
                        "notes": {"type": "string"},
                        "rollback_reference": {"type": "string"}
                    },
                    "required": ["candidate_id", "decision", "decided_by"]
                }),
            },
        ],
    });

    let root_for_health = workspace_root.clone();
    server.register_handler(
        "health",
        traced_mcp_handler(langsmith.clone(), "health", move |_| {
            Ok(serde_json::json!({
                "status": "healthy",
                "workspace_root": root_for_health,
            }))
        }),
    );

    let root_for_list = workspace_root.clone();
    server.register_handler(
        "list_files",
        traced_mcp_handler(langsmith.clone(), "list_files", move |args| {
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
        }),
    );

    let root_for_read = workspace_root.clone();
    let root_for_opt_register = root_for_read.clone();
    server.register_handler(
        "read_file",
        traced_mcp_handler(langsmith.clone(), "read_file", move |args| {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                openrustclaw_core::error::Error::Mcp(
                    openrustclaw_core::error::McpError::ToolExecution(
                        "Missing 'path' parameter".to_string(),
                    ),
                )
            })?;
            let resolved = resolve_workspace_path(&root_for_read, path)?;
            let content = std::fs::read_to_string(&resolved).map_err(|e| {
                openrustclaw_core::error::Error::Mcp(
                    openrustclaw_core::error::McpError::ToolExecution(e.to_string()),
                )
            })?;
            Ok(serde_json::json!({
                "path": resolved,
                "content": content,
            }))
        }),
    );

    let memory_store_for_search = memory_store.clone();
    let event_bus_for_search = event_bus.clone();
    server.register_handler(
        "search_memory",
        traced_mcp_handler(langsmith.clone(), "search_memory", move |args| {
            let request: McpMemorySearchArgs = parse_tool_args(args)?;
            let memory_store = memory_store_for_search.clone();
            let event_bus = event_bus_for_search.clone();
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
                if let Err(error) = event_bus
                    .publish(Event::MemorySearched {
                        query: query.text.clone(),
                        result_count: results.len(),
                    })
                    .await
                {
                    warn!(error = %error, "Failed to publish memory.searched event");
                }
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
        }),
    );

    let memory_store_for_store = memory_store.clone();
    let event_bus_for_store = event_bus.clone();
    server.register_handler(
        "store_memory",
        traced_mcp_handler(langsmith.clone(), "store_memory", move |args| {
            let request: McpStoreMemoryArgs = parse_tool_args(args)?;
            let memory_store = memory_store_for_store.clone();
            let event_bus = event_bus_for_store.clone();
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
                    source: request.source.or_else(|| Some("mcp_server".to_string())),
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
                let memory_type = entry.memory_type;
                memory_store.store(entry).await?;
                if let Err(error) = event_bus
                    .publish(Event::MemoryStored {
                        entry_id: id,
                        memory_type,
                        source: MemorySource::ConversationSummary,
                    })
                    .await
                {
                    warn!(error = %error, "Failed to publish memory.stored event");
                }
                Ok(serde_json::json!({
                    "stored": true,
                    "id": id,
                }))
            })
        }),
    );

    let core_memory_for_render = core_memory_store.clone();
    server.register_handler(
        "render_core_memory",
        traced_mcp_handler(langsmith.clone(), "render_core_memory", move |args| {
            let request: McpRenderCoreMemoryArgs = parse_tool_args(args)?;
            let core_memory_store = core_memory_for_render.clone();
            block_on_tool(async move {
                let content = core_memory_store.render(&request.user_id).await?;
                Ok(serde_json::json!({ "content": content }))
            })
        }),
    );

    let core_memory_for_set = core_memory_store.clone();
    server.register_handler(
        "set_core_memory",
        traced_mcp_handler(langsmith.clone(), "set_core_memory", move |args| {
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
        }),
    );

    let memory_store_for_get = memory_store.clone();
    server.register_handler(
        "get_memory",
        traced_mcp_handler(langsmith.clone(), "get_memory", move |args| {
            let request: McpGetMemoryArgs = parse_tool_args(args)?;
            let memory_store = memory_store_for_get.clone();
            block_on_tool(async move {
                let entry = memory_store.get(&request.id).await?;
                Ok(serde_json::json!({ "memory": entry }))
            })
        }),
    );

    let memory_store_for_namespaces = memory_store.clone();
    server.register_handler(
        "list_memory_namespaces",
        traced_mcp_handler(langsmith.clone(), "list_memory_namespaces", move |_| {
            let memory_store = memory_store_for_namespaces.clone();
            block_on_tool(async move {
                let namespaces = memory_store.list_namespaces().await?;
                Ok(serde_json::json!({ "namespaces": namespaces }))
            })
        }),
    );

    let memory_store_for_timeline = memory_store.clone();
    server.register_handler(
        "memory_timeline",
        traced_mcp_handler(langsmith.clone(), "memory_timeline", move |args| {
            let request: McpMemoryTimelineArgs = parse_tool_args(args)?;
            let memory_store = memory_store_for_timeline.clone();
            block_on_tool(async move {
                let entries = memory_store
                    .list_recent(
                        request.namespace.as_deref(),
                        request.limit.unwrap_or(20).max(1),
                    )
                    .await?;
                Ok(serde_json::json!({ "entries": entries }))
            })
        }),
    );

    let memory_store_for_archive = memory_store.clone();
    server.register_handler(
        "list_memory_archive",
        traced_mcp_handler(langsmith.clone(), "list_memory_archive", move |args| {
            let request: McpMemoryTimelineArgs = parse_tool_args(args)?;
            let memory_store = memory_store_for_archive.clone();
            block_on_tool(async move {
                let entries = memory_store
                    .list_archive_entries(
                        request.namespace.as_deref(),
                        request.limit.unwrap_or(20).max(1),
                    )
                    .await?;
                Ok(serde_json::json!({ "entries": entries }))
            })
        }),
    );

    let session_store_for_list = session_store.clone();
    server.register_handler(
        "list_sessions",
        traced_mcp_handler(langsmith.clone(), "list_sessions", move |args| {
            let request: McpListSessionsArgs = parse_tool_args(args)?;
            let session_store = session_store_for_list.clone();
            block_on_tool(async move {
                let status = request.status.as_deref().and_then(parse_mcp_session_status);
                let sessions = session_store
                    .list_sessions(status, request.limit.unwrap_or(20).max(1))
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                Ok(serde_json::json!({ "sessions": sessions }))
            })
        }),
    );

    let session_store_for_inspect = session_store.clone();
    server.register_handler(
        "inspect_session",
        traced_mcp_handler(langsmith.clone(), "inspect_session", move |args| {
            let request: McpInspectSessionArgs = parse_tool_args(args)?;
            let session_store = session_store_for_inspect.clone();
            block_on_tool(async move {
                let session = session_store
                    .get_session(&request.id)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                let history = session_store
                    .list_history(&request.id, request.history_limit.unwrap_or(50).max(1))
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                Ok(serde_json::json!({
                    "session": session,
                    "history": history,
                }))
            })
        }),
    );

    let session_store_for_spawn = session_store.clone();
    server.register_handler(
        "spawn_session",
        traced_mcp_handler(langsmith.clone(), "spawn_session", move |args| {
            let request: McpSpawnSessionArgs = parse_tool_args(args)?;
            let session_store = session_store_for_spawn.clone();
            block_on_tool(async move {
                let mut session = openrustclaw_core::types::Session::new(
                    parse_mcp_session_type(request.session_type.as_deref().unwrap_or("dm")),
                    request.user_id,
                    parse_mcp_platform(request.platform.as_deref().unwrap_or("webchat")),
                );
                session.workspace_id = request.workspace_id;
                session.metadata = serde_json::json!({
                    "spawned_by": "mcp",
                    "route_key": request.route_key,
                });
                session_store
                    .create_or_update(
                        &session,
                        request.route_key.as_deref(),
                        openrustclaw_db::SessionStatus::Active,
                    )
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                Ok(serde_json::json!({ "session": session }))
            })
        }),
    );

    let session_store_for_send = session_store.clone();
    let pool_for_send = pool.clone();
    let workspace_for_send = workspace_root.clone();
    let config_for_send = config.clone();
    server.register_handler(
        "send_to_session",
        traced_mcp_handler(langsmith.clone(), "send_to_session", move |args| {
            let request: McpSendToSessionArgs = parse_tool_args(args)?;
            let session_store = session_store_for_send.clone();
            let pool = pool_for_send.clone();
            let workspace_root = workspace_for_send.clone();
            let config = config_for_send.clone();
            block_on_tool(async move {
                let session = session_store
                    .get_session(&request.id)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?
                    .ok_or_else(|| mcp_tool_error(format!("Session '{}' not found", request.id)))?;
                let history = session_store
                    .list_history(&request.id, 128)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                let provider =
                    build_channel_provider(&config).map_err(|e| mcp_tool_error(e.to_string()))?;
                let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
                let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));
                let runtime = AgentRuntime::with_memory_stores(
                    provider,
                    "OpenRustClaw".to_string(),
                    memory_store,
                    core_memory_store.clone(),
                )
                .with_workspace_path(workspace_root);
                let user_message = openrustclaw_core::types::Message::user(&request.content);
                session_store
                    .append_message(&request.id, &user_message)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                let mut conversation = history;
                conversation.push(user_message);
                let core_memory = core_memory_store
                    .get_all(&session.session.user_id)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                let response = runtime
                    .process(
                        &conversation,
                        &core_memory,
                        &session.session.id.to_string(),
                        &session.session.user_id,
                    )
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                session_store
                    .append_message(&request.id, &response.message)
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                Ok(serde_json::json!({ "response": response.message }))
            })
        }),
    );

    let session_store_for_close = session_store.clone();
    server.register_handler(
        "close_session",
        traced_mcp_handler(langsmith.clone(), "close_session", move |args| {
            let request: McpCloseSessionArgs = parse_tool_args(args)?;
            let session_store = session_store_for_close.clone();
            block_on_tool(async move {
                session_store
                    .set_status(
                        &request.id,
                        if request.archive.unwrap_or(false) {
                            openrustclaw_db::SessionStatus::Archived
                        } else {
                            openrustclaw_db::SessionStatus::Closed
                        },
                        request.reason.as_deref(),
                    )
                    .await
                    .map_err(|e| mcp_tool_error(e.to_string()))?;
                Ok(serde_json::json!({ "closed": true }))
            })
        }),
    );

    let workspace_for_artifacts = workspace_root.clone();
    server.register_handler(
        "scan_workspace_artifacts",
        traced_mcp_handler(langsmith.clone(), "scan_workspace_artifacts", move |_| {
            let workspace_root = workspace_for_artifacts.clone();
            Ok(serde_json::json!({
                "artifacts": WorkspaceArtifactRegistry::scan(&workspace_root)
                    .map_err(|e| mcp_tool_error(e.to_string()))?,
            }))
        }),
    );

    let workspace_for_artifact_render = workspace_root.clone();
    server.register_handler(
        "render_workspace_artifacts",
        traced_mcp_handler(
            langsmith.clone(),
            "render_workspace_artifacts",
            move |args| {
                let request: McpRenderArtifactsArgs = parse_tool_args(args)?;
                let workspace_root = workspace_for_artifact_render.clone();
                Ok(serde_json::json!(
                    WorkspaceArtifactRegistry::resolve(&workspace_root, &request.model)
                        .map_err(|e| mcp_tool_error(e.to_string()))?
                ))
            },
        ),
    );

    let workspace_for_artifact_sync = workspace_root.clone();
    server.register_handler(
        "sync_workspace_artifacts",
        traced_mcp_handler(langsmith.clone(), "sync_workspace_artifacts", move |args| {
            let request: McpRenderArtifactsArgs = parse_tool_args(args)?;
            let workspace_root = workspace_for_artifact_sync.clone();
            Ok(serde_json::json!({
                "written": WorkspaceArtifactRegistry::sync_preferred(&workspace_root, &request.model)
                    .map_err(|e| mcp_tool_error(e.to_string()))?,
            }))
        }),
    );

    let workspace_for_agent_profiles = workspace_root.clone();
    server.register_handler(
        "list_agent_profiles",
        traced_mcp_handler(langsmith.clone(), "list_agent_profiles", move |_| {
            let control_root = control::control_root_for(&workspace_for_agent_profiles);
            let description = control::describe_registry(control_root)
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::json!({
                "agent_profiles": description["agent_profiles"].clone(),
            }))
        }),
    );

    let workspace_for_agent_profile = workspace_root.clone();
    server.register_handler(
        "inspect_agent_profile",
        traced_mcp_handler(langsmith.clone(), "inspect_agent_profile", move |args| {
            let id = args
                .get("id")
                .and_then(|value| value.as_str())
                .context("Missing required field 'id'")
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            let control_root = control::control_root_for(&workspace_for_agent_profile);
            let registry =
                control::load_registry(control_root).map_err(|e| mcp_tool_error(e.to_string()))?;
            let profile = registry
                .agent_profiles
                .get(id)
                .cloned()
                .with_context(|| format!("Unknown agent profile '{id}'"))
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::to_value(profile).unwrap_or_default())
        }),
    );

    let workspace_for_model_profiles = workspace_root.clone();
    server.register_handler(
        "list_model_profiles",
        traced_mcp_handler(langsmith.clone(), "list_model_profiles", move |_| {
            let control_root = control::control_root_for(&workspace_for_model_profiles);
            let description = control::describe_registry(control_root)
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::json!({
                "model_profiles": description["model_profiles"].clone(),
            }))
        }),
    );

    let workspace_for_model_profile = workspace_root.clone();
    server.register_handler(
        "inspect_model_profile",
        traced_mcp_handler(langsmith.clone(), "inspect_model_profile", move |args| {
            let id = args
                .get("id")
                .and_then(|value| value.as_str())
                .context("Missing required field 'id'")
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            let control_root = control::control_root_for(&workspace_for_model_profile);
            let registry =
                control::load_registry(control_root).map_err(|e| mcp_tool_error(e.to_string()))?;
            let profile = registry
                .model_profiles
                .get(id)
                .cloned()
                .with_context(|| format!("Unknown model profile '{id}'"))
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::to_value(profile).unwrap_or_default())
        }),
    );

    let workspace_for_claws = workspace_root.clone();
    server.register_handler(
        "list_claws",
        traced_mcp_handler(langsmith.clone(), "list_claws", move |_| {
            let control_root = control::control_root_for(&workspace_for_claws);
            let description = control::describe_registry(control_root)
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::json!({
                "claws": description["available_claws"].clone(),
            }))
        }),
    );

    let workspace_for_claw = workspace_root.clone();
    server.register_handler(
        "inspect_claw",
        traced_mcp_handler(langsmith.clone(), "inspect_claw", move |args| {
            let id = args
                .get("id")
                .and_then(|value| value.as_str())
                .context("Missing required field 'id'")
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            let control_root = control::control_root_for(&workspace_for_claw);
            let registry =
                control::load_registry(control_root).map_err(|e| mcp_tool_error(e.to_string()))?;
            let claw = registry
                .claws
                .get(id)
                .cloned()
                .with_context(|| format!("Unknown claw '{id}'"))
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::to_value(claw).unwrap_or_default())
        }),
    );

    let workspace_for_runtime_mode = workspace_root.clone();
    server.register_handler(
        "inspect_runtime_mode",
        traced_mcp_handler(langsmith.clone(), "inspect_runtime_mode", move |_| {
            let control_root = control::control_root_for(&workspace_for_runtime_mode);
            let description = control::describe_registry(control_root)
                .map_err(|e| mcp_tool_error(e.to_string()))?;
            Ok(serde_json::json!({
                "execution_mode": description["execution_mode"].clone(),
                "default_claw": description["default_claw"].clone(),
                "orchestrator_claw": description["orchestrator_claw"].clone(),
                "allow_shared_context": description["allow_shared_context"].clone(),
                "isolation_mode": description["isolation_mode"].clone(),
                "task_assignments": description["task_assignments"].clone(),
                "category_assignments": description["category_assignments"].clone(),
            }))
        }),
    );

    let workspace_for_self_description = workspace_root.clone();
    server.register_handler(
        "self_describe_runtime",
        traced_mcp_handler(langsmith.clone(), "self_describe_runtime", move |_| {
            let control_root = control::control_root_for(&workspace_for_self_description);
            control::describe_registry(control_root).map_err(|e| mcp_tool_error(e.to_string()))
        }),
    );

    let pool_for_list_jobs = pool.clone();
    server.register_handler(
        "list_scheduled_jobs",
        traced_mcp_handler(langsmith.clone(), "list_scheduled_jobs", move |args| {
            let request: McpListScheduledJobsArgs = parse_tool_args(args)?;
            let pool = pool_for_list_jobs.clone();
            block_on_tool(async move {
                let mut sql = String::from(
                    r#"
                SELECT id, name, description, workflow_id, trigger_type, trigger_config,
                       state, timezone, max_retries, priority, source_kind, owner,
                       tags, disabled_until, manifest_path, task_notes_path,
                       next_run_at, last_run_at, run_count, consecutive_failures,
                       metadata, created_at
                FROM scheduled_jobs
                "#,
                );
                let state = request.state.unwrap_or_default();
                if state.is_empty() {
                    sql.push_str(" ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?");
                } else {
                    sql.push_str(
                        " WHERE state = ? ORDER BY priority ASC, next_run_at ASC, created_at DESC LIMIT ?",
                    );
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
                        "priority": row.get::<i64, _>("priority"),
                        "source_kind": row.get::<String, _>("source_kind"),
                        "owner": row.get::<Option<String>, _>("owner"),
                        "tags": parse_json_column(row.get::<String, _>("tags")),
                        "disabled_until": row.get::<Option<String>, _>("disabled_until"),
                        "manifest_path": row.get::<Option<String>, _>("manifest_path"),
                        "task_notes_path": row.get::<Option<String>, _>("task_notes_path"),
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
        }),
    );

    let pool_for_create_jobs = pool.clone();
    let pool_for_create_job_handler = pool_for_create_jobs.clone();
    let langsmith_for_create_jobs = langsmith.clone();
    server.register_handler(
        "create_scheduled_job",
        traced_mcp_handler(
            langsmith_for_create_jobs,
            "create_scheduled_job",
            move |args| {
                let request: McpCreateScheduledJobArgs = parse_tool_args(args)?;
                let pool = pool_for_create_job_handler.clone();
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

                    let (trigger_type, trigger_config, next_run_at) =
                        if let Some(run_at) = request.run_at {
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
                    idempotency_key, state, timezone, max_retries, priority, source_kind, tags, next_run_at,
                    run_count, metadata, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'mcp', '[]', ?, ?, ?, ?)
                "#,
                    )
                    .bind(&id)
                    .bind(&request.name)
                    .bind(request.description.unwrap_or_else(|| {
                        format!("Auto-created job for workflow {}", request.workflow_id)
                    }))
                    .bind(&request.workflow_id)
                    .bind(trigger_type)
                    .bind(trigger_config.to_string())
                    .bind(&idempotency_key)
                    .bind("active")
                    .bind(&timezone)
                    .bind(request.max_retries.unwrap_or(3) as i64)
                    .bind(request.priority.unwrap_or(100))
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
                            "priority": request.priority.unwrap_or(100),
                            "next_run_at": next_run_at.to_rfc3339(),
                            "timezone": timezone,
                        }
                    }))
                })
            },
        ),
    );

    let pool_for_inspect_job = pool_for_create_jobs.clone();
    server.register_handler(
        "inspect_scheduled_job",
        traced_mcp_handler(langsmith.clone(), "inspect_scheduled_job", move |args| {
            let request: McpInspectScheduledJobArgs = parse_tool_args(args)?;
            let pool = pool_for_inspect_job.clone();
            block_on_tool(async move {
                let row = sqlx::query(
                    r#"
                    SELECT id, name, description, workflow_id, trigger_type, trigger_config,
                           state, timezone, max_retries, priority, source_kind, owner,
                           tags, disabled_until, manifest_path, task_notes_path,
                           next_run_at, last_run_at, run_count, consecutive_failures,
                           metadata, created_at
                    FROM scheduled_jobs
                    WHERE id = ? OR name = ?
                    "#,
                )
                .bind(&request.id)
                .bind(&request.id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to inspect scheduled job: {}",
                        e
                    )))
                })?
                .ok_or_else(|| {
                    mcp_tool_error(format!("Scheduled job '{}' not found", request.id))
                })?;

                let latest_run = sqlx::query(
                    r#"
                    SELECT id, status, started_at, completed_at, retry_count, langsmith_trace_id
                    FROM job_runs
                    WHERE job_id = ?
                    ORDER BY started_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(row.get::<String, _>("id"))
                .fetch_optional(&pool)
                .await
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to load latest job run: {}",
                        e
                    )))
                })?;

                Ok(serde_json::json!({
                    "job": {
                        "id": row.get::<String, _>("id"),
                        "name": row.get::<String, _>("name"),
                        "description": row.get::<Option<String>, _>("description"),
                        "workflow_id": row.get::<String, _>("workflow_id"),
                        "trigger_type": row.get::<String, _>("trigger_type"),
                        "trigger_config": parse_json_column(row.get::<String, _>("trigger_config")),
                        "state": row.get::<String, _>("state"),
                        "timezone": row.get::<String, _>("timezone"),
                        "max_retries": row.get::<i64, _>("max_retries"),
                        "priority": row.get::<i64, _>("priority"),
                        "source_kind": row.get::<String, _>("source_kind"),
                        "owner": row.get::<Option<String>, _>("owner"),
                        "tags": parse_json_column(row.get::<String, _>("tags")),
                        "disabled_until": row.get::<Option<String>, _>("disabled_until"),
                        "manifest_path": row.get::<Option<String>, _>("manifest_path"),
                        "task_notes_path": row.get::<Option<String>, _>("task_notes_path"),
                        "next_run_at": row.get::<Option<String>, _>("next_run_at"),
                        "last_run_at": row.get::<Option<String>, _>("last_run_at"),
                        "run_count": row.get::<i64, _>("run_count"),
                        "consecutive_failures": row.get::<i64, _>("consecutive_failures"),
                        "metadata": parse_json_column(row.get::<String, _>("metadata")),
                        "created_at": row.get::<String, _>("created_at"),
                    },
                    "latest_run": latest_run.map(|run| serde_json::json!({
                        "id": run.get::<String, _>("id"),
                        "status": run.get::<String, _>("status"),
                        "started_at": run.get::<String, _>("started_at"),
                        "completed_at": run.get::<Option<String>, _>("completed_at"),
                        "retry_count": run.get::<i64, _>("retry_count"),
                        "langsmith_trace_id": run.get::<Option<String>, _>("langsmith_trace_id"),
                    }))
                }))
            })
        }),
    );

    let pool_for_reprioritize_job = pool_for_create_jobs.clone();
    server.register_handler(
        "reprioritize_scheduled_job",
        traced_mcp_handler(
            langsmith.clone(),
            "reprioritize_scheduled_job",
            move |args| {
                let request: McpReprioritizeScheduledJobArgs = parse_tool_args(args)?;
                let pool = pool_for_reprioritize_job.clone();
                block_on_tool(async move {
                    let result = sqlx::query(
                        "UPDATE scheduled_jobs SET priority = ? WHERE id = ? OR name = ?",
                    )
                    .bind(request.priority)
                    .bind(&request.id)
                    .bind(&request.id)
                    .execute(&pool)
                    .await
                    .map_err(|e| {
                        CoreError::Mcp(McpError::ToolExecution(format!(
                            "Failed to reprioritize scheduled job: {}",
                            e
                        )))
                    })?;

                    if result.rows_affected() == 0 {
                        return Err(mcp_tool_error(format!(
                            "Scheduled job '{}' not found",
                            request.id
                        )));
                    }

                    Ok(serde_json::json!({
                        "updated": true,
                        "id": request.id,
                        "priority": request.priority
                    }))
                })
            },
        ),
    );

    server.register_handler(
        "list_workflows",
        traced_mcp_handler(langsmith.clone(), "list_workflows", move |_| {
            Ok(serde_json::json!({
                "workflows": [
                    {
                        "workflow_id": "agent",
                        "tier": "rust_native",
                        "description": "Rust-native agent execution over the core runtime"
                    },
                    {
                        "workflow_id": "memory_maintenance",
                        "tier": "rust_native",
                        "description": "Rust-native episodic memory consolidation and archive maintenance"
                    },
                    {
                        "workflow_id": "rag",
                        "tier": "rust_native",
                        "description": "Rust-native RAG indexing, retrieval, and context assembly"
                    },
                    {
                        "workflow_id": "scheduler",
                        "tier": "rust_native",
                        "description": "Rust-native scheduled workflow envelope and inner workflow dispatch"
                    },
                    {
                        "workflow_id": "reminder",
                        "tier": "rust_native",
                        "description": "Rust-native reminder delivery with channel-aware fallback policies"
                    },
                    {
                        "workflow_id": "*",
                        "tier": "compat_sidecar",
                        "description": "Optional bounded compatibility bridge for legacy sidecar workflows when sidecar.role=compatibility"
                    },
                    {
                        "workflow_id": "*",
                        "tier": "experimental_langgraph",
                        "description": "Experimental LangGraph authoring/prototyping lane; not part of the production-critical runtime path"
                    }
                ]
            }))
        }),
    );

    let pool_for_job_runs = pool_for_create_jobs.clone();
    server.register_handler(
        "list_job_runs",
        traced_mcp_handler(langsmith.clone(), "list_job_runs", move |args| {
            let request: McpListRunsArgs = parse_tool_args(args)?;
            let pool = pool_for_job_runs.clone();
            block_on_tool(async move {
                let rows = if let Some(job) = request.job {
                    sqlx::query(
                        r#"
                        SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                               r.retry_count, r.langsmith_trace_id, r.result
                        FROM job_runs r
                        JOIN scheduled_jobs j ON j.id = r.job_id
                        WHERE r.job_id = ? OR j.name = ?
                        ORDER BY r.started_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(&job)
                    .bind(&job)
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                } else {
                    sqlx::query(
                        r#"
                        SELECT r.id, r.job_id, j.name, r.status, r.started_at, r.completed_at,
                               r.retry_count, r.langsmith_trace_id, r.result
                        FROM job_runs r
                        JOIN scheduled_jobs j ON j.id = r.job_id
                        ORDER BY r.started_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                }
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to list job runs: {}",
                        e
                    )))
                })?;

                Ok(serde_json::json!({
                    "runs": rows.into_iter().map(|row| serde_json::json!({
                        "id": row.get::<String, _>("id"),
                        "job_id": row.get::<String, _>("job_id"),
                        "job_name": row.get::<String, _>("name"),
                        "status": row.get::<String, _>("status"),
                        "started_at": row.get::<String, _>("started_at"),
                        "completed_at": row.get::<Option<String>, _>("completed_at"),
                        "retry_count": row.get::<i64, _>("retry_count"),
                        "langsmith_trace_id": row.get::<Option<String>, _>("langsmith_trace_id"),
                        "result": row.get::<Option<String>, _>("result"),
                    })).collect::<Vec<_>>()
                }))
            })
        }),
    );

    let pool_for_dead_letters = pool_for_create_jobs.clone();
    server.register_handler(
        "list_dead_letters",
        traced_mcp_handler(langsmith.clone(), "list_dead_letters", move |args| {
            let request: McpListRunsArgs = parse_tool_args(args)?;
            let pool = pool_for_dead_letters.clone();
            block_on_tool(async move {
                let rows = if let Some(job) = request.job {
                    sqlx::query(
                        r#"
                        SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count,
                               d.original_payload, d.resolved, d.resolved_at
                        FROM dead_letter_queue d
                        JOIN scheduled_jobs j ON j.id = d.job_id
                        WHERE d.job_id = ? OR j.name = ?
                        ORDER BY d.failed_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(&job)
                    .bind(&job)
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                } else {
                    sqlx::query(
                        r#"
                        SELECT d.id, d.job_id, j.name, d.last_error, d.failed_at, d.retry_count,
                               d.original_payload, d.resolved, d.resolved_at
                        FROM dead_letter_queue d
                        JOIN scheduled_jobs j ON j.id = d.job_id
                        ORDER BY d.failed_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                }
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to list dead letters: {}",
                        e
                    )))
                })?;

                Ok(serde_json::json!({
                    "dead_letters": rows.into_iter().map(|row| serde_json::json!({
                        "id": row.get::<String, _>("id"),
                        "job_id": row.get::<String, _>("job_id"),
                        "job_name": row.get::<String, _>("name"),
                        "last_error": row.get::<String, _>("last_error"),
                        "failed_at": row.get::<String, _>("failed_at"),
                        "retry_count": row.get::<i64, _>("retry_count"),
                        "original_payload": parse_optional_json_column(row.get::<Option<String>, _>("original_payload")),
                        "resolved": row.get::<i64, _>("resolved") == 1,
                        "resolved_at": row.get::<Option<String>, _>("resolved_at"),
                    })).collect::<Vec<_>>()
                }))
            })
        }),
    );

    let pool_for_replay_dead_letter = pool_for_create_jobs.clone();
    server.register_handler(
        "replay_dead_letter",
        traced_mcp_handler(langsmith.clone(), "replay_dead_letter", move |args| {
            let request: McpReplayDeadLetterArgs = parse_tool_args(args)?;
            let pool = pool_for_replay_dead_letter.clone();
            block_on_tool(async move {
                let row = sqlx::query("SELECT id, job_id FROM dead_letter_queue WHERE id = ?")
                    .bind(&request.id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| {
                        CoreError::Mcp(McpError::ToolExecution(format!(
                            "Failed to load dead-letter entry: {}",
                            e
                        )))
                    })?;
                let Some(row) = row else {
                    return Err(mcp_tool_error(format!(
                        "Dead-letter entry '{}' not found",
                        request.id
                    )));
                };
                let job_id: String = row.get("job_id");
                sqlx::query(
                    r#"
                    UPDATE scheduled_jobs
                    SET state = 'active',
                        consecutive_failures = 0,
                        lease_owner = NULL,
                        lease_expires_at = NULL,
                        next_run_at = COALESCE(next_run_at, ?)
                    WHERE id = ?
                    "#,
                )
                .bind((Utc::now() + chrono::Duration::minutes(1)).to_rfc3339())
                .bind(&job_id)
                .execute(&pool)
                .await
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to reset scheduled job: {}",
                        e
                    )))
                })?;
                sqlx::query(
                    "UPDATE dead_letter_queue SET resolved = 1, resolved_at = ? WHERE id = ?",
                )
                .bind(Utc::now().to_rfc3339())
                .bind(&request.id)
                .execute(&pool)
                .await
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to resolve dead-letter entry: {}",
                        e
                    )))
                })?;
                Ok(serde_json::json!({
                    "replayed": true,
                    "dead_letter_id": request.id,
                    "job_id": job_id,
                }))
            })
        }),
    );

    let pool_for_runtime_events = pool_for_create_jobs.clone();
    server.register_handler(
        "list_runtime_events",
        traced_mcp_handler(langsmith.clone(), "list_runtime_events", move |args| {
            let request: McpListRuntimeEventsArgs = parse_tool_args(args)?;
            let pool = pool_for_runtime_events.clone();
            block_on_tool(async move {
                let rows = if let Some(event_name) = request.event_name {
                    sqlx::query(
                        r#"
                        SELECT id, event_name, event_type, session_id, payload, status, created_at, processed_at
                        FROM runtime_events
                        WHERE event_name = ?
                        ORDER BY created_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(&event_name)
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                } else {
                    sqlx::query(
                        r#"
                        SELECT id, event_name, event_type, session_id, payload, status, created_at, processed_at
                        FROM runtime_events
                        ORDER BY created_at DESC
                        LIMIT ?
                        "#,
                    )
                    .bind(request.limit.unwrap_or(20).max(1) as i64)
                    .fetch_all(&pool)
                    .await
                }
                .map_err(|e| {
                    CoreError::Mcp(McpError::ToolExecution(format!(
                        "Failed to list runtime events: {}",
                        e
                    )))
                })?;

                Ok(serde_json::json!({
                    "events": rows.into_iter().map(|row| serde_json::json!({
                        "id": row.get::<String, _>("id"),
                        "event_name": row.get::<String, _>("event_name"),
                        "event_type": row.get::<String, _>("event_type"),
                        "session_id": row.get::<Option<String>, _>("session_id"),
                        "payload": parse_json_column(row.get::<String, _>("payload")),
                        "status": row.get::<String, _>("status"),
                        "created_at": row.get::<String, _>("created_at"),
                        "processed_at": row.get::<Option<String>, _>("processed_at"),
                    })).collect::<Vec<_>>()
                }))
            })
        }),
    );

    let rag_store_for_list = rag_store.clone();
    let langsmith_for_list_rag = langsmith.clone();
    server.register_handler(
        "list_rag_collections",
        traced_mcp_handler(
            langsmith_for_list_rag,
            "list_rag_collections",
            move |args| {
                let request: McpListRagCollectionsArgs = parse_tool_args(args)?;
                let rag_store = rag_store_for_list.clone();
                block_on_tool(async move {
                    let collections = rag_store.list_collection_stats(request.limit).await?;
                    Ok(serde_json::json!({
                        "collections": collections.into_iter().map(|stats| serde_json::json!({
                            "collection_name": stats.collection_name,
                            "chunk_count": stats.chunk_count,
                            "source_count": stats.source_count,
                            "total_content_bytes": stats.total_content_bytes,
                            "last_updated_at": stats.last_updated_at,
                        })).collect::<Vec<_>>()
                    }))
                })
            },
        ),
    );

    let rag_store_for_load = rag_store;
    server.register_handler(
        "load_rag_chunks",
        traced_mcp_handler(langsmith.clone(), "load_rag_chunks", move |args| {
            let request: McpLoadRagChunksArgs = parse_tool_args(args)?;
            let rag_store = rag_store_for_load.clone();
            block_on_tool(async move {
                let chunks = rag_store
                    .load_collection(&request.collection_name, request.limit)
                    .await?;
                Ok(serde_json::json!({
                    "collection_name": request.collection_name,
                    "chunks": chunks.into_iter().map(|chunk| serde_json::json!({
                        "id": chunk.chunk_id,
                        "source_id": chunk.source_id,
                        "chunk_index": chunk.chunk_index,
                        "content": chunk.content,
                        "metadata": chunk.metadata,
                        "created_at": chunk.created_at,
                    })).collect::<Vec<_>>()
                }))
            })
        }),
    );

    let optimization_for_list_targets = optimization_store.clone();
    let langsmith_for_opt_list = langsmith.clone();
    server.register_handler(
        "list_optimization_targets",
        traced_mcp_handler(
            langsmith_for_opt_list,
            "list_optimization_targets",
            move |_| {
                let store = optimization_for_list_targets.clone();
                block_on_tool(async move {
                    let targets = store.list_targets().await?;
                    Ok(serde_json::json!({
                        "targets": targets
                    }))
                })
            },
        ),
    );

    let optimization_for_register = optimization_store.clone();
    let langsmith_for_opt_register = langsmith.clone();
    server.register_handler(
        "register_optimization_target",
        traced_mcp_handler(
            langsmith_for_opt_register,
            "register_optimization_target",
            move |args| {
                let request: McpRegisterOptimizationTargetArgs = parse_tool_args(args)?;
                let store = optimization_for_register.clone();
                let workspace_root = root_for_opt_register.clone();
                block_on_tool(async move {
                    let target = store
                        .register_target(TargetRegistration {
                            name: request.name,
                            description: request.description,
                            target_kind: parse_mcp_enum(&request.target_kind, "target_kind")?,
                            execution_tier: parse_mcp_enum(
                                &request.execution_tier,
                                "execution_tier",
                            )?,
                            risk_class: parse_mcp_enum(
                                request.risk_class.as_deref().unwrap_or("safe_config"),
                                "risk_class",
                            )?,
                            ship_status: parse_mcp_enum(
                                request.ship_status.as_deref().unwrap_or("experimental"),
                                "ship_status",
                            )?,
                            workspace_root: request
                                .workspace_root
                                .map(|root| resolve_workspace_path(&workspace_root, &root))
                                .transpose()?
                                .unwrap_or(workspace_root.clone())
                                .display()
                                .to_string(),
                            mutation_policy: request.mutation_policy.unwrap_or_default(),
                            eval_suite: request.eval_suite.unwrap_or_default(),
                            promotion_policy: request.promotion_policy.unwrap_or_default(),
                            metadata: request.metadata.unwrap_or_else(|| serde_json::json!({})),
                        })
                        .await?;
                    Ok(serde_json::json!({ "target": target }))
                })
            },
        ),
    );

    let optimization_for_submit = optimization_store.clone();
    let langsmith_for_opt_submit = langsmith.clone();
    server.register_handler(
        "submit_optimization_candidate",
        traced_mcp_handler(
            langsmith_for_opt_submit,
            "submit_optimization_candidate",
            move |args| {
                let request: McpSubmitOptimizationCandidateArgs = parse_tool_args(args)?;
                let store = optimization_for_submit.clone();
                block_on_tool(async move {
                    let target = store.get_target(&request.target).await?;
                    let candidate = store
                        .submit_candidate(
                            &target.id,
                            &request.hypothesis,
                            request.proposed_by.as_deref().unwrap_or("mcp_operator"),
                            request.changes,
                            request.trace_id,
                        )
                        .await?;
                    Ok(serde_json::json!({ "candidate": candidate }))
                })
            },
        ),
    );

    let optimization_for_list_candidates = optimization_store.clone();
    let langsmith_for_opt_list_candidates = langsmith.clone();
    server.register_handler(
        "list_optimization_candidates",
        traced_mcp_handler(
            langsmith_for_opt_list_candidates,
            "list_optimization_candidates",
            move |args| {
                let request: McpListOptimizationCandidatesArgs = parse_tool_args(args)?;
                let store = optimization_for_list_candidates.clone();
                block_on_tool(async move {
                    let target_id = match request.target {
                        Some(target) => Some(store.get_target(&target).await?.id),
                        None => None,
                    };
                    let status = request
                        .status
                        .as_deref()
                        .map(|raw| parse_mcp_enum(raw, "candidate status"))
                        .transpose()?;
                    let candidates = store
                        .list_candidates(target_id.as_deref(), status, request.limit)
                        .await?;
                    Ok(serde_json::json!({ "candidates": candidates }))
                })
            },
        ),
    );

    let optimization_for_inspect = optimization_store.clone();
    let langsmith_for_opt_inspect = langsmith.clone();
    server.register_handler(
        "inspect_optimization_candidate",
        traced_mcp_handler(
            langsmith_for_opt_inspect,
            "inspect_optimization_candidate",
            move |args| {
                let request: McpInspectOptimizationCandidateArgs = parse_tool_args(args)?;
                let store = optimization_for_inspect.clone();
                block_on_tool(async move {
                    let candidate = store.get_candidate(&request.candidate_id).await?;
                    let evaluations = store.list_evaluations(&request.candidate_id).await?;
                    let promotions = store.list_promotions(&request.candidate_id).await?;
                    Ok(serde_json::json!({
                        "candidate": candidate,
                        "evaluations": evaluations,
                        "promotions": promotions,
                    }))
                })
            },
        ),
    );

    let optimization_for_run = optimization_store.clone();
    let langsmith_for_opt_run = langsmith.clone();
    server.register_handler(
        "run_optimization_candidate",
        traced_mcp_handler(
            langsmith_for_opt_run.clone(),
            "run_optimization_candidate",
            move |args| {
                let request: McpRunOptimizationCandidateArgs = parse_tool_args(args)?;
                let store = optimization_for_run.clone();
                let langsmith = langsmith_for_opt_run.clone();
                block_on_tool(async move {
                    let runner =
                        CandidateRunner::new(store, langsmith, CandidateRunnerConfig::default());
                    let summary = runner.run_candidate(&request.candidate_id).await?;
                    Ok(serde_json::json!({ "summary": summary }))
                })
            },
        ),
    );

    let optimization_for_promote = optimization_store;
    server.register_handler(
        "promote_optimization_candidate",
        traced_mcp_handler(
            langsmith.clone(),
            "promote_optimization_candidate",
            move |args| {
                let request: McpPromoteOptimizationCandidateArgs = parse_tool_args(args)?;
                let store = optimization_for_promote.clone();
                block_on_tool(async move {
                    let event = store
                        .record_promotion(
                            &request.candidate_id,
                            parse_mcp_enum(&request.decision, "decision")?,
                            &request.decided_by,
                            request.notes.as_deref(),
                            request.rollback_reference.as_deref(),
                            None,
                        )
                        .await?;
                    Ok(serde_json::json!({ "promotion": event }))
                })
            },
        ),
    );

    server
}

fn traced_mcp_handler<F>(
    langsmith: Option<LangSmithClient>,
    tool_name: &'static str,
    handler: F,
) -> impl Fn(serde_json::Value) -> openrustclaw_core::error::Result<serde_json::Value>
+ Send
+ Sync
+ 'static
where
    F: Fn(serde_json::Value) -> openrustclaw_core::error::Result<serde_json::Value>
        + Send
        + Sync
        + 'static,
{
    move |args| {
        let trace_client = langsmith.clone();
        let trace_args = args.clone();
        let mut trace = trace_client.as_ref().map(|client| {
            client.new_run(
                "mcp_tool_call",
                RunType::Tool,
                serde_json::json!({
                    "tool_name": tool_name,
                    "args": trace_args,
                }),
            )
        });
        let result = handler(args);

        if let (Some(client), Some(mut run)) = (trace_client, trace.take()) {
            run.outputs = result
                .as_ref()
                .ok()
                .map(|value| serde_json::json!({"result": value}));
            run.error = result.as_ref().err().map(|error| error.to_string());
            run.end_time = Some(Utc::now());
            if let Err(trace_error) = block_on_async(client.trace_run(&run)) {
                warn!(error = %trace_error, tool = %tool_name, "Failed to send LangSmith MCP trace");
            }
        }

        result
    }
}

fn block_on_tool<F>(future: F) -> openrustclaw_core::error::Result<serde_json::Value>
where
    F: Future<Output = openrustclaw_core::error::Result<serde_json::Value>>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn block_on_async<F, T>(future: F) -> openrustclaw_core::error::Result<T>
where
    F: Future<Output = openrustclaw_core::error::Result<T>>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn mcp_tool_error(message: impl Into<String>) -> CoreError {
    CoreError::Mcp(McpError::ToolExecution(message.into()))
}

fn parse_tool_args<T: serde::de::DeserializeOwned>(
    args: serde_json::Value,
) -> openrustclaw_core::error::Result<T> {
    serde_json::from_value(args)
        .map_err(|e| mcp_tool_error(format!("Invalid tool arguments: {}", e)))
}

fn parse_json_column(raw: String) -> serde_json::Value {
    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!(raw))
}

fn parse_optional_json_column(raw: Option<String>) -> serde_json::Value {
    raw.map(parse_json_column)
        .unwrap_or(serde_json::Value::Null)
}

fn parse_mcp_enum<T>(raw: &str, label: &str) -> openrustclaw_core::error::Result<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    raw.parse::<T>()
        .map_err(|error| mcp_tool_error(format!("Invalid {} '{}': {}", label, raw, error)))
}

fn parse_mcp_timestamp(raw: &str) -> openrustclaw_core::error::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| mcp_tool_error(format!("Invalid RFC3339 timestamp '{}': {}", raw, e)))
}

fn parse_mcp_session_status(raw: &str) -> Option<openrustclaw_db::SessionStatus> {
    match raw.to_lowercase().as_str() {
        "active" => Some(openrustclaw_db::SessionStatus::Active),
        "archived" => Some(openrustclaw_db::SessionStatus::Archived),
        "closed" => Some(openrustclaw_db::SessionStatus::Closed),
        _ => None,
    }
}

fn parse_mcp_platform(raw: &str) -> Platform {
    match raw.to_lowercase().as_str() {
        "telegram" => Platform::Telegram,
        "discord" => Platform::Discord,
        "slack" => Platform::Slack,
        "cli" => Platform::Cli,
        "api" => Platform::Api,
        _ => Platform::WebChat,
    }
}

fn parse_mcp_session_type(raw: &str) -> SessionType {
    match raw.to_lowercase().as_str() {
        "group" => SessionType::Group,
        "isolated" => SessionType::Isolated,
        _ => SessionType::Dm,
    }
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

#[derive(serde::Deserialize)]
struct McpGetMemoryArgs {
    id: String,
}

#[derive(serde::Deserialize, Default)]
struct McpMemoryTimelineArgs {
    namespace: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct McpListSessionsArgs {
    status: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpInspectSessionArgs {
    id: String,
    history_limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpSpawnSessionArgs {
    user_id: String,
    platform: Option<String>,
    session_type: Option<String>,
    route_key: Option<String>,
    workspace_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct McpSendToSessionArgs {
    id: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct McpCloseSessionArgs {
    id: String,
    archive: Option<bool>,
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct McpRenderArtifactsArgs {
    model: String,
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
    priority: Option<i64>,
}

#[derive(serde::Deserialize)]
struct McpInspectScheduledJobArgs {
    id: String,
}

#[derive(serde::Deserialize)]
struct McpReprioritizeScheduledJobArgs {
    id: String,
    priority: i64,
}

#[derive(serde::Deserialize, Default)]
struct McpListRunsArgs {
    job: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpReplayDeadLetterArgs {
    id: String,
}

#[derive(serde::Deserialize, Default)]
struct McpListRuntimeEventsArgs {
    event_name: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct McpListRagCollectionsArgs {
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpLoadRagChunksArgs {
    collection_name: String,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpRegisterOptimizationTargetArgs {
    name: String,
    description: Option<String>,
    target_kind: String,
    execution_tier: String,
    risk_class: Option<String>,
    ship_status: Option<String>,
    workspace_root: Option<String>,
    mutation_policy: Option<MutationPolicy>,
    eval_suite: Option<Vec<EvaluationSpec>>,
    promotion_policy: Option<PromotionPolicy>,
    metadata: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct McpSubmitOptimizationCandidateArgs {
    target: String,
    hypothesis: String,
    proposed_by: Option<String>,
    changes: Vec<CandidateChange>,
    trace_id: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct McpListOptimizationCandidatesArgs {
    target: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct McpInspectOptimizationCandidateArgs {
    candidate_id: String,
}

#[derive(serde::Deserialize)]
struct McpRunOptimizationCandidateArgs {
    candidate_id: String,
}

#[derive(serde::Deserialize)]
struct McpPromoteOptimizationCandidateArgs {
    candidate_id: String,
    decision: String,
    decided_by: String,
    notes: Option<String>,
    rollback_reference: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use openrustclaw_core::config::{AppConfig, DiscordConfig, SlackConfig, SlackMode};
    use openrustclaw_core::error::{Error, ProviderError};
    use openrustclaw_core::types::{FinishReason, IncomingMessage, Role, TokenUsage};
    use tempfile::tempdir;

    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(
            &self,
            request: CompletionRequest,
        ) -> openrustclaw_core::error::Result<CompletionResponse> {
            let last_user_message = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == Role::User)
                .map(|message| message.content.clone())
                .unwrap_or_default();

            Ok(CompletionResponse {
                id: "mock-response".to_string(),
                message: Message::assistant(format!("Echo: {}", last_user_message)),
                model: "mock-model".to_string(),
                usage: TokenUsage::default(),
                provider: "mock".to_string(),
                finish_reason: FinishReason::Stop,
            })
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> openrustclaw_core::error::Result<
            Pin<Box<dyn Stream<Item = openrustclaw_core::error::Result<StreamChunk>> + Send>>,
        > {
            Err(Error::Provider(ProviderError::StreamError {
                provider: "mock".to_string(),
                message: "streaming unsupported in tests".to_string(),
            }))
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }

        fn max_tokens(&self) -> usize {
            4096
        }

        fn provider_name(&self) -> &str {
            "mock"
        }

        fn supports_strict_tools(&self) -> bool {
            false
        }

        fn supports_streaming_tool_deltas(&self) -> bool {
            false
        }

        fn native_tool_format(&self) -> ToolFormat {
            ToolFormat::OpenAi
        }
    }

    async fn test_channel_agent() -> (ChannelAgent, sqlx::SqlitePool) {
        let runtime = AgentRuntime::new(
            Arc::new(MockProvider),
            Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
            "OpenRustClaw".to_string(),
        );
        let pool = openrustclaw_db::init_pool("sqlite::memory:", 1)
            .await
            .expect("pool");
        openrustclaw_db::run_migrations(&pool)
            .await
            .expect("migrations");

        let registry_root =
            std::env::temp_dir().join(format!("openrustclaw-channel-tests-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&registry_root).expect("registry root");

        (
            ChannelAgent {
                runtime: Arc::new(runtime),
                session_manager: Arc::new(SessionManager::new()),
                core_memory_store: None,
                max_history_messages: 24,
                session_routing: AppConfig::default().session_routing,
                channel_registry: Arc::new(tokio::sync::RwLock::new(ChannelRegistry {
                    root: registry_root,
                    ..ChannelRegistry::default()
                })),
                langsmith: None,
                event_bus: DurableEventBus::new(pool.clone(), 16),
            },
            pool,
        )
    }

    #[test]
    fn gate_nonshipping_channels_disables_gated_runtime_surfaces() {
        let mut config = AppConfig::default().channels;
        config.teams.enabled = true;
        config.google_chat.enabled = true;
        config.gmail_pubsub.enabled = true;
        config.matrix.enabled = true;
        config.line.enabled = true;
        config.viber.enabled = true;
        config.wechat.enabled = true;
        config.meta.enabled = true;

        gate_nonshipping_channels(&mut config);

        assert!(config.teams.enabled);
        assert!(config.google_chat.enabled);
        assert!(!config.gmail_pubsub.enabled);
        assert!(config.matrix.enabled);
        assert!(!config.line.enabled);
        assert!(!config.viber.enabled);
        assert!(!config.wechat.enabled);
        assert!(!config.meta.enabled);
    }

    #[tokio::test]
    async fn inbound_channel_messages_reuse_session_for_same_route() {
        let (agent, _pool) = test_channel_agent().await;
        let mut route_sessions = HashMap::new();

        let first_reply = agent
            .handle_incoming_message(
                &mut route_sessions,
                IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: "user-1".to_string(),
                    content: "hello".to_string(),
                    platform: openrustclaw_core::types::Platform::Telegram,
                    metadata: serde_json::json!({
                        "telegram_chat_id": "chat-123"
                    }),
                },
            )
            .await
            .unwrap()
            .unwrap();

        let second_reply = agent
            .handle_incoming_message(
                &mut route_sessions,
                IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: "user-1".to_string(),
                    content: "again".to_string(),
                    platform: openrustclaw_core::types::Platform::Telegram,
                    metadata: serde_json::json!({
                        "telegram_chat_id": "chat-123"
                    }),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(route_sessions.len(), 1);
        assert_eq!(first_reply.session_id, second_reply.session_id);
        assert_eq!(second_reply.content, "Echo: again");
        assert_eq!(second_reply.metadata["telegram_chat_id"], "chat-123");
    }

    #[tokio::test]
    async fn shared_group_scope_isolated_by_user() {
        let (agent, _pool) = test_channel_agent().await;
        let mut route_sessions = HashMap::new();

        let first_reply = agent
            .handle_incoming_message(
                &mut route_sessions,
                IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: "user-1".to_string(),
                    content: "hello".to_string(),
                    platform: openrustclaw_core::types::Platform::Slack,
                    metadata: serde_json::json!({
                        "slack_channel": "C123",
                        "slack_team_id": "T123"
                    }),
                },
            )
            .await
            .unwrap()
            .unwrap();

        let second_reply = agent
            .handle_incoming_message(
                &mut route_sessions,
                IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: "user-2".to_string(),
                    content: "hello".to_string(),
                    platform: openrustclaw_core::types::Platform::Slack,
                    metadata: serde_json::json!({
                        "slack_channel": "C123",
                        "slack_team_id": "T123"
                    }),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(route_sessions.len(), 2);
        assert_ne!(first_reply.session_id, second_reply.session_id);
        assert_eq!(first_reply.metadata["slack_channel"], "C123");
    }

    #[tokio::test]
    async fn channel_agent_publishes_runtime_lifecycle_and_message_events() {
        let (agent, pool) = test_channel_agent().await;
        let mut route_sessions = HashMap::new();

        let reply = agent
            .handle_incoming_message(
                &mut route_sessions,
                IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: "user-1".to_string(),
                    content: "hello".to_string(),
                    platform: openrustclaw_core::types::Platform::Telegram,
                    metadata: serde_json::json!({
                        "telegram_chat_id": "chat-123"
                    }),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(reply.content, "Echo: hello");
        agent
            .close_route_sessions(
                &mut route_sessions,
                openrustclaw_core::types::Platform::Telegram,
                "test_completed",
            )
            .await;

        let names: Vec<String> =
            sqlx::query_scalar("SELECT event_name FROM runtime_events ORDER BY created_at ASC")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(names.contains(&"session.created".to_string()));
        assert!(names.contains(&"session.start".to_string()));
        assert!(names.contains(&"message.received".to_string()));
        assert!(names.contains(&"message.sent".to_string()));
        assert!(names.contains(&"session.post_turn".to_string()));
        assert!(names.contains(&"session.end".to_string()));
        assert!(names.contains(&"session.closed".to_string()));
    }

    #[test]
    fn discord_thread_scope_takes_priority_over_channel_scope() {
        let scope = channel_scope_from_metadata(
            &serde_json::json!({
                "discord_channel_id": "channel-1",
                "discord_thread_id": "thread-1"
            }),
            true,
        );

        assert_eq!(scope.as_deref(), Some("discord_thread_id=thread-1"));
    }

    #[test]
    fn expand_outgoing_message_chunks_long_replies() {
        let messages = expand_outgoing_message(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: "one two three four five six seven eight nine ten".to_string(),
            metadata: serde_json::json!({
                "claw_send_policy": {
                    "mode": "blocks",
                    "max_chunk_chars": 12,
                    "chunk_delay_ms": 0,
                    "coalesce_below_chars": 0,
                    "preview_chars": 5
                }
            }),
        });

        assert!(messages.len() > 1);
        assert_eq!(messages[0].metadata["reply_part"], 1);
    }

    #[test]
    fn binding_route_key_includes_workspace_agent_and_account() {
        let route_key = channel_route_key_with_binding(
            &IncomingMessage {
                session_id: Uuid::new_v4(),
                user_id: "user-1".to_string(),
                content: "hello".to_string(),
                platform: openrustclaw_core::types::Platform::Slack,
                metadata: serde_json::json!({
                    "slack_channel": "C123"
                }),
            },
            "shared_main",
            "shared_channel",
            true,
            Some("workspace-a"),
            Some("ops"),
            Some("slack:T123:user-1"),
        );

        assert!(route_key.contains("workspace=workspace-a"));
        assert!(route_key.contains("agent=ops"));
        assert!(route_key.contains("account=slack:T123:user-1"));
    }

    #[tokio::test]
    async fn slack_http_ingress_handles_url_verification() {
        let channel = openrustclaw_channels::SlackChannel::new(SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: None,
            app_token: None,
            signing_secret: None,
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec![],
            app_home_enabled: true,
        });
        let state = SlackIngressState {
            handler: Arc::new(channel.event_handler()),
            langsmith: None,
        };

        let response = slack_events_handler(
            State(state),
            HeaderMap::new(),
            Bytes::from_static(br#"{"type":"url_verification","challenge":"abc123"}"#),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn slack_http_ingress_enqueues_messages_for_channel_runtime() {
        let channel = openrustclaw_channels::SlackChannel::new(SlackConfig {
            enabled: true,
            token: "xoxb-test".to_string(),
            api_base_url: None,
            app_token: None,
            signing_secret: None,
            mode: SlackMode::Http,
            socket_mode: false,
            rate_limit_requests_per_second: 10,
            allowed_workspaces: vec!["T123".to_string()],
            app_home_enabled: true,
        });
        let state = SlackIngressState {
            handler: Arc::new(channel.event_handler()),
            langsmith: None,
        };

        let response = slack_events_handler(
            State(state),
            HeaderMap::new(),
            Bytes::from_static(
                br#"{
                    "type":"event_callback",
                    "team_id":"T123",
                    "event":{
                        "type":"message",
                        "user":"U123",
                        "text":"hello from slack",
                        "channel":"C123",
                        "thread_ts":"171234.000100"
                    }
                }"#,
            ),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.user_id, "U123");
        assert_eq!(incoming.content, "hello from slack");
        assert_eq!(incoming.metadata["slack_channel"], "C123");
        assert_eq!(incoming.metadata["slack_thread_ts"], "171234.000100");
    }

    #[tokio::test]
    async fn discord_http_ingress_handles_ping() {
        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let channel = openrustclaw_channels::DiscordChannel::new(DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: Some(hex::encode(verifying_key.to_bytes())),
            api_base_url: None,
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        });
        let state = DiscordIngressState {
            handler: Arc::new(channel.interactions_handler().unwrap()),
            langsmith: None,
        };
        let body = Bytes::from_static(br#"{"type":1}"#);
        let timestamp = "1712550000";
        let signature = hex::encode(
            signing_key
                .sign(&[timestamp.as_bytes(), body.as_ref()].concat())
                .to_bytes(),
        );
        let mut headers = HeaderMap::new();
        headers.insert("x-signature-ed25519", signature.parse().unwrap());
        headers.insert("x-signature-timestamp", timestamp.parse().unwrap());

        let response = discord_interactions_handler(State(state), headers, body)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn discord_http_ingress_enqueues_interaction_messages() {
        let signing_key = SigningKey::from_bytes(&[12u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let channel = openrustclaw_channels::DiscordChannel::new(DiscordConfig {
            enabled: true,
            token: "token".to_string(),
            application_id: "app-1".to_string(),
            interaction_public_key: Some(hex::encode(verifying_key.to_bytes())),
            api_base_url: None,
            rate_limit_requests_per_second: 5,
            allowed_guilds: vec![],
            allowed_channels: vec![],
            dm_enabled: true,
        });
        let state = DiscordIngressState {
            handler: Arc::new(channel.interactions_handler().unwrap()),
            langsmith: None,
        };
        let body = Bytes::from_static(
            br#"{
                "id":"interaction-1",
                "application_id":"app-1",
                "type":2,
                "token":"interaction-token",
                "guild_id":"guild-1",
                "channel_id":"channel-1",
                "data":{"name":"ask","options":[{"name":"prompt","value":"hello from discord"}]},
                "member":{"user":{"id":"user-1"}}
            }"#,
        );
        let timestamp = "1712550001";
        let signature = hex::encode(
            signing_key
                .sign(&[timestamp.as_bytes(), body.as_ref()].concat())
                .to_bytes(),
        );
        let mut headers = HeaderMap::new();
        headers.insert("x-signature-ed25519", signature.parse().unwrap());
        headers.insert("x-signature-timestamp", timestamp.parse().unwrap());

        let response = discord_interactions_handler(State(state), headers, body)
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.user_id, "user-1");
        assert_eq!(incoming.content, "hello from discord");
        assert_eq!(incoming.metadata["discord_channel_id"], "channel-1");
        assert_eq!(
            incoming.metadata["discord_interaction_token"],
            "interaction-token"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_memory_tools_persist_and_render() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-memory.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool.clone(),
            AppConfig::default(),
            None,
        );

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
        let search_text = search_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
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
        let set_core_text = set_core_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
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
        let render_text = render_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let render_payload: serde_json::Value = serde_json::from_str(render_text).unwrap();
        assert!(
            render_payload["content"]
                .as_str()
                .unwrap()
                .contains("language_preference")
        );

        let event_names: Vec<String> =
            sqlx::query_scalar("SELECT event_name FROM runtime_events ORDER BY created_at ASC")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(event_names.contains(&"memory.stored".to_string()));
        assert!(event_names.contains(&"memory.searched".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_scheduler_tools_create_and_list_jobs() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-scheduler.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );

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
        let create_text = create_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
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
        assert_eq!(
            list_payload["jobs"][0]["metadata"]["workflow_metadata"]["purpose"],
            "test"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_lists_rag_collections() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-rag.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let rag_store = SqliteRagStore::new(pool.clone());
        rag_store
            .replace_collection(
                "docs",
                &[openrustclaw_db::RagChunkInput {
                    chunk_id: "chunk-1".to_string(),
                    source_id: "doc-1".to_string(),
                    chunk_index: 0,
                    content: "Rust ownership".to_string(),
                    metadata: serde_json::json!({"source_type": "doc"}),
                }],
            )
            .await
            .unwrap();

        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );
        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "list_rag_collections",
                "arguments": {
                    "limit": 5
                }
            }
        });

        let list_resp = server.handle_request(&list_req);
        let list_text = list_resp["result"]["content"][0]["text"].as_str().unwrap();
        let list_payload: serde_json::Value = serde_json::from_str(list_text).unwrap();
        assert_eq!(list_payload["collections"].as_array().unwrap().len(), 1);
        assert_eq!(list_payload["collections"][0]["collection_name"], "docs");
        assert_eq!(list_payload["collections"][0]["chunk_count"], 1);
        assert_eq!(list_payload["collections"][0]["source_count"], 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_loads_rag_chunks() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-rag-load.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let rag_store = SqliteRagStore::new(pool.clone());
        rag_store
            .replace_collection(
                "docs",
                &[
                    openrustclaw_db::RagChunkInput {
                        chunk_id: "chunk-1".to_string(),
                        source_id: "doc-1".to_string(),
                        chunk_index: 0,
                        content: "Rust ownership".to_string(),
                        metadata: serde_json::json!({"source_type": "doc"}),
                    },
                    openrustclaw_db::RagChunkInput {
                        chunk_id: "chunk-2".to_string(),
                        source_id: "doc-1".to_string(),
                        chunk_index: 1,
                        content: "Borrow checker".to_string(),
                        metadata: serde_json::json!({"source_type": "doc"}),
                    },
                ],
            )
            .await
            .unwrap();

        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );
        let load_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "tools/call",
            "params": {
                "name": "load_rag_chunks",
                "arguments": {
                    "collection_name": "docs",
                    "limit": 1
                }
            }
        });

        let load_resp = server.handle_request(&load_req);
        let load_text = load_resp["result"]["content"][0]["text"].as_str().unwrap();
        let load_payload: serde_json::Value = serde_json::from_str(load_text).unwrap();
        assert_eq!(load_payload["collection_name"], "docs");
        assert_eq!(load_payload["chunks"].as_array().unwrap().len(), 1);
        assert_eq!(load_payload["chunks"][0]["content"], "Rust ownership");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_runs_optimization_candidate_end_to_end() {
        let workspace = tempdir().unwrap();
        std::fs::write(workspace.path().join("prompt.txt"), "original prompt").unwrap();
        let db_path = workspace.path().join("mcp-optimization.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );

        let register_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "tools/call",
            "params": {
                "name": "register_optimization_target",
                "arguments": {
                    "name": "prompt.optimize",
                    "target_kind": "prompt_policy",
                    "execution_tier": "rust_native",
                    "workspace_root": ".",
                    "mutation_policy": {
                        "allowed_paths": ["prompt.txt"],
                        "forbidden_paths": [".git", "target"],
                        "allowed_fields": [],
                        "max_changed_files": 2,
                        "max_total_bytes": 4096,
                        "max_diff_lines": 50,
                        "mandatory_evals": ["verify"],
                        "required_tests": []
                    },
                    "eval_suite": [{
                        "name": "verify",
                        "command": {
                            "program": "bash",
                            "args": ["-lc", "grep -q optimized prompt.txt"]
                        },
                        "timeout_secs": 30,
                        "metadata": {}
                    }]
                }
            }
        });
        let register_resp = server.handle_request(&register_req);
        let register_text = register_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let register_payload: serde_json::Value = serde_json::from_str(register_text).unwrap();
        assert_eq!(register_payload["target"]["name"], "prompt.optimize");

        let submit_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": {
                "name": "submit_optimization_candidate",
                "arguments": {
                    "target": "prompt.optimize",
                    "hypothesis": "simpler prompt works better",
                    "changes": [{
                        "path": "prompt.txt",
                        "new_content": "optimized",
                        "summary": "replace prompt",
                        "field_path": null,
                        "metadata": {}
                    }]
                }
            }
        });
        let submit_resp = server.handle_request(&submit_req);
        let submit_text = submit_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let submit_payload: serde_json::Value = serde_json::from_str(submit_text).unwrap();
        let candidate_id = submit_payload["candidate"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let run_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 32,
            "method": "tools/call",
            "params": {
                "name": "run_optimization_candidate",
                "arguments": {
                    "candidate_id": candidate_id
                }
            }
        });
        let run_resp = server.handle_request(&run_req);
        let run_text = run_resp["result"]["content"][0]["text"].as_str().unwrap();
        let run_payload: serde_json::Value = serde_json::from_str(run_text).unwrap();
        assert_eq!(run_payload["summary"]["metrics"]["status"], "passed");

        let promote_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 33,
            "method": "tools/call",
            "params": {
                "name": "promote_optimization_candidate",
                "arguments": {
                    "candidate_id": run_payload["summary"]["candidate"]["id"],
                    "decision": "approve",
                    "decided_by": "tester"
                }
            }
        });
        let promote_resp = server.handle_request(&promote_req);
        let promote_text = promote_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let promote_payload: serde_json::Value = serde_json::from_str(promote_text).unwrap();
        assert_eq!(promote_payload["promotion"]["decision"], "approve");
    }
}
