---
marp: true
theme: default
paginate: true
class: invert
header: 'Provider Ecosystem'
footer: '© 2026 OpenRustClaw Project'
---

<!--
Speaker Notes: Overview of the LLM provider ecosystem in OpenRustClaw. Covers the 4 main providers and fallback mechanisms.
-->

<style>
section {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}
h1, h2 {
  color: #27ae60;
}
strong {
  color: #3498db;
}
table {
  font-size: 0.85em;
}
code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.85em;
}
</style>

# 🔌 Provider Ecosystem

## Multi-Provider LLM Support with Fallback

### 4 Native Providers • 400+ Models • Zero Downtime

---

<!--
Speaker Notes: Give an overview of all supported providers and their unique characteristics.
-->

## 🌐 Supported Providers

### Provider Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│   │  ANTHROPIC  │  │   OPENAI    │  │ OPENROUTER  │            │
│   │             │  │             │  │             │            │
│   │   Claude    │  │   GPT-4     │  │  400+ models│            │
│   │             │  │   GPT-3.5   │  │             │            │
│   │ • Best for  │  │ • Broad     │  │ • Price opt │            │
│   │   coding    │  │   ecosystem │  │ • Fallback  │            │
│   │ • Long ctx  │  │ • Functions │  │ • Variety   │            │
│   │ • Tool use  │  │ • Vision    │  │             │            │
│   └─────────────┘  └─────────────┘  └─────────────┘            │
│                                                                 │
│   ┌─────────────────────────────────────────────────────┐      │
│   │                      OLLAMA                          │      │
│   │                                                      │      │
│   │   Local LLM Execution                                │      │
│   │   • Privacy-first   • No API costs   • Offline       │      │
│   │   • Llama 3         • Mistral        • CodeLlama     │      │
│   └─────────────────────────────────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Provider Comparison

| Feature | Anthropic | OpenAI | OpenRouter | Ollama |
|---------|-----------|--------|------------|--------|
| Cloud | ✅ | ✅ | ✅ | ❌ (Local) |
| Streaming | ✅ | ✅ | ✅ | ✅ |
| Tool Use | ✅ Native | ✅ Functions | ✅ | ⚠️ Varies |
| Vision | ✅ | ✅ | ✅ Model-dep | ⚠️ Varies |
| Fallback | ✅ Chain | ✅ Chain | ✅ Chain | N/A |
| Cost/M | $3-15 | $5-30 | $0.5-20 | $0 |

---

<!--
Speaker Notes: Deep dive into Anthropic. Claude is the recommended default for many use cases.
-->

## 🤖 Anthropic (Claude)

### Native SDK Integration

```rust
// crates/providers/src/anthropic.rs
use anthropic_rust::Client; // Native SDK (planned)

pub struct AnthropicProvider {
    client: Client,
    config: ProviderConfig,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, request: Request) -> Result<Response> {
        let messages = convert_to_anthropic_format(request.messages);
        
        let response = self.client
            .messages()
            .create(CreateMessageRequest {
                model: request.model.unwrap_or("claude-3-5-sonnet-20241022"),
                max_tokens: request.max_tokens.unwrap_or(4096),
                messages,
                tools: request.tools.map(convert_tools),
                ..Default::default()
            })
            .await?;
        
        Ok(convert_from_anthropic_response(response))
    }
    
    async fn complete_stream(&self, request: Request) -> Result<Stream> {
        // Streaming implementation with tool_call delta fix
        self.client.messages().create_stream(...).await
    }
}
```

### Claude Features

| Model | Context | Best For |
|-------|---------|----------|
| claude-3-5-sonnet | 200K | General purpose, coding |
| claude-3-opus | 200K | Complex reasoning, analysis |
| claude-3-haiku | 200K | Speed, cost-sensitive |

### Tool Use (Native)

```rust
// Strict tool use ensures reliable function calling
ToolChoice::Auto      // Model decides
ToolChoice::Any       // Must use a tool
ToolChoice::Tool(name) // Must use specific tool
```

---

<!--
Speaker Notes: OpenAI is the broadest ecosystem. Cover both the new Responses API and legacy Chat Completions.
-->

## 🧠 OpenAI (GPT-4)

### Dual API Support

```rust
// crates/providers/src/openai.rs
pub enum OpenAiApi {
    Responses,    // New primary API
    ChatCompletions, // Legacy fallback
}

pub struct OpenAiProvider {
    client: async_openai::Client, // Native SDK (planned)
    api_version: OpenAiApi,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, request: Request) -> Result<Response> {
        match self.api_version {
            OpenAiApi::Responses => {
                // Use new Responses API
                self.responses_api(request).await
            }
            OpenAiApi::ChatCompletions => {
                // Fallback to Chat Completions
                self.chat_completions_api(request).await
            }
        }
    }
}
```

### GPT-4 Model Lineup

| Model | Context | Features |
|-------|---------|----------|
| gpt-4o | 128K | Multimodal, fast |
| gpt-4o-mini | 128K | Cost-effective |
| gpt-4-turbo | 128K | Legacy, reliable |
| o1-preview | 128K | Reasoning, slow |
| o1-mini | 128K | Reasoning, fast |

### Function Calling

```rust
// OpenAI function format (auto-converted from MCP)
Function {
    name: "memory_search",
    description: "Search agent memory",
    parameters: json!({
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "limit": {"type": "integer", "default": 10}
        },
        "required": ["query"]
    }),
}
```

---

<!--
Speaker Notes: OpenRouter is the Swiss Army knife—great for price optimization and accessing many models.
-->

## 🌉 OpenRouter (400+ Models)

### Universal Model Access

```rust
// crates/providers/src/openrouter.rs
pub struct OpenRouterProvider {
    client: openrouter_api::Client, // Native SDK (planned)
    routing_strategy: RoutingStrategy,
}

pub enum RoutingStrategy {
    Cheapest,      // Minimize cost
    Fastest,       // Minimize latency
    BestQuality,   // Best benchmark scores
    Balanced,      // Optimize for both
    Specific(String), // Specific model
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete(&self, request: Request) -> Result<Response> {
        let route = match self.routing_strategy {
            RoutingStrategy::Cheapest => {
                self.find_cheapest_model(&request).await?
            }
            RoutingStrategy::Fastest => {
                self.find_fastest_model(&request).await?
            }
            // ...
        };
        
        self.client.complete(route.model, request).await
    }
}
```

### Auto-Routing

```rust
// Automatic model selection based on criteria
OpenRouterRequest {
    models: vec!["anthropic/claude-3.5-sonnet", "openai/gpt-4o"],
    route: Route::Fallback,  // Try first, fallback to second
    // OR
    route: Route::Auto {
        criteria: vec![
            Criterion::MaxPrice(0.01),
            Criterion::MinThroughput(100),
        ],
    },
}
```

---

<!--
Speaker Notes: Ollama is crucial for privacy-conscious users and offline scenarios.
-->

## 🖥️ Ollama (Local)

### Local LLM Execution

```rust
// crates/providers/src/ollama.rs
pub struct OllamaProvider {
    base_url: String,  // Default: http://localhost:11434
    default_model: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: Request) -> Result<Response> {
        let ollama_request = OllamaGenerateRequest {
            model: request.model.unwrap_or(&self.default_model),
            prompt: format_messages(&request.messages),
            stream: request.stream,
            options: Some(Options {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            }),
        };
        
        let response = self.http_client
            .post(format!("{}/api/generate", self.base_url))
            .json(&ollama_request)
            .send()
            .await?;
        
        Ok(parse_ollama_response(response).await?)
    }
}
```

### Recommended Models

| Model | Size | Use Case |
|-------|------|----------|
| llama3.1 | 8B-405B | General purpose |
| mistral-nemo | 12B | Fast, efficient |
| codellama | 7B-70B | Code generation |
| qwen2.5-coder | 7B-32B | Coding, multilingual |
| phi3 | 3.8B | Resource-constrained |

### Configuration

```toml
# config/providers.toml
[providers.ollama]
enabled = true
base_url = "http://localhost:11434"
default_model = "llama3.1"

# Optional: Auto-pull models
auto_pull = true
```

---

<!--
Speaker Notes: The fallback chain is a key production feature. Explain how it ensures high availability.
-->

## 🔗 Fallback Chain

### Automatic Provider Failover

```
┌─────────────────────────────────────────────────────────────┐
│                    FALLBACK CHAIN                            │
│                                                              │
│   Primary:    Anthropic ─────────────────────────┐          │
│                  │                               │          │
│                  │ (Failure)                      │          │
│                  ▼                               │          │
│   Secondary:  OpenAI ◄───────────────────────────┘          │
│                  │          (Retry with backup key)         │
│                  │ (Failure)                                │
│                  ▼                                          │
│   Tertiary:   OpenRouter ───► Try multiple models          │
│                  │ (Failure)                                │
│                  ▼                                          │
│   Final:      Ollama (local) ──► Offline capable           │
│                  │ (Failure)                                │
│                  ▼                                          │
│   Result:     Error ──► Queue for retry                    │
└─────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
// crates/providers/src/fallback.rs
pub struct FallbackChain {
    providers: Vec<ProviderEntry>,
    cooldowns: HashMap<String, Instant>,
}

struct ProviderEntry {
    name: String,
    provider: Box<dyn LlmProvider>,
    priority: u8,
    retry_keys: Vec<String>, // Backup API keys
}

impl FallbackChain {
    pub async fn complete(&self, request: Request) -> Result<Response> {
        for entry in self.providers_by_priority() {
            // Skip if in cooldown
            if self.is_in_cooldown(&entry.name) {
                continue;
            }
            
            // Try primary key
            match entry.provider.complete(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!("Provider {} failed: {}", entry.name, e);
                    
                    // Try backup keys
                    for key in &entry.retry_keys {
                        entry.provider.set_api_key(key);
                        if let Ok(response) = entry.provider.complete(request.clone()).await {
                            return Ok(response);
                        }
                    }
                    
                    // Enter cooldown
                    self.set_cooldown(&entry.name, Duration::from_secs(60));
                }
            }
        }
        
        Err(Error::AllProvidersFailed)
    }
}
```

---

<!--
Speaker Notes: Streaming is essential for good UX. Show how it's implemented consistently across providers.
-->

## 📡 Streaming Support

### Unified Streaming Interface

```rust
// crates/providers/src/lib.rs
pub trait LlmProvider: Send + Sync {
    // Synchronous completion
    async fn complete(&self, request: Request) -> Result<Response>;
    
    // Streaming completion
    async fn complete_stream(
        &self, 
        request: Request
    ) -> Result<BoxStream<'static, Result<StreamEvent>>>;
}

pub enum StreamEvent {
    Token(String),           // Text token
    ToolCallStart { name: String },
    ToolCallDelta { name: String, delta: String },
    ToolCallEnd { name: String, arguments: Value },
    Usage(Usage),            // Token counts
    Done,                    // Stream complete
}
```

### Provider-Specific Streaming

```rust
// Anthropic: Server-sent events with tool_call delta fix
// OpenAI: Server-sent events (standard)
// OpenRouter: Pass-through from upstream
// Ollama: NDJSON stream

// Unified handling
while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::Token(token) => {
            sender.send(Message::Text(token)).await?;
        }
        StreamEvent::ToolCallStart { name } => {
            current_tool = Some(name);
        }
        // ...
    }
}
```

---

<!--
Speaker Notes: Cost optimization is a major concern. Show the tools and strategies available.
-->

## 💰 Cost Optimization

### Cost Comparison (per 1M tokens)

| Provider | Input | Output | Best For |
|----------|-------|--------|----------|
| Claude 3.5 Sonnet | $3 | $15 | Quality, coding |
| GPT-4o | $5 | $15 | Multimodal |
| GPT-4o-mini | $0.15 | $0.60 | Cost-sensitive |
| OpenRouter (mix) | $0.5-10 | $1-20 | Flexibility |
| Ollama (local) | $0 | $0 | Privacy, volume |

### Optimization Strategies

```rust
// 1. Model Selection by Complexity
pub fn select_model(complexity: TaskComplexity) -> String {
    match complexity {
        TaskComplexity::Simple => "gpt-4o-mini",  // Cheap
        TaskComplexity::Standard => "claude-3-5-sonnet",  // Balanced
        TaskComplexity::Complex => "claude-3-opus",  // Quality
    }
}

// 2. Caching (Anthropic prompt caching)
CachedRequest {
    system: "Long system prompt...",  // Cached
    messages: vec![...],              // Not cached
    cache_control: CacheControl::Ephemeral,
}
// 90% cost reduction on repeated contexts

// 3. Batch API (Anthropic)
BatchRequest {
    requests: vec![req1, req2, ..., req100],
}
// 50% cost savings, 24h turnaround
```

---

<!--
Speaker Notes: Provide guidance on choosing the right model for different use cases.
-->

## 📋 Model Selection Guide

### By Use Case

| Use Case | Recommended | Fallback |
|----------|-------------|----------|
| **Code Generation** | Claude 3.5 Sonnet | GPT-4o |
| **Complex Analysis** | Claude 3 Opus | GPT-4-turbo |
| **Chat/General** | Claude 3.5 Sonnet | Llama 3.1 |
| **High Volume** | GPT-4o-mini | Local (Ollama) |
| **Multimodal** | GPT-4o | Claude 3.5 Sonnet |
| **Reasoning** | o1-preview | Claude 3 Opus |
| **Privacy-Critical** | Local (Ollama) | — |
| **Cost-Sensitive** | OpenRouter (cheapest) | GPT-4o-mini |

### Configuration Example

```toml
# config/providers.toml
[providers]

[providers.anthropic]
enabled = true
api_key = "${ANTHROPIC_API_KEY}"
default_model = "claude-3-5-sonnet-20241022"
priority = 1  # Primary

[providers.openai]
enabled = true
api_key = "${OPENAI_API_KEY}"
default_model = "gpt-4o"
priority = 2  # Fallback
backup_keys = ["${OPENAI_BACKUP_KEY}"]

[providers.openrouter]
enabled = true
api_key = "${OPENROUTER_API_KEY}"
routing_strategy = "balanced"
priority = 3

[providers.ollama]
enabled = true
priority = 4  # Final fallback
```

---

<!--
Speaker Notes: Summary and key takeaways for the provider ecosystem.
-->

## 🎯 Provider Ecosystem Summary

### Key Features

```
┌─────────────────────────────────────────────────────────────┐
│  ✅ 4 Native Providers                                       │
│     • Anthropic (Claude)                                     │
│     • OpenAI (GPT-4)                                         │
│     • OpenRouter (400+ models)                               │
│     • Ollama (Local)                                         │
│                                                              │
│  ✅ Automatic Fallback Chain                                 │
│     • Per-key retry logic                                    │
│     • Cooldown management                                    │
│     • Zero-downtime failover                                 │
│                                                              │
│  ✅ Unified Interface                                        │
│     • Same API for all providers                             │
│     • Streaming support                                      │
│     • Tool use (auto-converted)                              │
│                                                              │
│  ✅ Cost Optimization                                        │
│     • Smart model selection                                  │
│     • Prompt caching                                         │
│     • Batch API support                                      │
└─────────────────────────────────────────────────────────────┘
```

### Quick Reference

| Command | Description |
|---------|-------------|
| `openrustclaw models list` | List available models |
| `openrustclaw chat --provider anthropic` | Chat with specific provider |
| `openrustclaw chat --provider openrouter --model meta-llama/llama-3.1-70b` | Specific model |

---

## 📚 Additional Resources

### Documentation

| Resource | Location |
|----------|----------|
| Provider Implementation | `crates/providers/src/` |
| Fallback Chain | `crates/providers/src/fallback.rs` |
| Tool Format Conversion | `crates/providers/src/tool_formats.rs` |
| Configuration | `config/providers.toml` |

### External Links

- 📖 [Anthropic API Docs](https://docs.anthropic.com)
- 📖 [OpenAI API Docs](https://platform.openai.com/docs)
- 📖 [OpenRouter Docs](https://openrouter.ai/docs)
- 📖 [Ollama Library](https://ollama.com/library)

### Environment Variables

```bash
# Required for cloud providers
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export OPENROUTER_API_KEY="sk-or-..."

# Optional
export OPENAI_BACKUP_KEY="sk-..."  # For fallback
```
