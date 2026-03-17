//! Configuration structs for OpenRustClaw.
//!
//! Maps to `config/default.toml` and environment variable overrides.
//! Uses the `config` crate for layered configuration loading.

use serde::{Deserialize, Serialize};

/// Root configuration for the entire OpenRustClaw system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gateway: GatewayConfig,
    pub database: DatabaseConfig,
    pub providers: ProvidersConfig,
    pub memory: MemoryConfig,
    pub scheduler: SchedulerConfig,
    pub security: SecurityConfig,
    pub sidecar: SidecarConfig,
    pub observability: ObservabilityConfig,
    pub channels: ChannelsConfig,
}

/// Gateway (WebSocket server) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub wal_mode: bool,
    pub max_connections: u32,
}

/// Provider configuration container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    pub anthropic: AnthropicConfig,
    pub openai: OpenAiConfig,
    pub openrouter: OpenRouterConfig,
    pub ollama: OllamaConfig,
}

/// Anthropic provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub model: String,
    pub api_version: String,
    pub strict_tools: bool,
    pub streaming_tool_deltas: bool,
}

/// OpenAI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub model: String,
    pub use_responses_api: bool,
    pub strict_tools: bool,
}

/// OpenRouter provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub route_strategy: String,
}

/// Ollama (local model) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

/// Memory system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub core_memory_max_tokens: usize,
    pub core_memory_max_entries: usize,
    pub embedding_concurrency: usize,
    pub dedupe_cosine_threshold: f32,
    pub decay_half_life_days: f64,
    pub ttl: MemoryTtlConfig,
    pub consolidation: ConsolidationConfig,
}

/// Memory TTL (time-to-live) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTtlConfig {
    /// Days until episodic memories expire (0 = never).
    pub episodic_days: u64,
    /// Days until semantic memories expire (0 = never).
    pub semantic_days: u64,
    /// Days until procedural memories expire (0 = never).
    pub procedural_days: u64,
}

/// Memory consolidation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    pub enabled: bool,
    pub threshold_entries: usize,
    pub schedule_interval_hours: u64,
}

/// Scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub poll_interval_ms: u64,
    pub lease_duration_secs: u64,
    pub max_retries: u32,
    pub base_retry_delay_secs: u64,
    pub max_retry_delay_secs: u64,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_auth: bool,
    pub origin_validation: bool,
    pub prompt_injection_defense: bool,
    pub skill_signature_required: bool,
}

/// Python sidecar configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarConfig {
    pub grpc_port: u16,
    pub python_path: String,
    pub auto_start: bool,
    pub restart_on_crash: bool,
}

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub langsmith_enabled: bool,
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub metrics_port: u16,
}

/// Channel integrations configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub telegram: TelegramConfig,
    pub discord: DiscordConfig,
    pub slack: SlackConfig,
}

/// Telegram bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub token: String,
    pub mode: TelegramMode,
    pub webhook_url: Option<String>,
    pub webhook_port: Option<u16>,
    pub allowed_users: Vec<String>,
    pub rate_limit_per_second: u32,
}

/// Telegram connection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramMode {
    Polling,
    Webhook,
}

/// Discord bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub token: String,
    pub application_id: String,
    pub rate_limit_requests_per_second: u32,
    pub allowed_guilds: Vec<String>,
    pub allowed_channels: Vec<String>,
    pub dm_enabled: bool,
}

/// Slack app configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub enabled: bool,
    pub token: String,
    pub app_token: Option<String>,
    pub signing_secret: Option<String>,
    pub mode: SlackMode,
    pub socket_mode: bool,
    pub rate_limit_requests_per_second: u32,
    pub allowed_workspaces: Vec<String>,
    pub app_home_enabled: bool,
}

/// Slack connection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlackMode {
    Http,
    SocketMode,
}

impl AppConfig {
    /// Load configuration from default.toml, then overlay environment variables.
    ///
    /// Environment variables use the prefix `OPENRUSTCLAW_` with `__` as separator.
    /// Example: `OPENRUSTCLAW_GATEWAY__PORT=8080`
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                config::Environment::with_prefix("OPENRUSTCLAW")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Load from a specific config file path.
    pub fn load_from(path: &str) -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(
                config::Environment::with_prefix("OPENRUSTCLAW")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig {
                host: "127.0.0.1".to_string(),
                port: 18789,
                allowed_origins: vec![
                    "http://localhost:3000".to_string(),
                    "http://127.0.0.1:3000".to_string(),
                ],
            },
            database: DatabaseConfig {
                url: "sqlite://data/openrustclaw.db".to_string(),
                wal_mode: true,
                max_connections: 10,
            },
            providers: ProvidersConfig {
                default_provider: "anthropic".to_string(),
                fallback_chain: vec![
                    "anthropic".to_string(),
                    "openai".to_string(),
                    "openrouter".to_string(),
                ],
                anthropic: AnthropicConfig {
                    model: "claude-sonnet-4-20250514".to_string(),
                    api_version: "2023-06-01".to_string(),
                    strict_tools: true,
                    streaming_tool_deltas: true,
                },
                openai: OpenAiConfig {
                    model: "gpt-4o".to_string(),
                    use_responses_api: true,
                    strict_tools: true,
                },
                openrouter: OpenRouterConfig {
                    route_strategy: "quality".to_string(),
                },
                ollama: OllamaConfig {
                    base_url: "http://localhost:11434".to_string(),
                    model: "llama3.1".to_string(),
                },
            },
            memory: MemoryConfig {
                core_memory_max_tokens: 500,
                core_memory_max_entries: 20,
                embedding_concurrency: 4,
                dedupe_cosine_threshold: 0.92,
                decay_half_life_days: 30.0,
                ttl: MemoryTtlConfig {
                    episodic_days: 90,
                    semantic_days: 0,
                    procedural_days: 0,
                },
                consolidation: ConsolidationConfig {
                    enabled: true,
                    threshold_entries: 1000,
                    schedule_interval_hours: 24,
                },
            },
            scheduler: SchedulerConfig {
                poll_interval_ms: 1000,
                lease_duration_secs: 60,
                max_retries: 3,
                base_retry_delay_secs: 5,
                max_retry_delay_secs: 300,
            },
            security: SecurityConfig {
                require_auth: true,
                origin_validation: true,
                prompt_injection_defense: true,
                skill_signature_required: false,
            },
            sidecar: SidecarConfig {
                grpc_port: 50051,
                python_path: "python3".to_string(),
                auto_start: true,
                restart_on_crash: true,
            },
            observability: ObservabilityConfig {
                langsmith_enabled: false,
                tracing_enabled: true,
                metrics_enabled: true,
                metrics_port: 9090,
            },
            channels: ChannelsConfig {
                telegram: TelegramConfig {
                    enabled: false,
                    token: String::new(),
                    mode: TelegramMode::Polling,
                    webhook_url: None,
                    webhook_port: None,
                    allowed_users: Vec::new(),
                    rate_limit_per_second: 30,
                },
                discord: DiscordConfig {
                    enabled: false,
                    token: String::new(),
                    application_id: String::new(),
                    rate_limit_requests_per_second: 5,
                    allowed_guilds: Vec::new(),
                    allowed_channels: Vec::new(),
                    dm_enabled: true,
                },
                slack: SlackConfig {
                    enabled: false,
                    token: String::new(),
                    app_token: None,
                    signing_secret: None,
                    mode: SlackMode::SocketMode,
                    socket_mode: true,
                    rate_limit_requests_per_second: 10,
                    allowed_workspaces: Vec::new(),
                    app_home_enabled: true,
                },
            },
        }
    }
}
