# Provider SDK API Reference

This reference documents the LLM provider integrations for OpenRustClaw.

**Crate**: `openrustclaw-providers`

---

## Overview

The provider SDK provides unified access to multiple LLM providers:
- **Anthropic** (Claude models)
- **OpenAI** (GPT models)
- **OpenRouter** (multi-provider routing)
- **Ollama** (local models)

Each provider implements the [`LlmProvider`](./provider-traits.md) trait for seamless interchangeability.

---

## `LlmProvider` Trait Usage

All providers implement the `LlmProvider` trait from `openrustclaw-core`:

```rust
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};

async fn use_provider(provider: &dyn LlmProvider) {
    let request = CompletionRequest {
        messages: vec![Message::user("Hello!")],
        model: None,
        max_tokens: Some(1024),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };
    
    match provider.complete(request).await {
        Ok(response) => println!("{}", response.message.content),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

---

## Anthropic Provider

### `AnthropicProvider`

Provider for Anthropic's Messages API.

```rust
pub struct AnthropicProvider {
    // Internal fields
}

impl AnthropicProvider {
    /// Create with API key and model
    pub fn new(api_key: String, model: String) -> Self;
    
    /// Create with custom base URL and API version
    pub fn with_config(
        api_key: String,
        model: String,
        base_url: String,
        api_version: String,
    ) -> Self;
}
```

**Supported models**:
- `claude-opus-4-20250514`
- `claude-sonnet-4-20250514`
- `claude-haiku-4-20250514`

**Example**:
```rust
use openrustclaw_providers::AnthropicProvider;
use std::sync::Arc;

let provider = Arc::new(AnthropicProvider::new(
    std::env::var("ANTHROPIC_API_KEY").unwrap(),
    "claude-sonnet-4-20250514".to_string(),
));

// Use with agent runtime
let runtime = AgentRuntime::new(
    provider.clone(),
    tool_registry,
    "MyAgent".to_string(),
);
```

**Configuration**:
```toml
[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"
# Optional:
# base_url = "https://api.anthropic.com"
# api_version = "2023-06-01"
```

---

## OpenAI Provider

### `OpenAiProvider`

Provider for OpenAI's Chat Completions API.

```rust
pub struct OpenAiProvider {
    // Internal fields
}

impl OpenAiProvider {
    /// Create with API key and model
    pub fn new(api_key: String, model: String) -> Self;
    
    /// Create with custom base URL
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self;
}
```

**Supported models**:
- `gpt-4o`
- `gpt-4o-mini`
- `gpt-4-turbo`
- `gpt-3.5-turbo`

**Example**:
```rust
use openrustclaw_providers::OpenAiProvider;

let provider = Arc::new(OpenAiProvider::new(
    std::env::var("OPENAI_API_KEY").unwrap(),
    "gpt-4o".to_string(),
));
```

**Configuration**:
```toml
[providers.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"
# Optional:
# base_url = "https://api.openai.com"
```

---

## OpenRouter Provider

### `OpenRouterProvider`

Provider for OpenRouter's unified API with automatic routing.

```rust
pub struct OpenRouterProvider {
    // Internal fields
}

impl OpenRouterProvider {
    /// Create with API key and model (default quality routing)
    pub fn new(api_key: String, model: String) -> Self;
    
    /// Create with specific routing strategy
    pub fn with_strategy(
        api_key: String,
        model: String,
        strategy: RouteStrategy,
    ) -> Self;
}
```

### `RouteStrategy`

Routing strategy for model selection:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    Price,       // `:floor` - lowest price
    Throughput,  // `:nitro` - highest throughput
    Quality,     // Default - best quality
    WebSearch,   // `:online` - include web search
}
```

**Example**:
```rust
use openrustclaw_providers::{OpenRouterProvider, RouteStrategy};

// Default quality routing
let provider = OpenRouterProvider::new(
    std::env::var("OPENROUTER_API_KEY").unwrap(),
    "anthropic/claude-sonnet-4".to_string(),
);

// Optimize for price
let cheap_provider = OpenRouterProvider::with_strategy(
    std::env::var("OPENROUTER_API_KEY").unwrap(),
    "openai/gpt-4o".to_string(),
    RouteStrategy::Price,
);
```

**Configuration**:
```toml
[providers.openrouter]
api_key = "${OPENROUTER_API_KEY}"
model = "anthropic/claude-sonnet-4"
strategy = "quality"  # Options: quality, price, throughput, web_search
```

---

## Ollama Provider

### `OllamaProvider`

Provider for local Ollama instances.

```rust
pub struct OllamaProvider {
    // Internal fields
}

impl OllamaProvider {
    /// Create with model name (uses default localhost:11434)
    pub fn new(model: String) -> Self;
    
    /// Create with custom base URL
    pub fn with_base_url(model: String, base_url: String) -> Self;
}
```

**Supported models** (requires local installation):
- `llama3.1`
- `codellama`
- `mistral`
- `mixtral`
- Any model pulled via `ollama pull`

**Example**:
```rust
use openrustclaw_providers::OllamaProvider;

// Default localhost:11434
let provider = OllamaProvider::new("llama3.1".to_string());

// Custom server
let provider = OllamaProvider::with_base_url(
    "codellama".to_string(),
    "http://ollama.internal:11434".to_string(),
);
```

**Prerequisites**:
```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model
ollama pull llama3.1

# Start server
ollama serve
```

---

## Provider Chain (Fallback)

### `ProviderChain`

Automatic fallback between multiple providers with cooldown management.

```rust
pub struct ProviderChain {
    // Internal fields
}

impl ProviderChain {
    /// Create chain with providers (default 60s cooldown)
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Self;
    
    /// Create chain with custom cooldown duration
    pub fn with_cooldown(
        providers: Vec<Arc<dyn LlmProvider>>,
        cooldown: Duration,
    ) -> Self;
    
    /// Try providers in order until one succeeds
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Get provider count
    pub fn provider_count(&self) -> usize;
    
    /// Get provider names
    pub fn provider_names(&self) -> Vec<&str>;
    
    /// Check if provider is in cooldown
    pub fn is_in_cooldown(&self, provider_name: &str) -> bool;
    
    /// Clear cooldown for a provider
    pub fn clear_cooldown(&self, provider_name: &str);
    
    /// Clear all cooldowns
    pub fn clear_all_cooldowns(&self);
}
```

**Example**:
```rust
use openrustclaw_providers::ProviderChain;
use std::sync::Arc;
use std::time::Duration;

// Create providers
let primary = Arc::new(AnthropicProvider::new(
    anthropic_key,
    "claude-sonnet-4-20250514".to_string(),
));
let fallback1 = Arc::new(OpenAiProvider::new(
    openai_key,
    "gpt-4o".to_string(),
));
let fallback2 = Arc::new(OpenRouterProvider::new(
    openrouter_key,
    "anthropic/claude-sonnet-4".to_string(),
));

// Create chain with 2-minute cooldown
let chain = ProviderChain::with_cooldown(
    vec![primary, fallback1, fallback2],
    Duration::from_secs(120),
);

// Use chain - automatically tries fallback on rate limits
let response = chain.complete(request).await?;
```

**Behavior**:
1. Tries providers in order
2. On rate limit or unavailable error, places provider in cooldown
3. Continues to next provider
4. Returns first successful response
5. Returns last error if all providers fail

---

## Tool Format Translation

### `translate_tool_definition`

Converts unified `ToolDefinition` to provider-specific format.

```rust
use openrustclaw_providers::translate_tool_definition;
use openrustclaw_core::types::{ToolDefinition, ToolFormat};
use serde_json::json;

let tool = ToolDefinition {
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
};

// Convert to OpenAI format
let openai_format = translate_tool_definition(&tool, ToolFormat::OpenAi);

// Convert to Anthropic format
let anthropic_format = translate_tool_definition(&tool, ToolFormat::Anthropic);
```

---

## Provider Capabilities Matrix

| Feature | Anthropic | OpenAI | OpenRouter | Ollama |
|---------|-----------|--------|------------|--------|
| Streaming | ✅ | ✅ | ✅ | ✅ |
| Tool Use | ✅ | ✅ | ✅ | ✅ |
| Strict Tools | ✅ | ✅ | ✅* | ❌ |
| Streaming Tool Deltas | ✅ | ❌ | ❌ | ❌ |
| Max Context | 200K | 128K | 128K | 8K |
| Local Execution | ❌ | ❌ | ❌ | ✅ |
| API Key Required | ✅ | ✅ | ✅ | ❌ |

*Depends on underlying model

---

## Error Handling

Provider errors are wrapped in `Error::Provider`:

```rust
use openrustclaw_core::error::{Error, ProviderError};

match result {
    Err(Error::Provider(ProviderError::RateLimited { provider, retry_after_secs })) => {
        println!("{} rate limited, retry after {:?}s", provider, retry_after_secs);
    }
    Err(Error::Provider(ProviderError::AuthFailed { provider, message })) => {
        println!("{} auth failed: {}", provider, message);
    }
    Err(Error::Provider(ProviderError::ModelNotFound { provider, model })) => {
        println!("Model {} not found on {}", model, provider);
    }
    Err(Error::Provider(ProviderError::Unavailable { provider, message })) => {
        println!("{} unavailable: {}", provider, message);
    }
    _ => {}
}
```

---

## Configuration Examples

### Single Provider

```toml
[providers]
primary = "anthropic"

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"
```

### With Fallback Chain

```toml
[providers]
primary = "anthropic"
fallback = ["openai", "openrouter"]

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"

[providers.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"

[providers.openrouter]
api_key = "${OPENROUTER_API_KEY}"
model = "anthropic/claude-sonnet-4"
```

### Local Development with Ollama

```toml
[providers]
primary = "ollama"

[providers.ollama]
model = "llama3.1"
base_url = "http://localhost:11434"
```

---

## Environment Variables

| Variable | Description | Required For |
|----------|-------------|--------------|
| `ANTHROPIC_API_KEY` | Anthropic API key | Anthropic provider |
| `OPENAI_API_KEY` | OpenAI API key | OpenAI provider |
| `OPENROUTER_API_KEY` | OpenRouter API key | OpenRouter provider |
| `OLLAMA_HOST` | Ollama host (default: localhost:11434) | Ollama provider |
