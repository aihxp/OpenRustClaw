# Core API Reference

This reference documents the core types, traits, and error handling for OpenRustClaw.

**Crate**: `openrustclaw-core`

---

## Overview

The `openrustclaw-core` crate provides the foundational types and error handling used across all OpenRustClaw components. It defines the data structures for messages, sessions, completions, memory, and the unified error type.

## Message Types

### `Role`

The role of a participant in a conversation.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,       // A human user
    Assistant,  // The AI assistant
    System,     // A system-level instruction
    Tool,       // Output produced by a tool invocation
}
```

**Example**:
```rust
use openrustclaw_core::types::Role;

let role = Role::User;
assert_eq!(role.to_string(), "user");
```

---

### `Message`

A single message in a conversation.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub token_count: Option<usize>,
    pub created_at: DateTime<Utc>,
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `Message::new(role, content)` | Create a new message with the given role and content |
| `Message::user(content)` | Create a user message (convenience) |
| `Message::assistant(content)` | Create an assistant message (convenience) |
| `Message::system(content)` | Create a system message (convenience) |
| `Message::tool(tool_call_id, content)` | Create a tool-result message |

**Example**:
```rust
use openrustclaw_core::types::{Message, Role};

// Create messages using convenience constructors
let user_msg = Message::user("What is the weather?");
let system_msg = Message::system("You are a helpful assistant.");
let tool_msg = Message::tool("call_123", "The weather is sunny.");

// Create with explicit role
let custom_msg = Message::new(Role::Assistant, "Let me check that.");
```

---

### `ToolCall`

A tool invocation requested by the assistant.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,              // Provider-assigned identifier
    pub name: String,            // Name of the tool to invoke
    pub arguments: serde_json::Value,  // JSON arguments
}
```

**Example**:
```rust
use openrustclaw_core::types::ToolCall;
use serde_json::json;

let tool_call = ToolCall {
    id: "call_abc123".to_string(),
    name: "get_weather".to_string(),
    arguments: json!({"location": "New York"}),
};
```

---

### `ToolOutput`

The result of executing a tool.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_call_id: String,    // ID of the tool call this responds to
    pub content: String,         // Textual output from the tool
    pub is_error: bool,          // Whether execution resulted in an error
}
```

**Example**:
```rust
use openrustclaw_core::types::ToolOutput;

let output = ToolOutput {
    tool_call_id: "call_abc123".to_string(),
    content: "The weather in New York is 72°F and sunny.".to_string(),
    is_error: false,
};
```

---

## Session Types

### `SessionType`

The kind of conversation session.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Dm,         // Direct (1:1) conversation
    Group,      // Group conversation with multiple participants
    Isolated,   // Isolated session for automated/background work
}
```

---

### `Session`

A conversation session with an agent.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub session_type: SessionType,
    pub user_id: String,
    pub channel: Platform,
    pub workspace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `Session::new(session_type, user_id, channel)` | Create a new session |
| `Session::new_dm(user_id, channel)` | Create a new DM session (convenience) |

**Example**:
```rust
use openrustclaw_core::types::{Session, Platform, SessionType};

// Create a DM session
let session = Session::new_dm("user_42", Platform::WebChat);
assert_eq!(session.session_type, SessionType::Dm);
assert_eq!(session.user_id, "user_42");
```

---

### `Platform`

The platform/channel through which users interact with the agent.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    WebChat,   // Built-in WebSocket-based web chat
    Telegram,  // Telegram bot integration
    Discord,   // Discord bot integration
    Slack,     // Slack app integration
    Cli,       // CLI / terminal interface
    Api,       // REST API access (headless)
}
```

---

## Completion Types

### `CompletionRequest`

A request to an LLM provider for a completion.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>,           // Override default model
    pub max_tokens: Option<usize>,       // Maximum tokens to generate
    pub temperature: Option<f32>,        // Sampling temperature (0.0-1.0+)
    pub tools: Option<Vec<ToolDefinition>>,  // Available tools
    pub system_prompt: Option<String>,   // System prompt override
    pub stream: bool,                    // Whether to stream response
}
```

**Example**:
```rust
use openrustclaw_core::types::{CompletionRequest, Message, ToolDefinition};
use serde_json::json;

let request = CompletionRequest {
    messages: vec![Message::user("Hello!")],
    model: Some("claude-sonnet-4-20250514".to_string()),
    max_tokens: Some(1024),
    temperature: Some(0.7),
    tools: Some(vec![ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
        strict: true,
    }]),
    system_prompt: Some("You are helpful.".to_string()),
    stream: false,
};
```

---

### `CompletionResponse`

A completed response from an LLM provider.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub message: Message,
    pub model: String,
    pub usage: TokenUsage,
    pub provider: String,
    pub finish_reason: FinishReason,
}
```

---

### `TokenUsage`

Token usage statistics for a completion.

```rust
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub cost_usd: Option<f64>,
}
```

---

### `FinishReason`

The reason a model stopped generating tokens.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,           // Natural end-of-turn
    ToolUse,        // Wants to invoke one or more tools
    MaxTokens,      // max_tokens limit reached
    ContentFilter,  // Blocked by content filter
}
```

---

### `StreamChunk`

A chunk emitted during a streaming completion.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StreamChunk {
    ContentDelta { delta: String },
    ToolCallDelta { 
        id: String, 
        name: Option<String>, 
        arguments_delta: String 
    },
    Done { response: CompletionResponse },
}
```

---

### `ToolDefinition`

Schema definition for a tool that an LLM may invoke.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
    pub strict: bool,                   // Enforce strict schema adherence
}
```

---

### `ToolFormat`

The tool schema format expected by a given LLM provider.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFormat {
    Mcp,        // Model Context Protocol format
    Anthropic,  // Anthropic's native tool format
    OpenAi,     // OpenAI's function-calling format
}
```

---

## Memory Types

### `MemoryType`

Classification of a memory entry by cognitive function.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,    // Events, conversations, specific interactions
    Semantic,    // Factual/conceptual knowledge
    Procedural,  // How-to knowledge, workflows
}
```

---

### `MemoryEntry`

A single entry in the memory system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub content: String,
    pub content_hash: String,       // SHA-256 hash for deduplication
    pub source: Option<String>,
    pub source_type: Option<SourceType>,
    pub session_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub namespace: String,
    pub importance: f32,            // 0.0 (trivial) to 1.0 (critical)
    pub confidence: f32,            // 0.0 to 1.0 accuracy confidence
    pub access_count: u32,
    pub last_accessed: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}
```

---

### `MemoryQuery`

Parameters for searching the memory system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub text: String,
    pub memory_types: Vec<MemoryType>,      // Filter by type (empty = all)
    pub source_types: Vec<SourceType>,      // Filter by source (empty = all)
    pub namespace: Option<String>,          // Restrict to namespace
    pub limit: usize,                       // Max results (default: 10)
    pub min_confidence: f32,                // Minimum confidence threshold
    pub recency_weight: f32,                // Weight for recency ranking
}
```

**Default values**:
- `limit`: 10
- `min_confidence`: 0.0
- `recency_weight`: 0.0

**Example**:
```rust
use openrustclaw_core::types::{MemoryQuery, MemoryType};

let query = MemoryQuery {
    text: "user's favorite color".to_string(),
    memory_types: vec![MemoryType::Semantic],
    source_types: vec![],
    namespace: Some("user_42".to_string()),
    limit: 5,
    min_confidence: 0.5,
    recency_weight: 0.3,
};
```

---

### `ScoredMemory`

A memory entry paired with its relevance score.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub entry: MemoryEntry,
    pub score: f32,  // Higher = more relevant (typically 0.0-1.0)
}
```

---

### `CoreEntry`

A slot in the agent's persistent core memory (always in prompt).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreEntry {
    pub key: String,
    pub value: String,
    pub importance: f32,
    pub token_count: usize,
    pub updated_at: DateTime<Utc>,
}
```

---

### `SourceType`

The kind of source material a memory was ingested from.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Document,
    Code,
    Config,
    Conversation,
    Runbook,
    ToolSchema,
}
```

---

### `MemorySource`

How a memory entry was produced.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    ExplicitUserStatement,
    UserCorrection,
    ToolResult,
    AgentInference,
    ConversationSummary,
    BackgroundIngestion,
}
```

---

## Error Types

### `Error`

The top-level error type for OpenRustClaw operations.

```rust
#[derive(Debug, Error)]
pub enum Error {
    Provider(ProviderError),
    Database(DatabaseError),
    Memory(MemoryError),
    Security(SecurityError),
    Scheduler(SchedulerError),
    Mcp(McpError),
    Tool(ToolError),
    Gateway(GatewayError),
    Config(String),
    Sidecar(String),
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

---

### `ProviderError`

LLM provider errors.

```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    RateLimited { provider: String, retry_after_secs: Option<u64> },
    AuthFailed { provider: String, message: String },
    ModelNotFound { provider: String, model: String },
    ContextLengthExceeded { used: usize, max: usize },
    Unavailable { provider: String, message: String },
    InvalidToolCall { provider: String, message: String },
    StreamError { provider: String, message: String },
    BatchError { provider: String, message: String },
    AllProvidersExhausted,
    Request(String),
    Parse(String),
}
```

---

### `MemoryError`

Memory system errors.

```rust
#[derive(Debug, Error)]
pub enum MemoryError {
    Embedding(String),
    Search(String),
    CoreMemoryBudgetExceeded { used: usize, max: usize },
    Duplicate { existing_id: String },
    Store(String),
}
```

---

### `SecurityError`

Security errors.

```rust
#[derive(Debug, Error)]
pub enum SecurityError {
    AuthRequired,
    InvalidOrigin { origin: String },
    TokenExpired,
    TokenInvalid(String),
    SkillVerificationFailed(String),
    PromptInjectionDetected(String),
    PermissionDenied(String),
    IsolationViolation(String),
}
```

---

### `ToolError`

Tool execution errors.

```rust
#[derive(Debug, Error)]
pub enum ToolError {
    NotFound(String),
    ExecutionFailed { tool: String, message: String },
    InputValidation { tool: String, message: String },
    CapabilityDenied { tool: String, capability: String },
    SandboxViolation(String),
    Timeout { tool: String, timeout_ms: u64 },
}
```

---

## Event Types

### `Event`

Domain events emitted throughout the system for observability.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Event {
    MessageReceived { session_id: Uuid, message: Message },
    MessageSent { session_id: Uuid, message: Message },
    ToolExecuted { session_id: Uuid, tool_name: String, output: ToolOutput, duration_ms: u64 },
    MemoryStored { entry_id: Uuid, memory_type: MemoryType, source: MemorySource },
    MemorySearched { query: String, result_count: usize },
    SessionCreated { session: Session },
    SessionClosed { session_id: Uuid },
    SchedulerJobFired { job_id: String, job_name: String },
    ApprovalRequested { request_id: Uuid, session_id: Uuid, description: String },
    ApprovalReceived { request_id: Uuid, approved: bool, reviewer: String },
    Error { message: String, session_id: Option<Uuid>, severity: ErrorSeverity },
}
```

---

### `ErrorSeverity`

Severity levels for error events.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}
```

---

## Skill Types

### `SkillCapability`

A capability that a skill may require.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCapability {
    FileRead,
    FileWrite,
    NetworkAccess,
    ShellExec,
    DatabaseAccess,
    MemoryWrite,
}
```

---

### `SkillSource`

Where a skill originated from.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    Workspace,
    Managed,
    Bundled,
    Marketplace,
}
```

---

## Configuration

### `AppConfig`

Application configuration loaded from TOML files.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub providers: ProvidersConfig,
    pub gateway: GatewayConfig,
    pub scheduler: SchedulerConfig,
    pub security: SecurityConfig,
}
```

**Configuration file format**:
```toml
[database]
url = "sqlite://data/openrustclaw.db"

[providers]
primary = "anthropic"
fallback = ["openai", "openrouter"]

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"

[gateway]
host = "127.0.0.1"
port = 8080

[scheduler]
enabled = true
poll_interval_secs = 30

[security]
allowed_origins = ["https://example.com"]
skill_verification_required = true
```

---

## Error Handling Guidance

### Matching Errors

```rust
use openrustclaw_core::error::{Error, ProviderError, MemoryError};

match result {
    Ok(response) => response,
    Err(Error::Provider(ProviderError::RateLimited { provider, retry_after_secs })) => {
        // Handle rate limit with retry logic
        tokio::time::sleep(Duration::from_secs(retry_after_secs.unwrap_or(60))).await;
        retry().await
    }
    Err(Error::Provider(ProviderError::ContextLengthExceeded { used, max })) => {
        // Trim context and retry
        trim_context(used - max).await
    }
    Err(Error::Memory(MemoryError::Duplicate { existing_id })) => {
        // Memory already exists, use existing
        use_existing(existing_id).await
    }
    Err(e) => return Err(e),
}
```

### Converting Errors

The error types implement `From` for easy conversion:

```rust
fn my_function() -> Result<()> {
    let db_result = sqlx::query("...").fetch_one(&pool).await
        .map_err(|e| Error::Database(DatabaseError::Query(e.to_string())))?;
    
    let json_result = serde_json::from_str(data)
        .map_err(|e| Error::Internal(format!("JSON parse failed: {}", e)))?;
    
    Ok(())
}
```
