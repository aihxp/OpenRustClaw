//! Session operator commands.

use anyhow::{Context, Result};
use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::{CoreMemoryStore, LlmProvider};
use openrustclaw_core::types::{Platform, Session, SessionType};
use openrustclaw_db::{
    SessionStatus, SqliteCoreMemoryStore, SqliteMemoryStore, SqliteSessionStore, init_pool,
    run_migrations,
};
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider, ProviderChain,
    openrouter::RouteStrategy,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn list(status: Option<&str>, limit: usize) -> Result<()> {
    let (store, _) = open_store().await?;
    let parsed_status = status.and_then(parse_status);
    let sessions = store.list_sessions(parsed_status, limit).await?;
    for session in sessions {
        println!(
            "{}  {}  {}  {}  {}",
            session.session.id,
            session.status.as_str(),
            session.session.channel,
            session.session.user_id,
            session.route_key.as_deref().unwrap_or("<no-route>")
        );
    }
    Ok(())
}

pub async fn show(id: &str, history_limit: usize) -> Result<()> {
    let (store, _) = open_store().await?;
    let session = store
        .get_session(id)
        .await?
        .with_context(|| format!("Session '{}' not found", id))?;
    println!(
        "{}\nstatus: {}\nuser: {}\nchannel: {}\ntype: {}\nroute: {}\ncreated: {}\nupdated: {}",
        session.session.id,
        session.status.as_str(),
        session.session.user_id,
        session.session.channel,
        session_type_label(session.session.session_type),
        session.route_key.as_deref().unwrap_or("<no-route>"),
        session.session.created_at,
        session.session.updated_at,
    );
    println!(
        "metadata: {}",
        serde_json::to_string_pretty(&session.session.metadata)?
    );
    println!("\nhistory:");
    for message in store.list_history(id, history_limit).await? {
        println!(
            "- [{}] {}: {}",
            message.created_at.to_rfc3339(),
            message.role,
            message.content
        );
    }
    Ok(())
}

pub async fn spawn(
    user_id: &str,
    platform: &str,
    session_type: &str,
    route_key: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<()> {
    let (store, _) = open_store().await?;
    let mut session = Session::new(
        parse_session_type(session_type),
        user_id,
        parse_platform(platform),
    );
    session.workspace_id = workspace_id.map(|value| value.to_string());
    session.metadata = json!({
        "spawned_by": "operator",
        "route_key": route_key,
    });
    store
        .create_or_update(&session, route_key, SessionStatus::Active)
        .await?;
    println!("{}", session.id);
    Ok(())
}

pub async fn close(id: &str, archive: bool, reason: Option<&str>) -> Result<()> {
    let (store, _) = open_store().await?;
    store
        .set_status(
            id,
            if archive {
                SessionStatus::Archived
            } else {
                SessionStatus::Closed
            },
            reason,
        )
        .await?;
    println!("{} -> {}", id, if archive { "archived" } else { "closed" });
    Ok(())
}

pub async fn send(id: &str, content: &str) -> Result<()> {
    let (store, config) = open_store().await?;
    let session = store
        .get_session(id)
        .await?
        .with_context(|| format!("Session '{}' not found", id))?;
    let history = store.list_history(id, 128).await?;
    let provider = build_provider(&config)?;
    let memory_store = Arc::new(SqliteMemoryStore::new(open_pool(&config).await?));
    let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(open_pool(&config).await?));
    let workspace_root = std::env::current_dir()?;
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "OpenRustClaw".to_string(),
        memory_store,
        core_memory_store.clone(),
    )
    .with_workspace_path(workspace_root);

    let user_message = openrustclaw_core::types::Message::user(content);
    store.append_message(id, &user_message).await?;
    let mut conversation = history;
    conversation.push(user_message);
    let core_memory = core_memory_store.get_all(&session.session.user_id).await?;
    let response = runtime
        .process(
            &conversation,
            &core_memory,
            &session.session.id.to_string(),
            &session.session.user_id,
        )
        .await?;
    store.append_message(id, &response.message).await?;
    println!("{}", response.message.content);
    Ok(())
}

async fn open_store() -> Result<(SqliteSessionStore, AppConfig)> {
    let config = AppConfig::load().unwrap_or_default();
    let pool = open_pool(&config).await?;
    Ok((SqliteSessionStore::new(pool), config))
}

async fn open_pool(config: &AppConfig) -> Result<sqlx::SqlitePool> {
    let pool = init_pool(&config.database.url, 4)
        .await
        .context("Failed to connect to database")?;
    run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(pool)
}

fn build_provider(config: &AppConfig) -> Result<Arc<dyn LlmProvider>> {
    let mut provider_names = Vec::new();
    provider_names.push(config.providers.default_provider.clone());
    provider_names.extend(config.providers.fallback_chain.clone());

    let mut providers = Vec::new();
    let mut seen = HashSet::new();
    for name in provider_names {
        if !seen.insert(name.clone()) {
            continue;
        }
        match create_provider_from_config(&name, config) {
            Ok(provider) => providers.push(provider),
            Err(error) if providers.is_empty() => return Err(error),
            Err(_) => {}
        }
    }

    if providers.is_empty() {
        anyhow::bail!("No providers could be initialized for session send");
    }
    if providers.len() == 1 {
        return Ok(providers.remove(0));
    }
    let primary = providers[0].clone();
    Ok(Arc::new(SessionProviderChain::new(providers, primary)))
}

fn create_provider_from_config(
    provider_name: &str,
    config: &AppConfig,
) -> Result<Arc<dyn LlmProvider>> {
    match provider_name.to_lowercase().as_str() {
        "anthropic" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY environment variable not set")?;
            Ok(Arc::new(AnthropicProvider::new(
                api_key,
                config.providers.anthropic.model.clone(),
            )))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY environment variable not set")?;
            Ok(Arc::new(OpenAiProvider::new(
                api_key,
                config.providers.openai.model.clone(),
            )))
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
            Ok(Arc::new(OpenRouterProvider::with_strategy(
                api_key,
                "anthropic/claude-sonnet-4".to_string(),
                strategy,
            )))
        }
        "ollama" => Ok(Arc::new(OllamaProvider::with_base_url(
            config.providers.ollama.model.clone(),
            config.providers.ollama.base_url.clone(),
        ))),
        _ => anyhow::bail!("Unknown provider '{}'", provider_name),
    }
}

struct SessionProviderChain {
    chain: ProviderChain,
    primary: Arc<dyn LlmProvider>,
}

impl SessionProviderChain {
    fn new(providers: Vec<Arc<dyn LlmProvider>>, primary: Arc<dyn LlmProvider>) -> Self {
        Self {
            chain: ProviderChain::new(providers),
            primary,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for SessionProviderChain {
    async fn complete(
        &self,
        request: openrustclaw_core::types::CompletionRequest,
    ) -> openrustclaw_core::error::Result<openrustclaw_core::types::CompletionResponse> {
        self.chain.complete(request).await
    }

    async fn stream(
        &self,
        _request: openrustclaw_core::types::CompletionRequest,
    ) -> openrustclaw_core::error::Result<
        std::pin::Pin<
            Box<
                dyn futures::Stream<
                        Item = openrustclaw_core::error::Result<
                            openrustclaw_core::types::StreamChunk,
                        >,
                    > + Send,
            >,
        >,
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

    fn native_tool_format(&self) -> openrustclaw_core::types::ToolFormat {
        self.primary.native_tool_format()
    }
}

fn parse_status(value: &str) -> Option<SessionStatus> {
    match value.to_lowercase().as_str() {
        "active" => Some(SessionStatus::Active),
        "archived" => Some(SessionStatus::Archived),
        "closed" => Some(SessionStatus::Closed),
        _ => None,
    }
}

fn parse_session_type(value: &str) -> SessionType {
    match value.to_lowercase().as_str() {
        "group" => SessionType::Group,
        "isolated" => SessionType::Isolated,
        _ => SessionType::Dm,
    }
}

fn session_type_label(value: SessionType) -> &'static str {
    match value {
        SessionType::Dm => "dm",
        SessionType::Group => "group",
        SessionType::Isolated => "isolated",
    }
}

fn parse_platform(value: &str) -> Platform {
    match value.to_lowercase().as_str() {
        "telegram" => Platform::Telegram,
        "discord" => Platform::Discord,
        "slack" => Platform::Slack,
        "cli" => Platform::Cli,
        "api" => Platform::Api,
        _ => Platform::WebChat,
    }
}

trait StatusLabel {
    fn as_str(&self) -> &'static str;
}

impl StatusLabel for SessionStatus {
    fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Archived => "archived",
            SessionStatus::Closed => "closed",
        }
    }
}
