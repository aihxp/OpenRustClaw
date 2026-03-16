# Provider Traits API Reference

This reference documents the traits for LLM providers and related functionality.

---

## `LlmProvider`

The main trait for LLM completion providers.

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a completion request and return the full response
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Send a completion request and return a stream of chunks
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;

    /// The model identifier (e.g. "claude-sonnet-4-20250514")
    fn model_id(&self) -> &str;

    /// Maximum context window size in tokens
    fn max_tokens(&self) -> usize;

    /// Provider name for logging and fallback chain
    fn provider_name(&self) -> &str;

    /// Whether this provider supports strict tool mode
    fn supports_strict_tools(&self) -> bool;

    /// Whether this provider supports streaming tool call deltas
    fn supports_streaming_tool_deltas(&self) -> bool;

    /// The native tool definition format this provider expects
    fn native_tool_format(&self) -> ToolFormat;
}
```

### Example Implementation

```rust
use async_trait::async_trait;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::*;

pub struct MyProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl LlmProvider for MyProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Implementation
        todo!()
    }
    
    async fn stream(&self, request: CompletionRequest) 
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> 
    {
        // Implementation
        todo!()
    }
    
    fn model_id(&self) -> &str { &self.model }
    fn max_tokens(&self) -> usize { 200_000 }
    fn provider_name(&self) -> &str { "my_provider" }
    fn supports_strict_tools(&self) -> bool { true }
    fn supports_streaming_tool_deltas(&self) -> bool { false }
    fn native_tool_format(&self) -> ToolFormat { ToolFormat::OpenAi }
}
```

---

## `BatchProvider`

For providers that support batch processing.

```rust
#[async_trait]
pub trait BatchProvider: Send + Sync {
    /// Submit a batch of completion requests for async processing
    async fn batch(&self, requests: Vec<CompletionRequest>) -> Result<Vec<CompletionResponse>>;

    /// Maximum number of requests in a single batch
    fn max_batch_size(&self) -> usize;
}
```

---

## `EmbeddingProvider`

For text embedding providers used by the memory/RAG system.

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed one or more texts into vectors
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// The dimensionality of the embedding vectors
    fn dimensions(&self) -> usize;

    /// The embedding model identifier
    fn model_id(&self) -> &str;
}
```

### Example Implementation

```rust
use async_trait::async_trait;
use openrustclaw_core::traits::EmbeddingProvider;

pub struct OpenAIEmbeddingProvider {
    client: reqwest::Client,
    api_key: String,
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": "text-embedding-3-small",
                "input": texts,
            }))
            .send()
            .await?;
        
        let body: EmbeddingResponse = response.json().await?;
        Ok(body.data.into_iter().map(|d| d.embedding).collect())
    }
    
    fn dimensions(&self) -> usize { 1536 }
    fn model_id(&self) -> &str { "text-embedding-3-small" }
}
```

---

## `ProviderChain`

Automatic fallback between multiple providers.

```rust
pub struct ProviderChain {
    providers: Vec<Arc<dyn LlmProvider>>,
    cooldowns: Arc<DashMap<String, (Instant, Option<Duration>)>>,
}

impl ProviderChain {
    /// Create a new provider chain
    pub fn builder() -> ProviderChainBuilder;
    
    /// Send completion, trying providers in order
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Check if a provider is currently in cooldown
    pub fn is_in_cooldown(&self, provider: &str) -> bool;
}

pub struct ProviderChainBuilder {
    providers: Vec<Arc<dyn LlmProvider>>,
    timeout: Option<Duration>,
}

impl ProviderChainBuilder {
    pub fn add_provider(mut self, provider: Arc<dyn LlmProvider>) -> Self;
    pub fn timeout(mut self, timeout: Duration) -> Self;
    pub fn build(self) -> ProviderChain;
}
```

### Usage Example

```rust
use openrustclaw_providers::{ProviderChain, AnthropicProvider, OpenAiProvider};

let chain = ProviderChain::builder()
    .add_provider(Arc::new(AnthropicProvider::new(
        env::var("ANTHROPIC_API_KEY")?,
        "claude-sonnet-4-20250514".into(),
    )))
    .add_provider(Arc::new(OpenAiProvider::new(
        env::var("OPENAI_API_KEY")?,
        "gpt-4o".into(),
    )))
    .timeout(Duration::from_secs(30))
    .build();

// Automatically tries each provider
let response = chain.complete(request).await?;
```

---

## Provider Error Types

```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Rate limited by {provider} (retry after {retry_after_secs:?}s)")]
    RateLimited { provider: String, retry_after_secs: Option<u64> },

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
```

---

## Tool Format Translation

```rust
/// Translate tool definition to provider-specific format
pub fn translate_tool_definition(
    tool: &ToolDefinition,
    target_format: ToolFormat,
) -> Value;

/// Translate tool call between formats
pub fn translate_tool_call(
    call: &ToolCall,
    source_format: ToolFormat,
    target_format: ToolFormat,
) -> ToolCall;
```

### Example

```rust
use openrustclaw_providers::tool_formats::translate_tool_definition;

let tool = ToolDefinition {
    name: "memory_search".into(),
    description: "Search memories".into(),
    parameters: json!({
        "type": "object",
        "properties": {
            "query": {"type": "string"}
        },
        "required": ["query"]
    }),
    strict: true,
};

// Convert to Anthropic format
let anthropic_format = translate_tool_definition(&tool, ToolFormat::Anthropic);

// Convert to OpenAI format
let openai_format = translate_tool_definition(&tool, ToolFormat::OpenAi);
```
