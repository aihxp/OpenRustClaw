//! Start command - Initialize and run the OpenRustClaw server.

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        Extension, Path as AxumPath, Query, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use futures::{SinkExt, Stream, StreamExt};
use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_channels::discord::DiscordInteractionsHandler;
use openrustclaw_channels::gmail_pubsub::GmailWebhookHandler;
use openrustclaw_channels::google_chat::GoogleChatWebhookHandler;
use openrustclaw_channels::google_meet::GoogleMeetWebhookHandler;
use openrustclaw_channels::imessage::{BlueBubblesMessage, IMessageWebhookHandler};
use openrustclaw_channels::mattermost::MattermostWebhookHandler;
use openrustclaw_channels::slack::SlackEventHandler;
use openrustclaw_channels::teams::TeamsWebhookHandler;
use openrustclaw_channels::telegram::TelegramWebhookHandler;
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
use std::sync::{Arc, OnceLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::sync::watch;
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};

use openrustclaw_channels::{ChannelFactory, ChannelType, parse_channels_list};
use openrustclaw_core::config::{AppConfig, SessionRoutingConfig, SlackMode};
use openrustclaw_core::traits::{
    CoreMemoryStore as CoreMemoryStoreTrait, MemoryStore as MemoryStoreTrait,
};
#[cfg(feature = "cursor")]
use openrustclaw_cursor::tools as cursor_tools;
use openrustclaw_db::{
    SqliteCoreMemoryStore, SqliteMemoryStore, SqliteRagStore, SqliteSessionStore, init_pool,
    run_migrations,
};
use openrustclaw_gateway::metrics_endpoint::{install_metrics, metrics_middleware, metrics_routes};
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
use openrustclaw_providers::ProviderChain;
use openrustclaw_scheduler::{DurableEventBus, ReminderSender, RustWorkflowDispatcher};
use openrustclaw_scheduler::{SchedulerWorker, worker::SchedulerConfig as WorkerSchedulerConfig};
use openrustclaw_security::OriginValidator;
use openrustclaw_skills::{
    CompiledSkillArtifact, CompiledSkillStatus, compiled_skill_background_services,
    list_compiled_manifests, load_compiled_artifact,
};
use sqlx::Row;
use uuid::Uuid;

static OPERATOR_EXECUTION_ROOT: OnceLock<PathBuf> = OnceLock::new();

mod auth;

use self::auth::{
    ControlAuthState, EnterpriseAccessState, load_control_api_token, load_trusted_proxy_token,
    protect_control_router, protect_enterprise_router,
};

use super::channels::{
    ChannelBindingSpec, ChannelRegistry, ChannelRouteStatus, ChannelSendPolicy,
    channel_route_key_with_binding as registry_channel_route_key_with_binding,
    channel_scope_from_metadata as registry_channel_scope_from_metadata, load_registry,
    message_bot_mentioned, preview_route, resolve_root,
};
#[cfg(feature = "voice")]
use super::talk;
use super::voice_runtime;
use super::voice_runtime::InboundVoiceTranscriber;
use super::{
    assistant, browser, control, control_ui, doctor, enterprise_access, enterprise_autonomy,
    enterprise_policy, inspect, logs, mobile, orchestrate, runtime, security, services, skills,
    tools,
};

/// Run the start command - load config, optionally start the compatibility/experimental sidecar, and start the gateway.
pub async fn run(config_path: &str, channels: Option<&str>) -> Result<()> {
    let workspace_root =
        std::env::current_dir().context("Failed to determine current workspace root")?;
    logs::init_runtime_logging(&workspace_root)?;
    let _ = OPERATOR_EXECUTION_ROOT.set(workspace_root.clone());

    info!("Starting OpenRustClaw...");
    let started_at = Utc::now();

    // Load configuration
    let mut config = runtime::load_effective_config(config_path, &workspace_root)?;
    let control_api_token = load_control_api_token(&config)?;
    let trusted_proxy_token = load_trusted_proxy_token(&config)?;
    validate_gateway_network_mode(&config)?;

    info!(config_path = %config_path, "Configuration loaded");
    if let Some(env_name) = config.security.control_api_token_env.as_deref() {
        info!(env = %env_name, "Control API bearer auth enabled");
    }
    if let Some(env_name) = config.security.trusted_proxy_token_env.as_deref() {
        info!(env = %env_name, "Trusted proxy auth enabled");
    }

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
                ChannelType::Mattermost => {
                    config.channels.mattermost.enabled = true;
                    info!("Mattermost channel enabled");
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
        trusted_proxy_token: trusted_proxy_token.clone(),
        memory_store: Some(memory_store.clone()),
        core_memory_store: Some(core_memory_store.clone()),
        rag_store: Some(rag_store),
        langsmith: gateway_langsmith_client(&config),
    };

    // Create and start gateway server
    let gateway = GatewayServer::new(config.gateway.host.clone(), config.gateway.port);
    let metrics_handle = install_metrics();

    let mut app = gateway.router(gateway_state);
    let addr = gateway.addr();
    let control_auth_state = ControlAuthState {
        bearer_token: control_api_token,
        trusted_proxy_token,
        origin_validation: config.security.origin_validation,
        origin_validator: Arc::new(OriginValidator::new(config.gateway.allowed_origins.clone())),
    };
    let _runtime_lock = runtime::acquire_runtime_lock(config_path, &workspace_root, &addr)?;

    info!(addr = %addr, "Starting gateway server");

    // Initialize enabled channels
    let mut channel_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let sidecar_addr = format!("http://127.0.0.1:{}", config.sidecar.grpc_port);
    let mut channel_config = config.channels.clone();
    let mut discord_ingress_handler = None;
    let mut telegram_ingress_handler = None;
    let mut slack_ingress_handler = None;
    let mut teams_ingress_handler = None;
    let mut mattermost_ingress_handler = None;
    let mut google_chat_ingress_handler = None;
    let mut google_meet_ingress_handler = None;
    let mut gmail_ingress_handler = None;
    let mut imessage_ingress_handler = None;
    let mut enabled_channels = Vec::new();
    let channel_registry_root = resolve_root(None)?;
    let channel_registry = Arc::new(tokio::sync::RwLock::new(
        load_registry(channel_registry_root.clone()).unwrap_or_else(|error| {
            warn!(error = %error, "Failed to load file-backed channel registry; continuing with defaults");
            ChannelRegistry {
                root: channel_registry_root.clone(),
                ..ChannelRegistry::default()
            }
        }),
    ));
    let control_root = control::control_root_for(&workspace_root);

    if channel_config.telegram.enabled
        && channel_config.telegram.mode == openrustclaw_core::config::TelegramMode::Webhook
    {
        match ChannelFactory::create_telegram(channel_config.telegram.clone()) {
            Ok(telegram_channel) => {
                telegram_ingress_handler = Some(telegram_channel.webhook_handler());
                enabled_channels.push(Box::new(telegram_channel) as Box<dyn Channel>);
                channel_config.telegram.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Telegram channel");
                channel_config.telegram.enabled = false;
            }
        }
    }

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

    if channel_config.mattermost.enabled {
        match ChannelFactory::create_mattermost(channel_config.mattermost.clone()) {
            Ok(mattermost_channel) => {
                mattermost_ingress_handler = Some((
                    channel_config.mattermost.webhook_path.clone(),
                    mattermost_channel.webhook_handler(),
                ));
                enabled_channels.push(Box::new(mattermost_channel) as Box<dyn Channel>);
                channel_config.mattermost.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Mattermost channel");
                channel_config.mattermost.enabled = false;
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

    if channel_config.google_meet.enabled {
        match ChannelFactory::create_google_meet(channel_config.google_meet.clone()) {
            Ok(google_meet_client) => {
                if let Err(error) = google_meet_client.connect().await {
                    error!(error = %error, "Failed to connect Google Meet runtime client");
                } else {
                    google_meet_ingress_handler = Some((
                        channel_config.google_meet.webhook_path.clone(),
                        google_meet_client.webhook_handler(),
                    ));
                }
                channel_config.google_meet.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Google Meet client");
                channel_config.google_meet.enabled = false;
            }
        }
    }

    if channel_config.gmail_pubsub.enabled {
        match ChannelFactory::create_gmail_pubsub(channel_config.gmail_pubsub.clone()) {
            Ok(gmail_channel) => {
                gmail_ingress_handler = Some(gmail_channel.webhook_handler());
                enabled_channels.push(Box::new(gmail_channel) as Box<dyn Channel>);
                channel_config.gmail_pubsub.enabled = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to create Gmail Pub/Sub channel");
                channel_config.gmail_pubsub.enabled = false;
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
        Some(Arc::new(tokio::sync::RwLock::new(build_channel_agent(
            &config,
            memory_store.clone(),
            core_memory_store.clone(),
            session_manager.clone(),
            channel_langsmith_client(&config),
            event_bus.clone(),
            channel_registry.clone(),
        )?)))
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
    if let Some(handler) = telegram_ingress_handler {
        app = app.merge(telegram_ingress_router(
            handler,
            channel_langsmith_client(&config),
        ));
        info!("Telegram webhook ingress enabled at /webhooks/telegram/events");
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
    if let Some((webhook_path, handler)) = mattermost_ingress_handler {
        app = app.merge(mattermost_ingress_router(webhook_path.as_str(), handler));
        info!(path = %webhook_path, "Mattermost ingress enabled");
    }
    if let Some(handler) = google_chat_ingress_handler {
        app = app.merge(google_chat_ingress_router(handler));
        info!("Google Chat ingress enabled at /webhooks/google-chat/events");
    }
    if let Some((webhook_path, handler)) = google_meet_ingress_handler {
        app = app.merge(google_meet_ingress_router(
            webhook_path.as_str(),
            handler,
            event_bus.clone(),
        ));
        info!(path = %webhook_path, "Google Meet ingress enabled");
    }
    if let Some(handler) = gmail_ingress_handler {
        app = app.merge(gmail_ingress_router(handler, workspace_root.clone()));
        info!("Gmail Pub/Sub ingress enabled at /webhooks/gmail/pubsub");
    }
    if let Some(handler) = imessage_ingress_handler {
        app = app.merge(imessage_ingress_router(handler));
        info!("iMessage BlueBubbles ingress enabled at /webhooks/imessage/bluebubbles");
    }
    app = app.merge(metrics_routes(metrics_handle));
    app = app.merge(protect_control_router(
        channel_registry_router(channel_registry.clone()),
        control_auth_state.clone(),
    ));
    app = app.merge(protect_control_router(
        protect_enterprise_router(
            control_plane_router(ControlPlaneApiState {
                control_root,
                workspace_root: workspace_root.clone(),
                config_path: config_path.to_string(),
            }),
            EnterpriseAccessState {
                workspace_root: workspace_root.clone(),
            },
        ),
        control_auth_state.clone(),
    ));
    app = app.merge(protect_control_router(
        protect_enterprise_router(
            runtime_control_router(RuntimeControlState {
                config_path: config_path.to_string(),
                workspace_root: workspace_root.clone(),
                pool: pool.clone(),
                memory_store: memory_store.clone(),
                core_memory_store: core_memory_store.clone(),
                session_manager: session_manager.clone(),
                channel_registry: channel_registry.clone(),
                langsmith: channel_langsmith_client(&config),
                event_bus: event_bus.clone(),
                channel_agent: channel_agent.clone(),
                gateway_addr: addr.clone(),
                started_at,
                sidecar_running: sidecar.is_some(),
            }),
            EnterpriseAccessState {
                workspace_root: workspace_root.clone(),
            },
        ),
        control_auth_state,
    ));
    app = app.layer(metrics_middleware());

    runtime::mark_runtime_applied(config_path, &workspace_root)?;
    match runtime::scan_runtime_health(config_path, &workspace_root).await {
        Ok(report) => {
            if report.degraded_control_plane_mode {
                warn!(
                    default_provider = %report.default_provider,
                    recommended_provider = ?report.recommended_control_plane_provider,
                    "Runtime is starting in degraded control-plane mode; a fallback provider is healthy but the default provider is not"
                );
            }
        }
        Err(error) => {
            warn!(error = %error, "Failed to validate runtime fallback health during startup");
        }
    }
    let _ = runtime::refresh_runtime_beacon(
        config_path,
        &workspace_root,
        &addr,
        Some(started_at),
        sidecar.is_some(),
    )
    .await;
    let _ = services::channel_probes_status(config_path, &workspace_root, true).await;
    let (channel_restart_tx, channel_restart_rx) = watch::channel::<Option<String>>(None);

    let runtime_health_config_path = config_path.to_string();
    let runtime_health_root = workspace_root.clone();
    let runtime_health_events = event_bus.clone();
    channel_tasks.push(tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(300));
        loop {
            ticker.tick().await;
            match runtime::scan_runtime_health(&runtime_health_config_path, &runtime_health_root)
                .await
            {
                Ok(report) => {
                    let _ = runtime_health_events
                        .publish_named(
                            "runtime.health_scanned",
                            "runtime_control",
                            None,
                            &serde_json::json!(report),
                            None,
                        )
                        .await;
                }
                Err(error) => {
                    warn!(error = %error, "Failed to refresh runtime health report");
                }
            }
        }
    }));

    let beacon_config_path = config_path.to_string();
    let beacon_root = workspace_root.clone();
    let beacon_addr = addr.clone();
    let beacon_started_at = started_at;
    let beacon_sidecar_running = sidecar.is_some();
    let beacon_events = event_bus.clone();
    channel_tasks.push(tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            match runtime::refresh_runtime_beacon(
                &beacon_config_path,
                &beacon_root,
                &beacon_addr,
                Some(beacon_started_at),
                beacon_sidecar_running,
            )
            .await
            {
                Ok(beacon) => {
                    let _ = beacon_events
                        .publish_named(
                            "runtime.beacon_refreshed",
                            "runtime_control",
                            None,
                            &serde_json::json!(beacon),
                            None,
                        )
                        .await;
                }
                Err(error) => {
                    warn!(error = %error, "Failed to refresh runtime beacon");
                }
            }
        }
    }));

    if config.channels.runtime.health_monitor_enabled {
        let probe_config_path = config_path.to_string();
        let probe_root = workspace_root.clone();
        let probe_events = event_bus.clone();
        let probe_restart_tx = channel_restart_tx.clone();
        let probe_interval_secs = config.channels.runtime.probe_interval_secs.max(15);
        channel_tasks.push(tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(probe_interval_secs));
            loop {
                ticker.tick().await;
                match services::channel_probes_status(&probe_config_path, &probe_root, true).await {
                    Ok(report) => {
                        let _ = probe_events
                            .publish_named(
                                "runtime.channel_probes_scanned",
                                "runtime_control",
                                None,
                                &serde_json::json!(report),
                                None,
                            )
                            .await;
                        if report.monitor.restart_requested
                            && let Some(reason) = report.monitor.restart_reason.clone()
                        {
                            let _ = probe_events
                                .publish_named(
                                    "runtime.channel_restart_requested",
                                    "runtime_control",
                                    None,
                                    &serde_json::json!({
                                        "reason": reason,
                                        "monitor": report.monitor,
                                    }),
                                    None,
                                )
                                .await;
                            let _ = probe_restart_tx.send_replace(Some(reason));
                        }
                    }
                    Err(error) => {
                        warn!(error = %error, "Failed to refresh channel readiness probes");
                    }
                }
            }
        }));
    }

    #[derive(Debug)]
    enum ShutdownReason {
        Signal,
        ChannelRestart(String),
    }

    // Create shutdown signal handler
    let mut shutdown_restart_rx = channel_restart_rx.clone();
    let shutdown = async move {
        let mut sigterm = match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(sig) => sig,
            Err(e) => {
                error!(error = %e, "Failed to create SIGTERM handler");
                return ShutdownReason::Signal;
            }
        };
        let mut sigint = match signal::unix::signal(signal::unix::SignalKind::interrupt()) {
            Ok(sig) => sig,
            Err(e) => {
                error!(error = %e, "Failed to create SIGINT handler");
                return ShutdownReason::Signal;
            }
        };

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down...");
                ShutdownReason::Signal
            },
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down...");
                ShutdownReason::Signal
            },
            result = shutdown_restart_rx.changed() => {
                match result {
                    Ok(()) => {
                        let reason = shutdown_restart_rx
                            .borrow()
                            .clone()
                            .unwrap_or_else(|| "Channel health monitor requested restart".to_string());
                        warn!(reason = %reason, "Channel health monitor triggered managed restart");
                        ShutdownReason::ChannelRestart(reason)
                    }
                    Err(_) => ShutdownReason::Signal,
                }
            },
        }
    };

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("OpenRustClaw is ready!");
    info!("Gateway: http://{}", addr);
    info!("Prometheus metrics: http://{}/metrics", addr);
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
    log_assistant_handoff(&session_store, &workspace_root).await;

    // Run server with graceful shutdown
    let shutdown_reason = tokio::select! {
        result = axum::serve(listener, app) => {
            result.context("Server error")?;
            None
        }
        reason = shutdown => {
            info!("Shutdown signal received, stopping server...");
            Some(reason)
        }
    };

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
    if let Some(ShutdownReason::ChannelRestart(reason)) = shutdown_reason {
        anyhow::bail!(reason);
    }
    Ok(())
}

async fn log_assistant_handoff(session_store: &SqliteSessionStore, workspace_root: &Path) {
    let user_id = std::env::var("USER").unwrap_or_else(|_| "cli_user".to_string());
    let route_key = assistant::cli_route_key(&user_id, workspace_root);
    let active_cli_session = session_store
        .find_active_by_route_key(&route_key)
        .await
        .ok()
        .flatten()
        .is_some();
    let handoff = assistant::startup_handoff_json(active_cli_session);
    info!(
        active_cli_session = active_cli_session,
        command = %handoff["command"].as_str().unwrap_or("openrustclaw assistant"),
        "{}",
        handoff["message"].as_str().unwrap_or("Assistant handoff available.")
    );
}

fn gate_nonshipping_channels(config: &mut openrustclaw_core::config::ChannelsConfig) {
    if config.gmail_pubsub.enabled {
        warn!(
            "Gmail Pub/Sub is on a partial shipped path; deeper operator polish is still incomplete"
        );
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
    let config = runtime::load_effective_config(config_path, &workspace_root)?;
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

fn validate_gateway_network_mode(config: &AppConfig) -> Result<()> {
    let mode = config.gateway.network_mode.trim().to_lowercase();
    let host = config.gateway.host.trim().to_lowercase();
    let is_loopback = matches!(host.as_str(), "127.0.0.1" | "::1" | "localhost");
    let is_wildcard = matches!(host.as_str(), "0.0.0.0" | "::");

    match mode.as_str() {
        "loopback" => {
            if !is_loopback {
                anyhow::bail!(
                    "gateway.network_mode=loopback requires a loopback bind host, got '{}'",
                    config.gateway.host
                );
            }
        }
        "lan" | "remote" => {
            if is_loopback {
                anyhow::bail!(
                    "gateway.network_mode={} requires a non-loopback bind host, got '{}'",
                    mode,
                    config.gateway.host
                );
            }
            if mode == "remote" && config.gateway.allowed_origins.is_empty() {
                anyhow::bail!("gateway.network_mode=remote requires at least one allowed origin");
            }
            if mode == "remote"
                && !config.security.require_auth
                && config.security.control_api_token_env.is_none()
                && config.security.trusted_proxy_token_env.is_none()
            {
                anyhow::bail!(
                    "gateway.network_mode=remote requires direct auth or an explicit control/proxy token gate"
                );
            }
        }
        other => {
            anyhow::bail!(
                "Unknown gateway.network_mode '{}'. Expected loopback, lan, or remote",
                other
            );
        }
    }

    if is_wildcard && mode == "loopback" {
        anyhow::bail!(
            "gateway.network_mode=loopback cannot be used with wildcard bind host '{}'",
            config.gateway.host
        );
    }

    Ok(())
}

fn spawn_channel_task(
    channel: SharedChannel,
    channel_agent: Option<SharedChannelAgent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let platform = channel.platform();
        let mut route_sessions = HashMap::new();
        info!(platform = ?platform, "Starting channel receive loop");
        loop {
            let receive_started_at = std::time::Instant::now();
            match channel.receive().await {
                Ok(message) => {
                    let ingress_tool_name = format!("channels.{}.ingress", platform);
                    record_operator_tool_status(&ingress_tool_name, receive_started_at, "success");
                    info!(
                        platform = ?platform,
                        user_id = %message.user_id,
                        session_id = %message.session_id,
                        content = %message.content,
                        "Channel received incoming message"
                    );
                    if let Some(agent) = channel_agent.as_ref() {
                        match agent
                            .read()
                            .await
                            .handle_incoming_message(&mut route_sessions, message)
                            .await
                        {
                            Ok(Some(reply)) => {
                                for outbound in expand_outgoing_message(reply) {
                                    let send_started_at = std::time::Instant::now();
                                    let send_result = channel.send(outbound.clone()).await;
                                    let send_tool_name = format!("channels.{}.send", platform);
                                    record_operator_tool_result(
                                        &send_tool_name,
                                        send_started_at,
                                        &send_result,
                                    );
                                    if let Err(e) = send_result {
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
                    let ingress_tool_name = format!("channels.{}.ingress", platform);
                    let failed = Err::<(), _>(&e);
                    record_operator_tool_result(&ingress_tool_name, receive_started_at, &failed);
                    error!(platform = ?platform, error = %e, "Channel receive failed");
                    break;
                }
            }
        }

        if let Some(agent) = channel_agent.as_ref() {
            agent
                .read()
                .await
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
    voice_transcriber: Option<Arc<InboundVoiceTranscriber>>,
    workspace_root: PathBuf,
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
    channel_extension: Option<BoundChannelExtension>,
}

#[derive(Debug, Clone)]
struct BoundChannelExtension {
    skill_name: String,
    service: Option<String>,
    component: Option<String>,
    trigger: String,
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

        let started_at = std::time::Instant::now();
        let send_result = channel.send(message).await.map_err(|error| {
            openrustclaw_core::error::SchedulerError::WorkflowFailed(error.to_string())
        });
        let tool_name = format!("channels.{}.send", platform);
        record_operator_tool_result(&tool_name, started_at, &send_result);
        send_result
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
        let incoming = if let Some(transcriber) = self.voice_transcriber.as_ref() {
            transcriber.enrich_incoming_message(incoming).await
        } else {
            incoming
        };
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
                    Some(assistant::session_metadata(
                        "channel",
                        Some(&route_key),
                        Some(&self.workspace_root),
                        serde_json::json!({
                            "channel_account_id": decision.account_id,
                            "binding_id": decision.binding_id,
                            "agent_id": decision.agent_id,
                            "assistant_scope": decision.session_type.to_string(),
                        }),
                    )),
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
        if let Err(error) = self
            .maybe_schedule_channel_extension(
                &incoming,
                trimmed_content,
                route_state.session_id,
                &route_key,
                &decision,
            )
            .await
        {
            warn!(error = %error, route_key = %route_key, "Failed to schedule bound channel extension");
        }
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
        let preview = {
            let mut registry = self.channel_registry.write().await;
            preview_route(&mut registry, incoming, &self.session_routing)?
        };

        if matches!(
            preview.status,
            ChannelRouteStatus::Blocked | ChannelRouteStatus::Disabled
        ) {
            return Ok(None);
        }
        if preview.status == ChannelRouteStatus::PendingApproval {
            let identity = super::channels::identity_from_message(incoming);
            let _ = self
                .event_bus
                .publish_named(
                    "channel.pairing_requested",
                    "channel_pairing",
                    None,
                    &serde_json::json!({
                        "platform": incoming.platform.to_string(),
                        "account_id": preview.account_id,
                        "user_id": incoming.user_id,
                        "workspace_id": identity.workspace_id,
                    }),
                    None,
                )
                .await;
            return Ok(None);
        }

        let identity = super::channels::identity_from_message(incoming);
        let session_type = if identity.is_group {
            SessionType::Group
        } else {
            SessionType::Dm
        };

        Ok(Some(ChannelRouteDecision {
            route_key: preview.route_key,
            session_type,
            workspace_id: preview.workspace_id,
            agent_id: preview.agent_id,
            activation_mode: preview.activation_mode,
            send_policy: preview.send_policy,
            account_id: preview.account_id,
            binding_id: preview.binding_id.clone(),
            channel_extension: {
                let registry = self.channel_registry.read().await;
                let binding = preview.binding_id.as_deref().and_then(|binding_id| {
                    registry
                        .bindings
                        .iter()
                        .find(|value| value.id == binding_id)
                });
                parse_channel_extension_binding(binding)
            },
        }))
    }

    async fn maybe_schedule_channel_extension(
        &self,
        incoming: &openrustclaw_core::types::IncomingMessage,
        trimmed_content: &str,
        session_id: Uuid,
        route_key: &str,
        decision: &ChannelRouteDecision,
    ) -> Result<()> {
        let Some(extension) = decision.channel_extension.as_ref() else {
            return Ok(());
        };
        if !channel_extension_should_trigger(extension, incoming) {
            return Ok(());
        }

        let input = serde_json::json!({
            "channel": incoming.platform.to_string(),
            "user_id": incoming.user_id,
            "content": trimmed_content,
            "metadata": incoming.metadata,
            "session_id": session_id,
            "route_key": route_key,
            "workspace_id": decision.workspace_id,
            "agent_id": decision.agent_id,
            "binding_id": decision.binding_id,
        });
        let run_at = chrono::Utc::now().to_rfc3339();
        let input_json = input.to_string();
        skills::schedule_background_service_data(
            &extension.skill_name,
            skills::SkillScheduleBackgroundOptions {
                service: extension.service.as_deref(),
                component: extension.component.as_deref(),
                input: Some(input_json.as_str()),
                every_seconds: None,
                at: Some(run_at.as_str()),
                priority: 90,
            },
        )
        .await?;
        Ok(())
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
    let runtime = assistant::build_runtime(
        provider,
        memory_store,
        core_memory_store.clone(),
        workspace_root.clone(),
    );
    let voice_transcriber =
        InboundVoiceTranscriber::try_from_config(config, &workspace_root)?.map(Arc::new);

    Ok(ChannelAgent {
        runtime: Arc::new(runtime),
        session_manager,
        core_memory_store: Some(core_memory_store),
        max_history_messages: 24,
        session_routing: config.session_routing.clone(),
        channel_registry,
        langsmith,
        event_bus,
        voice_transcriber,
        workspace_root,
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

        match runtime::create_provider_from_config(&provider_name, config) {
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
    registry_channel_route_key_with_binding(
        message,
        direct_strategy,
        group_strategy,
        thread_overrides_channel,
        workspace_id,
        agent_id,
        account_id,
    )
}

fn parse_channel_extension_binding(
    binding: Option<&ChannelBindingSpec>,
) -> Option<BoundChannelExtension> {
    let binding = binding?;
    let extension = binding
        .metadata
        .get("skill_channel_extension")?
        .as_object()?;
    Some(BoundChannelExtension {
        skill_name: extension.get("skill_name")?.as_str()?.to_string(),
        service: extension
            .get("service")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        component: extension
            .get("component")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        trigger: extension
            .get("trigger")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("message")
            .to_string(),
    })
}

fn channel_extension_should_trigger(
    extension: &BoundChannelExtension,
    incoming: &openrustclaw_core::types::IncomingMessage,
) -> bool {
    match extension.trigger.as_str() {
        "mentioned" => !message_is_group_candidate(incoming) || message_bot_mentioned(incoming),
        _ => true,
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
    registry_channel_scope_from_metadata(metadata, thread_overrides_channel)
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

#[derive(Clone)]
struct TelegramIngressState {
    handler: Arc<TelegramWebhookHandler>,
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

fn telegram_ingress_router(
    handler: TelegramWebhookHandler,
    langsmith: Option<LangSmithClient>,
) -> Router {
    Router::new()
        .route("/webhooks/telegram/events", post(telegram_events_handler))
        .with_state(TelegramIngressState {
            handler: Arc::new(handler),
            langsmith,
        })
}

async fn slack_events_handler(
    State(state): State<SlackIngressState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
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
            record_operator_tool_status("channels.slack.ingress", started_at, "success");
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
            record_operator_tool_status("channels.slack.ingress", started_at, "success");
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
            record_operator_tool_status("channels.slack.ingress", started_at, "failure");
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
            record_operator_tool_status("channels.slack.ingress", started_at, "failure");
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
            record_operator_tool_status("channels.slack.ingress", started_at, "failure");
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

async fn telegram_events_handler(
    State(state): State<TelegramIngressState>,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let mut trace = ingress_trace(
        state.langsmith.as_ref(),
        "telegram_ingress",
        serde_json::json!({
            "body_bytes": body.len(),
        }),
    );
    start_ingress_trace(state.langsmith.as_ref(), trace.as_ref()).await;

    match state.handler.handle_event(&body).await {
        Ok(()) => {
            record_operator_tool_status("channels.telegram.ingress", started_at, "success");
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
        Err(error) => {
            record_operator_tool_status("channels.telegram.ingress", started_at, "failure");
            let message = error.to_string();
            warn!(error = %message, "Failed to handle Telegram webhook");
            complete_ingress_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(serde_json::json!({
                    "status": StatusCode::BAD_REQUEST.as_u16(),
                    "outcome": "error",
                })),
                Some(message.clone()),
            )
            .await;
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": message })),
            )
                .into_response()
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

type SharedChannelAgent = Arc<tokio::sync::RwLock<ChannelAgent>>;

#[derive(Clone)]
struct ControlPlaneApiState {
    control_root: PathBuf,
    workspace_root: PathBuf,
    config_path: String,
}

#[derive(Clone)]
struct RuntimeControlState {
    config_path: String,
    workspace_root: PathBuf,
    pool: sqlx::SqlitePool,
    memory_store: Arc<SqliteMemoryStore>,
    core_memory_store: Arc<SqliteCoreMemoryStore>,
    session_manager: Arc<SessionManager>,
    channel_registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
    langsmith: Option<LangSmithClient>,
    event_bus: DurableEventBus,
    channel_agent: Option<SharedChannelAgent>,
    gateway_addr: String,
    started_at: DateTime<Utc>,
    sidecar_running: bool,
}

fn record_operator_tool_result<T, E>(
    tool_name: &str,
    started_at: std::time::Instant,
    result: &std::result::Result<T, E>,
) where
    E: std::fmt::Display,
{
    let status = if result.is_ok() { "success" } else { "failure" };
    let error = result.as_ref().err().map(|value| value.to_string());
    openrustclaw_observability::metrics::record_tool_execution(tool_name, status);
    openrustclaw_observability::metrics::record_tool_duration(
        tool_name,
        started_at.elapsed().as_secs_f64(),
    );
    persist_operator_execution_record(
        tool_name,
        "runtime_tool",
        status,
        started_at,
        error.as_deref(),
        None,
        None,
        None,
    );
}

fn record_operator_tool_status(tool_name: &str, started_at: std::time::Instant, status: &str) {
    openrustclaw_observability::metrics::record_tool_execution(tool_name, status);
    openrustclaw_observability::metrics::record_tool_duration(
        tool_name,
        started_at.elapsed().as_secs_f64(),
    );
    persist_operator_execution_record(
        tool_name,
        "runtime_tool",
        status,
        started_at,
        None,
        None,
        None,
        None,
    );
}

fn persist_operator_execution_record(
    tool_name: &str,
    source: &str,
    status: &str,
    started_at: std::time::Instant,
    error: Option<&str>,
    artifact_path: Option<String>,
    args: Option<serde_json::Value>,
    result_preview: Option<serde_json::Value>,
) {
    let Some(workspace_root) = OPERATOR_EXECUTION_ROOT.get() else {
        return;
    };
    let record = inspect::new_tool_execution_record(
        tool_name,
        source,
        status,
        classify_execution_status(status, error),
        started_at.elapsed().as_millis() as u64,
        error.map(ToString::to_string),
        artifact_path,
        args,
        result_preview,
    );
    if let Err(error) = inspect::append_tool_execution_record(workspace_root, &record) {
        warn!(
            tool = %tool_name,
            source = %source,
            error = %error,
            "Failed to persist tool execution record"
        );
    }
}

fn classify_execution_status(status: &str, error: Option<&str>) -> String {
    if status != "failure" {
        return status.to_string();
    }

    let lower = error.unwrap_or("").to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        "timeout".to_string()
    } else if lower.contains("invalid") || lower.contains("parse") || lower.contains("missing") {
        "validation_error".to_string()
    } else if lower.contains("not found") {
        "not_found".to_string()
    } else if lower.contains("forbidden")
        || lower.contains("denied")
        || lower.contains("refused")
        || lower.contains("not allowed")
        || lower.contains("blocked")
    {
        "refused".to_string()
    } else {
        "failure".to_string()
    }
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
            "/control/channels/accounts",
            get(channel_registry_accounts_handler).post(channel_registry_create_account_handler),
        )
        .route(
            "/control/channels/accounts/{id}",
            get(channel_registry_account_handler)
                .put(channel_registry_update_account_handler)
                .delete(channel_registry_delete_account_handler),
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
        .route(
            "/control/channels/bindings",
            post(channel_registry_bind_handler),
        )
        .route(
            "/control/channels/bindings/{id}",
            get(channel_registry_binding_handler)
                .put(channel_registry_update_binding_handler)
                .delete(channel_registry_delete_binding_handler),
        )
        .with_state(ChannelRegistryApiState { registry })
}

fn control_plane_router(state: ControlPlaneApiState) -> Router {
    Router::new()
        .route("/control/runtime", get(control_runtime_handler))
        .route("/control/autonomy", get(control_autonomy_handler))
        .route(
            "/control/autonomy/lessons",
            get(control_autonomy_lessons_handler).post(control_autonomy_create_lesson_handler),
        )
        .route(
            "/control/autonomy/lessons/{id}/deactivate",
            post(control_autonomy_deactivate_lesson_handler),
        )
        .route(
            "/control/config",
            get(control_config_handler).put(control_config_update_handler),
        )
        .route(
            "/control/config/validate",
            post(control_config_validate_handler),
        )
        .route("/control/diagnostics", get(control_diagnostics_handler))
        .route(
            "/control/diagnostics/ws",
            get(control_diagnostics_ws_handler),
        )
        .with_state(state)
}

fn runtime_control_router(state: RuntimeControlState) -> Router {
    let router = Router::new()
        .route("/control/ui", get(control_ui_handler))
        .route("/control/runtime/status", get(runtime_status_handler))
        .route(
            "/control/runtime/operator-ops",
            get(runtime_operator_ops_handler),
        )
        .route(
            "/control/self-hosted/product-mode",
            get(self_hosted_product_mode_handler).post(self_hosted_product_mode_transition_handler),
        )
        .route("/control/setup/handoff", get(setup_handoff_handler))
        .route("/control/enterprise/access", get(enterprise_access_handler))
        .route("/control/enterprise/admin", get(enterprise_admin_handler))
        .route(
            "/control/enterprise/autonomy",
            get(enterprise_autonomy_handler),
        )
        .route(
            "/control/enterprise/access/bootstrap",
            post(enterprise_access_bootstrap_handler),
        )
        .route(
            "/control/enterprise/access/operators",
            post(enterprise_access_upsert_operator_handler),
        )
        .route(
            "/control/enterprise/governance/rules",
            post(enterprise_governance_upsert_rule_handler),
        )
        .route(
            "/control/enterprise/autonomy/enable",
            post(enterprise_autonomy_enable_handler),
        )
        .route(
            "/control/enterprise/autonomy/disable",
            post(enterprise_autonomy_disable_handler),
        )
        .route(
            "/control/enterprise/autonomy/kill-switch",
            post(enterprise_autonomy_kill_switch_handler),
        )
        .route(
            "/control/enterprise/foundations",
            get(enterprise_foundations_handler),
        )
        .route(
            "/control/enterprise/policy",
            get(enterprise_policy_handler).put(enterprise_policy_update_handler),
        )
        .route(
            "/control/enterprise/audit/review",
            get(enterprise_audit_review_handler),
        )
        .route(
            "/control/enterprise/audit/export",
            post(enterprise_audit_export_handler),
        )
        .route("/control/security/posture", get(security_posture_handler))
        .route("/control/runtime/health", get(runtime_health_handler))
        .route("/control/runtime/beacon", get(runtime_beacon_handler))
        .route("/control/voice/status", get(voice_status_handler))
        .route("/control/voice/providers", get(voice_providers_handler))
        .route("/control/voice/metrics", get(voice_metrics_handler))
        .route(
            "/control/voice/operator-summary",
            get(voice_operator_report_handler),
        )
        .route("/control/voice/outcomes", get(voice_outcomes_handler))
        .route("/control/voice/sessions", get(voice_sessions_handler))
        .route(
            "/control/voice/sessions/health",
            get(voice_session_health_handler),
        )
        .route(
            "/control/voice/sessions/reap",
            post(voice_reap_sessions_handler),
        )
        .route(
            "/control/voice/sessions/start",
            post(voice_start_session_handler),
        )
        .route("/control/voice/sessions/{id}", get(voice_session_handler))
        .route(
            "/control/voice/sessions/{id}/metrics",
            get(voice_session_metrics_handler),
        )
        .route(
            "/control/voice/sessions/{id}/append-user",
            post(voice_append_user_handler),
        )
        .route(
            "/control/voice/sessions/{id}/transcript",
            get(voice_session_transcript_handler),
        )
        .route(
            "/control/voice/sessions/{id}/artifacts",
            get(voice_session_artifacts_handler),
        )
        .route(
            "/control/voice/sessions/{id}/events",
            get(voice_session_events_handler),
        )
        .route(
            "/control/voice/sessions/{id}/respond",
            post(voice_respond_handler),
        )
        .route(
            "/control/voice/sessions/{id}/reconnect",
            post(voice_reconnect_session_handler),
        )
        .route(
            "/control/voice/sessions/{id}/pause",
            post(voice_pause_session_handler),
        )
        .route(
            "/control/voice/sessions/{id}/resume",
            post(voice_resume_session_handler),
        )
        .route(
            "/control/voice/sessions/{id}/interrupt",
            post(voice_interrupt_session_handler),
        )
        .route(
            "/control/voice/sessions/{id}/end",
            post(voice_end_session_handler),
        )
        .route("/control/voice/voices", get(voice_voices_handler))
        .route("/control/voice/prewarm", post(voice_prewarm_handler))
        .route("/control/voice/transcribe", post(voice_transcribe_handler))
        .route("/control/voice/synthesize", post(voice_synthesize_handler));

    #[cfg(feature = "voice")]
    let router = router
        .route("/control/talk/status", get(talk_status_handler))
        .route("/control/talk/metrics", get(talk_metrics_handler))
        .route("/control/talk/sessions", get(talk_sessions_handler))
        .route("/control/talk/sessions/{id}", get(talk_session_handler))
        .route(
            "/control/talk/sessions/{id}/events",
            get(talk_session_events_handler),
        );
    #[cfg(feature = "voice")]
    let router = router.route(
        "/control/talk/sessions/{id}/metrics",
        get(talk_session_metrics_handler),
    );

    let router = router
        .route("/control/mobile/pairings", get(mobile_pairings_handler))
        .route("/control/mobile/nodes", get(mobile_nodes_handler))
        .route("/control/mobile/nodes/pair", post(mobile_pair_handler))
        .route("/control/mobile/nodes/{id}", get(mobile_node_handler))
        .route(
            "/control/mobile/nodes/{id}/summary",
            get(mobile_node_summary_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/unpair",
            post(mobile_node_unpair_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/status",
            get(mobile_node_status_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/app-sessions",
            get(mobile_node_app_sessions_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/push",
            get(mobile_node_push_handler).post(mobile_node_register_push_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/sync",
            get(mobile_node_sync_handler).post(mobile_node_report_sync_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/sync-conflicts",
            get(mobile_node_sync_conflicts_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/capabilities",
            get(mobile_node_capabilities_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/activity",
            get(mobile_node_activity_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/runtime",
            get(mobile_node_runtime_handler),
        )
        .route(
            "/control/mobile/metrics",
            get(control_mobile_metrics_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/heartbeat",
            post(mobile_node_heartbeat_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/wake",
            post(mobile_node_wake_handler),
        )
        .route(
            "/control/mobile/nodes/{id}/rehydrate",
            post(mobile_node_rehydrate_handler),
        )
        .route("/control/mobile/commands", get(mobile_commands_handler))
        .route(
            "/control/mobile/commands/dispatch",
            post(mobile_command_dispatch_handler),
        )
        .route("/control/mobile/commands/{id}", get(mobile_command_handler))
        .route(
            "/control/mobile/commands/{id}/events",
            get(mobile_command_events_handler),
        )
        .route(
            "/control/mobile/commands/metrics",
            get(mobile_command_metrics_handler),
        )
        .route(
            "/control/mobile/commands/{id}/approve",
            post(mobile_command_approve_handler),
        )
        .route(
            "/control/mobile/commands/{id}/reject",
            post(mobile_command_reject_handler),
        )
        .route(
            "/control/mobile/notifications",
            get(mobile_notifications_handler).post(mobile_notification_send_handler),
        )
        .route(
            "/control/mobile/notifications/{id}",
            get(mobile_notification_handler),
        )
        .route(
            "/control/mobile/notifications/{id}/ack",
            post(mobile_notification_ack_handler),
        )
        .route(
            "/control/mobile/inbox",
            get(mobile_inbox_handler).post(mobile_inbox_report_handler),
        )
        .route(
            "/control/mobile/inbox/{id}",
            get(mobile_inbox_message_handler),
        )
        .route(
            "/control/mobile/inbox/{id}/ack",
            post(mobile_inbox_ack_handler),
        )
        .route(
            "/control/mobile/outbox",
            get(mobile_outbox_handler).post(mobile_outbox_send_handler),
        )
        .route(
            "/control/mobile/outbox/{id}",
            get(mobile_outbox_message_handler),
        )
        .route(
            "/control/mobile/outbox/{id}/ack",
            post(mobile_outbox_ack_handler),
        )
        .route(
            "/control/mobile/sync-conflicts",
            get(mobile_sync_conflicts_handler).post(mobile_sync_conflict_report_handler),
        )
        .route(
            "/control/mobile/sync-conflicts/{id}",
            get(mobile_sync_conflict_handler),
        )
        .route(
            "/control/mobile/sync-conflicts/{id}/resolve",
            post(mobile_sync_conflict_resolve_handler),
        )
        .route(
            "/control/mobile/app-sessions",
            get(mobile_app_sessions_handler),
        )
        .route(
            "/control/mobile/app-sessions/metrics",
            get(mobile_app_session_metrics_handler),
        )
        .route(
            "/control/mobile/app-sessions/{id}",
            get(mobile_app_session_handler),
        )
        .route(
            "/control/mobile/app-sessions/{id}/events",
            get(mobile_app_session_events_handler),
        )
        .route(
            "/control/mobile/messages/preview",
            post(mobile_message_preview_handler),
        )
        .route(
            "/control/mobile/notifications/preview",
            post(mobile_notification_preview_handler),
        )
        .route(
            "/control/mobile/sync/preview",
            post(mobile_sync_preview_handler),
        )
        .route(
            "/control/mobile/capabilities/preview",
            post(mobile_capability_preview_handler),
        )
        .route(
            "/control/mobile/capabilities/execute",
            post(mobile_capability_execute_handler),
        )
        .route(
            "/control/mobile/capabilities/executions",
            get(mobile_capability_executions_handler),
        )
        .route(
            "/control/mobile/capabilities/executions/{id}",
            get(mobile_capability_execution_handler),
        )
        .route(
            "/control/mobile/media-artifacts",
            get(mobile_media_artifacts_handler),
        )
        .route(
            "/control/mobile/media-artifacts/{id}",
            get(mobile_media_artifact_handler),
        )
        .route("/control/media/providers", get(media_providers_handler))
        .route("/control/media/inspect", post(media_inspect_handler))
        .route(
            "/control/media/extract-text",
            post(media_extract_text_handler),
        )
        .route("/control/media/describe", post(media_describe_handler))
        .route(
            "/control/runtime/reload-plan",
            get(runtime_reload_plan_handler),
        )
        .route(
            "/control/runtime/upgrade-plan",
            get(runtime_upgrade_plan_handler),
        )
        .route(
            "/control/runtime/self-update-plan",
            get(runtime_self_update_plan_handler),
        )
        .route(
            "/control/runtime/rollback-plan",
            get(runtime_rollback_plan_handler),
        )
        .route("/control/tools", get(tool_status_handler))
        .route(
            "/control/tool-executions",
            get(tool_execution_history_handler),
        )
        .route(
            "/control/coding-artifacts",
            get(coding_artifact_history_handler),
        )
        .route("/control/tools/add", post(tool_add_handler))
        .route("/control/tools/setup", post(tool_setup_handler))
        .route("/control/tools/sync", post(tool_sync_handler))
        .route("/control/tools/{name}", get(tool_detail_handler))
        .route(
            "/control/runtime/health/scan",
            post(runtime_health_scan_handler),
        )
        .route("/control/runtime/reload", post(runtime_reload_handler))
        .route(
            "/control/runtime/switch-provider",
            post(runtime_switch_provider_handler),
        )
        .route(
            "/control/runtime/switch-model",
            post(runtime_switch_model_handler),
        )
        .route("/control/runtime/vault", get(runtime_vault_status_handler))
        .route(
            "/control/runtime/vault/{key}",
            put(runtime_vault_set_handler).delete(runtime_vault_delete_handler),
        )
        .route(
            "/control/orchestration/resolve",
            post(orchestration_resolve_handler),
        )
        .route(
            "/control/orchestration/run",
            post(orchestration_run_handler),
        )
        .route(
            "/control/orchestration/submit",
            post(orchestration_submit_handler),
        )
        .route(
            "/control/orchestration/active",
            get(orchestration_active_runs_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}",
            get(orchestration_active_run_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/events",
            get(orchestration_active_run_events_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/supervision",
            get(orchestration_active_run_supervision_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/pause",
            post(orchestration_active_run_pause_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/resume",
            post(orchestration_active_run_resume_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/kill",
            post(orchestration_active_run_kill_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/escalate",
            post(orchestration_active_run_escalate_handler),
        )
        .route(
            "/control/orchestration/active/{run_id}/rollback",
            post(orchestration_active_run_rollback_handler),
        )
        .route(
            "/control/orchestration/runs",
            get(orchestration_runs_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}",
            get(orchestration_run_receipt_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/checkpoints",
            get(orchestration_run_checkpoints_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/supervision",
            get(orchestration_run_supervision_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/trace",
            get(orchestration_run_trace_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/transcript",
            get(orchestration_run_transcript_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/resources",
            get(orchestration_run_resources_handler),
        )
        .route(
            "/control/orchestration/runs/{receipt_id}/reflection-candidates/{index}/promote",
            post(orchestration_promote_reflection_candidate_handler),
        )
        .route("/control/browser/navigate", post(browser_navigate_handler))
        .route(
            "/control/browser/sessions",
            post(browser_open_session_handler),
        )
        .route(
            "/control/browser/sessions",
            get(browser_list_sessions_handler),
        )
        .route("/control/browser/inspect", post(browser_inspect_handler))
        .route(
            "/control/browser/run-sequence",
            post(browser_run_sequence_handler),
        )
        .route(
            "/control/browser/read-page",
            post(browser_read_page_handler),
        )
        .route(
            "/control/browser/crawl-site",
            post(browser_crawl_site_handler),
        )
        .route("/control/browser/extract", post(browser_extract_handler))
        .route("/control/browser/artifacts", get(browser_artifacts_handler))
        .route(
            "/control/browser/backend-policy",
            get(browser_backend_policy_handler),
        )
        .route(
            "/control/browser/backend-audit",
            get(browser_backend_audit_handler),
        )
        .route(
            "/control/browser/workflow-history",
            get(browser_workflow_history_handler),
        )
        .route("/control/skills", get(control_skills_handler))
        .route(
            "/control/skills/compile",
            post(control_skill_compile_handler),
        )
        .route(
            "/control/skills/compiled",
            get(control_compiled_skills_handler),
        )
        .route(
            "/control/skills/compiled/{name}",
            get(control_compiled_skill_detail_handler),
        )
        .route(
            "/control/skills/extensions",
            get(control_extension_manifests_handler),
        )
        .route(
            "/control/skills/extensions/{name}",
            get(control_extension_manifest_handler),
        )
        .route(
            "/control/skills/auth-plugins",
            get(control_skill_auth_plugins_handler),
        )
        .route(
            "/control/skills/auth-plugins/bind",
            post(control_skill_bind_auth_plugin_handler),
        )
        .route(
            "/control/skills/auth-plugins/callback",
            get(control_skill_auth_callback_handler),
        )
        .route(
            "/control/skills/voice-plugins",
            get(control_skill_voice_plugins_handler),
        )
        .route(
            "/control/skills/voice-plugins/bind",
            post(control_skill_bind_voice_plugin_handler),
        )
        .route(
            "/control/skills/voice-plugins/{plugin_id}/prewarm",
            post(control_skill_prewarm_voice_plugin_handler),
        )
        .route(
            "/control/skills/voice-calls",
            get(control_skill_voice_calls_handler),
        )
        .route(
            "/control/skills/voice-calls/health",
            get(control_skill_voice_call_health_handler),
        )
        .route(
            "/control/skills/voice-calls/metrics",
            get(control_skill_voice_call_metrics_handler),
        )
        .route(
            "/control/skills/voice-calls/{call_id}/events",
            get(control_skill_voice_call_events_handler),
        )
        .route(
            "/control/skills/voice-calls/{call_id}/artifacts",
            get(control_skill_voice_call_artifacts_handler),
        )
        .route(
            "/control/skills/voice-calls/start",
            post(control_skill_start_voice_call_handler),
        )
        .route(
            "/control/skills/voice-calls/{call_id}/end",
            post(control_skill_end_voice_call_handler),
        )
        .route(
            "/control/skills/voice-calls/{call_id}/reconnect",
            post(control_skill_reconnect_voice_call_handler),
        )
        .route(
            "/control/skills/voice-calls/reap",
            post(control_skill_reap_voice_calls_handler),
        )
        .route(
            "/control/skills/channel-extensions",
            get(control_skill_channel_extensions_handler),
        )
        .route(
            "/control/skills/channel-extensions/bind",
            post(control_skill_bind_channel_extension_handler),
        )
        .route("/control/skills/search", get(control_skills_search_handler))
        .route(
            "/control/skills/popular",
            get(control_skills_popular_handler),
        )
        .route(
            "/control/skills/trending",
            get(control_skills_trending_handler),
        )
        .route(
            "/control/skills/install",
            post(control_skill_install_handler),
        )
        .route("/control/skills/{name}", get(control_skill_detail_handler))
        .route(
            "/control/skills/{name}/compile",
            post(control_skill_compile_by_name_handler),
        )
        .route(
            "/control/skills/{name}/update",
            post(control_skill_update_handler),
        )
        .route(
            "/control/skills/{name}/uninstall",
            post(control_skill_uninstall_handler),
        )
        .route(
            "/control/skills/{name}/verify",
            post(control_skill_verify_handler),
        )
        .route(
            "/control/skills/{name}/invoke",
            post(control_skill_invoke_handler),
        )
        .route(
            "/control/skills/{name}/execute",
            post(control_skill_execute_handler),
        )
        .route(
            "/control/skills/{name}/background-services",
            get(control_skill_background_services_handler),
        )
        .route(
            "/control/skills/{name}/background-services/schedule",
            post(control_skill_schedule_background_handler),
        )
        .route(
            "/control/skills/auth-plugins/{provider_id}/authorize",
            post(control_skill_auth_authorize_handler),
        )
        .route(
            "/control/skills/auth-plugins/{provider_id}/exchange",
            post(control_skill_auth_exchange_handler),
        )
        .route("/control/services/status", get(service_status_handler))
        .route(
            "/control/services/scheduler",
            get(service_scheduler_handler),
        )
        .route(
            "/control/services/runtime-events",
            get(service_runtime_events_handler),
        )
        .route(
            "/control/services/channels",
            get(service_channel_probes_handler),
        )
        .route("/control/services/beacon", get(service_beacon_handler))
        .route("/control/logs/recent", get(control_logs_recent_handler))
        .route("/control/logs/ws", get(control_logs_ws_handler))
        .route("/control/sessions", get(control_sessions_handler))
        .route("/control/sessions/{id}", get(control_session_handler))
        .route(
            "/control/memory/namespaces",
            get(control_memory_namespaces_handler),
        )
        .route(
            "/control/memory/timeline",
            get(control_memory_timeline_handler),
        )
        .route(
            "/control/memory/archive",
            get(control_memory_archive_handler),
        )
        .route("/control/jobs", get(control_jobs_handler))
        .route("/control/jobs/{id}", get(control_job_handler))
        .route(
            "/control/browser/screenshot",
            post(browser_screenshot_handler),
        )
        .route("/control/browser/pdf", post(browser_pdf_handler))
        .with_state(state);

    router
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

async fn channel_registry_accounts_handler(
    State(state): State<ChannelRegistryApiState>,
) -> Json<serde_json::Value> {
    let registry = state.registry.read().await;
    let mut accounts: Vec<_> = registry.accounts.values().cloned().collect();
    accounts.sort_by(|left, right| left.id.cmp(&right.id));
    Json(serde_json::json!({ "accounts": accounts }))
}

async fn channel_registry_account_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let registry = state.registry.read().await;
    match registry.accounts.get(&id) {
        Some(account) => (StatusCode::OK, Json(serde_json::json!(account))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "account not found"})),
        )
            .into_response(),
    }
}

async fn channel_registry_create_account_handler(
    State(state): State<ChannelRegistryApiState>,
    Json(payload): Json<super::channels::ChannelAccountSpec>,
) -> impl IntoResponse {
    upsert_channel_registry_account(state.registry, None, payload).await
}

async fn channel_registry_update_account_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<super::channels::ChannelAccountSpec>,
) -> impl IntoResponse {
    upsert_channel_registry_account(state.registry, Some(id), payload).await
}

async fn channel_registry_delete_account_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
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
    match super::channels::delete_account(Some(&root), &id) {
        Ok(()) => match reload_channel_registry(&state.registry).await {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "account_id": id, "action": "delete"})),
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
    upsert_channel_registry_binding(
        state.registry,
        None,
        ChannelBindingSpec {
            id: payload.id,
            platform: payload.platform,
            enabled: true,
            priority: 100,
            workspace_match: payload.workspace_match,
            account_match: payload.account_match,
            channel_match: payload.channel_match,
            workspace_target: payload.workspace_target,
            agent_id: payload.agent_id,
            activation_mode: payload.activation_mode,
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({}),
        },
    )
    .await
}

async fn channel_registry_binding_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let registry = state.registry.read().await;
    match registry.bindings.iter().find(|binding| binding.id == id) {
        Some(binding) => (StatusCode::OK, Json(serde_json::json!(binding))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "binding not found"})),
        )
            .into_response(),
    }
}

async fn channel_registry_update_binding_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<ChannelBindingSpec>,
) -> impl IntoResponse {
    upsert_channel_registry_binding(state.registry, Some(id), payload).await
}

async fn channel_registry_delete_binding_handler(
    State(state): State<ChannelRegistryApiState>,
    AxumPath(id): AxumPath<String>,
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
    match super::channels::delete_binding(Some(&root), &id) {
        Ok(()) => match reload_channel_registry(&state.registry).await {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "binding_id": id, "action": "delete"})),
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

async fn upsert_channel_registry_account(
    registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
    expected_id: Option<String>,
    payload: super::channels::ChannelAccountSpec,
) -> axum::response::Response {
    if let Some(expected_id) = expected_id.as_deref()
        && payload.id != expected_id
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "account id does not match path"})),
        )
            .into_response();
    }

    let root = { registry.read().await.root.to_string_lossy().to_string() };
    match super::channels::upsert_account(Some(&root), payload.clone()) {
        Ok(()) => match reload_channel_registry(&registry).await {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "account_id": payload.id})),
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

async fn upsert_channel_registry_binding(
    registry: Arc<tokio::sync::RwLock<ChannelRegistry>>,
    expected_id: Option<String>,
    payload: ChannelBindingSpec,
) -> axum::response::Response {
    if let Some(expected_id) = expected_id.as_deref()
        && payload.id != expected_id
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "binding id does not match path"})),
        )
            .into_response();
    }

    let root = { registry.read().await.root.to_string_lossy().to_string() };
    match super::channels::upsert_binding(Some(&root), payload.clone()) {
        Ok(()) => match reload_channel_registry(&registry).await {
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
        "activation" => super::channels::activation(Some(&root), id, mode.as_deref().unwrap_or("")),
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

#[derive(serde::Deserialize, Default)]
struct DiagnosticsQuery {
    #[serde(default)]
    deep: bool,
    #[serde(default)]
    repair: bool,
    #[serde(default)]
    interval_secs: Option<u64>,
}

async fn control_runtime_handler(State(state): State<ControlPlaneApiState>) -> impl IntoResponse {
    match control::describe_registry(state.control_root.clone()) {
        Ok(description) => (StatusCode::OK, Json(description)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_autonomy_handler(State(state): State<ControlPlaneApiState>) -> impl IntoResponse {
    match control::describe_registry(state.control_root.clone()) {
        Ok(description) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "execution_mode": description["execution_mode"].clone(),
                "default_claw": description["default_claw"].clone(),
                "orchestrator_claw": description["orchestrator_claw"].clone(),
                "allow_shared_context": description["allow_shared_context"].clone(),
                "isolation_mode": description["isolation_mode"].clone(),
                "autonomy": description["autonomy"].clone(),
                "decision_lessons": description["decision_lessons"].clone(),
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_autonomy_lessons_handler(
    State(state): State<ControlPlaneApiState>,
) -> impl IntoResponse {
    match control::describe_registry(state.control_root.clone()) {
        Ok(description) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "decision_lessons": description["decision_lessons"].clone(),
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct AutonomyLessonPayload {
    id: String,
    #[serde(default = "default_active_true")]
    active: bool,
    signal: String,
    recommendation: String,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    claw_id: Option<String>,
    #[serde(default)]
    model_profile_id: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    autonomy_level: Option<String>,
    #[serde(default)]
    execution_mode: Option<String>,
}

fn default_active_true() -> bool {
    true
}

async fn control_autonomy_create_lesson_handler(
    State(state): State<ControlPlaneApiState>,
    Json(payload): Json<AutonomyLessonPayload>,
) -> impl IntoResponse {
    let control_root = state.control_root.to_string_lossy().to_string();
    match control::create_lesson(
        Some(&control_root),
        control::NewLessonInput {
            id: &payload.id,
            active: payload.active,
            signal: &payload.signal,
            recommendation: &payload.recommendation,
            rationale: payload.rationale.as_deref(),
            confidence: payload.confidence.unwrap_or(0.7),
            source: payload.source.as_deref(),
            task_id: payload.task_id.as_deref(),
            category: payload.category.as_deref(),
            claw_id: payload.claw_id.as_deref(),
            model_profile_id: payload.model_profile_id.as_deref(),
            provider: payload.provider.as_deref(),
            autonomy_level: payload.autonomy_level.as_deref(),
            execution_mode: payload.execution_mode.as_deref(),
        },
    ) {
        Ok(()) => match control::describe_registry(state.control_root.clone()) {
            Ok(description) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "decision_lessons": description["decision_lessons"].clone(),
                })),
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

async fn control_autonomy_deactivate_lesson_handler(
    State(state): State<ControlPlaneApiState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let control_root = state.control_root.to_string_lossy().to_string();
    match control::deactivate_lesson(Some(&control_root), &id) {
        Ok(()) => match control::describe_registry(state.control_root.clone()) {
            Ok(description) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "decision_lessons": description["decision_lessons"].clone(),
                })),
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

#[derive(serde::Deserialize, Default)]
struct SkillSearchQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct SkillInstallPayload {
    name: String,
}

#[derive(serde::Deserialize)]
struct SkillBindChannelExtensionPayload {
    binding_id: String,
    skill_name: String,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    trigger: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillBindAuthPluginPayload {
    provider_id: String,
    skill_name: String,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    issuer: Option<String>,
    #[serde(default)]
    authorization_endpoint: Option<String>,
    #[serde(default)]
    token_endpoint: Option<String>,
    #[serde(default)]
    client_id_key: Option<String>,
    #[serde(default)]
    client_secret_key: Option<String>,
    #[serde(default)]
    scopes: Option<String>,
    #[serde(default)]
    vault_key_prefix: Option<String>,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    component: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillBindVoicePluginPayload {
    plugin_id: String,
    skill_name: String,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    greeting_text: Option<String>,
    #[serde(default)]
    default_voice: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillCompilePayload {
    #[serde(default)]
    name: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillInvokePayload {
    #[serde(default)]
    args: Option<String>,
    #[serde(default)]
    reference: Option<String>,
    #[serde(default)]
    max_chars: Option<usize>,
    #[serde(default)]
    detail: bool,
}

#[derive(serde::Deserialize, Default)]
struct SkillExecutePayload {
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    input: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillAuthAuthorizePayload {
    #[serde(default)]
    redirect_uri: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillAuthExchangePayload {
    code: String,
    state: String,
    #[serde(default)]
    redirect_uri: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillAuthCallbackQuery {
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillStartVoiceCallPayload {
    plugin_id: String,
    #[serde(default)]
    remote: Option<String>,
    #[serde(default)]
    greeting_text: Option<String>,
    #[serde(default)]
    voice: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
    #[serde(default)]
    stale_after_secs: Option<u64>,
}

#[derive(serde::Deserialize, Default)]
struct SkillEndVoiceCallPayload {
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillReconnectVoiceCallPayload {
    #[serde(default)]
    remote: Option<String>,
    #[serde(default)]
    greeting_text: Option<String>,
    #[serde(default)]
    voice: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
    #[serde(default)]
    stale_after_secs: Option<u64>,
}

#[derive(serde::Deserialize, Default)]
struct SkillPrewarmVoicePluginPayload {
    #[serde(default)]
    greeting_text: Option<String>,
    #[serde(default)]
    voice: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SkillReapVoiceCallsPayload {
    #[serde(default)]
    stale_after_secs: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct SkillScheduleBackgroundPayload {
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    every_seconds: Option<u64>,
    #[serde(default)]
    at: Option<String>,
    #[serde(default = "default_skill_background_priority")]
    priority: i64,
}

fn default_skill_background_priority() -> i64 {
    100
}

async fn control_skills_handler() -> impl IntoResponse {
    match skills::installed_skills_data().await {
        Ok(entries) => (
            StatusCode::OK,
            Json(serde_json::json!({ "skills": entries })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_detail_handler(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    match skills::installed_skill_detail_data(&name).await {
        Ok(skill) => {
            let compiled = skills::compiled_skill_detail_data(&name).await.ok();
            let extension_manifest = skills::extension_manifest_data(&name).await.ok();
            let background_services = skills::background_services_data(&name).await.ok();
            let auth_plugins = skills::auth_plugins_for_skill_data(&name).await.ok();
            let voice_plugins = skills::voice_plugins_for_skill_data(&name).await.ok();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "skill": skill,
                    "compiled": compiled,
                    "extension_manifest": extension_manifest,
                    "background_services": background_services,
                    "auth_plugins": auth_plugins,
                    "voice_plugins": voice_plugins,
                })),
            )
                .into_response()
        }
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skills_search_handler(Query(query): Query<SkillSearchQuery>) -> impl IntoResponse {
    let Some(search) = query.q.as_deref().filter(|value| !value.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "query parameter `q` is required" })),
        )
            .into_response();
    };

    match skills::search_data(
        search,
        query.category.as_deref(),
        query.sort.as_deref().unwrap_or("relevance"),
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skills_popular_handler(
    Query(query): Query<SkillSearchQuery>,
) -> impl IntoResponse {
    match skills::popular_data(query.limit.unwrap_or(12).max(1)).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skills_trending_handler(
    Query(query): Query<SkillSearchQuery>,
) -> impl IntoResponse {
    match skills::trending_data(query.limit.unwrap_or(12).max(1)).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_install_handler(
    Json(payload): Json<SkillInstallPayload>,
) -> impl IntoResponse {
    match skills::install_data(&payload.name).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_compile_handler(
    Json(payload): Json<SkillCompilePayload>,
) -> impl IntoResponse {
    match skills::compile_data(payload.name.as_deref()).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_compiled_skills_handler() -> impl IntoResponse {
    match skills::compiled_skills_data().await {
        Ok(compiled) => (
            StatusCode::OK,
            Json(serde_json::json!({ "compiled": compiled })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_compiled_skill_detail_handler(
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match skills::compiled_skill_detail_data(&name).await {
        Ok(compiled) => (StatusCode::OK, Json(serde_json::json!(compiled))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_extension_manifests_handler() -> impl IntoResponse {
    match skills::extension_manifests_data().await {
        Ok(extensions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "extensions": extensions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_extension_manifest_handler(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    match skills::extension_manifest_data(&name).await {
        Ok(extension) => (StatusCode::OK, Json(serde_json::json!(extension))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_auth_plugins_handler() -> impl IntoResponse {
    match skills::auth_plugins_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_bind_auth_plugin_handler(
    Json(payload): Json<SkillBindAuthPluginPayload>,
) -> impl IntoResponse {
    match skills::bind_auth_plugin_data(
        &payload.provider_id,
        &payload.skill_name,
        skills::SkillBindAuthPluginOptions {
            redirect_uri: payload.redirect_uri.as_deref(),
            issuer: payload.issuer.as_deref(),
            authorization_endpoint: payload.authorization_endpoint.as_deref(),
            token_endpoint: payload.token_endpoint.as_deref(),
            client_id_key: payload.client_id_key.as_deref(),
            client_secret_key: payload.client_secret_key.as_deref(),
            scopes: payload.scopes.as_deref(),
            vault_key_prefix: payload.vault_key_prefix.as_deref(),
            service: payload.service.as_deref(),
            component: payload.component.as_deref(),
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_auth_callback_handler(
    Query(query): Query<SkillAuthCallbackQuery>,
) -> impl IntoResponse {
    if let Some(error_code) = query.error.as_deref() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": error_code,
                "description": query.error_description,
            })),
        )
            .into_response();
    }

    let Some(code) = query.code.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "query parameter `code` is required" })),
        )
            .into_response();
    };
    let Some(state) = query.state.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "query parameter `state` is required" })),
        )
            .into_response();
    };

    match skills::exchange_auth_plugin_callback_data(
        query.provider_id.as_deref(),
        code,
        state,
        query.redirect_uri.as_deref(),
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_plugins_handler() -> impl IntoResponse {
    match skills::voice_plugins_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_bind_voice_plugin_handler(
    Json(payload): Json<SkillBindVoicePluginPayload>,
) -> impl IntoResponse {
    match skills::bind_voice_plugin_data(
        &payload.plugin_id,
        &payload.skill_name,
        skills::SkillBindVoicePluginOptions {
            service: payload.service.as_deref(),
            component: payload.component.as_deref(),
            greeting_text: payload.greeting_text.as_deref(),
            default_voice: payload.default_voice.as_deref(),
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_calls_handler() -> impl IntoResponse {
    match skills::voice_calls_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_call_health_handler() -> impl IntoResponse {
    match skills::voice_call_health_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_call_metrics_handler() -> impl IntoResponse {
    match skills::voice_call_metrics_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_call_events_handler(
    AxumPath(call_id): AxumPath<String>,
) -> impl IntoResponse {
    match skills::voice_call_events_data(&call_id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_voice_call_artifacts_handler(
    AxumPath(call_id): AxumPath<String>,
) -> impl IntoResponse {
    match skills::voice_call_artifacts_data(&call_id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_prewarm_voice_plugin_handler(
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<SkillPrewarmVoicePluginPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::prewarm_voice_plugin_data(
        &plugin_id,
        skills::SkillPrewarmVoicePluginOptions {
            greeting_text: payload.greeting_text.as_deref(),
            voice: payload.voice.as_deref(),
        },
    )
    .await;
    record_operator_tool_result("skills.voice_plugin.prewarm", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_start_voice_call_handler(
    Json(payload): Json<SkillStartVoiceCallPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::start_voice_call_data(
        &payload.plugin_id,
        skills::SkillStartVoiceCallOptions {
            remote: payload.remote.as_deref(),
            greeting_text: payload.greeting_text.as_deref(),
            voice: payload.voice.as_deref(),
            metadata: payload.metadata.as_deref(),
            stale_after_secs: payload.stale_after_secs,
        },
    )
    .await;
    record_operator_tool_result("skills.voice_call.start", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_reap_voice_calls_handler(
    Json(payload): Json<SkillReapVoiceCallsPayload>,
) -> impl IntoResponse {
    match skills::reap_voice_calls_data(skills::SkillReapVoiceCallsOptions {
        stale_after_secs: payload.stale_after_secs,
        limit: payload.limit,
    })
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_end_voice_call_handler(
    AxumPath(call_id): AxumPath<String>,
    Json(payload): Json<SkillEndVoiceCallPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::end_voice_call_data(
        &call_id,
        skills::SkillEndVoiceCallOptions {
            reason: payload.reason.as_deref(),
            metadata: payload.metadata.as_deref(),
        },
    )
    .await;
    record_operator_tool_result("skills.voice_call.end", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_reconnect_voice_call_handler(
    AxumPath(call_id): AxumPath<String>,
    Json(payload): Json<SkillReconnectVoiceCallPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::reconnect_voice_call_data(
        &call_id,
        skills::SkillReconnectVoiceCallOptions {
            remote: payload.remote.as_deref(),
            greeting_text: payload.greeting_text.as_deref(),
            voice: payload.voice.as_deref(),
            metadata: payload.metadata.as_deref(),
            stale_after_secs: payload.stale_after_secs,
        },
    )
    .await;
    record_operator_tool_result("skills.voice_call.reconnect", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_channel_extensions_handler() -> impl IntoResponse {
    match skills::channel_extensions_data().await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_bind_channel_extension_handler(
    Json(payload): Json<SkillBindChannelExtensionPayload>,
) -> impl IntoResponse {
    match skills::bind_channel_extension_data(
        &payload.binding_id,
        &payload.skill_name,
        skills::SkillBindChannelExtensionOptions {
            service: payload.service.as_deref(),
            component: payload.component.as_deref(),
            trigger: payload.trigger.as_deref(),
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_compile_by_name_handler(
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match skills::compile_data(Some(&name)).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_invoke_handler(
    AxumPath(name): AxumPath<String>,
    Json(payload): Json<SkillInvokePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::invoke_data(
        &name,
        skills::SkillInvokeOptions {
            args: payload.args.as_deref(),
            reference: payload.reference.as_deref(),
            max_chars: payload.max_chars,
            detail: payload.detail,
        },
    )
    .await;
    record_operator_tool_result("skills.invoke", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_execute_handler(
    AxumPath(name): AxumPath<String>,
    Json(payload): Json<SkillExecutePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = skills::execute_data(
        &name,
        skills::SkillExecuteOptions {
            component: payload.component.as_deref(),
            input: payload.input.as_deref(),
        },
    )
    .await;
    record_operator_tool_result("skills.execute", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_auth_authorize_handler(
    AxumPath(provider_id): AxumPath<String>,
    Json(payload): Json<SkillAuthAuthorizePayload>,
) -> impl IntoResponse {
    match skills::authorize_auth_plugin_data(
        &provider_id,
        skills::SkillAuthAuthorizeOptions {
            redirect_uri: payload.redirect_uri.as_deref(),
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_auth_exchange_handler(
    AxumPath(provider_id): AxumPath<String>,
    Json(payload): Json<SkillAuthExchangePayload>,
) -> impl IntoResponse {
    match skills::exchange_auth_plugin_data(
        &provider_id,
        skills::SkillAuthExchangeOptions {
            code: &payload.code,
            state: &payload.state,
            redirect_uri: payload.redirect_uri.as_deref(),
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_background_services_handler(
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match skills::background_services_data(&name).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_schedule_background_handler(
    AxumPath(name): AxumPath<String>,
    Json(payload): Json<SkillScheduleBackgroundPayload>,
) -> impl IntoResponse {
    match skills::schedule_background_service_data(
        &name,
        skills::SkillScheduleBackgroundOptions {
            service: payload.service.as_deref(),
            component: payload.component.as_deref(),
            input: payload.input.as_deref(),
            every_seconds: payload.every_seconds,
            at: payload.at.as_deref(),
            priority: payload.priority,
        },
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_update_handler(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    match skills::update_data(&name).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_uninstall_handler(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    match skills::uninstall_data(&name).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_skill_verify_handler(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    match skills::verify_data(&name).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn control_config_handler(State(state): State<ControlPlaneApiState>) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "path": state.config_path,
                "config": config,
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_config_validate_handler(
    State(state): State<ControlPlaneApiState>,
    Json(payload): Json<AppConfig>,
) -> impl IntoResponse {
    match validate_control_config(&payload) {
        Ok(rendered) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "path": state.config_path,
                "bytes": rendered.len(),
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_config_update_handler(
    State(state): State<ControlPlaneApiState>,
    Json(payload): Json<AppConfig>,
) -> impl IntoResponse {
    let rendered_len = validate_control_config(&payload).map(|value| value.len());
    match rendered_len.and_then(|bytes| {
        runtime::write_config_with_backup(&state.config_path, &payload)?;
        Ok(bytes)
    }) {
        Ok(bytes) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "path": state.config_path,
                "bytes": bytes,
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_diagnostics_handler(
    State(state): State<ControlPlaneApiState>,
    Query(query): Query<DiagnosticsQuery>,
) -> impl IntoResponse {
    match doctor::collect_report(query.repair, query.deep, Some(&state.config_path)).await {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_diagnostics_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ControlPlaneApiState>,
    Query(query): Query<DiagnosticsQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| diagnostics_ws_session(socket, state, query))
}

async fn diagnostics_ws_session(
    socket: WebSocket,
    state: ControlPlaneApiState,
    query: DiagnosticsQuery,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut ticker = interval(Duration::from_secs(query.interval_secs.unwrap_or(5).max(1)));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let payload = match doctor::collect_report(query.repair, query.deep, Some(&state.config_path)).await {
                    Ok(report) => serde_json::json!({"type": "diagnostics", "report": report}),
                    Err(error) => serde_json::json!({"type": "error", "error": error.to_string()}),
                };
                if sender
                    .send(WsMessage::Text(payload.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            message = receiver.next() => {
                match message {
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

fn validate_control_config(config: &AppConfig) -> Result<String> {
    toml::to_string_pretty(config).context("Failed to render config TOML")
}

#[derive(serde::Deserialize)]
struct RuntimeSwitchProviderRequest {
    provider: String,
    model: Option<String>,
    api_key_env: Option<String>,
    fallback_chain: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct RuntimeSwitchModelRequest {
    provider: String,
    model: String,
}

#[derive(serde::Deserialize)]
struct RuntimeVaultValueRequest {
    value: String,
}

#[derive(serde::Deserialize)]
struct EnterpriseAccessBootstrapPayload {
    organization_id: String,
    organization_name: String,
    owner_id: String,
    owner_name: Option<String>,
    owner_email: Option<String>,
    owner_token: String,
}

#[derive(serde::Deserialize)]
struct EnterpriseAccessOperatorPayload {
    id: String,
    name: Option<String>,
    email: Option<String>,
    role: String,
    token: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default = "default_active_true")]
    active: bool,
}

#[derive(serde::Deserialize)]
struct EnterpriseGovernanceRulePayload {
    scope: String,
    approval_mode: String,
    #[serde(default)]
    requester_roles: Vec<String>,
    #[serde(default)]
    approver_roles: Vec<String>,
    #[serde(default = "default_active_true")]
    forbid_self_approval: bool,
    #[serde(default = "default_active_true")]
    active: bool,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct EnterpriseAutonomyEnablePayload {
    operator_id: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    max_delegations: Option<usize>,
    #[serde(default)]
    max_iterations: Option<usize>,
    #[serde(default)]
    max_runtime_secs: Option<u64>,
    #[serde(default)]
    max_lesson_hints: Option<usize>,
}

#[derive(serde::Deserialize)]
struct EnterpriseAutonomyDisablePayload {
    operator_id: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct SelfHostedProductTransitionPayload {
    target_mode: String,
    actor: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct EnterpriseAutonomyKillSwitchPayload {
    operator_id: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct ListLimitQuery {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct SessionListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct SessionInspectQuery {
    #[serde(default)]
    history_limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MemoryListQuery {
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct JobsListQuery {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct JobInspectQuery {
    #[serde(default)]
    run_limit: Option<usize>,
    #[serde(default)]
    dead_letter_limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct OrchestrationRequestPayload {
    prompt: Option<String>,
    task_id: Option<String>,
    category: Option<String>,
    claw_id: Option<String>,
    mode: Option<String>,
    model_profile_id: Option<String>,
    worker_model_profile_id: Option<String>,
    autonomy_level: Option<String>,
    max_delegations: Option<usize>,
    max_iterations: Option<usize>,
    max_runtime_secs: Option<u64>,
    approval_policy: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct OrchestrationLifecyclePayload {
    #[serde(default)]
    requested_by: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    rollback_reference: Option<String>,
}

#[derive(serde::Deserialize)]
struct ActiveRunListQuery {
    #[serde(default = "default_active_true")]
    active_only: bool,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct RefreshQuery {
    #[serde(default)]
    refresh: bool,
}

#[derive(serde::Deserialize)]
struct RuntimeArtifactQuery {
    artifact: String,
}

#[derive(serde::Deserialize, Default)]
struct ToolStatusQuery {
    #[serde(default)]
    name: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct ToolExecutionQuery {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct BrowserWorkflowQuery {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct CodingArtifactQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct VoiceOutcomeQuery {
    #[serde(default)]
    stale_after_secs: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct ToolAddRequest {
    name: String,
    path: Option<String>,
    #[serde(default)]
    hosts: Vec<tools::AiHost>,
}

#[derive(serde::Deserialize, Default)]
struct ToolSetupRequest {
    #[serde(default)]
    tool_names: Vec<String>,
    #[serde(default)]
    hosts: Vec<tools::AiHost>,
}

#[derive(serde::Deserialize, Default)]
struct ToolSyncRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    hosts: Vec<tools::AiHost>,
    #[serde(default)]
    apply: bool,
}

#[derive(serde::Deserialize, Default)]
struct MobileCommandsQuery {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MobileCommandMetricsQuery {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MobileLimitQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MobileSyncConflictsQuery {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MobileAppSessionsQuery {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct MobileCapabilityExecutionsQuery {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    capability: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn runtime_status_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::runtime_status(&state.config_path, &state.workspace_root) {
        Ok(status) => (StatusCode::OK, Json(serde_json::json!(status))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_status_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => (
            StatusCode::OK,
            Json(serde_json::json!(voice_runtime::voice_status(
                &config,
                &state.workspace_root,
            ))),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_providers_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => (
            StatusCode::OK,
            Json(serde_json::json!(voice_runtime::voice_provider_catalog(
                &config
            ))),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_metrics_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match voice_runtime::voice_metrics(&state.workspace_root).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_operator_report_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<inspect::VoiceOperatorReportRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => match inspect::voice_operator_report_summary(
            &config,
            &state.workspace_root,
            query.limit.unwrap_or(12),
            query.stale_after_secs,
        )
        .await
        {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
            Err(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response(),
        },
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_outcomes_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<VoiceOutcomeQuery>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_outcomes(
        &state.workspace_root,
        query.stale_after_secs,
        query.limit,
    )
    .await
    {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_sessions_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match voice_runtime::list_voice_sessions(&state.workspace_root).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_health_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<voice_runtime::VoiceSessionHealthRequest>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_health(&state.workspace_root, query).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_reap_sessions_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<voice_runtime::VoiceSessionReapRequest>,
) -> impl IntoResponse {
    match voice_runtime::reap_voice_sessions(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_start_session_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<voice_runtime::VoiceSessionStartRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::start_voice_session(&config, &state.workspace_root, payload).await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match voice_runtime::inspect_voice_session(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_metrics_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_metrics(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_append_user_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionAppendRequest>,
) -> impl IntoResponse {
    match voice_runtime::append_voice_session_user(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_transcript_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_transcript(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_artifacts_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_artifacts(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_session_events_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match voice_runtime::voice_session_events(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_respond_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionRespondRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::respond_voice_session(&config, &state.workspace_root, &id, payload)
                .await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_reconnect_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionReconnectRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::reconnect_voice_session(
                &config,
                &state.workspace_root,
                &id,
                payload,
            )
            .await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_pause_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionControlRequest>,
) -> impl IntoResponse {
    match voice_runtime::pause_voice_session(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_resume_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionControlRequest>,
) -> impl IntoResponse {
    match voice_runtime::resume_voice_session(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_interrupt_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionControlRequest>,
) -> impl IntoResponse {
    match voice_runtime::interrupt_voice_session(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_end_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<voice_runtime::VoiceSessionEndRequest>,
) -> impl IntoResponse {
    match voice_runtime::end_voice_session(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_voices_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::list_voices_with_config(&config, &state.workspace_root) {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_prewarm_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<voice_runtime::VoicePrewarmRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::prewarm_voice_runtime(&config, &state.workspace_root, payload)
                .await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_transcribe_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<voice_runtime::VoiceTranscribeRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::transcribe_with_config(&config, &state.workspace_root, payload)
                .await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn voice_synthesize_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<voice_runtime::VoiceSynthesizeRequest>,
) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            match voice_runtime::synthesize_with_config(&config, &state.workspace_root, payload)
                .await
            {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_status_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<talk::TalkRuntimeListRequest>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);
    match talk::runtime_status(&state.workspace_root, limit).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_metrics_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match talk::runtime_metrics_data(&state.workspace_root).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_sessions_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<talk::TalkRuntimeListRequest>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);
    match talk::list_talk_sessions(&state.workspace_root, limit).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match talk::inspect_talk_session(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_session_events_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match talk::talk_session_events_data(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "voice")]
async fn talk_session_metrics_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match talk::talk_session_metrics_data(&state.workspace_root, &id).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_nodes_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match mobile::list_nodes_data(&state.workspace_root) {
        Ok(nodes) => (StatusCode::OK, Json(serde_json::json!({ "nodes": nodes }))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_pair_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobilePairRequest>,
) -> impl IntoResponse {
    match mobile::pair_node_data(&state.workspace_root, payload) {
        Ok(node) => (StatusCode::OK, Json(serde_json::json!(node))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_pairings_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandsQuery>,
) -> impl IntoResponse {
    match mobile::list_pairing_data(&state.workspace_root, query.node_id.as_deref(), query.limit) {
        Ok(pairings) => (
            StatusCode::OK,
            Json(serde_json::json!({ "pairings": pairings })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_unpair_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileUnpairRequest>,
) -> impl IntoResponse {
    match mobile::unpair_node_data(&state.workspace_root, &id, payload) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_node_data(&state.workspace_root, &id) {
        Ok(node) => (StatusCode::OK, Json(serde_json::json!(node))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_summary_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<MobileLimitQuery>,
) -> impl IntoResponse {
    match mobile::mobile_node_report_data(&state.workspace_root, &id, query.limit.or(Some(12))) {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_status_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::node_status_data(&state.workspace_root, &id) {
        Ok(status) => (StatusCode::OK, Json(serde_json::json!(status))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_app_sessions_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<MobileAppSessionsQuery>,
) -> impl IntoResponse {
    match mobile::list_app_session_data(
        &state.workspace_root,
        Some(&id),
        query.status.as_deref(),
        query.limit,
    ) {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_runtime_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::node_runtime_data(&state.workspace_root, &id) {
        Ok(runtime) => (StatusCode::OK, Json(serde_json::json!(runtime))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_mobile_metrics_handler() -> impl IntoResponse {
    match mobile::mobile_metrics_data() {
        Ok(metrics) => (StatusCode::OK, Json(serde_json::json!(metrics))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_push_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::node_push_state_data(&state.workspace_root, &id) {
        Ok(push) => (StatusCode::OK, Json(serde_json::json!(push))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_register_push_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobilePushRegistrationRequest>,
) -> impl IntoResponse {
    match mobile::register_push_data(&state.workspace_root, &id, payload) {
        Ok(push) => (StatusCode::OK, Json(serde_json::json!(push))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_sync_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::node_sync_state_data(&state.workspace_root, &id) {
        Ok(sync) => (StatusCode::OK, Json(serde_json::json!(sync))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_report_sync_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileSyncReportRequest>,
) -> impl IntoResponse {
    match mobile::report_sync_data(&state.workspace_root, &id, payload) {
        Ok(sync) => (StatusCode::OK, Json(serde_json::json!(sync))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_sync_conflicts_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<MobileSyncConflictsQuery>,
) -> impl IntoResponse {
    match mobile::list_sync_conflict_data(
        &state.workspace_root,
        Some(&id),
        query.status.as_deref(),
        query.limit,
    ) {
        Ok(conflicts) => (
            StatusCode::OK,
            Json(serde_json::json!({ "conflicts": conflicts })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_capabilities_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::node_capabilities_data(&state.workspace_root, &id) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_activity_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<MobileLimitQuery>,
) -> impl IntoResponse {
    match mobile::node_activity_data(&state.workspace_root, &id, query.limit) {
        Ok(activity) => (StatusCode::OK, Json(serde_json::json!(activity))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_heartbeat_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileHeartbeatRequest>,
) -> impl IntoResponse {
    match mobile::heartbeat_node_data(&state.workspace_root, &id, payload) {
        Ok(runtime) => (StatusCode::OK, Json(serde_json::json!(runtime))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_wake_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileWakeRequest>,
) -> impl IntoResponse {
    match mobile::wake_node_data(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_node_rehydrate_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileRehydrateRequest>,
) -> impl IntoResponse {
    match mobile::rehydrate_node_data(&state.workspace_root, &id, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_commands_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandsQuery>,
) -> impl IntoResponse {
    match mobile::list_command_data(&state.workspace_root, query.node_id.as_deref(), query.limit) {
        Ok(commands) => (
            StatusCode::OK,
            Json(serde_json::json!({ "commands": commands })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_command_data(&state.workspace_root, &id) {
        Ok(command) => (StatusCode::OK, Json(serde_json::json!(command))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_events_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::command_events_data(&state.workspace_root, &id) {
        Ok(events) => (StatusCode::OK, Json(serde_json::json!(events))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_metrics_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandMetricsQuery>,
) -> impl IntoResponse {
    match mobile::command_metrics_data(&state.workspace_root, query.node_id.as_deref(), query.limit)
    {
        Ok(metrics) => (StatusCode::OK, Json(serde_json::json!(metrics))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_dispatch_handler(
    State(state): State<RuntimeControlState>,
    operator: Option<Extension<enterprise_access::EnterpriseAuthenticatedOperator>>,
    Json(payload): Json<mobile::MobileCommandDispatchRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let mut payload = payload;
    if let Some(Extension(operator)) = operator
        && payload.approved_by.is_none()
    {
        payload.approved_by = Some(operator.id);
    }
    let result = mobile::dispatch_command_data(&state.workspace_root, payload).await;
    record_operator_tool_result("mobile.command.dispatch", started_at, &result);
    match result {
        Ok(command) => (StatusCode::OK, Json(serde_json::json!(command))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_approve_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    operator: Option<Extension<enterprise_access::EnterpriseAuthenticatedOperator>>,
    Json(payload): Json<mobile::MobileCommandDecisionRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let payload = if let Some(Extension(operator)) = operator {
        mobile::MobileCommandDecisionRequest {
            decided_by: operator.id,
            reason: payload.reason,
        }
    } else {
        payload
    };
    let result = mobile::approve_command_data(&state.workspace_root, &id, payload).await;
    record_operator_tool_result("mobile.command.approve", started_at, &result);
    match result {
        Ok(command) => (StatusCode::OK, Json(serde_json::json!(command))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_command_reject_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    operator: Option<Extension<enterprise_access::EnterpriseAuthenticatedOperator>>,
    Json(payload): Json<mobile::MobileCommandDecisionRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let payload = if let Some(Extension(operator)) = operator {
        mobile::MobileCommandDecisionRequest {
            decided_by: operator.id,
            reason: payload.reason,
        }
    } else {
        payload
    };
    let result = mobile::reject_command_data(&state.workspace_root, &id, payload);
    record_operator_tool_result("mobile.command.reject", started_at, &result);
    match result {
        Ok(command) => (StatusCode::OK, Json(serde_json::json!(command))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_notifications_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandsQuery>,
) -> impl IntoResponse {
    match mobile::list_notification_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.limit,
    ) {
        Ok(notifications) => (
            StatusCode::OK,
            Json(serde_json::json!({ "notifications": notifications })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_notification_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_notification_data(&state.workspace_root, &id) {
        Ok(notification) => (StatusCode::OK, Json(serde_json::json!(notification))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_notification_send_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileNotificationSendRequest>,
) -> impl IntoResponse {
    match mobile::send_notification_data(&state.workspace_root, payload).await {
        Ok(notification) => (StatusCode::OK, Json(serde_json::json!(notification))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_notification_ack_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileNotificationAckRequest>,
) -> impl IntoResponse {
    match mobile::acknowledge_notification_data(&state.workspace_root, &id, payload) {
        Ok(notification) => (StatusCode::OK, Json(serde_json::json!(notification))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_inbox_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandsQuery>,
) -> impl IntoResponse {
    match mobile::list_inbound_message_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.limit,
    ) {
        Ok(messages) => (
            StatusCode::OK,
            Json(serde_json::json!({ "messages": messages })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_inbox_message_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_inbound_message_data(&state.workspace_root, &id) {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_inbox_report_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileInboundMessageReportRequest>,
) -> impl IntoResponse {
    match mobile::report_inbound_message_data(&state.workspace_root, payload) {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_inbox_ack_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileInboundMessageAckRequest>,
) -> impl IntoResponse {
    match mobile::acknowledge_inbound_message_data(&state.workspace_root, &id, payload) {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_outbox_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCommandsQuery>,
) -> impl IntoResponse {
    match mobile::list_outbound_message_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.limit,
    ) {
        Ok(messages) => (
            StatusCode::OK,
            Json(serde_json::json!({ "messages": messages })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_outbox_message_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_outbound_message_data(&state.workspace_root, &id) {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_outbox_send_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileOutboundMessageSendRequest>,
) -> impl IntoResponse {
    match mobile::send_outbound_message_data(&state.workspace_root, payload).await {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_outbox_ack_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileOutboundMessageAckRequest>,
) -> impl IntoResponse {
    match mobile::acknowledge_outbound_message_data(&state.workspace_root, &id, payload) {
        Ok(message) => (StatusCode::OK, Json(serde_json::json!(message))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_sync_conflicts_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileSyncConflictsQuery>,
) -> impl IntoResponse {
    match mobile::list_sync_conflict_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.status.as_deref(),
        query.limit,
    ) {
        Ok(conflicts) => (
            StatusCode::OK,
            Json(serde_json::json!({ "conflicts": conflicts })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_sync_conflict_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_sync_conflict_data(&state.workspace_root, &id) {
        Ok(conflict) => (StatusCode::OK, Json(serde_json::json!(conflict))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_sync_conflict_report_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileSyncConflictReportRequest>,
) -> impl IntoResponse {
    match mobile::report_sync_conflict_data(&state.workspace_root, payload) {
        Ok(conflict) => (StatusCode::OK, Json(serde_json::json!(conflict))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_sync_conflict_resolve_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<mobile::MobileSyncConflictResolveRequest>,
) -> impl IntoResponse {
    match mobile::resolve_sync_conflict_data(&state.workspace_root, &id, payload) {
        Ok(conflict) => (StatusCode::OK, Json(serde_json::json!(conflict))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_app_sessions_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileAppSessionsQuery>,
) -> impl IntoResponse {
    match mobile::list_app_session_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.status.as_deref(),
        query.limit,
    ) {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_app_session_metrics_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileAppSessionsQuery>,
) -> impl IntoResponse {
    match mobile::app_session_metrics_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.status.as_deref(),
    ) {
        Ok(metrics) => (StatusCode::OK, Json(serde_json::json!(metrics))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_app_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_app_session_data(&state.workspace_root, &id) {
        Ok(session) => (StatusCode::OK, Json(serde_json::json!(session))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_app_session_events_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::app_session_events_data(&state.workspace_root, &id) {
        Ok(events) => (StatusCode::OK, Json(serde_json::json!(events))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_message_preview_handler(
    Json(payload): Json<mobile::MobileMessagePreviewRequest>,
) -> impl IntoResponse {
    match mobile::preview_message_data(payload) {
        Ok(preview) => (StatusCode::OK, Json(serde_json::json!(preview))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_notification_preview_handler(
    Json(payload): Json<mobile::MobileNotificationPreviewRequest>,
) -> impl IntoResponse {
    match mobile::preview_notification_data(payload) {
        Ok(preview) => (StatusCode::OK, Json(serde_json::json!(preview))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_sync_preview_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileSyncPreviewRequest>,
) -> impl IntoResponse {
    match mobile::preview_sync_data(&state.workspace_root, payload) {
        Ok(preview) => (StatusCode::OK, Json(serde_json::json!(preview))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_capability_preview_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileCapabilityPreviewRequest>,
) -> impl IntoResponse {
    match mobile::preview_capability_data(&state.workspace_root, payload) {
        Ok(preview) => (StatusCode::OK, Json(serde_json::json!(preview))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_capability_execute_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<mobile::MobileCapabilityExecuteRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = mobile::execute_capability_data(&state.workspace_root, payload);
    record_operator_tool_result("mobile.capability.execute", started_at, &result);
    match result {
        Ok(execution) => (StatusCode::OK, Json(serde_json::json!(execution))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_capability_executions_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCapabilityExecutionsQuery>,
) -> impl IntoResponse {
    match mobile::list_capability_execution_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.capability.as_deref(),
        query.limit,
    ) {
        Ok(executions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "executions": executions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_capability_execution_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_capability_execution_data(&state.workspace_root, &id) {
        Ok(execution) => (StatusCode::OK, Json(serde_json::json!(execution))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_media_artifacts_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MobileCapabilityExecutionsQuery>,
) -> impl IntoResponse {
    match mobile::list_media_artifact_data(
        &state.workspace_root,
        query.node_id.as_deref(),
        query.capability.as_deref(),
        query.limit,
    ) {
        Ok(artifacts) => (
            StatusCode::OK,
            Json(serde_json::json!({ "artifacts": artifacts })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn mobile_media_artifact_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    match mobile::inspect_media_artifact_data(&state.workspace_root, &id) {
        Ok(artifact) => (StatusCode::OK, Json(serde_json::json!(artifact))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn media_inspect_handler(
    Json(payload): Json<super::media::MediaInspectRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = super::media::inspect_data(payload).await;
    record_operator_tool_result("media.inspect", started_at, &result);
    match result {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn media_providers_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            let result = super::media::media_provider_catalog(&config);
            (StatusCode::OK, Json(serde_json::json!(result))).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn media_extract_text_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<super::media::MediaExtractTextRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            let result =
                super::media::extract_text_with_config(&config, &state.workspace_root, payload)
                    .await;
            record_operator_tool_result("media.extract_text", started_at, &result);
            match result {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => {
            openrustclaw_observability::metrics::record_tool_execution(
                "media.extract_text",
                "failure",
            );
            openrustclaw_observability::metrics::record_tool_duration(
                "media.extract_text",
                started_at.elapsed().as_secs_f64(),
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response()
        }
    }
}

async fn media_describe_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<super::media::MediaDescribeRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            let result =
                super::media::describe_with_config(&config, &state.workspace_root, payload).await;
            record_operator_tool_result("media.describe", started_at, &result);
            match result {
                Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
                Err(error) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(error) => {
            openrustclaw_observability::metrics::record_tool_execution("media.describe", "failure");
            openrustclaw_observability::metrics::record_tool_duration(
                "media.describe",
                started_at.elapsed().as_secs_f64(),
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response()
        }
    }
}

async fn runtime_health_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match runtime::runtime_health_status(&state.config_path, &state.workspace_root, false).await {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_operator_ops_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match runtime::runtime_operator_ops_summary(
        &state.config_path,
        &state.workspace_root,
        &state.gateway_addr,
        Some(state.started_at),
        state.sidecar_running,
    )
    .await
    {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn security_posture_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match security::posture_summary(&state.config_path, &state.workspace_root) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_access_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match inspect::enterprise_access_summary(&state.workspace_root) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn self_hosted_product_mode_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match inspect::self_hosted_product_mode_summary(&state.workspace_root) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn setup_handoff_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match inspect::setup_handoff_summary(&state.workspace_root) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn self_hosted_product_mode_transition_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<SelfHostedProductTransitionPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = inspect::transition_self_hosted_product_mode_summary(
        &state.workspace_root,
        payload.target_mode,
        payload.actor,
        payload.reason,
    );
    record_operator_tool_result("self_hosted.product_mode.transition", started_at, &result);
    match result {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_admin_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match inspect::enterprise_admin_summary(&state.workspace_root, &state.config_path) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_autonomy_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match enterprise_autonomy::summary(&state.workspace_root, 10) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_access_bootstrap_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseAccessBootstrapPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_access::bootstrap_manifest(
        &state.workspace_root,
        enterprise_access::EnterpriseAccessBootstrapRequest {
            organization_id: payload.organization_id,
            organization_name: payload.organization_name,
            owner_id: payload.owner_id,
            owner_name: payload.owner_name,
            owner_email: payload.owner_email,
            owner_token: payload.owner_token,
        },
    );
    record_operator_tool_result("enterprise.access.bootstrap", started_at, &result);
    match result {
        Ok(_) => match inspect::enterprise_access_summary(&state.workspace_root) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_access_upsert_operator_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseAccessOperatorPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_access::upsert_operator(
        &state.workspace_root,
        enterprise_access::EnterpriseAccessOperatorRequest {
            id: payload.id,
            name: payload.name,
            email: payload.email,
            role: payload.role,
            token: payload.token,
            scopes: payload.scopes,
            active: payload.active,
        },
    );
    record_operator_tool_result("enterprise.access.upsert_operator", started_at, &result);
    match result {
        Ok(_) => match inspect::enterprise_access_summary(&state.workspace_root) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_governance_upsert_rule_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseGovernanceRulePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_access::upsert_governance_rule(
        &state.workspace_root,
        enterprise_access::EnterpriseGovernanceRuleRequest {
            scope: payload.scope,
            approval_mode: payload.approval_mode,
            requester_roles: payload.requester_roles,
            approver_roles: payload.approver_roles,
            forbid_self_approval: payload.forbid_self_approval,
            active: payload.active,
            detail: payload.detail,
        },
    );
    record_operator_tool_result("enterprise.governance.upsert_rule", started_at, &result);
    match result {
        Ok(_) => match inspect::enterprise_access_summary(&state.workspace_root) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_autonomy_enable_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseAutonomyEnablePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_autonomy::enable(
        &state.workspace_root,
        enterprise_autonomy::EnterpriseAutonomyEnableRequest {
            operator_id: payload.operator_id,
            note: payload.note,
            max_delegations: payload.max_delegations,
            max_iterations: payload.max_iterations,
            max_runtime_secs: payload.max_runtime_secs,
            max_lesson_hints: payload.max_lesson_hints,
        },
    );
    record_operator_tool_result("enterprise.autonomy.enable", started_at, &result);
    match result {
        Ok(_) => match enterprise_autonomy::summary(&state.workspace_root, 10) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_autonomy_disable_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseAutonomyDisablePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_autonomy::disable(
        &state.workspace_root,
        enterprise_autonomy::EnterpriseAutonomyDisableRequest {
            operator_id: payload.operator_id,
            reason: payload.reason,
        },
    );
    record_operator_tool_result("enterprise.autonomy.disable", started_at, &result);
    match result {
        Ok(_) => match enterprise_autonomy::summary(&state.workspace_root, 10) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_autonomy_kill_switch_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<EnterpriseAutonomyKillSwitchPayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result = enterprise_autonomy::kill_switch(
        &state.workspace_root,
        enterprise_autonomy::EnterpriseAutonomyKillSwitchRequest {
            operator_id: payload.operator_id,
            reason: payload.reason,
        },
    );
    record_operator_tool_result("enterprise.autonomy.kill_switch", started_at, &result);
    match result {
        Ok(_) => match enterprise_autonomy::summary(&state.workspace_root, 10) {
            Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
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

async fn enterprise_foundations_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match inspect::enterprise_foundations_summary(&state.workspace_root, 12) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_policy_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match enterprise_policy::summary(&state.workspace_root, &state.config_path) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_policy_update_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<enterprise_policy::EnterprisePolicyUpdateRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result =
        enterprise_policy::update_policy(&state.workspace_root, &state.config_path, payload);
    record_operator_tool_result("enterprise.policy.update", started_at, &result);
    match result {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_audit_export_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<enterprise_policy::EnterpriseAuditExportRequest>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let result =
        enterprise_policy::export_audit_bundle(&state.workspace_root, &state.config_path, payload);
    record_operator_tool_result("enterprise.audit.export", started_at, &result);
    match result {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn enterprise_audit_review_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match enterprise_policy::review_summary(&state.workspace_root, &state.config_path) {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!(summary))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_beacon_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<RefreshQuery>,
) -> impl IntoResponse {
    match runtime::runtime_beacon_status(
        &state.config_path,
        &state.workspace_root,
        query.refresh,
        &state.gateway_addr,
        Some(state.started_at),
        state.sidecar_running,
    )
    .await
    {
        Ok(beacon) => (StatusCode::OK, Json(serde_json::json!(beacon))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_reload_plan_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match runtime::runtime_reload_plan(&state.config_path, &state.workspace_root) {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_upgrade_plan_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match runtime::runtime_upgrade_plan(&state.config_path, &state.workspace_root).await {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_self_update_plan_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<RuntimeArtifactQuery>,
) -> impl IntoResponse {
    match runtime::runtime_self_update_plan(
        &state.config_path,
        &state.workspace_root,
        &query.artifact,
    )
    .await
    {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_rollback_plan_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<RuntimeArtifactQuery>,
) -> impl IntoResponse {
    match runtime::runtime_rollback_plan(&state.config_path, &state.workspace_root, &query.artifact)
        .await
    {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn tool_status_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ToolStatusQuery>,
) -> impl IntoResponse {
    match tools::status_data(&state.workspace_root, query.name.as_deref()) {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn tool_execution_history_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ToolExecutionQuery>,
) -> impl IntoResponse {
    match inspect::tool_execution_history(
        &state.workspace_root,
        query.limit.unwrap_or(20),
        query.source.as_deref(),
        query.status.as_deref(),
        query.tool_name.as_deref(),
    ) {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(feature = "cursor")]
async fn coding_artifact_history_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<CodingArtifactQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20).max(1);
    match cursor_tools::load_execution_artifacts(&state.workspace_root, limit) {
        Ok(entries) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "limit": limit,
                "artifact_root": cursor_tools::execution_artifact_root(&state.workspace_root),
                "entries": entries,
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(not(feature = "cursor"))]
async fn coding_artifact_history_handler(
    State(_state): State<RuntimeControlState>,
    Query(query): Query<CodingArtifactQuery>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Cursor support is disabled in this build.",
            "limit": query.limit.unwrap_or(20).max(1),
            "entries": [],
        })),
    )
        .into_response()
}

async fn tool_detail_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match tools::detail_data(&state.workspace_root, &name) {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn tool_add_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<ToolAddRequest>,
) -> impl IntoResponse {
    match tools::add_tool(
        &state.workspace_root,
        &payload.name,
        payload.path.as_deref(),
        &payload.hosts,
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn tool_setup_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<ToolSetupRequest>,
) -> impl IntoResponse {
    match tools::setup_tools(&state.workspace_root, &payload.tool_names, &payload.hosts).await {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn tool_sync_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<ToolSyncRequest>,
) -> impl IntoResponse {
    match tools::sync_tools(
        &state.workspace_root,
        payload.name.as_deref(),
        &payload.hosts,
        payload.apply,
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_ui_handler() -> Html<&'static str> {
    control_ui::dashboard()
}

async fn runtime_health_scan_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match runtime::scan_runtime_health(&state.config_path, &state.workspace_root).await {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_reload_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    match reload_runtime_agent(&state).await {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_switch_provider_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<RuntimeSwitchProviderRequest>,
) -> impl IntoResponse {
    let previous = std::fs::read_to_string(&state.config_path).unwrap_or_default();
    let result = runtime::switch_provider(
        &state.config_path,
        &state.workspace_root,
        &payload.provider,
        payload.model.as_deref(),
        payload.api_key_env.as_deref(),
        payload.fallback_chain,
    );

    match result {
        Ok(_) => match reload_runtime_agent(&state).await {
            Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
            Err(error) => {
                let _ = std::fs::write(&state.config_path, previous);
                let _ = reload_runtime_agent(&state).await;
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string(), "rolled_back": true})),
                )
                    .into_response()
            }
        },
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_switch_model_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<RuntimeSwitchModelRequest>,
) -> impl IntoResponse {
    let previous = std::fs::read_to_string(&state.config_path).unwrap_or_default();
    let result = runtime::switch_model(
        &state.config_path,
        &state.workspace_root,
        &payload.provider,
        &payload.model,
    );

    match result {
        Ok(_) => match reload_runtime_agent(&state).await {
            Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
            Err(error) => {
                let _ = std::fs::write(&state.config_path, previous);
                let _ = reload_runtime_agent(&state).await;
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": error.to_string(), "rolled_back": true})),
                )
                    .into_response()
            }
        },
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_vault_status_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    let path = runtime::vault_path_for(&state.workspace_root);
    match runtime::list_vault_keys(&state.workspace_root) {
        Ok(keys) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "path": path,
                "present": path.exists(),
                "entries": keys,
                "count": keys.len(),
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string(), "path": path})),
        )
            .into_response(),
    }
}

async fn runtime_vault_set_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(key): AxumPath<String>,
    Json(payload): Json<RuntimeVaultValueRequest>,
) -> impl IntoResponse {
    match runtime::set_vault_secret(&state.workspace_root, &key, &payload.value) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "key": key})),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn runtime_vault_delete_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(key): AxumPath<String>,
) -> impl IntoResponse {
    match runtime::delete_vault_secret(&state.workspace_root, &key) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "key": key})),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_resolve_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<OrchestrationRequestPayload>,
) -> impl IntoResponse {
    match orchestrate::resolve(
        orchestrate::OrchestrationRequest {
            prompt: payload.prompt.unwrap_or_default(),
            task_id: payload.task_id,
            category: payload.category,
            claw_id: payload.claw_id,
            mode: payload.mode.unwrap_or_else(|| "auto".to_string()),
            overrides: orchestrate::OrchestrationRequestOverrides {
                model_profile_id: payload.model_profile_id,
                worker_model_profile_id: payload.worker_model_profile_id,
                autonomy_level: payload.autonomy_level,
                max_delegations: payload.max_delegations,
                max_iterations: payload.max_iterations,
                max_runtime_secs: payload.max_runtime_secs,
                approval_policy: payload.approval_policy,
            },
        },
        &state.workspace_root,
    ) {
        Ok(decision) => (StatusCode::OK, Json(serde_json::json!(decision))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<OrchestrationRequestPayload>,
) -> impl IntoResponse {
    let prompt = payload.prompt.unwrap_or_default();
    if prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "prompt is required"})),
        )
            .into_response();
    }

    match orchestrate::run(
        orchestrate::OrchestrationRequest {
            prompt,
            task_id: payload.task_id,
            category: payload.category,
            claw_id: payload.claw_id,
            mode: payload.mode.unwrap_or_else(|| "auto".to_string()),
            overrides: orchestrate::OrchestrationRequestOverrides {
                model_profile_id: payload.model_profile_id,
                worker_model_profile_id: payload.worker_model_profile_id,
                autonomy_level: payload.autonomy_level,
                max_delegations: payload.max_delegations,
                max_iterations: payload.max_iterations,
                max_runtime_secs: payload.max_runtime_secs,
                approval_policy: payload.approval_policy,
            },
        },
        &state.workspace_root,
    )
    .await
    {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_submit_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<OrchestrationRequestPayload>,
) -> impl IntoResponse {
    let prompt = payload.prompt.unwrap_or_default();
    if prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "prompt is required"})),
        )
            .into_response();
    }

    match orchestrate::submit(
        orchestrate::OrchestrationRequest {
            prompt,
            task_id: payload.task_id,
            category: payload.category,
            claw_id: payload.claw_id,
            mode: payload.mode.unwrap_or_else(|| "orchestrated".to_string()),
            overrides: orchestrate::OrchestrationRequestOverrides {
                model_profile_id: payload.model_profile_id,
                worker_model_profile_id: payload.worker_model_profile_id,
                autonomy_level: payload.autonomy_level,
                max_delegations: payload.max_delegations,
                max_iterations: payload.max_iterations,
                max_runtime_secs: payload.max_runtime_secs,
                approval_policy: payload.approval_policy,
            },
        },
        &state.workspace_root,
    )
    .await
    {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_runs_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ActiveRunListQuery>,
) -> impl IntoResponse {
    match orchestrate::list_active_runs(
        &state.workspace_root,
        query.active_only,
        query.limit.unwrap_or(20),
    ) {
        Ok(runs) => (StatusCode::OK, Json(serde_json::json!({ "runs": runs }))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_active_run(&state.workspace_root, &run_id) {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_events_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match orchestrate::read_active_run_events(
        &state.workspace_root,
        &run_id,
        query.limit.unwrap_or(20),
    ) {
        Ok(events) => (
            StatusCode::OK,
            Json(serde_json::json!({ "events": events })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_supervision_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match orchestrate::read_active_run_supervision(
        &state.workspace_root,
        &run_id,
        query.limit.unwrap_or(20),
    ) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_pause_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::pause_active_run(&state.workspace_root, &run_id) {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_escalate_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
    operator: Option<Extension<enterprise_access::EnterpriseAuthenticatedOperator>>,
    Json(payload): Json<OrchestrationLifecyclePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let payload = if let Some(Extension(operator)) = operator {
        orchestrate::ActiveRunInterventionRequest {
            requested_by: Some(operator.id),
            reason: payload.reason,
            rollback_reference: payload.rollback_reference,
        }
    } else {
        orchestrate::ActiveRunInterventionRequest {
            requested_by: payload.requested_by,
            reason: payload.reason,
            rollback_reference: payload.rollback_reference,
        }
    };
    let result = orchestrate::escalate_active_run(&state.workspace_root, &run_id, payload);
    record_operator_tool_result("orchestration.active.escalate", started_at, &result);
    match result {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_resume_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::resume_active_run(&state.workspace_root, &run_id) {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_rollback_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
    operator: Option<Extension<enterprise_access::EnterpriseAuthenticatedOperator>>,
    Json(payload): Json<OrchestrationLifecyclePayload>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let payload = if let Some(Extension(operator)) = operator {
        orchestrate::ActiveRunInterventionRequest {
            requested_by: Some(operator.id),
            reason: payload.reason,
            rollback_reference: payload.rollback_reference,
        }
    } else {
        orchestrate::ActiveRunInterventionRequest {
            requested_by: payload.requested_by,
            reason: payload.reason,
            rollback_reference: payload.rollback_reference,
        }
    };
    let result = orchestrate::rollback_active_run(&state.workspace_root, &run_id, payload);
    record_operator_tool_result("orchestration.active.rollback", started_at, &result);
    match result {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_active_run_kill_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::kill_active_run(&state.workspace_root, &run_id) {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_runs_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match orchestrate::list_runs(&state.workspace_root, query.limit.unwrap_or(20)) {
        Ok(runs) => (StatusCode::OK, Json(serde_json::json!({ "runs": runs }))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_receipt_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run(&state.workspace_root, &receipt_id) {
        Ok(run) => (StatusCode::OK, Json(serde_json::json!(run))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_checkpoints_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run_checkpoints(&state.workspace_root, &receipt_id) {
        Ok(checkpoints) => (
            StatusCode::OK,
            Json(serde_json::json!({ "checkpoints": checkpoints })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_supervision_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run_supervision(&state.workspace_root, &receipt_id) {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_trace_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run_trace(&state.workspace_root, &receipt_id) {
        Ok(trace) => (StatusCode::OK, Json(trace)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_transcript_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run_transcript(&state.workspace_root, &receipt_id) {
        Ok(transcript) => (StatusCode::OK, Json(transcript)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn orchestration_run_resources_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(receipt_id): AxumPath<String>,
) -> impl IntoResponse {
    match orchestrate::read_run_resources(&state.workspace_root, &receipt_id) {
        Ok(resources) => (StatusCode::OK, Json(resources)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct PromoteReflectionPayload {
    #[serde(default)]
    lesson_id: Option<String>,
    #[serde(default = "default_active_true")]
    active: bool,
    #[serde(default)]
    signal: Option<String>,
    #[serde(default)]
    recommendation: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    claw_id: Option<String>,
    #[serde(default)]
    model_profile_id: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    autonomy_level: Option<String>,
    #[serde(default)]
    execution_mode: Option<String>,
}

async fn orchestration_promote_reflection_candidate_handler(
    State(state): State<RuntimeControlState>,
    AxumPath((receipt_id, index)): AxumPath<(String, usize)>,
    Json(payload): Json<PromoteReflectionPayload>,
) -> impl IntoResponse {
    match orchestrate::promote_reflection_candidate(
        &state.workspace_root,
        &receipt_id,
        index,
        orchestrate::PromoteReflectionInput {
            lesson_id: payload.lesson_id,
            active: payload.active,
            signal: payload.signal,
            recommendation: payload.recommendation,
            rationale: payload.rationale,
            confidence: payload.confidence,
            source: payload.source,
            category: payload.category,
            claw_id: payload.claw_id,
            model_profile_id: payload.model_profile_id,
            provider: payload.provider,
            autonomy_level: payload.autonomy_level,
            execution_mode: payload.execution_mode,
        },
    ) {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_navigate_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserNavigateRequest>,
) -> impl IntoResponse {
    match browser::navigate(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_open_session_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserOpenSessionRequest>,
) -> impl IntoResponse {
    match browser::open_session(&state.workspace_root, payload) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_list_sessions_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match browser::list_sessions(&state.workspace_root, query.limit.unwrap_or(25)) {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_inspect_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserInspectRequest>,
) -> impl IntoResponse {
    match browser::inspect(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_run_sequence_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserRunSequenceRequest>,
) -> impl IntoResponse {
    match browser::run_sequence(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_read_page_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserReadPageRequest>,
) -> impl IntoResponse {
    match browser::read_page(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_crawl_site_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserCrawlRequest>,
) -> impl IntoResponse {
    match browser::crawl_site(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_extract_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserExtractRequest>,
) -> impl IntoResponse {
    match browser::extract(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_screenshot_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserScreenshotRequest>,
) -> impl IntoResponse {
    match browser::screenshot(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_pdf_handler(
    State(state): State<RuntimeControlState>,
    Json(payload): Json<browser::BrowserPdfRequest>,
) -> impl IntoResponse {
    match browser::pdf(&state.workspace_root, payload).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_artifacts_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match browser::list_artifacts(&state.workspace_root, query.limit.unwrap_or(50)) {
        Ok(artifacts) => (
            StatusCode::OK,
            Json(serde_json::json!({ "artifacts": artifacts })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_backend_policy_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match browser::backend_policy(&state.workspace_root) {
        Ok(policy) => (StatusCode::OK, Json(serde_json::json!(policy))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_backend_audit_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match browser::list_backend_audit(&state.workspace_root, query.limit.unwrap_or(50)) {
        Ok(entries) => (
            StatusCode::OK,
            Json(serde_json::json!({ "entries": entries })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn browser_workflow_history_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<BrowserWorkflowQuery>,
) -> impl IntoResponse {
    match browser::list_workflow_history(
        &state.workspace_root,
        query.limit.unwrap_or(20),
        query.action.as_deref(),
        query.backend.as_deref(),
    ) {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn service_status_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    let registry = state.channel_registry.read().await;
    let result = match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => {
            services::status_with_pool(
                &config,
                &state.pool,
                &registry,
                &services::LiveRuntimeMetadata {
                    config_path: state.config_path.clone(),
                    gateway_addr: state.gateway_addr.clone(),
                    started_at: Some(state.started_at),
                    sidecar_running: state.sidecar_running,
                },
            )
            .await
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn service_scheduler_handler(State(state): State<RuntimeControlState>) -> impl IntoResponse {
    let result = match runtime::load_effective_config(&state.config_path, &state.workspace_root) {
        Ok(config) => services::scheduler_with_pool(&config, &state.pool, 10).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn service_runtime_events_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match services::runtime_events_with_pool(
        &state.pool,
        query.name.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await
    {
        Ok(events) => (
            StatusCode::OK,
            Json(serde_json::json!({ "events": events })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn service_channel_probes_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<RefreshQuery>,
) -> impl IntoResponse {
    let result =
        services::channel_probes_status(&state.config_path, &state.workspace_root, query.refresh)
            .await;
    match result {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn service_beacon_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<RefreshQuery>,
) -> impl IntoResponse {
    match runtime::runtime_beacon_status(
        &state.config_path,
        &state.workspace_root,
        query.refresh,
        &state.gateway_addr,
        Some(state.started_at),
        state.sidecar_running,
    )
    .await
    {
        Ok(beacon) => (StatusCode::OK, Json(serde_json::json!(beacon))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_logs_recent_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match logs::recent_runtime_logs(&state.workspace_root, query.limit.unwrap_or(50)) {
        Ok(entries) => (
            StatusCode::OK,
            Json(serde_json::json!({ "entries": entries })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_logs_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<RuntimeControlState>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| logs_ws_session(socket, state.workspace_root, query.limit))
}

async fn logs_ws_session(socket: WebSocket, workspace_root: PathBuf, limit: Option<usize>) {
    let (mut sender, mut receiver) = socket.split();

    let initial_entries =
        logs::recent_runtime_logs(&workspace_root, limit.unwrap_or(50)).unwrap_or_default();
    for entry in initial_entries {
        let payload = serde_json::json!({ "type": "log", "entry": entry });
        if sender
            .send(WsMessage::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            return;
        }
    }

    let Some(mut subscription) = logs::subscribe_runtime_logs() else {
        return;
    };

    loop {
        tokio::select! {
            entry = subscription.recv() => {
                match entry {
                    Ok(entry) => {
                        let payload = serde_json::json!({ "type": "log", "entry": entry });
                        if sender
                            .send(WsMessage::Text(payload.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let payload = serde_json::json!({ "type": "warning", "message": "log stream lagged; some entries were skipped" });
                        if sender
                            .send(WsMessage::Text(payload.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            message = receiver.next() => {
                match message {
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

async fn control_sessions_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<SessionListQuery>,
) -> impl IntoResponse {
    match inspect::list_sessions(
        &state.pool,
        query.status.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await
    {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_session_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<SessionInspectQuery>,
) -> impl IntoResponse {
    match inspect::inspect_session(&state.pool, &id, query.history_limit.unwrap_or(50)).await {
        Ok(report) => {
            if report.session.is_none() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "session not found"})),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(serde_json::json!(report))).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_memory_namespaces_handler(
    State(state): State<RuntimeControlState>,
) -> impl IntoResponse {
    match inspect::memory_namespaces(&state.memory_store).await {
        Ok(namespaces) => (
            StatusCode::OK,
            Json(serde_json::json!({ "namespaces": namespaces })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_memory_timeline_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MemoryListQuery>,
) -> impl IntoResponse {
    match inspect::memory_timeline(
        &state.memory_store,
        query.namespace.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_memory_archive_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<MemoryListQuery>,
) -> impl IntoResponse {
    match inspect::memory_archive(
        &state.memory_store,
        query.namespace.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await
    {
        Ok(report) => (StatusCode::OK, Json(serde_json::json!(report))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_jobs_handler(
    State(state): State<RuntimeControlState>,
    Query(query): Query<JobsListQuery>,
) -> impl IntoResponse {
    match inspect::list_jobs(
        &state.pool,
        query.state.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await
    {
        Ok(jobs) => (StatusCode::OK, Json(serde_json::json!({ "jobs": jobs }))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn control_job_handler(
    State(state): State<RuntimeControlState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<JobInspectQuery>,
) -> impl IntoResponse {
    match inspect::inspect_job(
        &state.pool,
        &id,
        query.run_limit.unwrap_or(10),
        query.dead_letter_limit.unwrap_or(10),
    )
    .await
    {
        Ok(report) => {
            if report.job.is_none() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "job not found"})),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(serde_json::json!(report))).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}

async fn reload_runtime_agent(state: &RuntimeControlState) -> Result<serde_json::Value> {
    let plan = runtime::runtime_reload_plan(&state.config_path, &state.workspace_root)?;
    if plan.restart_required {
        return Ok(serde_json::json!({
            "status": "restart_required",
            "reload_plan": plan,
        }));
    }

    let config = runtime::load_effective_config(&state.config_path, &state.workspace_root)?;
    if let Some(channel_agent) = &state.channel_agent {
        let agent = build_channel_agent(
            &config,
            state.memory_store.clone(),
            state.core_memory_store.clone(),
            state.session_manager.clone(),
            state.langsmith.clone(),
            state.event_bus.clone(),
            state.channel_registry.clone(),
        )?;
        *channel_agent.write().await = agent;
    }

    let _ = runtime::mark_runtime_applied(&state.config_path, &state.workspace_root)?;
    let beacon = runtime::refresh_runtime_beacon(
        &state.config_path,
        &state.workspace_root,
        &state.gateway_addr,
        Some(state.started_at),
        state.sidecar_running,
    )
    .await
    .ok();
    let reload_plan = plan.clone();

    let _ = state
        .event_bus
        .publish_named(
            "runtime.reloaded",
            "runtime_control",
            None,
            &serde_json::json!({
                "default_provider": config.providers.default_provider,
                "fallback_chain": config.providers.fallback_chain,
                "reload_plan": reload_plan,
            }),
            None,
        )
        .await;

    Ok(serde_json::json!({
        "status": if plan.live_reload_ready { "reloaded" } else { "up_to_date" },
        "default_provider": config.providers.default_provider,
        "fallback_chain": config.providers.fallback_chain,
        "reload_plan": plan,
        "beacon": beacon,
        "models": {
            "anthropic": config.providers.anthropic.model,
            "openai": config.providers.openai.model,
            "openrouter": config.providers.openrouter.model,
            "ollama": config.providers.ollama.model,
        }
    }))
}

#[derive(Clone)]
struct IMessageIngressState {
    handler: Arc<IMessageWebhookHandler>,
}

fn imessage_ingress_router(handler: IMessageWebhookHandler) -> Router {
    Router::new()
        .route(
            "/webhooks/imessage/bluebubbles",
            post(imessage_bluebubbles_handler),
        )
        .with_state(IMessageIngressState {
            handler: Arc::new(handler),
        })
}

async fn imessage_bluebubbles_handler(
    State(state): State<IMessageIngressState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Json(payload): Json<BlueBubblesMessage>,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let candidate = params
        .get("password")
        .or_else(|| params.get("guid"))
        .or_else(|| params.get("token"))
        .map(|value| value.as_str())
        .or_else(|| {
            headers
                .get("x-password")
                .and_then(|value| value.to_str().ok())
        })
        .or_else(|| headers.get("x-guid").and_then(|value| value.to_str().ok()));

    if !state.handler.verify_password(candidate) {
        record_operator_tool_status("channels.imessage.ingress", started_at, "failure");
        return (StatusCode::UNAUTHORIZED, "invalid BlueBubbles password").into_response();
    }

    match state.handler.handle_event(payload).await {
        Ok(()) => {
            record_operator_tool_status("channels.imessage.ingress", started_at, "success");
            StatusCode::OK.into_response()
        }
        Err(error) => {
            record_operator_tool_status("channels.imessage.ingress", started_at, "failure");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct TeamsIngressState {
    handler: Arc<TeamsWebhookHandler>,
}

#[derive(Clone)]
struct MattermostIngressState {
    handler: Arc<MattermostWebhookHandler>,
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
    let started_at = std::time::Instant::now();
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        match state.handler.verify_token(token).await {
            Ok(true) => {}
            Ok(false) => {
                record_operator_tool_status("channels.teams.ingress", started_at, "failure");
                return (StatusCode::UNAUTHORIZED, "Invalid Teams auth token").into_response();
            }
            Err(error) => {
                record_operator_tool_status("channels.teams.ingress", started_at, "failure");
                return (StatusCode::UNAUTHORIZED, error.to_string()).into_response();
            }
        }
    }

    match state.handler.handle_request(payload).await {
        Ok(response) => {
            record_operator_tool_status("channels.teams.ingress", started_at, "success");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => {
            record_operator_tool_status("channels.teams.ingress", started_at, "failure");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

fn mattermost_ingress_router(path: &str, handler: MattermostWebhookHandler) -> Router {
    Router::new()
        .route(path, post(mattermost_events_handler))
        .with_state(MattermostIngressState {
            handler: Arc::new(handler),
        })
}

async fn mattermost_events_handler(
    State(state): State<MattermostIngressState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());

    match state.handler.handle_request(content_type, &body).await {
        Ok(response) => {
            record_operator_tool_status("channels.mattermost.ingress", started_at, "success");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => {
            record_operator_tool_status("channels.mattermost.ingress", started_at, "failure");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct GoogleChatIngressState {
    handler: Arc<GoogleChatWebhookHandler>,
}

fn google_chat_ingress_router(handler: GoogleChatWebhookHandler) -> Router {
    Router::new()
        .route(
            "/webhooks/google-chat/events",
            post(google_chat_events_handler),
        )
        .with_state(GoogleChatIngressState {
            handler: Arc::new(handler),
        })
}

async fn google_chat_events_handler(
    State(state): State<GoogleChatIngressState>,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    match state.handler.handle_event(&body).await {
        Ok(Some(response)) => {
            record_operator_tool_status("channels.google_chat.ingress", started_at, "success");
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            record_operator_tool_status("channels.google_chat.ingress", started_at, "success");
            StatusCode::OK.into_response()
        }
        Err(error) => {
            record_operator_tool_status("channels.google_chat.ingress", started_at, "failure");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct GoogleMeetIngressState {
    handler: Arc<GoogleMeetWebhookHandler>,
    event_bus: DurableEventBus,
}

fn google_meet_ingress_router(
    path: &str,
    handler: GoogleMeetWebhookHandler,
    event_bus: DurableEventBus,
) -> Router {
    Router::new()
        .route(path, post(google_meet_events_handler))
        .with_state(GoogleMeetIngressState {
            handler: Arc::new(handler),
            event_bus,
        })
}

fn google_meet_dedupe_key(event: &openrustclaw_channels::DecodedGoogleMeetEvent) -> Option<String> {
    let resource = event
        .transcript
        .as_deref()
        .or(event.recording.as_deref())
        .or(event.participant.as_deref())
        .or(event.conference_record.as_deref())
        .or(event.space.as_deref())?;
    Some(format!("{}:{}", event.event_type, resource))
}

async fn google_meet_events_handler(
    State(state): State<GoogleMeetIngressState>,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    match state.handler.decode_push(&body).await {
        Ok(event) => {
            let payload = serde_json::json!({
                "event_type": event.event_type,
                "conference_record": event.conference_record,
                "transcript": event.transcript,
                "recording": event.recording,
                "participant": event.participant,
                "space": event.space,
                "transcript_text": event.transcript_text,
                "raw": event.raw,
            });

            if let Err(error) = state
                .event_bus
                .publish_named(
                    "google_meet.event_received",
                    "google_meet_event",
                    None,
                    &payload,
                    google_meet_dedupe_key(&event).as_deref(),
                )
                .await
            {
                record_operator_tool_status("channels.google_meet.ingress", started_at, "failure");
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }

            if let Some(transcript_text) = event.transcript_text.as_ref() {
                let transcript_payload = serde_json::json!({
                    "event_type": event.event_type,
                    "transcript": event.transcript,
                    "conference_record": event.conference_record,
                    "space": event.space,
                    "transcript_text": transcript_text,
                });
                if let Err(error) = state
                    .event_bus
                    .publish_named(
                        "google_meet.transcript_hydrated",
                        "google_meet_transcript",
                        None,
                        &transcript_payload,
                        event.transcript.as_deref(),
                    )
                    .await
                {
                    record_operator_tool_status(
                        "channels.google_meet.ingress",
                        started_at,
                        "failure",
                    );
                    return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
                }
            }

            record_operator_tool_status("channels.google_meet.ingress", started_at, "success");
            (StatusCode::OK, Json(payload)).into_response()
        }
        Err(error) => {
            record_operator_tool_status("channels.google_meet.ingress", started_at, "failure");
            (StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct GmailIngressState {
    handler: Arc<GmailWebhookHandler>,
    workspace_root: PathBuf,
}

fn gmail_ingress_router(handler: GmailWebhookHandler, workspace_root: PathBuf) -> Router {
    Router::new()
        .route("/webhooks/gmail/pubsub", post(gmail_pubsub_handler))
        .with_state(GmailIngressState {
            handler: Arc::new(handler),
            workspace_root,
        })
}

async fn gmail_pubsub_handler(
    State(state): State<GmailIngressState>,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
    match state.handler.handle_push(&body).await {
        Ok(report) => {
            openrustclaw_observability::metrics::record_tool_execution(
                "channels.gmail_pubsub.ingress",
                "success",
            );
            openrustclaw_observability::metrics::record_tool_duration(
                "channels.gmail_pubsub.ingress",
                started_at.elapsed().as_secs_f64(),
            );
            persist_operator_execution_record(
                "channels.gmail_pubsub.ingress",
                "runtime_tool",
                "success",
                started_at,
                None,
                None,
                Some(serde_json::json!({
                    "body_bytes": body.len(),
                    "workspace_root": state.workspace_root.display().to_string(),
                })),
                Some(serde_json::json!(report)),
            );
            StatusCode::OK.into_response()
        }
        Err(error) => {
            let error_text = error.to_string();
            openrustclaw_observability::metrics::record_tool_execution(
                "channels.gmail_pubsub.ingress",
                "failure",
            );
            openrustclaw_observability::metrics::record_tool_duration(
                "channels.gmail_pubsub.ingress",
                started_at.elapsed().as_secs_f64(),
            );
            persist_operator_execution_record(
                "channels.gmail_pubsub.ingress",
                "runtime_tool",
                "failure",
                started_at,
                Some(error_text.as_str()),
                None,
                Some(serde_json::json!({
                    "body_bytes": body.len(),
                    "workspace_root": state.workspace_root.display().to_string(),
                })),
                None,
            );
            (StatusCode::BAD_REQUEST, error_text).into_response()
        }
    }
}

async fn discord_interactions_handler(
    State(state): State<DiscordIngressState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = std::time::Instant::now();
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
            record_operator_tool_status("channels.discord.ingress", started_at, "success");
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
            record_operator_tool_status("channels.discord.ingress", started_at, "failure");
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
            record_operator_tool_status("channels.discord.ingress", started_at, "failure");
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
            record_operator_tool_status("channels.discord.ingress", started_at, "failure");
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
    let compiled_skill_artifacts = load_compiled_skill_artifacts(&workspace_root);
    let mut server = McpServer::new(McpServerConfig {
        name: "openrustclaw".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tools: {
            let mut tools = vec![
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
            McpServerTool {
                name: "browser_open_session".to_string(),
                description: "Create a durable browser session descriptor for native or agent-browser execution.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backend": {"type": "string"},
                        "session_id": {"type": "string"},
                        "label": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "browser_list_sessions".to_string(),
                description: "List durable browser session descriptors.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            McpServerTool {
                name: "browser_read_page".to_string(),
                description: "Fetch a page over HTTP and save a normalized read artifact.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "max_chars": {"type": "integer", "minimum": 1},
                        "path": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1}
                    },
                    "required": ["url"]
                }),
            },
            McpServerTool {
                name: "browser_crawl_site".to_string(),
                description: "Run a bounded same-domain crawl and save a crawl artifact.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "max_pages": {"type": "integer", "minimum": 1},
                        "max_chars_per_page": {"type": "integer", "minimum": 1},
                        "path": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1}
                    },
                    "required": ["url"]
                }),
            },
            McpServerTool {
                name: "browser_navigate".to_string(),
                description: "Navigate to a page through the bounded browser runtime.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "backend": {"type": "string"},
                        "session_id": {"type": "string"},
                        "wait_until": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1}
                    },
                    "required": ["url"]
                }),
            },
            McpServerTool {
                name: "browser_extract".to_string(),
                description: "Extract text, HTML, links, images, forms, or headings from a page.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "backend": {"type": "string"},
                        "session_id": {"type": "string"},
                        "what": {"type": "string"},
                        "selector": {"type": "string"},
                        "wait_until": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1},
                        "max_results": {"type": "integer", "minimum": 1},
                        "max_chars": {"type": "integer", "minimum": 1}
                    },
                    "required": ["url"]
                }),
            },
            McpServerTool {
                name: "browser_inspect".to_string(),
                description: "Inspect a page as a DOM/accessibility snapshot or structured surface.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "backend": {"type": "string"},
                        "kind": {"type": "string"},
                        "session_id": {"type": "string"},
                        "selector": {"type": "string"},
                        "interactive_only": {"type": "boolean"},
                        "snapshot_depth": {"type": "integer", "minimum": 1},
                        "wait_until": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1},
                        "path": {"type": "string"}
                    },
                    "required": ["url"]
                }),
            },
            McpServerTool {
                name: "browser_run_sequence".to_string(),
                description: "Run a bounded browser action sequence through the shared control contract.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backend": {"type": "string"},
                        "session_id": {"type": "string"},
                        "timeout_ms": {"type": "integer", "minimum": 1},
                        "path": {"type": "string"},
                        "steps": {"type": "array", "items": {"type": "object"}}
                    },
                    "required": ["steps"]
                }),
            },
            McpServerTool {
                name: "browser_list_artifacts".to_string(),
                description: "List browser artifacts saved under the workspace browser root.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "minimum": 1}
                    }
                }),
            },
            ];
            tools.extend(compiled_skill_mcp_tools(&compiled_skill_artifacts));
            tools
        },
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

    let root_for_browser_sessions = workspace_root.clone();
    server.register_handler(
        "browser_open_session",
        traced_mcp_handler(langsmith.clone(), "browser_open_session", move |args| {
            let request: browser::BrowserOpenSessionRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_sessions.clone();
            block_on_tool(async move {
                let result = browser::open_session(&workspace_root, request)
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_session_list = workspace_root.clone();
    server.register_handler(
        "browser_list_sessions",
        traced_mcp_handler(langsmith.clone(), "browser_list_sessions", move |args| {
            let limit = args
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(20) as usize;
            let workspace_root = root_for_browser_session_list.clone();
            block_on_tool(async move {
                browser::list_sessions(&workspace_root, limit)
                    .map(|sessions| serde_json::json!({ "sessions": sessions }))
                    .map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_read = workspace_root.clone();
    server.register_handler(
        "browser_read_page",
        traced_mcp_handler(langsmith.clone(), "browser_read_page", move |args| {
            let request: browser::BrowserReadPageRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_read.clone();
            block_on_tool(async move {
                let result = browser::read_page(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_crawl = workspace_root.clone();
    server.register_handler(
        "browser_crawl_site",
        traced_mcp_handler(langsmith.clone(), "browser_crawl_site", move |args| {
            let request: browser::BrowserCrawlRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_crawl.clone();
            block_on_tool(async move {
                let result = browser::crawl_site(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_navigate = workspace_root.clone();
    server.register_handler(
        "browser_navigate",
        traced_mcp_handler(langsmith.clone(), "browser_navigate", move |args| {
            let request: browser::BrowserNavigateRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_navigate.clone();
            block_on_tool(async move {
                let result = browser::navigate(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_extract = workspace_root.clone();
    server.register_handler(
        "browser_extract",
        traced_mcp_handler(langsmith.clone(), "browser_extract", move |args| {
            let request: browser::BrowserExtractRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_extract.clone();
            block_on_tool(async move {
                let result = browser::extract(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_inspect = workspace_root.clone();
    server.register_handler(
        "browser_inspect",
        traced_mcp_handler(langsmith.clone(), "browser_inspect", move |args| {
            let request: browser::BrowserInspectRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_inspect.clone();
            block_on_tool(async move {
                let result = browser::inspect(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_sequence = workspace_root.clone();
    server.register_handler(
        "browser_run_sequence",
        traced_mcp_handler(langsmith.clone(), "browser_run_sequence", move |args| {
            let request: browser::BrowserRunSequenceRequest = parse_tool_args(args)?;
            let workspace_root = root_for_browser_sequence.clone();
            block_on_tool(async move {
                let result = browser::run_sequence(&workspace_root, request)
                    .await
                    .map_err(|error| mcp_tool_error(error.to_string()))?;
                serde_json::to_value(result).map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    let root_for_browser_artifacts = workspace_root.clone();
    server.register_handler(
        "browser_list_artifacts",
        traced_mcp_handler(langsmith.clone(), "browser_list_artifacts", move |args| {
            let limit = args
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(20) as usize;
            let workspace_root = root_for_browser_artifacts.clone();
            block_on_tool(async move {
                browser::list_artifacts(&workspace_root, limit)
                    .map(|artifacts| serde_json::json!({ "artifacts": artifacts }))
                    .map_err(|error| mcp_tool_error(error.to_string()))
            })
        }),
    );

    register_compiled_skill_mcp_handlers(&mut server, &compiled_skill_artifacts, langsmith.clone());

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
    let workspace_root_for_spawn = workspace_root.clone();
    server.register_handler(
        "spawn_session",
        traced_mcp_handler(langsmith.clone(), "spawn_session", move |args| {
            let request: McpSpawnSessionArgs = parse_tool_args(args)?;
            let session_store = session_store_for_spawn.clone();
            let workspace_root = workspace_root_for_spawn.clone();
            block_on_tool(async move {
                let platform = parse_mcp_platform(request.platform.as_deref().unwrap_or("webchat"));
                let mut session = openrustclaw_core::types::Session::new(
                    parse_mcp_session_type(request.session_type.as_deref().unwrap_or("dm")),
                    request.user_id,
                    platform,
                );
                session.workspace_id = request.workspace_id;
                session.metadata = assistant::session_metadata_for_platform(
                    platform,
                    request.route_key.as_deref(),
                    Some(&workspace_root),
                    serde_json::json!({
                        "spawned_by": "mcp",
                        "route_key": request.route_key,
                    }),
                );
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

fn compiled_skill_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".claw").join("skills").join("compiled")
}

fn sanitize_compiled_skill_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' => ch,
            _ => '-',
        })
        .collect::<String>()
        .replace('/', "__")
}

fn compiled_skill_tool_prefix(name: &str) -> String {
    format!("skill.{}", sanitize_compiled_skill_name(name))
}

fn compiled_skill_summary_tool_name(artifact: &CompiledSkillArtifact) -> String {
    format!(
        "{}.summary",
        compiled_skill_tool_prefix(&artifact.manifest.name)
    )
}

fn compiled_skill_details_tool_name(artifact: &CompiledSkillArtifact) -> String {
    format!(
        "{}.details",
        compiled_skill_tool_prefix(&artifact.manifest.name)
    )
}

fn compiled_skill_reference_tool_name(artifact: &CompiledSkillArtifact, reference: &str) -> String {
    format!(
        "{}.reference.{}",
        compiled_skill_tool_prefix(&artifact.manifest.name),
        sanitize_compiled_skill_name(reference)
    )
}

fn compiled_skill_execute_tool_name(artifact: &CompiledSkillArtifact) -> String {
    format!(
        "{}.execute",
        compiled_skill_tool_prefix(&artifact.manifest.name)
    )
}

fn compiled_skill_schedule_tool_name(artifact: &CompiledSkillArtifact) -> String {
    format!(
        "{}.schedule",
        compiled_skill_tool_prefix(&artifact.manifest.name)
    )
}

fn compiled_skill_executable_components(artifact: &CompiledSkillArtifact) -> Vec<String> {
    artifact
        .manifest
        .scripts
        .iter()
        .chain(artifact.manifest.references.iter())
        .filter(|path| path.ends_with(".wasm") || path.ends_with(".wat"))
        .cloned()
        .collect()
}

fn load_compiled_skill_artifacts(workspace_root: &Path) -> Vec<CompiledSkillArtifact> {
    let root = compiled_skill_root(workspace_root);
    let manifests = match list_compiled_manifests(&root) {
        Ok(manifests) => manifests,
        Err(error) => {
            warn!(
                path = %root.display(),
                error = %error,
                "Failed to load compiled skill manifests for MCP registration"
            );
            return Vec::new();
        }
    };

    manifests
        .into_iter()
        .filter_map(
            |manifest| match load_compiled_artifact(&root, &manifest.name) {
                Ok(artifact) => Some(artifact),
                Err(error) => {
                    warn!(
                        skill = %manifest.name,
                        path = %root.display(),
                        error = %error,
                        "Failed to load compiled skill artifact for MCP registration"
                    );
                    None
                }
            },
        )
        .collect()
}

fn compiled_skill_mcp_tools(artifacts: &[CompiledSkillArtifact]) -> Vec<McpServerTool> {
    if artifacts.is_empty() {
        return Vec::new();
    }

    let mut tools = vec![
        McpServerTool {
            name: "list_compiled_skills".to_string(),
            description: "List compiled skills that are available to the MCP server.".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        McpServerTool {
            name: "inspect_compiled_skill".to_string(),
            description: "Inspect one compiled skill artifact bundle from the workspace cache."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            }),
        },
    ];

    for artifact in artifacts {
        tools.push(McpServerTool {
            name: compiled_skill_summary_tool_name(artifact),
            description: format!(
                "Return the token-efficient compiled summary for skill '{}'.",
                artifact.manifest.name
            ),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        });
        tools.push(McpServerTool {
            name: compiled_skill_details_tool_name(artifact),
            description: format!(
                "Return the compiled detail bundle for skill '{}'.",
                artifact.manifest.name
            ),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        });

        let executable_components = compiled_skill_executable_components(artifact);
        let background_services = compiled_skill_background_services(artifact);
        if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked)
            && !executable_components.is_empty()
        {
            let mut execute_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "description": "Optional JSON value forwarded into the bounded Rust/WASM skill executor."
                    }
                }
            });
            if executable_components.len() > 1 {
                execute_schema["properties"]["component"] = serde_json::json!({
                    "type": "string",
                    "description": "Executable `.wasm` or `.wat` component path from the compiled skill artifact."
                });
                execute_schema["required"] = serde_json::json!(["component"]);
            } else {
                execute_schema["properties"]["component"] = serde_json::json!({
                    "type": "string",
                    "description": "Optional executable component path. Omit to use the only available `.wasm`/`.wat` artifact."
                });
            }
            tools.push(McpServerTool {
                name: compiled_skill_execute_tool_name(artifact),
                description: format!(
                    "Execute bounded Rust/WASM component{} for skill '{}'.",
                    if executable_components.len() == 1 {
                        format!(" '{}'", executable_components[0])
                    } else {
                        "s".to_string()
                    },
                    artifact.manifest.name
                ),
                input_schema: execute_schema,
            });
        }

        if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked)
            && !background_services.is_empty()
        {
            tools.push(McpServerTool {
                name: compiled_skill_schedule_tool_name(artifact),
                description: format!(
                    "Schedule a durable background workflow for compiled skill '{}'.",
                    artifact.manifest.name
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "service": {"type": "string"},
                        "component": {"type": "string"},
                        "input": {
                            "description": "Optional JSON value forwarded into the background workflow executor."
                        },
                        "every_seconds": {"type": "integer", "minimum": 1},
                        "at": {"type": "string", "description": "Optional RFC3339 timestamp for a one-shot execution."},
                        "priority": {"type": "integer"}
                    }
                }),
            });
        }

        if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked) {
            for reference in &artifact.manifest.references {
                tools.push(McpServerTool {
                    name: compiled_skill_reference_tool_name(artifact, reference),
                    description: format!(
                        "Read compiled skill reference '{}' from skill '{}'.",
                        reference, artifact.manifest.name
                    ),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "max_chars": {"type": "integer", "minimum": 1}
                        }
                    }),
                });
            }
        }
    }

    tools
}

fn compiled_skill_summary_payload(artifact: &CompiledSkillArtifact) -> serde_json::Value {
    serde_json::json!({
        "manifest": &artifact.manifest,
        "help": {
            "summary": &artifact.help_index.summary,
            "argument_hint": &artifact.help_index.argument_hint,
            "allowed_tools": &artifact.help_index.allowed_tools,
            "capabilities": &artifact.help_index.capabilities,
            "scripts": &artifact.help_index.scripts,
            "references": &artifact.help_index.references,
            "safety_notes": &artifact.help_index.safety_notes,
        },
        "cli": &artifact.cli_schema,
        "mcp": {
            "summary_tool": compiled_skill_summary_tool_name(artifact),
            "details_tool": compiled_skill_details_tool_name(artifact),
            "execute_tool": if compiled_skill_executable_components(artifact).is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(compiled_skill_execute_tool_name(artifact))
            },
            "schedule_tool": if compiled_skill_background_services(artifact).is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(compiled_skill_schedule_tool_name(artifact))
            },
            "executable_components": compiled_skill_executable_components(artifact),
            "background_services": compiled_skill_background_services(artifact),
            "reference_tools": artifact
                .manifest
                .references
                .iter()
                .map(|reference| compiled_skill_reference_tool_name(artifact, reference))
                .collect::<Vec<_>>(),
        },
        "scan_report": &artifact.scan_report,
    })
}

fn compiled_skill_detail_payload(artifact: &CompiledSkillArtifact) -> serde_json::Value {
    let blocked = matches!(artifact.manifest.status, CompiledSkillStatus::Blocked);
    serde_json::json!({
        "manifest": &artifact.manifest,
        "help_index": {
            "summary": &artifact.help_index.summary,
            "body_excerpt": if blocked { serde_json::Value::Null } else { serde_json::json!(&artifact.help_index.body_excerpt) },
            "argument_hint": &artifact.help_index.argument_hint,
            "allowed_tools": &artifact.help_index.allowed_tools,
            "capabilities": &artifact.help_index.capabilities,
            "scripts": &artifact.help_index.scripts,
            "references": &artifact.help_index.references,
            "examples": if blocked { serde_json::json!([]) } else { serde_json::json!(&artifact.help_index.examples) },
            "safety_notes": &artifact.help_index.safety_notes,
        },
        "mcp_schema": &artifact.mcp_schema,
        "cli_schema": &artifact.cli_schema,
        "background_services": compiled_skill_background_services(artifact),
        "scan_report": &artifact.scan_report,
        "blocked_content_redacted": blocked,
    })
}

fn read_compiled_skill_reference(
    artifact: &CompiledSkillArtifact,
    reference: &str,
    max_chars: Option<usize>,
) -> openrustclaw_core::error::Result<serde_json::Value> {
    let skill_file = PathBuf::from(&artifact.manifest.local_path);
    let skill_root = skill_file.parent().ok_or_else(|| {
        mcp_tool_error(format!(
            "Compiled skill '{}' does not have a resolvable root",
            artifact.manifest.name
        ))
    })?;
    let canonical_root = skill_root.canonicalize().map_err(|error| {
        mcp_tool_error(format!(
            "Failed to canonicalize skill root for '{}': {}",
            artifact.manifest.name, error
        ))
    })?;
    let reference_path = skill_root.join(reference);
    let canonical_reference = reference_path.canonicalize().map_err(|error| {
        mcp_tool_error(format!(
            "Failed to resolve reference '{}' for '{}': {}",
            reference, artifact.manifest.name, error
        ))
    })?;
    if !canonical_reference.starts_with(&canonical_root) {
        return Err(mcp_tool_error(format!(
            "Reference '{}' escapes the skill root for '{}'",
            reference, artifact.manifest.name
        )));
    }

    let bytes = std::fs::read(&canonical_reference).map_err(|error| {
        mcp_tool_error(format!(
            "Failed to read reference '{}' for '{}': {}",
            reference, artifact.manifest.name, error
        ))
    })?;
    let metadata = std::fs::metadata(&canonical_reference).map_err(|error| {
        mcp_tool_error(format!(
            "Failed to stat reference '{}' for '{}': {}",
            reference, artifact.manifest.name, error
        ))
    })?;
    let max_chars = max_chars.unwrap_or(4000).max(1);
    match String::from_utf8(bytes) {
        Ok(text) => {
            let char_len = text.chars().count();
            let truncated = char_len > max_chars;
            let content = if truncated {
                text.chars().take(max_chars).collect::<String>()
            } else {
                text
            };
            Ok(serde_json::json!({
                "skill": artifact.manifest.name,
                "reference": reference,
                "path": canonical_reference.display().to_string(),
                "binary": false,
                "bytes": metadata.len(),
                "truncated": truncated,
                "content": content,
            }))
        }
        Err(error) => Ok(serde_json::json!({
            "skill": artifact.manifest.name,
            "reference": reference,
            "path": canonical_reference.display().to_string(),
            "binary": true,
            "bytes": metadata.len(),
            "encoding_error": error.to_string(),
        })),
    }
}

fn register_compiled_skill_mcp_handlers(
    server: &mut McpServer,
    artifacts: &[CompiledSkillArtifact],
    langsmith: Option<LangSmithClient>,
) {
    if artifacts.is_empty() {
        return;
    }

    let manifests = artifacts
        .iter()
        .map(|artifact| artifact.manifest.clone())
        .collect::<Vec<_>>();
    server.register_handler(
        "list_compiled_skills",
        traced_mcp_handler(langsmith.clone(), "list_compiled_skills", move |_| {
            Ok(serde_json::json!({ "skills": manifests }))
        }),
    );

    let artifacts_by_name = artifacts
        .iter()
        .map(|artifact| (artifact.manifest.name.clone(), artifact.clone()))
        .collect::<HashMap<_, _>>();
    server.register_handler(
        "inspect_compiled_skill",
        traced_mcp_handler(langsmith.clone(), "inspect_compiled_skill", move |args| {
            let request: McpInspectCompiledSkillArgs = parse_tool_args(args)?;
            let artifact = artifacts_by_name.get(&request.name).ok_or_else(|| {
                mcp_tool_error(format!("Compiled skill '{}' was not found", request.name))
            })?;
            Ok(compiled_skill_detail_payload(artifact))
        }),
    );

    for artifact in artifacts {
        let summary_artifact = artifact.clone();
        let summary_tool = compiled_skill_summary_tool_name(artifact);
        server.register_handler(
            &summary_tool,
            traced_mcp_handler(langsmith.clone(), "compiled_skill_summary", move |_| {
                Ok(compiled_skill_summary_payload(&summary_artifact))
            }),
        );

        let detail_artifact = artifact.clone();
        let detail_tool = compiled_skill_details_tool_name(artifact);
        server.register_handler(
            &detail_tool,
            traced_mcp_handler(langsmith.clone(), "compiled_skill_details", move |_| {
                Ok(compiled_skill_detail_payload(&detail_artifact))
            }),
        );

        if matches!(artifact.manifest.status, CompiledSkillStatus::Blocked) {
            continue;
        }

        let executable_components = compiled_skill_executable_components(artifact);
        if !executable_components.is_empty() {
            let execute_tool = compiled_skill_execute_tool_name(artifact);
            let execute_artifact = artifact.clone();
            let available_components = executable_components.clone();
            server.register_handler(
                &execute_tool,
                traced_mcp_handler(langsmith.clone(), "compiled_skill_execute", move |args| {
                    let request: McpCompiledSkillExecuteArgs = parse_tool_args(args)?;
                    let execute_artifact = execute_artifact.clone();
                    let available_components = available_components.clone();
                    block_on_tool(async move {
                        let requested_component = request.component.as_deref();
                        if let Some(component) = requested_component
                            && !available_components
                                .iter()
                                .any(|candidate| candidate == component)
                        {
                            return Err(mcp_tool_error(format!(
                                "Component '{}' is not executable for compiled skill '{}'",
                                component, execute_artifact.manifest.name
                            )));
                        }
                        let input = request.input.as_ref().map(serde_json::Value::to_string);
                        let result = skills::execute_compiled_artifact_data(
                            &execute_artifact,
                            skills::SkillExecuteOptions {
                                component: requested_component,
                                input: input.as_deref(),
                            },
                        )
                        .await
                        .map_err(|error| mcp_tool_error(error.to_string()))?;
                        Ok(serde_json::to_value(result)
                            .map_err(|error| mcp_tool_error(error.to_string()))?)
                    })
                }),
            );
        }

        let background_services = compiled_skill_background_services(artifact);
        if !background_services.is_empty() {
            let schedule_tool = compiled_skill_schedule_tool_name(artifact);
            let skill_name = artifact.manifest.name.clone();
            server.register_handler(
                &schedule_tool,
                traced_mcp_handler(langsmith.clone(), "compiled_skill_schedule", move |args| {
                    let request: McpCompiledSkillScheduleArgs = parse_tool_args(args)?;
                    let skill_name = skill_name.clone();
                    block_on_tool(async move {
                        let input = request.input.as_ref().map(serde_json::Value::to_string);
                        let result = skills::schedule_background_service_data(
                            &skill_name,
                            skills::SkillScheduleBackgroundOptions {
                                service: request.service.as_deref(),
                                component: request.component.as_deref(),
                                input: input.as_deref(),
                                every_seconds: request.every_seconds,
                                at: request.at.as_deref(),
                                priority: request.priority.unwrap_or(100),
                            },
                        )
                        .await
                        .map_err(|error| mcp_tool_error(error.to_string()))?;
                        Ok(serde_json::to_value(result)
                            .map_err(|error| mcp_tool_error(error.to_string()))?)
                    })
                }),
            );
        }

        for reference in &artifact.manifest.references {
            let reference_name = reference.clone();
            let reference_tool = compiled_skill_reference_tool_name(artifact, &reference_name);
            let reference_artifact = artifact.clone();
            server.register_handler(
                &reference_tool,
                traced_mcp_handler(langsmith.clone(), "compiled_skill_reference", move |args| {
                    let request: McpCompiledSkillReferenceArgs = parse_tool_args(args)?;
                    read_compiled_skill_reference(
                        &reference_artifact,
                        &reference_name,
                        request.max_chars,
                    )
                }),
            );
        }
    }
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
        let started_at = std::time::Instant::now();
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
        let error = result.as_ref().err().map(|value| value.to_string());
        let result_preview = result.as_ref().ok().cloned();
        persist_operator_execution_record(
            tool_name,
            "mcp_tool",
            if result.is_ok() { "success" } else { "failure" },
            started_at,
            error.as_deref(),
            None,
            Some(trace_args.clone()),
            result_preview,
        );

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
        "mattermost" => Platform::Mattermost,
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
struct McpInspectCompiledSkillArgs {
    name: String,
}

#[derive(serde::Deserialize, Default)]
struct McpCompiledSkillReferenceArgs {
    max_chars: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
struct McpCompiledSkillExecuteArgs {
    component: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Default)]
struct McpCompiledSkillScheduleArgs {
    service: Option<String>,
    component: Option<String>,
    input: Option<serde_json::Value>,
    every_seconds: Option<u64>,
    at: Option<String>,
    priority: Option<i64>,
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
                    root: registry_root.clone(),
                    ..ChannelRegistry::default()
                })),
                langsmith: None,
                event_bus: DurableEventBus::new(pool.clone(), 16),
                voice_transcriber: None,
                workspace_root: registry_root.clone(),
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
        assert!(config.gmail_pubsub.enabled);
        assert!(config.matrix.enabled);
        assert!(!config.line.enabled);
        assert!(!config.viber.enabled);
        assert!(!config.wechat.enabled);
        assert!(!config.meta.enabled);
    }

    #[test]
    fn validate_gateway_network_mode_rejects_loopback_mismatch() {
        let mut config = AppConfig::default();
        config.gateway.network_mode = "loopback".to_string();
        config.gateway.host = "0.0.0.0".to_string();

        assert!(validate_gateway_network_mode(&config).is_err());
    }

    #[test]
    fn validate_gateway_network_mode_accepts_remote_with_auth() {
        let mut config = AppConfig::default();
        config.gateway.network_mode = "remote".to_string();
        config.gateway.host = "0.0.0.0".to_string();
        config.gateway.allowed_origins = vec!["https://console.example.com".to_string()];
        config.security.require_auth = true;

        assert!(validate_gateway_network_mode(&config).is_ok());
    }

    #[test]
    fn control_config_round_trip_writes_toml() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("config").join("runtime.toml");
        let config = AppConfig::default();

        let rendered = validate_control_config(&config).expect("render config");
        assert!(rendered.contains("[gateway]"));

        std::fs::create_dir_all(path.parent().expect("parent dir")).expect("mkdirs");
        std::fs::write(&path, rendered.as_bytes()).expect("write config");
        let bytes = rendered.len();
        assert!(bytes > 0);

        let written = std::fs::read_to_string(path).expect("read written config");
        assert!(written.contains("[providers]"));
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

    #[tokio::test]
    async fn channel_sessions_are_tagged_as_primary_assistant_surface() {
        let (agent, _pool) = test_channel_agent().await;
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

        let session = agent
            .session_manager
            .get_session(&reply.session_id.to_string())
            .await
            .unwrap();
        assert_eq!(session.metadata["assistant_identity"], "primary");
        assert_eq!(session.metadata["assistant_surface"], "channel");
        assert_eq!(session.metadata["assistant_session_model"], "persisted");
    }

    #[test]
    fn startup_handoff_message_changes_for_existing_cli_session() {
        assert!(assistant::startup_handoff_message(true).contains("resume"));
        assert!(assistant::startup_handoff_message(false).contains("start"));
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
            attachment_download_dir: None,
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
            attachment_download_dir: None,
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
    async fn mcp_server_exposes_compiled_skill_tools() {
        let workspace = tempdir().unwrap();
        let skill_dir = workspace.path().join("skills").join("demo");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"# Demo Skill

Useful compiled skill.

argument_hint: <topic>
"#,
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("scripts").join("echo.wat"),
            r#"(module
              (memory (export "memory") 1 1)
              (data (i32.const 1024) "{\"ok\":true}")
              (func (export "alloc") (param i32) (result i32) i32.const 0)
              (func (export "run") (param i32 i32) (result i64)
                (i64.or
                  (i64.shl (i64.extend_i32_u (i32.const 1024)) (i64.const 32))
                  (i64.extend_i32_u (i32.const 11)))))"#,
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("references").join("guide.md"),
            "This is the guide for the demo skill.",
        )
        .unwrap();
        openrustclaw_skills::compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &workspace
                .path()
                .join(".claw")
                .join("skills")
                .join("compiled"),
            openrustclaw_core::types::SkillSource::Workspace,
            true,
        )
        .unwrap();

        let db_path = workspace.path().join("mcp-compiled-skill.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );

        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/list",
            "params": {}
        });
        let list_resp = server.handle_request(&list_req);
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "list_compiled_skills")
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "skill.Demo-Skill.summary")
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "skill.Demo-Skill.details")
        );
        assert!(
            tools
                .iter()
                .any(|tool| { tool["name"] == "skill.Demo-Skill.reference.references__guide.md" })
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "skill.Demo-Skill.execute")
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "skill.Demo-Skill.schedule")
        );

        let summary_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "skill.Demo-Skill.summary",
                "arguments": {}
            }
        });
        let summary_resp = server.handle_request(&summary_req);
        let summary_text = summary_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let summary_payload: serde_json::Value = serde_json::from_str(summary_text).unwrap();
        assert_eq!(summary_payload["manifest"]["name"], "Demo Skill");
        assert_eq!(summary_payload["help"]["summary"], "Useful compiled skill.");
        assert_eq!(
            summary_payload["cli"]["command"],
            "openrustclaw skills invoke Demo Skill"
        );
        assert_eq!(
            summary_payload["mcp"]["execute_tool"],
            "skill.Demo-Skill.execute"
        );
        assert_eq!(
            summary_payload["mcp"]["schedule_tool"],
            "skill.Demo-Skill.schedule"
        );
        assert_eq!(
            summary_payload["mcp"]["executable_components"][0],
            "scripts/echo.wat"
        );
        assert_eq!(
            summary_payload["mcp"]["background_services"][0]["name"],
            "echo"
        );

        let reference_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "skill.Demo-Skill.reference.references__guide.md",
                "arguments": {
                    "max_chars": 4
                }
            }
        });
        let reference_resp = server.handle_request(&reference_req);
        let reference_text = reference_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let reference_payload: serde_json::Value = serde_json::from_str(reference_text).unwrap();
        assert_eq!(reference_payload["reference"], "references/guide.md");
        assert_eq!(reference_payload["content"], "This");
        assert_eq!(reference_payload["truncated"], true);

        let execute_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "tools/call",
            "params": {
                "name": "skill.Demo-Skill.execute",
                "arguments": {
                    "input": {
                        "text": "hello"
                    }
                }
            }
        });
        let execute_resp = server.handle_request(&execute_req);
        let execute_text = execute_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let execute_payload: serde_json::Value = serde_json::from_str(execute_text).unwrap();
        assert_eq!(execute_payload["skill_name"], "Demo Skill");
        assert_eq!(execute_payload["component"], "scripts/echo.wat");
        assert_eq!(execute_payload["output"], serde_json::json!({"ok": true}));
        assert_eq!(
            execute_payload["input"],
            serde_json::json!({"text": "hello"})
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mcp_server_redacts_blocked_compiled_skill_content() {
        let workspace = tempdir().unwrap();
        let skill_dir = workspace.path().join("skills").join("danger");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Danger Skill\n\nSecretive.\n").unwrap();
        std::fs::write(skill_dir.join("scripts").join("run.sh"), "rm -rf /\n").unwrap();
        std::fs::write(
            skill_dir.join("references").join("notes.md"),
            "should not be exposed",
        )
        .unwrap();
        openrustclaw_skills::compile_skill_to_dir(
            &skill_dir.join("SKILL.md"),
            &workspace
                .path()
                .join(".claw")
                .join("skills")
                .join("compiled"),
            openrustclaw_core::types::SkillSource::Marketplace,
            false,
        )
        .unwrap();

        let db_path = workspace.path().join("mcp-blocked-skill.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = init_pool(&db_url, 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let server = build_mcp_server(
            workspace.path().to_path_buf(),
            pool,
            AppConfig::default(),
            None,
        );

        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "tools/list",
            "params": {}
        });
        let list_resp = server.handle_request(&list_req);
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"] == "skill.Danger-Skill.summary")
        );
        assert!(
            !tools.iter().any(|tool| {
                tool["name"] == "skill.Danger-Skill.reference.references__notes.md"
            })
        );
        assert!(
            !tools
                .iter()
                .any(|tool| tool["name"] == "skill.Danger-Skill.execute")
        );
        assert!(
            !tools
                .iter()
                .any(|tool| tool["name"] == "skill.Danger-Skill.schedule")
        );

        let details_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 44,
            "method": "tools/call",
            "params": {
                "name": "skill.Danger-Skill.details",
                "arguments": {}
            }
        });
        let details_resp = server.handle_request(&details_req);
        let details_text = details_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let details_payload: serde_json::Value = serde_json::from_str(details_text).unwrap();
        assert_eq!(details_payload["manifest"]["status"], "blocked");
        assert!(details_payload["help_index"]["body_excerpt"].is_null());
        assert_eq!(details_payload["blocked_content_redacted"], true);
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
