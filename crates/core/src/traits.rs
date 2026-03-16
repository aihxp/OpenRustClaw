//! Core traits for OpenRustClaw.
//!
//! All major subsystems are defined as traits here, allowing for
//! pluggable implementations and easy testing via mocks.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::error::Result;
use crate::types::{
    CompletionRequest, CompletionResponse, CoreEntry, IncomingMessage, MemoryEntry, MemoryQuery,
    OutgoingMessage, Platform, ScoredMemory, SkillCapability, StreamChunk, ToolFormat, ToolOutput,
};

// ---------------------------------------------------------------------------
// LLM Provider
// ---------------------------------------------------------------------------

/// Trait for LLM completion providers (Anthropic, OpenAI, OpenRouter, Ollama).
///
/// Each provider uses its native SDK and implements this unified interface.
/// The `native_tool_format()` method indicates which tool schema format the
/// provider expects, enabling automatic translation via `crates/providers/tool_formats.rs`.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a completion request and return the full response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Send a completion request and return a stream of chunks.
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;

    /// The model identifier (e.g. "claude-sonnet-4-20250514", "gpt-4o").
    fn model_id(&self) -> &str;

    /// Maximum context window size in tokens.
    fn max_tokens(&self) -> usize;

    /// Provider name for logging and fallback chain ("anthropic", "openai", "openrouter", "ollama").
    fn provider_name(&self) -> &str;

    /// Whether this provider supports strict tool mode (guaranteed schema compliance).
    fn supports_strict_tools(&self) -> bool;

    /// Whether this provider supports streaming tool call deltas
    /// (e.g. Anthropic fine-grained tool streaming).
    fn supports_streaming_tool_deltas(&self) -> bool;

    /// The native tool definition format this provider expects.
    fn native_tool_format(&self) -> ToolFormat;
}

// ---------------------------------------------------------------------------
// Batch Provider
// ---------------------------------------------------------------------------

/// For providers that support batch processing (e.g. Anthropic Batch API — 50% cheaper).
#[async_trait]
pub trait BatchProvider: Send + Sync {
    /// Submit a batch of completion requests for async processing.
    async fn batch(&self, requests: Vec<CompletionRequest>) -> Result<Vec<CompletionResponse>>;

    /// Maximum number of requests in a single batch.
    fn max_batch_size(&self) -> usize;
}

// ---------------------------------------------------------------------------
// Embedding Provider
// ---------------------------------------------------------------------------

/// Trait for text embedding providers used by the memory/RAG system.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed one or more texts into vectors.
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// The dimensionality of the embedding vectors.
    fn dimensions(&self) -> usize;

    /// The embedding model identifier.
    fn model_id(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

/// Trait for executable tools that the agent can invoke.
///
/// Tools are registered in a `ToolRegistry` and matched to LLM tool calls.
/// Each tool declares its capabilities so the security layer can enforce
/// permission boundaries.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g. "memory_search", "file_read").
    fn name(&self) -> &str;

    /// Human-readable description for the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn schema(&self) -> Value;

    /// Capabilities this tool requires (used for WASM sandbox enforcement).
    fn capabilities_required(&self) -> Vec<SkillCapability>;

    /// Execute the tool with the given input and context.
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
}

/// Context provided to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current session ID.
    pub session_id: String,
    /// Current user ID.
    pub user_id: String,
    /// Workspace root path (if applicable).
    pub workspace_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Memory Store
// ---------------------------------------------------------------------------

/// Trait for the recall memory storage backend.
///
/// Implementations use SQLite (sqlx for general, libSQL for vectors).
/// All writes go through memory policies (dedupe, importance, TTL).
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a new memory entry (after policy enforcement).
    async fn store(&self, entry: MemoryEntry) -> Result<()>;

    /// Search memories using hybrid BM25 + vector + MMR + temporal decay.
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>>;

    /// Get a memory entry by ID.
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>>;

    /// Delete a memory entry by ID.
    async fn delete(&self, id: &str) -> Result<()>;

    /// Check if content with this hash already exists. Returns the existing entry ID if found.
    async fn dedupe_check(&self, content_hash: &str) -> Result<Option<String>>;

    /// Expire entries past their TTL. Returns count of expired entries.
    async fn expire_stale(&self) -> Result<u64>;
}

// ---------------------------------------------------------------------------
// Core Memory Store
// ---------------------------------------------------------------------------

/// Trait for the core memory tier (tiny, always in prompt, ~500 tokens).
#[async_trait]
pub trait CoreMemoryStore: Send + Sync {
    /// Get all core memory entries for a user.
    async fn get_all(&self, user_id: &str) -> Result<Vec<CoreEntry>>;

    /// Set a core memory entry. Overwrites if key exists.
    async fn set(&self, user_id: &str, entry: CoreEntry) -> Result<()>;

    /// Remove a core memory entry by key.
    async fn remove(&self, user_id: &str, key: &str) -> Result<()>;

    /// Render all core memory entries as a formatted string for system prompt injection.
    async fn render(&self, user_id: &str) -> Result<String>;

    /// Get total token count of all core memory entries for a user.
    async fn total_tokens(&self, user_id: &str) -> Result<usize>;
}

// ---------------------------------------------------------------------------
// Channel
// ---------------------------------------------------------------------------

/// Trait for messaging platform integrations (WebChat, Telegram, Discord, etc.).
#[async_trait]
pub trait Channel: Send + Sync {
    /// Which platform this channel connects to.
    fn platform(&self) -> Platform;

    /// Send a message through this channel.
    async fn send(&self, msg: OutgoingMessage) -> Result<()>;

    /// Receive the next incoming message (blocks until available).
    async fn receive(&self) -> Result<IncomingMessage>;

    /// Establish the connection to the platform.
    async fn connect(&mut self) -> Result<()>;

    /// Gracefully disconnect.
    async fn disconnect(&mut self) -> Result<()>;
}
