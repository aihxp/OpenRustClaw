//! Configuration structs for OpenRustClaw.
//!
//! Maps to `config/default.toml` and environment variable overrides.
//! Uses the `config` crate for layered configuration loading.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub skills: Option<SkillsConfig>,
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
    pub skill_verifying_key: Option<String>,
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
    pub whatsapp: WhatsAppConfig,
    pub teams: TeamsConfig,
    pub google_chat: GoogleChatConfig,
    pub gmail_pubsub: GmailPubSubConfig,
    pub signal: SignalConfig,
    pub matrix: MatrixConfig,
    pub x: XConfig,
    pub twilio: TwilioConfig,
    pub meta: MetaConfig,
    pub imessage: IMessageConfig,
    pub line: LineConfig,
    pub viber: ViberConfig,
    pub wechat: WeChatConfig,
}

/// Skills/Plugins configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// ClawHub registry URL
    pub registry_url: Option<String>,
    /// Auto-update skills on startup
    pub auto_update: Option<bool>,
    /// Additional skill directories
    pub skill_dirs: Option<Vec<String>>,
}

/// Telegram bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub token: String,
    #[serde(default)]
    pub api_base_url: Option<String>,
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
    #[serde(default)]
    pub interaction_public_key: Option<String>,
    #[serde(default)]
    pub api_base_url: Option<String>,
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
    #[serde(default)]
    pub api_base_url: Option<String>,
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

/// WhatsApp Web configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub enabled: bool,
    /// Path to store session credentials
    pub session_path: String,
    /// Use pairing code instead of QR code
    pub pairing_mode: bool,
    /// Allowed phone numbers for DMs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Optional webhook URL for notifications
    pub webhook_url: Option<String>,
    /// Path to the Baileys bridge script
    pub bridge_path: String,
    /// Rate limit for outgoing messages per second
    pub rate_limit_per_second: u32,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Initial reconnection delay in seconds (increases with backoff)
    pub reconnect_delay_secs: u64,
}

/// Microsoft Teams bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    pub enabled: bool,
    /// Microsoft App ID (from Azure Bot registration)
    pub app_id: String,
    /// Microsoft App Password (client secret)
    pub app_password: String,
    /// Tenant ID for single-tenant apps (None for multi-tenant)
    pub tenant_id: Option<String>,
    /// Webhook path for incoming messages
    pub webhook_path: String,
    /// List of allowed user emails or AAD object IDs
    pub allowlist: Vec<String>,
    /// Group policy for mentions
    pub group_policy: TeamsGroupPolicy,
    /// Rate limit for sending messages
    pub rate_limit_requests_per_second: u32,
    /// Enable Adaptive Cards support
    pub adaptive_cards_enabled: bool,
}

/// Group policy for Microsoft Teams mentions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamsGroupPolicy {
    /// Bot responds only when @mentioned
    Mention,
    /// Bot responds to all messages in the channel
    Open,
}

/// Google Chat bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    pub enabled: bool,
    /// Path to service account JSON key file
    pub service_account_key: String,
    /// Google Cloud project ID
    pub project_id: String,
    /// Webhook URL for receiving messages (if using HTTP push)
    pub webhook_url: Option<String>,
    /// Pub/Sub subscription name (if using Pub/Sub)
    pub pubsub_subscription: Option<String>,
    /// List of allowed user emails or Google Workspace user IDs
    pub allowlist: Vec<String>,
    /// List of allowed space IDs (empty = all spaces)
    pub allowed_spaces: Vec<String>,
    /// Rate limit for sending messages
    pub rate_limit_requests_per_second: u32,
    /// Enable Card-based responses
    pub cards_enabled: bool,
    /// Response mode for the bot
    pub response_mode: GoogleChatResponseMode,
}

/// Response mode for Google Chat bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoogleChatResponseMode {
    /// Bot responds only when @mentioned
    Mention,
    /// Bot responds to all messages in the space
    Open,
    /// Bot only responds to slash commands
    SlashCommands,
}

/// Gmail Pub/Sub configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailPubSubConfig {
    pub enabled: bool,
    /// Google Cloud project ID
    pub project_id: String,
    /// Pub/Sub subscription name
    pub subscription_name: String,
    /// Path to service account JSON key file
    pub service_account_key_path: String,
    /// Gmail user email address
    pub user_email: String,
    /// Label filters (e.g., ["INBOX", "UNREAD"])
    pub label_filters: Vec<String>,
    /// Optional query filter (e.g., "from:github.com")
    pub query_filter: Option<String>,
    /// Enable auto-reply functionality
    pub auto_reply: bool,
    /// Maximum number of history records to fetch per notification
    pub max_history_fetch: u32,
    /// Rate limit for Gmail API requests per second
    pub rate_limit_requests_per_second: u32,
}

/// Signal messenger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    pub enabled: bool,
    /// Bot's phone number (E.164 format, e.g., +1234567890)
    pub phone_number: String,
    /// Path to signal-cli data directory
    pub data_dir: PathBuf,
    /// Allowed phone numbers (empty = allow all)
    pub allowlist: Vec<String>,
    /// Allowed group IDs (empty = allow all)
    pub allowed_groups: Vec<String>,
    /// Path to signal-cli binary (None = use system PATH)
    pub signal_cli_path: Option<PathBuf>,
    /// Use native libsignal-client instead of signal-cli
    pub use_libsignal: bool,
    /// Rate limit for outgoing messages per minute
    pub rate_limit_per_minute: u32,
    /// Require allowlist for security
    pub require_allowlist: bool,
}

/// Matrix protocol configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub enabled: bool,
    /// Matrix homeserver URL (e.g., "https://matrix.org")
    pub homeserver: String,
    /// Matrix user ID (e.g., "@bot:matrix.org")
    pub user_id: String,
    /// Access token for authentication (preferred over password)
    pub access_token: Option<String>,
    /// Password for authentication (if access_token not provided)
    pub password: Option<String>,
    /// Device ID for the session
    pub device_id: Option<String>,
    /// Directory to store Matrix client data (encryption keys, sync tokens)
    pub data_dir: String,
    /// List of allowed MXIDs (empty = allow all)
    pub allowlist: Vec<String>,
    /// List of allowed room IDs (empty = allow all)
    pub room_allowlist: Vec<String>,
    /// Automatically join rooms when invited
    pub auto_join_rooms: bool,
    /// Enable end-to-end encryption
    pub enable_encryption: bool,
    /// Rate limit for sending messages per second
    pub rate_limit_per_second: u32,
}

/// X (Twitter) API v2 configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XConfig {
    pub enabled: bool,
    /// Bearer token for X API v2
    pub bearer_token: String,
    /// API key (for user context endpoints)
    pub api_key: String,
    /// API secret (for user context endpoints)
    pub api_secret: String,
    /// Access token (for user context endpoints)
    pub access_token: String,
    /// Access token secret (for user context endpoints)
    pub access_token_secret: String,
    /// Bot user ID (numeric string)
    pub bot_user_id: String,
    /// List of allowed usernames (without @, empty = all allowed)
    pub allowlist: Vec<String>,
    /// Respond to mentions
    pub respond_to_mentions: bool,
    /// Respond to DMs
    pub respond_to_dms: bool,
    /// Maximum tweet length (default: 280)
    pub max_tweet_length: usize,
    /// Polling interval for mentions in seconds
    pub mention_poll_interval_secs: u64,
    /// Polling interval for DMs in seconds
    pub dm_poll_interval_secs: u64,
    /// Rate limit for sending tweets per minute
    pub rate_limit_per_minute: u32,
}

/// Twilio SMS/MMS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub enabled: bool,
    /// Twilio Account SID
    pub account_sid: String,
    /// Twilio Auth Token
    pub auth_token: String,
    /// Twilio phone number (E.164 format, e.g., +1234567890)
    pub phone_number: String,
    /// Optional webhook URL for receiving messages
    pub webhook_url: Option<String>,
    /// List of allowed phone numbers (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Maximum message length (default 1600, Twilio's limit)
    pub max_message_length: usize,
    /// Enable MMS support
    pub enable_mms: bool,
    /// Rate limit for sending messages per second
    pub rate_limit_per_second: u32,
}

/// LINE Messaging API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    pub enabled: bool,
    /// LINE Channel Access Token
    pub channel_access_token: String,
    /// LINE Channel Secret (for webhook signature verification)
    pub channel_secret: String,
    /// Webhook path for receiving events
    pub webhook_path: String,
    /// List of allowed user IDs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Rate limit for sending messages per second (default: 1000)
    pub rate_limit_per_second: u32,
    /// Enable rich menu support
    pub enable_rich_menu: bool,
    /// Enable quick replies
    pub enable_quick_replies: bool,
}

/// Viber Bot API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViberConfig {
    pub enabled: bool,
    /// Viber Bot Authentication Token
    pub auth_token: String,
    /// Webhook URL for receiving callbacks (optional)
    pub webhook_url: Option<String>,
    /// Webhook path for receiving callbacks
    pub webhook_path: String,
    /// List of allowed user IDs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Rate limit for sending messages per minute (default: 300)
    pub rate_limit_per_minute: u32,
    /// Allow broadcast messages (admin only)
    pub allow_broadcast: bool,
    /// Welcome message for new conversations
    pub welcome_message: Option<String>,
    /// Enable keyboard support
    pub enable_keyboards: bool,
}

/// WeChat configuration (supports both Work and Official Accounts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatConfig {
    pub enabled: bool,
    /// App type: "work" for WeChat Work, "official_account" for Official Accounts
    pub app_type: String,
    // WeChat Work fields
    /// WeChat Work Corp ID
    pub corp_id: String,
    /// WeChat Work Corp Secret
    pub corp_secret: String,
    /// WeChat Work Agent ID
    pub agent_id: String,
    // Official Account fields
    /// WeChat Official Account App ID
    pub app_id: String,
    /// WeChat Official Account App Secret
    pub app_secret: String,
    /// Token for webhook signature verification (Official Accounts)
    pub token: String,
    /// Encoding AES key for message encryption (optional)
    pub encoding_aes_key: Option<String>,
    /// Webhook path for receiving messages
    pub webhook_path: String,
    /// List of allowed user IDs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Rate limit for API requests per second (default: 20)
    pub rate_limit_per_second: u32,
    /// Enable message encryption
    pub enable_encryption: bool,
}

/// Meta (Messenger & Instagram) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub enabled: bool,
    /// Meta App ID
    pub app_id: String,
    /// Meta App Secret
    pub app_secret: String,
    /// Page Access Token for Messenger
    pub page_access_token: String,
    /// Webhook verify token
    pub verify_token: String,
    /// Webhook path for receiving messages
    pub webhook_path: String,
    /// Facebook Page ID
    pub page_id: String,
    /// Instagram Account ID (optional, for Instagram Direct)
    pub instagram_account_id: Option<String>,
    /// List of allowed sender IDs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Respond to Messenger messages
    pub respond_to_messenger: bool,
    /// Respond to Instagram Direct messages
    pub respond_to_instagram: bool,
    /// Show typing indicator while processing
    pub show_typing_indicator: bool,
    /// Rate limit for sending messages per second
    pub rate_limit_per_second: u32,
}

/// iMessage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    pub enabled: bool,
    /// Bridge mode for iMessage integration
    pub bridge_mode: IMessageBridgeMode,
    /// List of allowed phone numbers or Apple IDs (empty = all allowed)
    pub allowlist: Vec<String>,
    /// Enable tapback/reaction support
    pub enable_tapbacks: bool,
    /// Enable typing indicator support
    pub enable_typing_indicator: bool,
}

/// Bridge mode for iMessage integration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum IMessageBridgeMode {
    /// BlueBubbles server (works remotely)
    BlueBubbles {
        /// BlueBubbles server URL
        server_url: String,
        /// BlueBubbles API password
        password: String,
    },
    /// Direct macOS AppleScript (local only, requires macOS)
    #[default]
    MacOSDirect,
    /// macOS Messages.app private API (advanced)
    PrivateApi,
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
                skill_verifying_key: None,
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
                    api_base_url: None,
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
                    interaction_public_key: None,
                    api_base_url: None,
                    rate_limit_requests_per_second: 5,
                    allowed_guilds: Vec::new(),
                    allowed_channels: Vec::new(),
                    dm_enabled: true,
                },
                slack: SlackConfig {
                    enabled: false,
                    token: String::new(),
                    api_base_url: None,
                    app_token: None,
                    signing_secret: None,
                    mode: SlackMode::SocketMode,
                    socket_mode: true,
                    rate_limit_requests_per_second: 10,
                    allowed_workspaces: Vec::new(),
                    app_home_enabled: true,
                },
                whatsapp: WhatsAppConfig {
                    enabled: false,
                    session_path: "./data/whatsapp-session".to_string(),
                    pairing_mode: false,
                    allowlist: Vec::new(),
                    webhook_url: None,
                    bridge_path: "./crates/channels/baileys-bridge/index.js".to_string(),
                    rate_limit_per_second: 10,
                    max_reconnect_attempts: 10,
                    reconnect_delay_secs: 5,
                },
                teams: TeamsConfig {
                    enabled: false,
                    app_id: String::new(),
                    app_password: String::new(),
                    tenant_id: None,
                    webhook_path: "/webhooks/teams".to_string(),
                    allowlist: Vec::new(),
                    group_policy: TeamsGroupPolicy::Mention,
                    rate_limit_requests_per_second: 10,
                    adaptive_cards_enabled: true,
                },
                google_chat: GoogleChatConfig {
                    enabled: false,
                    service_account_key: String::new(),
                    project_id: String::new(),
                    webhook_url: None,
                    pubsub_subscription: None,
                    allowlist: Vec::new(),
                    allowed_spaces: Vec::new(),
                    rate_limit_requests_per_second: 10,
                    cards_enabled: true,
                    response_mode: GoogleChatResponseMode::Mention,
                },
                gmail_pubsub: GmailPubSubConfig {
                    enabled: false,
                    project_id: String::new(),
                    subscription_name: String::new(),
                    service_account_key_path: String::new(),
                    user_email: String::new(),
                    label_filters: vec!["INBOX".to_string(), "UNREAD".to_string()],
                    query_filter: None,
                    auto_reply: false,
                    max_history_fetch: 100,
                    rate_limit_requests_per_second: 10,
                },
                signal: SignalConfig {
                    enabled: false,
                    phone_number: String::new(),
                    data_dir: std::path::PathBuf::from("./data/signal"),
                    allowlist: Vec::new(),
                    allowed_groups: Vec::new(),
                    signal_cli_path: None,
                    use_libsignal: false,
                    rate_limit_per_minute: 20,
                    require_allowlist: true,
                },
                matrix: MatrixConfig {
                    enabled: false,
                    homeserver: String::new(),
                    user_id: String::new(),
                    access_token: None,
                    password: None,
                    device_id: None,
                    data_dir: "./data/matrix".to_string(),
                    allowlist: Vec::new(),
                    room_allowlist: Vec::new(),
                    auto_join_rooms: true,
                    enable_encryption: true,
                    rate_limit_per_second: 10,
                },
                x: XConfig {
                    enabled: false,
                    bearer_token: String::new(),
                    api_key: String::new(),
                    api_secret: String::new(),
                    access_token: String::new(),
                    access_token_secret: String::new(),
                    bot_user_id: String::new(),
                    allowlist: Vec::new(),
                    respond_to_mentions: true,
                    respond_to_dms: true,
                    max_tweet_length: 280,
                    mention_poll_interval_secs: 60,
                    dm_poll_interval_secs: 120,
                    rate_limit_per_minute: 10,
                },
                twilio: TwilioConfig {
                    enabled: false,
                    account_sid: String::new(),
                    auth_token: String::new(),
                    phone_number: String::new(),
                    webhook_url: None,
                    allowlist: Vec::new(),
                    max_message_length: 1600,
                    enable_mms: true,
                    rate_limit_per_second: 10,
                },
                meta: MetaConfig {
                    enabled: false,
                    app_id: String::new(),
                    app_secret: String::new(),
                    page_access_token: String::new(),
                    verify_token: String::new(),
                    webhook_path: "/webhooks/meta".to_string(),
                    page_id: String::new(),
                    instagram_account_id: None,
                    allowlist: Vec::new(),
                    respond_to_messenger: true,
                    respond_to_instagram: true,
                    show_typing_indicator: true,
                    rate_limit_per_second: 10,
                },
                imessage: IMessageConfig {
                    enabled: false,
                    bridge_mode: IMessageBridgeMode::MacOSDirect,
                    allowlist: Vec::new(),
                    enable_tapbacks: true,
                    enable_typing_indicator: false,
                },
                line: LineConfig {
                    enabled: false,
                    channel_access_token: String::new(),
                    channel_secret: String::new(),
                    webhook_path: "/webhook/line".to_string(),
                    allowlist: Vec::new(),
                    rate_limit_per_second: 1000,
                    enable_rich_menu: true,
                    enable_quick_replies: true,
                },
                viber: ViberConfig {
                    enabled: false,
                    auth_token: String::new(),
                    webhook_url: None,
                    webhook_path: "/webhook/viber".to_string(),
                    allowlist: Vec::new(),
                    rate_limit_per_minute: 300,
                    allow_broadcast: false,
                    welcome_message: None,
                    enable_keyboards: true,
                },
                wechat: WeChatConfig {
                    enabled: false,
                    app_type: "work".to_string(),
                    corp_id: String::new(),
                    corp_secret: String::new(),
                    agent_id: String::new(),
                    app_id: String::new(),
                    app_secret: String::new(),
                    token: String::new(),
                    encoding_aes_key: None,
                    webhook_path: "/webhook/wechat".to_string(),
                    allowlist: Vec::new(),
                    rate_limit_per_second: 20,
                    enable_encryption: false,
                },
            },
            skills: None,
        }
    }
}
