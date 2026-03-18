//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use futures::Stream;
use openrustclaw_core::error::{ChannelError as CoreChannelError, Error as CoreError, McpError};
use openrustclaw_core::traits::{Channel, LlmProvider};
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, MemoryEntry, MemoryQuery, MemoryType, Message,
    OutgoingMessage, SessionType, SourceType, StreamChunk, ToolFormat,
};
use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_channels::discord::DiscordInteractionsHandler;
use openrustclaw_channels::slack::SlackEventHandler;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use std::collections::{HashMap, HashSet};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::{AppConfig, SlackMode};
use openrustclaw_core::traits::{
    CoreMemoryStore as CoreMemoryStoreTrait, MemoryStore as MemoryStoreTrait,
};
use openrustclaw_db::{
    SqliteCoreMemoryStore, SqliteMemoryStore, SqliteRagStore, init_pool, run_migrations,
};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_langbridge::{LangBridgeClient, sidecar::SidecarManager};
use openrustclaw_memory::MemoryPolicies;
use openrustclaw_mcp::server::{McpServer, McpServerConfig, McpServerTool};
use openrustclaw_observability::LangSmithClient;
use openrustclaw_observability::langsmith::RunType;
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider, ProviderChain,
    openrouter::RouteStrategy,
};
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
    let rag_store = Arc::new(SqliteRagStore::new(pool.clone()));

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
    let scheduler_task = spawn_scheduler_task(
        pool.clone(),
        sidecar_addr.clone(),
        config.scheduler.clone(),
        scheduler_langsmith_client(&config),
    );

    let mut channel_config = config.channels.clone();
    let mut discord_ingress_handler = None;
    let mut slack_ingress_handler = None;
    let mut enabled_channels = Vec::new();

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
        )?))
    };

    for channel in enabled_channels {
        channel_tasks.push(spawn_channel_task(channel, channel_agent.clone()));
    }

    if let Some(handler) = slack_ingress_handler {
        app = app.merge(slack_ingress_router(handler));
        info!("Slack HTTP ingress enabled at /webhooks/slack/events");
    }
    if let Some(handler) = discord_ingress_handler {
        app = app.merge(discord_ingress_router(handler));
        info!("Discord Interactions ingress enabled at /webhooks/discord/interactions");
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

fn gate_nonshipping_channels(config: &mut openrustclaw_core::config::ChannelsConfig) {
    if config.teams.enabled {
        warn!("Teams is currently gated and will not be started by `openrustclaw start`");
        config.teams.enabled = false;
    }
    if config.google_chat.enabled {
        warn!("Google Chat is currently gated and will not be started by `openrustclaw start`");
        config.google_chat.enabled = false;
    }
    if config.whatsapp.enabled {
        warn!("WhatsApp is currently gated and will not be started by `openrustclaw start`");
        config.whatsapp.enabled = false;
    }
    if config.gmail_pubsub.enabled {
        warn!("Gmail Pub/Sub is currently gated and will not be started by `openrustclaw start`");
        config.gmail_pubsub.enabled = false;
    }
    if config.matrix.enabled {
        warn!("Matrix is currently gated and will not be started by `openrustclaw start`");
        config.matrix.enabled = false;
    }
    if config.imessage.enabled {
        warn!("iMessage is currently gated and will not be started by `openrustclaw start`");
        config.imessage.enabled = false;
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
    let server = build_mcp_server(workspace_root.clone(), pool, mcp_langsmith_client(&config));

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
    langsmith_client: Option<LangSmithClient>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut worker = SchedulerWorker::new(WorkerSchedulerConfig {
            poll_interval: tokio::time::Duration::from_millis(scheduler_config.poll_interval_ms),
            lease_duration: tokio::time::Duration::from_secs(
                scheduler_config.lease_duration_secs,
            ),
            base_retry_delay_secs: scheduler_config.base_retry_delay_secs,
            max_retry_delay_secs: scheduler_config.max_retry_delay_secs,
        });
        if let Some(client) = langsmith_client {
            worker = worker.with_langsmith(client);
        }

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
    mut channel: Box<dyn Channel>,
    channel_agent: Option<Arc<ChannelAgent>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let platform = channel.platform();
        let mut route_sessions = HashMap::new();
        info!(platform = ?platform, "Starting channel");
        match channel.connect().await {
            Ok(()) => {
                info!(platform = ?platform, "Channel connected");
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
                                        if let Err(e) = channel.send(reply).await {
                                            error!(
                                                platform = ?platform,
                                                error = %e,
                                                "Failed to send channel reply"
                                            );
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
            }
            Err(e) => {
                error!(platform = ?platform, error = %e, "Failed to connect channel");
            }
        }
    })
}

#[derive(Clone)]
struct ChannelAgent {
    runtime: Arc<AgentRuntime>,
    session_manager: Arc<SessionManager>,
    core_memory_store: Option<Arc<dyn CoreMemoryStoreTrait>>,
    max_history_messages: usize,
    langsmith: Option<LangSmithClient>,
}

struct ChannelConversationState {
    session_id: Uuid,
    history: Vec<Message>,
    reply_metadata: serde_json::Value,
}

impl ChannelAgent {
    async fn handle_incoming_message(
        &self,
        route_sessions: &mut HashMap<String, ChannelConversationState>,
        incoming: openrustclaw_core::types::IncomingMessage,
    ) -> Result<Option<OutgoingMessage>> {
        let trimmed_content = incoming.content.trim();
        if trimmed_content.is_empty() {
            return Ok(None);
        }

        let route_key = channel_route_key(&incoming);
        if !route_sessions.contains_key(&route_key) {
            let session_type = infer_session_type(&incoming);
            let session = self
                .session_manager
                .create_session(&incoming.user_id, session_type, incoming.platform)
                .await?;
            route_sessions.insert(
                route_key.clone(),
                ChannelConversationState {
                    session_id: session.id,
                    history: Vec::new(),
                    reply_metadata: incoming.metadata.clone(),
                },
            );
        }

        let Some(route_state) = route_sessions.get_mut(&route_key) else {
            return Ok(None);
        };

        route_state.reply_metadata = incoming.metadata.clone();
        route_state.history.push(Message::user(trimmed_content));
        trim_history(&mut route_state.history, self.max_history_messages);
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
                    }));
                    if let Err(trace_error) = client.update_run(run).await {
                        warn!(error = %trace_error, "Failed to update LangSmith channel trace");
                    }
                }
                return Err(error.into());
            }
        };

        route_state.history.push(response.message.clone());
        trim_history(&mut route_state.history, self.max_history_messages);

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
                    "empty_reply": true,
                }));
                if let Err(trace_error) = client.update_run(run).await {
                    warn!(error = %trace_error, "Failed to update LangSmith channel trace");
                }
            }
            return Ok(None);
        }

        if let (Some(client), Some(run)) = (&self.langsmith, trace.as_mut()) {
            run.outputs = Some(serde_json::json!({"reply": response.message.content}));
            run.end_time = Some(chrono::Utc::now());
            run.extra = Some(serde_json::json!({
                "platform": incoming.platform.to_string(),
                "user_id": incoming.user_id,
                "session_id": route_state.session_id,
            }));
            if let Err(trace_error) = client.update_run(run).await {
                warn!(error = %trace_error, "Failed to update LangSmith channel trace");
            }
        }

        Ok(Some(OutgoingMessage {
            session_id: route_state.session_id,
            content: response.message.content,
            metadata: route_state.reply_metadata.clone(),
        }))
    }

    fn channel_trace(
        &self,
        incoming: &openrustclaw_core::types::IncomingMessage,
        session_id: &Uuid,
        content: &str,
    ) -> Option<openrustclaw_observability::langsmith::TraceRun> {
        let client = self.langsmith.as_ref()?;
        Some(client.new_run(
            "channel_inbound_message",
            RunType::Chain,
            serde_json::json!({
                "platform": incoming.platform.to_string(),
                "user_id": incoming.user_id,
                "session_id": session_id,
                "content": content,
                "metadata": incoming.metadata,
            }),
        ))
    }
}

fn build_channel_agent(
    config: &AppConfig,
    memory_store: Arc<SqliteMemoryStore>,
    core_memory_store: Arc<SqliteCoreMemoryStore>,
    session_manager: Arc<SessionManager>,
    langsmith: Option<LangSmithClient>,
) -> Result<ChannelAgent> {
    let provider = build_channel_provider(config)?;
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "OpenRustClaw".to_string(),
        memory_store,
        core_memory_store.clone(),
    );

    Ok(ChannelAgent {
        runtime: Arc::new(runtime),
        session_manager,
        core_memory_store: Some(core_memory_store),
        max_history_messages: 24,
        langsmith,
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
            let provider = AnthropicProvider::new(api_key, config.providers.anthropic.model.clone());
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
    async fn complete(&self, request: CompletionRequest) -> openrustclaw_core::error::Result<CompletionResponse> {
        self.chain.complete(request).await
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> openrustclaw_core::error::Result<Pin<Box<dyn Stream<Item = openrustclaw_core::error::Result<StreamChunk>> + Send>>>
    {
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

fn infer_session_type(message: &openrustclaw_core::types::IncomingMessage) -> SessionType {
    if channel_scope_from_metadata(&message.metadata).is_some() {
        SessionType::Group
    } else {
        SessionType::Dm
    }
}

fn channel_route_key(message: &openrustclaw_core::types::IncomingMessage) -> String {
    let scope = channel_scope_from_metadata(&message.metadata)
        .unwrap_or_else(|| "direct".to_string());
    format!("{}:{}:{}", message.platform, scope, message.user_id)
}

fn channel_scope_from_metadata(metadata: &serde_json::Value) -> Option<String> {
    const PRIMARY_KEYS: &[&str] = &[
        "slack_thread_ts",
        "slack_channel",
        "telegram_chat_id",
        "discord_channel_id",
        "google_chat_thread",
        "google_chat_space",
        "teams_conversation_id",
        "matrix_room_id",
        "whatsapp_chat_id",
        "line_room_id",
        "meta_thread_id",
    ];

    for key in PRIMARY_KEYS {
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

fn trim_history(history: &mut Vec<Message>, max_history_messages: usize) {
    if history.len() > max_history_messages {
        let drain_count = history.len() - max_history_messages;
        history.drain(0..drain_count);
    }
}

#[derive(Clone)]
struct SlackIngressState {
    handler: Arc<SlackEventHandler>,
}

fn slack_ingress_router(handler: SlackEventHandler) -> Router {
    Router::new()
        .route("/webhooks/slack/events", post(slack_events_handler))
        .with_state(SlackIngressState {
            handler: Arc::new(handler),
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

    match state.handler.handle_event(&body, timestamp, signature).await {
        Ok(Some(response)) => (StatusCode::OK, axum::Json(response)).into_response(),
        Ok(None) => StatusCode::OK.into_response(),
        Err(CoreError::Channel(CoreChannelError::AuthFailed { message, .. })) => {
            warn!(error = %message, "Rejected Slack webhook due to failed auth");
            (StatusCode::UNAUTHORIZED, message).into_response()
        }
        Err(CoreError::Channel(CoreChannelError::PermissionDenied { message, .. })) => {
            warn!(error = %message, "Rejected Slack webhook due to permission check");
            (StatusCode::FORBIDDEN, message).into_response()
        }
        Err(error) => {
            warn!(error = %error, "Failed to process Slack webhook");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct DiscordIngressState {
    handler: Arc<DiscordInteractionsHandler>,
}

fn discord_ingress_router(handler: DiscordInteractionsHandler) -> Router {
    Router::new()
        .route(
            "/webhooks/discord/interactions",
            post(discord_interactions_handler),
        )
        .with_state(DiscordIngressState {
            handler: Arc::new(handler),
        })
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

    match state.handler.handle_event(&body, signature, timestamp).await {
        Ok(response) => (StatusCode::OK, axum::Json(response)).into_response(),
        Err(CoreError::Channel(CoreChannelError::AuthFailed { message, .. })) => {
            warn!(error = %message, "Rejected Discord interaction due to failed auth");
            (StatusCode::UNAUTHORIZED, message).into_response()
        }
        Err(CoreError::Channel(CoreChannelError::PermissionDenied { message, .. })) => {
            warn!(error = %message, "Rejected Discord interaction due to permission check");
            (StatusCode::FORBIDDEN, message).into_response()
        }
        Err(error) => {
            warn!(error = %error, "Failed to process Discord interaction");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

fn build_mcp_server(
    workspace_root: PathBuf,
    pool: sqlx::SqlitePool,
    langsmith: Option<LangSmithClient>,
) -> McpServer {
    let memory_store = SqliteMemoryStore::new(pool.clone());
    let core_memory_store = SqliteCoreMemoryStore::new(pool.clone());
    let rag_store = SqliteRagStore::new(pool.clone());
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
        ],
    });

    let root_for_health = workspace_root.clone();
    server.register_handler("health", traced_mcp_handler(langsmith.clone(), "health", move |_| {
        Ok(serde_json::json!({
            "status": "healthy",
            "workspace_root": root_for_health,
        }))
    }));

    let root_for_list = workspace_root.clone();
    server.register_handler("list_files", traced_mcp_handler(langsmith.clone(), "list_files", move |args| {
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
    }));

    let root_for_read = workspace_root;
    server.register_handler("read_file", traced_mcp_handler(langsmith.clone(), "read_file", move |args| {
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
    }));

    let memory_store_for_search = memory_store.clone();
    server.register_handler("search_memory", traced_mcp_handler(langsmith.clone(), "search_memory", move |args| {
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
    }));

    let memory_store_for_store = memory_store.clone();
    server.register_handler("store_memory", traced_mcp_handler(langsmith.clone(), "store_memory", move |args| {
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
    }));

    let core_memory_for_render = core_memory_store.clone();
    server.register_handler("render_core_memory", traced_mcp_handler(langsmith.clone(), "render_core_memory", move |args| {
        let request: McpRenderCoreMemoryArgs = parse_tool_args(args)?;
        let core_memory_store = core_memory_for_render.clone();
        block_on_tool(async move {
            let content = core_memory_store.render(&request.user_id).await?;
            Ok(serde_json::json!({ "content": content }))
        })
    }));

    let core_memory_for_set = core_memory_store.clone();
    server.register_handler("set_core_memory", traced_mcp_handler(langsmith.clone(), "set_core_memory", move |args| {
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
    }));

    let pool_for_list_jobs = pool.clone();
    server.register_handler("list_scheduled_jobs", traced_mcp_handler(langsmith.clone(), "list_scheduled_jobs", move |args| {
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
    }));

    let pool_for_create_jobs = pool;
    let langsmith_for_create_jobs = langsmith.clone();
    server.register_handler("create_scheduled_job", traced_mcp_handler(langsmith_for_create_jobs, "create_scheduled_job", move |args| {
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
    }));

    let rag_store_for_list = rag_store;
    server.register_handler("list_rag_collections", traced_mcp_handler(langsmith, "list_rag_collections", move |args| {
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
    }));

    server
}

fn traced_mcp_handler<F>(
    langsmith: Option<LangSmithClient>,
    tool_name: &'static str,
    handler: F,
) -> impl Fn(serde_json::Value) -> openrustclaw_core::error::Result<serde_json::Value> + Send + Sync + 'static
where
    F: Fn(serde_json::Value) -> openrustclaw_core::error::Result<serde_json::Value> + Send + Sync + 'static,
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

#[derive(serde::Deserialize, Default)]
struct McpListRagCollectionsArgs {
    limit: Option<usize>,
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

    fn test_channel_agent() -> ChannelAgent {
        let runtime = AgentRuntime::new(
            Arc::new(MockProvider),
            Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
            "OpenRustClaw".to_string(),
        );

        ChannelAgent {
            runtime: Arc::new(runtime),
            session_manager: Arc::new(SessionManager::new()),
            core_memory_store: None,
            max_history_messages: 24,
            langsmith: None,
        }
    }

    #[test]
    fn gate_nonshipping_channels_disables_gated_runtime_surfaces() {
        let mut config = AppConfig::default().channels;
        config.teams.enabled = true;
        config.google_chat.enabled = true;
        config.whatsapp.enabled = true;
        config.gmail_pubsub.enabled = true;
        config.matrix.enabled = true;
        config.imessage.enabled = true;
        config.line.enabled = true;
        config.viber.enabled = true;
        config.wechat.enabled = true;
        config.meta.enabled = true;

        gate_nonshipping_channels(&mut config);

        assert!(!config.teams.enabled);
        assert!(!config.google_chat.enabled);
        assert!(!config.whatsapp.enabled);
        assert!(!config.gmail_pubsub.enabled);
        assert!(!config.matrix.enabled);
        assert!(!config.imessage.enabled);
        assert!(!config.line.enabled);
        assert!(!config.viber.enabled);
        assert!(!config.wechat.enabled);
        assert!(!config.meta.enabled);
    }

    #[tokio::test]
    async fn inbound_channel_messages_reuse_session_for_same_route() {
        let agent = test_channel_agent();
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
        let agent = test_channel_agent();
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
        };
        let body = Bytes::from_static(br#"{"type":1}"#);
        let timestamp = "1712550000";
        let signature =
            hex::encode(signing_key.sign(&[timestamp.as_bytes(), body.as_ref()].concat()).to_bytes());
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
        let signature =
            hex::encode(signing_key.sign(&[timestamp.as_bytes(), body.as_ref()].concat()).to_bytes());
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
        assert_eq!(incoming.metadata["discord_interaction_token"], "interaction-token");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_memory_tools_persist_and_render() {
        let workspace = tempdir().unwrap();
        let db_path = workspace.path().join("mcp-memory.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let server = build_mcp_server(workspace.path().to_path_buf(), pool, None);

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

        let server = build_mcp_server(workspace.path().to_path_buf(), pool, None);

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

        let server = build_mcp_server(workspace.path().to_path_buf(), pool, None);
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
}
