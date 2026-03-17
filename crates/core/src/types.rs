//! Core types for the OpenRustClaw AI agent framework.
//!
//! This module defines all shared data types used across crates: messages, sessions,
//! completion requests/responses, memory entries, events, skills, and channel types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ──────────────────────────────────────────────
// Message types
// ──────────────────────────────────────────────

/// The role of a participant in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A human user.
    User,
    /// The AI assistant.
    Assistant,
    /// A system-level instruction.
    System,
    /// Output produced by a tool invocation.
    Tool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message.
    pub id: Uuid,
    /// Role of the message author.
    pub role: Role,
    /// Text content of the message.
    pub content: String,
    /// Tool calls requested by the assistant (present when `role == Assistant`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The id of the tool call this message is responding to (present when `role == Tool`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Estimated token count for this message (populated after tokenisation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,
    /// When this message was created.
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            token_count: None,
            created_at: Utc::now(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a tool-result message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        let mut msg = Self::new(Role::Tool, content);
        msg.tool_call_id = Some(tool_call_id.into());
        msg
    }
}

/// A tool invocation requested by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned identifier for this tool call.
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// JSON arguments to pass to the tool.
    pub arguments: serde_json::Value,
}

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The id of the tool call this output corresponds to.
    pub tool_call_id: String,
    /// The textual output produced by the tool.
    pub content: String,
    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

// ──────────────────────────────────────────────
// Session types
// ──────────────────────────────────────────────

/// The kind of conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    /// A direct (1:1) conversation between one user and the agent.
    Dm,
    /// A group conversation with multiple participants.
    Group,
    /// An isolated session for automated / background work (e.g. scheduled jobs).
    Isolated,
}

/// A conversation session with an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: Uuid,
    /// Kind of session.
    pub session_type: SessionType,
    /// The primary user who owns / initiated this session.
    pub user_id: String,
    /// The platform through which this session is conducted.
    pub channel: Platform,
    /// Optional workspace that scopes this session (multi-tenant support).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last active.
    pub updated_at: DateTime<Utc>,
    /// Arbitrary metadata attached to the session.
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

impl Session {
    /// Create a new session with the given type, user, and platform.
    pub fn new(
        session_type: SessionType,
        user_id: impl Into<String>,
        channel: Platform,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            session_type,
            user_id: user_id.into(),
            channel,
            workspace_id: None,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a new DM session for the given user and platform.
    pub fn new_dm(user_id: impl Into<String>, channel: Platform) -> Self {
        Self::new(SessionType::Dm, user_id, channel)
    }
}

// ──────────────────────────────────────────────
// Platform
// ──────────────────────────────────────────────

/// The platform / channel through which a user interacts with the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// The built-in WebSocket-based web chat.
    WebChat,
    /// Telegram bot integration.
    Telegram,
    /// Discord bot integration.
    Discord,
    /// Slack app integration.
    Slack,
    /// WhatsApp Web integration via Baileys.
    WhatsApp,
    /// Microsoft Teams bot integration.
    Teams,
    /// Google Chat bot integration.
    GoogleChat,
    /// Gmail Pub/Sub integration.
    Gmail,
    /// Twilio SMS/MMS integration.
    Twilio,
    /// Signal messenger integration via signal-cli.
    Signal,
    /// Matrix protocol integration.
    Matrix,
    /// X (Twitter) integration.
    X,
    /// Meta Messenger integration.
    Messenger,
    /// Meta Instagram integration.
    Instagram,
    /// iMessage integration (macOS BlueBubbles or AppleScript).
    IMessage,
    /// LINE integration.
    Line,
    /// Viber integration.
    Viber,
    /// WeChat integration.
    WeChat,
    /// CLI / terminal interface.
    Cli,
    /// REST API access (headless).
    Api,
}

// ──────────────────────────────────────────────
// Completion types (LLM providers)
// ──────────────────────────────────────────────

/// A request to an LLM provider for a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The conversation history to send.
    pub messages: Vec<Message>,
    /// Override the model to use (provider default if `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Maximum number of tokens to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Sampling temperature (0.0 = deterministic, 1.0+ = creative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Tool definitions the model may invoke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// System prompt prepended to the conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Whether to stream the response token-by-token.
    #[serde(default)]
    pub stream: bool,
}

/// A completed response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Provider-assigned response identifier.
    pub id: String,
    /// The generated message.
    pub message: Message,
    /// The model that produced this response.
    pub model: String,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Which provider produced this response.
    pub provider: String,
    /// Why the model stopped generating.
    pub finish_reason: FinishReason,
}

/// Token usage statistics for a single completion.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens consumed by the prompt (input).
    pub prompt_tokens: usize,
    /// Tokens generated by the model (output).
    pub completion_tokens: usize,
    /// Total tokens (prompt + completion).
    pub total_tokens: usize,
    /// Estimated cost in USD for this request, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// The reason a model stopped generating tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model finished naturally (end-of-turn).
    Stop,
    /// The model wants to invoke one or more tools.
    ToolUse,
    /// The response was truncated because `max_tokens` was reached.
    MaxTokens,
    /// The response was blocked by a content filter.
    ContentFilter,
}

/// A chunk emitted during a streaming completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StreamChunk {
    /// A delta of text content.
    ContentDelta {
        /// The new piece of text.
        delta: String,
    },
    /// A delta for a tool call being assembled.
    ToolCallDelta {
        /// The tool call id (may appear only in the first delta for this call).
        id: String,
        /// The tool name (may appear only in the first delta for this call).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Incremental JSON fragment of the arguments.
        arguments_delta: String,
    },
    /// The stream is complete; the final assembled response is attached.
    Done {
        /// The fully-assembled completion response.
        response: CompletionResponse,
    },
}

/// Schema definition for a tool that an LLM may invoke.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The unique name of the tool.
    pub name: String,
    /// A human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    pub parameters: serde_json::Value,
    /// Whether the provider should enforce strict schema adherence.
    #[serde(default)]
    pub strict: bool,
}

// ──────────────────────────────────────────────
// Tool format
// ──────────────────────────────────────────────

/// The tool schema format expected by a given LLM provider.
///
/// Each provider has its own JSON structure for tool definitions; the agent
/// runtime translates [`ToolDefinition`] into the appropriate wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFormat {
    /// Model Context Protocol (MCP) format.
    Mcp,
    /// Anthropic's native tool format.
    Anthropic,
    /// OpenAI's function-calling format.
    OpenAi,
}

// ──────────────────────────────────────────────
// Memory types
// ──────────────────────────────────────────────

/// Classification of a memory entry by cognitive function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Episode-like memories: events, conversations, specific interactions.
    Episodic,
    /// Factual / conceptual knowledge: definitions, relationships, domain info.
    Semantic,
    /// How-to knowledge: procedures, workflows, skills.
    Procedural,
}

/// What kind of source material a memory was ingested from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// A document (PDF, markdown, etc.).
    Document,
    /// Source code.
    Code,
    /// Configuration files.
    Config,
    /// An archived conversation.
    Conversation,
    /// An operational runbook.
    Runbook,
    /// A tool's JSON schema.
    ToolSchema,
}

/// A single entry in the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: Uuid,
    /// The cognitive category of this memory.
    pub memory_type: MemoryType,
    /// The textual content stored in this entry.
    pub content: String,
    /// SHA-256 hash of `content`, used for deduplication.
    pub content_hash: String,
    /// Optional human-readable source description (e.g. file path, URL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// What kind of source this entry came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<SourceType>,
    /// Session from which this memory was captured, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    /// User who created or is associated with this memory, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Namespace for scoping memories (e.g. workspace, project).
    pub namespace: String,
    /// How important this memory is (0.0 = trivial, 1.0 = critical).
    pub importance: f32,
    /// Confidence in the accuracy of this memory (0.0 – 1.0).
    pub confidence: f32,
    /// How many times this memory has been retrieved.
    #[serde(default)]
    pub access_count: u32,
    /// When this memory was last accessed via search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<DateTime<Utc>>,
    /// When this memory was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry time after which the memory should be pruned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Arbitrary metadata attached to this memory entry.
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

/// Parameters for searching the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Natural-language query text to match against.
    pub text: String,
    /// Filter to these memory types (empty = all types).
    #[serde(default)]
    pub memory_types: Vec<MemoryType>,
    /// Filter to these source types (empty = all source types).
    #[serde(default)]
    pub source_types: Vec<SourceType>,
    /// Restrict search to a specific namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Maximum number of results to return.
    #[serde(default = "default_memory_limit")]
    pub limit: usize,
    /// Minimum confidence threshold (0.0 – 1.0).
    #[serde(default)]
    pub min_confidence: f32,
    /// Weight given to recency when ranking results (0.0 = ignore recency, 1.0 = heavily prefer recent).
    #[serde(default)]
    pub recency_weight: f32,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            memory_types: Vec::new(),
            source_types: Vec::new(),
            namespace: None,
            limit: default_memory_limit(),
            min_confidence: 0.0,
            recency_weight: 0.0,
        }
    }
}

/// A memory entry paired with its relevance score from a search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    /// The matching memory entry.
    pub entry: MemoryEntry,
    /// Relevance score (higher = more relevant, typically 0.0 – 1.0).
    pub score: f32,
}

/// A slot in the agent's persistent core memory (key-value pairs held in the system prompt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreEntry {
    /// The key identifying this core memory slot.
    pub key: String,
    /// The value stored in this slot.
    pub value: String,
    /// Importance weight (used when budget-trimming).
    pub importance: f32,
    /// Estimated token count of `value`.
    pub token_count: usize,
    /// When this entry was last updated.
    pub updated_at: DateTime<Utc>,
}

/// How a memory entry was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// The user explicitly stated a fact or preference.
    ExplicitUserStatement,
    /// The user corrected a previous belief.
    UserCorrection,
    /// Extracted from the output of a tool execution.
    ToolResult,
    /// Inferred by the agent from context.
    AgentInference,
    /// Distilled from a conversation summary.
    ConversationSummary,
    /// Ingested from a background data pipeline (e.g. document loader).
    BackgroundIngestion,
}

// ──────────────────────────────────────────────
// Event types
// ──────────────────────────────────────────────

/// Domain events emitted throughout the system for observability and side-effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Event {
    /// A message was received from a user or external system.
    MessageReceived {
        /// The session the message belongs to.
        session_id: Uuid,
        /// The received message.
        message: Message,
    },
    /// A message was sent to the user.
    MessageSent {
        /// The session the message was sent in.
        session_id: Uuid,
        /// The outgoing message.
        message: Message,
    },
    /// A tool was executed.
    ToolExecuted {
        /// The session context.
        session_id: Uuid,
        /// Name of the tool.
        tool_name: String,
        /// The tool's output.
        output: ToolOutput,
        /// Execution duration in milliseconds.
        duration_ms: u64,
    },
    /// A memory entry was stored.
    MemoryStored {
        /// The id of the stored entry.
        entry_id: Uuid,
        /// The type of memory stored.
        memory_type: MemoryType,
        /// The source of this memory.
        source: MemorySource,
    },
    /// A memory search was performed.
    MemorySearched {
        /// The query that was executed.
        query: String,
        /// How many results were returned.
        result_count: usize,
    },
    /// A new session was created.
    SessionCreated {
        /// The new session.
        session: Session,
    },
    /// A session was closed.
    SessionClosed {
        /// The id of the closed session.
        session_id: Uuid,
    },
    /// A scheduled job fired.
    SchedulerJobFired {
        /// The job identifier.
        job_id: String,
        /// Human-readable job name.
        job_name: String,
    },
    /// An approval was requested from a human operator.
    ApprovalRequested {
        /// Unique id for this approval request.
        request_id: Uuid,
        /// The session context.
        session_id: Uuid,
        /// Description of the action awaiting approval.
        description: String,
    },
    /// An approval response was received.
    ApprovalReceived {
        /// The approval request this responds to.
        request_id: Uuid,
        /// Whether the action was approved.
        approved: bool,
        /// The user who approved or denied.
        reviewer: String,
    },
    /// An error occurred.
    Error {
        /// Human-readable error message.
        message: String,
        /// Optional session context.
        session_id: Option<Uuid>,
        /// Error severity level.
        severity: ErrorSeverity,
    },
}

/// Severity levels for error events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    /// Non-critical, informational warning.
    Warning,
    /// An error that affects a single request or session.
    Error,
    /// A critical failure that may require operator intervention.
    Critical,
}

// ──────────────────────────────────────────────
// Skill types
// ──────────────────────────────────────────────

/// A capability that a skill may require to operate.
///
/// The security layer checks these capabilities against the workspace's
/// permission policy before allowing a skill to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCapability {
    /// Read files from the filesystem.
    FileRead,
    /// Write files to the filesystem.
    FileWrite,
    /// Make outbound network requests.
    NetworkAccess,
    /// Execute shell commands.
    ShellExec,
    /// Access a database.
    DatabaseAccess,
    /// Write to the memory system.
    MemoryWrite,
}

/// Where a skill originated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    /// Defined in the workspace's `skills/` directory.
    Workspace,
    /// Installed and managed by the platform operator.
    Managed,
    /// Shipped with the OpenRustClaw binary.
    Bundled,
    /// Downloaded from the skill marketplace.
    Marketplace,
}

// ──────────────────────────────────────────────
// Channel types
// ──────────────────────────────────────────────

/// A message arriving from an external channel / platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// The session this message should be routed to.
    pub session_id: Uuid,
    /// The user who sent the message.
    pub user_id: String,
    /// The text content of the message.
    pub content: String,
    /// The platform the message arrived on.
    pub platform: Platform,
    /// Arbitrary platform-specific metadata (e.g. message_id, attachments).
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

/// A message to be sent out through an external channel / platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    /// The session this message should be delivered to.
    pub session_id: Uuid,
    /// The text content of the message.
    pub content: String,
    /// Arbitrary platform-specific metadata (e.g. reply markup, attachments).
    #[serde(default = "default_json_object")]
    pub metadata: serde_json::Value,
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Default JSON value for metadata fields: an empty object `{}`.
fn default_json_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Default limit for memory queries.
fn default_memory_limit() -> usize {
    10
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::WebChat => write!(f, "web_chat"),
            Platform::Telegram => write!(f, "telegram"),
            Platform::Discord => write!(f, "discord"),
            Platform::Slack => write!(f, "slack"),
            Platform::Teams => write!(f, "teams"),
            Platform::GoogleChat => write!(f, "google_chat"),
            Platform::Gmail => write!(f, "gmail"),
            Platform::WhatsApp => write!(f, "whatsapp"),
            Platform::Twilio => write!(f, "twilio"),
            Platform::Signal => write!(f, "signal"),
            Platform::Matrix => write!(f, "matrix"),
            Platform::X => write!(f, "x"),
            Platform::Messenger => write!(f, "messenger"),
            Platform::Instagram => write!(f, "instagram"),
            Platform::IMessage => write!(f, "imessage"),
            Platform::Line => write!(f, "line"),
            Platform::Viber => write!(f, "viber"),
            Platform::WeChat => write!(f, "wechat"),
            Platform::Cli => write!(f, "cli"),
            Platform::Api => write!(f, "api"),
        }
    }
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::ToolUse => write!(f, "tool_use"),
            FinishReason::MaxTokens => write!(f, "max_tokens"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Episodic => write!(f, "episodic"),
            MemoryType::Semantic => write!(f, "semantic"),
            MemoryType::Procedural => write!(f, "procedural"),
        }
    }
}

impl std::fmt::Display for ToolFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolFormat::Mcp => write!(f, "mcp"),
            ToolFormat::Anthropic => write!(f, "anthropic"),
            ToolFormat::OpenAi => write!(f, "openai"),
        }
    }
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Dm => write!(f, "dm"),
            SessionType::Group => write!(f, "group"),
            SessionType::Isolated => write!(f, "isolated"),
        }
    }
}

impl std::fmt::Display for SkillCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillCapability::FileRead => write!(f, "file_read"),
            SkillCapability::FileWrite => write!(f, "file_write"),
            SkillCapability::NetworkAccess => write!(f, "network_access"),
            SkillCapability::ShellExec => write!(f, "shell_exec"),
            SkillCapability::DatabaseAccess => write!(f, "database_access"),
            SkillCapability::MemoryWrite => write!(f, "memory_write"),
        }
    }
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::Workspace => write!(f, "workspace"),
            SkillSource::Managed => write!(f, "managed"),
            SkillSource::Bundled => write!(f, "bundled"),
            SkillSource::Marketplace => write!(f, "marketplace"),
        }
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Document => write!(f, "document"),
            SourceType::Code => write!(f, "code"),
            SourceType::Config => write!(f, "config"),
            SourceType::Conversation => write!(f, "conversation"),
            SourceType::Runbook => write!(f, "runbook"),
            SourceType::ToolSchema => write!(f, "tool_schema"),
        }
    }
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::ExplicitUserStatement => write!(f, "explicit_user_statement"),
            MemorySource::UserCorrection => write!(f, "user_correction"),
            MemorySource::ToolResult => write!(f, "tool_result"),
            MemorySource::AgentInference => write!(f, "agent_inference"),
            MemorySource::ConversationSummary => write!(f, "conversation_summary"),
            MemorySource::BackgroundIngestion => write!(f, "background_ingestion"),
        }
    }
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_constructors() {
        let user = Message::user("Hello");
        assert_eq!(user.role, Role::User);
        assert_eq!(user.content, "Hello");

        let assistant = Message::assistant("Hi there");
        assert_eq!(assistant.role, Role::Assistant);

        let system = Message::system("You are helpful.");
        assert_eq!(system.role, Role::System);

        let tool = Message::tool("call_123", "result");
        assert_eq!(tool.role, Role::Tool);
        assert_eq!(tool.tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn role_serialization() {
        let json = serde_json::to_string(&Role::User).unwrap();
        assert_eq!(json, "\"user\"");

        let role: Role = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, Role::Assistant);
    }

    #[test]
    fn session_new_dm() {
        let session = Session::new_dm("user_42", Platform::WebChat);
        assert_eq!(session.session_type, SessionType::Dm);
        assert_eq!(session.user_id, "user_42");
        assert_eq!(session.channel, Platform::WebChat);
        assert!(session.workspace_id.is_none());
    }

    #[test]
    fn finish_reason_roundtrip() {
        for reason in [
            FinishReason::Stop,
            FinishReason::ToolUse,
            FinishReason::MaxTokens,
            FinishReason::ContentFilter,
        ] {
            let json = serde_json::to_string(&reason).unwrap();
            let back: FinishReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, back);
        }
    }

    #[test]
    fn tool_format_display() {
        assert_eq!(ToolFormat::Mcp.to_string(), "mcp");
        assert_eq!(ToolFormat::Anthropic.to_string(), "anthropic");
        assert_eq!(ToolFormat::OpenAi.to_string(), "openai");
    }

    #[test]
    fn memory_query_default() {
        let q = MemoryQuery::default();
        assert_eq!(q.limit, 10);
        assert_eq!(q.min_confidence, 0.0);
        assert!(q.memory_types.is_empty());
    }

    #[test]
    fn event_serialization() {
        let event = Event::SessionClosed {
            session_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"session_closed\""));
    }

    #[test]
    fn platform_display() {
        assert_eq!(Platform::WebChat.to_string(), "web_chat");
        assert_eq!(Platform::Discord.to_string(), "discord");
        assert_eq!(Platform::GoogleChat.to_string(), "google_chat");
        assert_eq!(Platform::WhatsApp.to_string(), "whatsapp");
        assert_eq!(Platform::Twilio.to_string(), "twilio");
        assert_eq!(Platform::IMessage.to_string(), "imessage");
    }
}
