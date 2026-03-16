# Core Types API Reference

This reference documents the core types used throughout OpenRustClaw.

---

## Message Types

### `Role`

The role of a message author.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}
```

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

impl Message {
    /// Create a new message with the given role and content
    pub fn new(role: Role, content: impl Into<String>) -> Self;
    
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self;
    
    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self;
    
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self;
    
    /// Create a tool-result message
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self;
}
```

### `ToolCall`

A tool invocation requested by the assistant.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
```

### `ToolOutput`

The result of executing a tool.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}
```

---

## Session Types

### `SessionType`

The kind of conversation session.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Dm,        // Direct message (1:1)
    Group,     // Group conversation
    Isolated,  // Background/automated work
}
```

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

impl Session {
    /// Create a new session
    pub fn new(
        session_type: SessionType,
        user_id: impl Into<String>,
        channel: Platform,
    ) -> Self;
    
    /// Create a new DM session
    pub fn new_dm(user_id: impl Into<String>, channel: Platform) -> Self;
}
```

### `Platform`

The platform/channel through which users interact.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    WebChat,
    Telegram,
    Discord,
    Slack,
    Cli,
    Api,
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
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub system_prompt: Option<String>,
    pub stream: bool,
}
```

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

### `FinishReason`

The reason a model stopped generating tokens.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    ToolUse,
    MaxTokens,
    ContentFilter,
}
```

### `StreamChunk`

A chunk emitted during a streaming completion.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StreamChunk {
    ContentDelta { delta: String },
    ToolCallDelta { id: String, name: Option<String>, arguments_delta: String },
    Done { response: CompletionResponse },
}
```

### `ToolDefinition`

Schema definition for a tool that an LLM may invoke.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub strict: bool,
}
```

### `ToolFormat`

The tool schema format expected by a given LLM provider.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFormat {
    Mcp,
    Anthropic,
    OpenAi,
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
    Episodic,    // Events, conversations
    Semantic,    // Facts, knowledge
    Procedural,  // How-to, workflows
}
```

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

### `MemoryEntry`

A single entry in the memory system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub content: String,
    pub content_hash: String,
    pub source: Option<String>,
    pub source_type: Option<SourceType>,
    pub session_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub namespace: String,
    pub importance: f32,
    pub confidence: f32,
    pub access_count: u32,
    pub last_accessed: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}
```

### `MemoryQuery`

Parameters for searching the memory system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub text: String,
    pub memory_types: Vec<MemoryType>,
    pub source_types: Vec<SourceType>,
    pub namespace: Option<String>,
    pub limit: usize,
    pub min_confidence: f32,
    pub recency_weight: f32,
}
```

### `ScoredMemory`

A memory entry paired with its relevance score.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub entry: MemoryEntry,
    pub score: f32,
}
```

### `CoreEntry`

A slot in the agent's persistent core memory.

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

## Event Types

### `Event`

Domain events emitted throughout the system.

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

## Channel Types

### `IncomingMessage`

A message arriving from an external channel.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub session_id: Uuid,
    pub user_id: String,
    pub content: String,
    pub platform: Platform,
    pub metadata: serde_json::Value,
}
```

### `OutgoingMessage`

A message to be sent out through an external channel.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub session_id: Uuid,
    pub content: String,
    pub metadata: serde_json::Value,
}
```
