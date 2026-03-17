//! Unified error types for OpenRustClaw.
//!
//! Uses `thiserror` for library errors. All crates map their errors into these types.

use thiserror::Error;

/// Top-level error type for OpenRustClaw operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    #[error("Scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),

    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Gateway error: {0}")]
    Gateway(#[from] GatewayError),

    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    #[error("Voice error: {0}")]
    Voice(#[from] VoiceError),

    #[error("Distributed error: {0}")]
    Distributed(#[from] DistributedError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Sidecar error: {0}")]
    Sidecar(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// LLM provider errors.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Rate limited by {provider} (retry after {retry_after_secs:?}s)")]
    RateLimited {
        provider: String,
        retry_after_secs: Option<u64>,
    },

    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed { provider: String, message: String },

    #[error("Model {model} not found on {provider}")]
    ModelNotFound { provider: String, model: String },

    #[error("Context length exceeded: {used} tokens used, {max} max")]
    ContextLengthExceeded { used: usize, max: usize },

    #[error("Provider {provider} unavailable: {message}")]
    Unavailable { provider: String, message: String },

    #[error("Invalid tool call from {provider}: {message}")]
    InvalidToolCall { provider: String, message: String },

    #[error("Streaming error from {provider}: {message}")]
    StreamError { provider: String, message: String },

    #[error("Batch error from {provider}: {message}")]
    BatchError { provider: String, message: String },

    #[error("All providers in fallback chain exhausted")]
    AllProvidersExhausted,

    #[error("Provider request error: {0}")]
    Request(String),

    #[error("Provider response parse error: {0}")]
    Parse(String),
}

/// Database layer errors.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Record not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Constraint violation: {0}")]
    Constraint(String),
}

/// Memory system errors.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Core memory budget exceeded: {used} tokens, max {max}")]
    CoreMemoryBudgetExceeded { used: usize, max: usize },

    #[error("Deduplication: entry already exists with id {existing_id}")]
    Duplicate { existing_id: String },

    #[error("Memory store error: {0}")]
    Store(String),
}

/// Security errors.
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Authentication required")]
    AuthRequired,

    #[error("Invalid origin: {origin}")]
    InvalidOrigin { origin: String },

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid: {0}")]
    TokenInvalid(String),

    #[error("Skill verification failed: {0}")]
    SkillVerificationFailed(String),

    #[error("Prompt injection detected: {0}")]
    PromptInjectionDetected(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Session isolation violation: {0}")]
    IsolationViolation(String),
}

/// Scheduler errors.
#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Lease acquisition failed for job {job_id}")]
    LeaseAcquisitionFailed { job_id: String },

    #[error("Idempotency conflict: job {job_id} already completed with key {key}")]
    IdempotencyConflict { job_id: String, key: String },

    #[error("Max retries exceeded for job {job_id} (attempts: {attempts})")]
    MaxRetriesExceeded { job_id: String, attempts: u32 },

    #[error("Workflow execution failed: {0}")]
    WorkflowFailed(String),

    #[error("Invalid trigger config: {0}")]
    InvalidTrigger(String),

    #[error("Heartbeat task not found: {0}")]
    HeartbeatTaskNotFound(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCronExpression(String),

    #[error("Missing heartbeat condition")]
    MissingHeartbeatCondition,

    #[error("Missing heartbeat action")]
    MissingHeartbeatAction,

    #[error("Heartbeat cooldown active for task {0}")]
    HeartbeatCooldown(String),

    #[error("File watch error: {0}")]
    FileWatchError(String),

    #[error("Heartbeat action failed: {0}")]
    HeartbeatActionFailed(String),
}

/// MCP protocol errors.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP connection error: {0}")]
    Connection(String),

    #[error("MCP tool not found: {server}/{tool}")]
    ToolNotFound { server: String, tool: String },

    #[error("MCP tool execution error: {0}")]
    ToolExecution(String),

    #[error("MCP schema translation error: {0}")]
    SchemaTranslation(String),

    #[error("MCP transport error: {0}")]
    Transport(String),
}

/// Tool execution errors.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {tool}: {message}")]
    ExecutionFailed { tool: String, message: String },

    #[error("Tool input validation error: {tool}: {message}")]
    InputValidation { tool: String, message: String },

    #[error("Tool capability not granted: {tool} requires {capability}")]
    CapabilityDenied { tool: String, capability: String },

    #[error("Tool sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("Tool timeout: {tool} exceeded {timeout_ms}ms")]
    Timeout { tool: String, timeout_ms: u64 },
}

/// Gateway errors.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Rate limit exceeded for session {session_id}")]
    RateLimitExceeded { session_id: String },

    #[error("Connection closed: {0}")]
    ConnectionClosed(String),
}

/// Distributed mode errors.
#[derive(Debug, Error)]
pub enum DistributedError {
    #[error("Cluster error: {0}")]
    Cluster(String),

    #[error("Node error: {0}")]
    Node(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Messaging error: {0}")]
    Messaging(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Lock error: {lock_name}: {message}")]
    Lock { lock_name: String, message: String },

    #[error("Task error: {0}")]
    Task(String),

    #[error("Leader not available")]
    LeaderNotAvailable,

    #[error("Not leader: current leader is {0:?}")]
    NotLeader(Option<String>),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Split brain detected: conflicting leader {0}")]
    SplitBrain(String),

    #[error("Quorum not reached: {0} of {1} nodes available")]
    QuorumNotReached(usize, usize),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}

/// Channel integration errors.
#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("{platform} authentication failed: {message}")]
    AuthFailed { platform: String, message: String },

    #[error("{platform} rate limited (retry after {retry_after_secs:?}s)")]
    RateLimited {
        platform: String,
        retry_after_secs: Option<u64>,
    },

    #[error("{platform} connection error: {message}")]
    Connection { platform: String, message: String },

    #[error("{platform} message send failed: {message}")]
    SendFailed { platform: String, message: String },

    #[error("{platform} invalid message format: {message}")]
    InvalidFormat { platform: String, message: String },

    #[error("{platform} configuration error: {message}")]
    Config { platform: String, message: String },

    #[error("{platform} not connected")]
    NotConnected { platform: String },

    #[error("{platform} Pub/Sub error: {message}")]
    PubSubError { platform: String, message: String },

    #[error("{platform} permission denied: {message}")]
    PermissionDenied { platform: String, message: String },
}

/// Voice system errors.
#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("Wake word error: {0}")]
    Wake(String),

    #[error("Speech-to-text error: {0}")]
    Stt(String),

    #[error("Text-to-speech error: {0}")]
    Tts(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Voice configuration error: {0}")]
    VoiceConfig(String),

    #[error("Component not initialized: {0}")]
    NotInitialized(String),

    #[error("Talk mode error: {0}")]
    TalkMode(String),

    #[error("Voice API error: {0}")]
    VoiceApi(String),

    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),

    #[error("Voice timeout: {0}")]
    VoiceTimeout(String),
}

/// Convenient Result type alias.
pub type Result<T> = std::result::Result<T, Error>;
