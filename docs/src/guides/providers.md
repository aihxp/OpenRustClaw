# Configuring LLM Providers

This guide covers configuring and using the various LLM providers supported by OpenRustClaw.

---

## 🎯 Common Provider Setups

This guide focuses on the most common provider setups. For the broader provider inventory, see `docs/src/architecture/provider-sdks.md` and the provider crate READMEs under `crates/`.

| Provider | Models | Best For |
|----------|--------|----------|
| **Anthropic** | Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Haiku | Tool use, reasoning |
| **OpenAI** | GPT-4o, GPT-4o-mini, o1 | General purpose |
| **OpenRouter** | 400+ models from various providers | Cost optimization, variety |
| **Ollama** | Llama, Mistral, Gemma, and more | Privacy, local inference |

---

## 🔐 API Key Setup

### Anthropic

1. Visit [console.anthropic.com](https://console.anthropic.com)
2. Sign up or log in
3. Go to "Get API keys"
4. Generate a new key
5. Add to `.env`:

```bash
ANTHROPIC_API_KEY=sk-ant-api03-xxxxxxxxxxxxxxxxxxxxxxxx
```

### OpenAI

1. Visit [platform.openai.com](https://platform.openai.com)
2. Create an account
3. Go to "API keys"
4. Create a new secret key
5. Add to `.env`:

```bash
OPENAI_API_KEY=sk-proj-xxxxxxxxxxxxxxxxxxxxxxxx
```

### OpenRouter

1. Visit [openrouter.ai](https://openrouter.ai)
2. Create an account
3. Go to "Keys"
4. Create a new key
5. Add to `.env`:

```bash
OPENROUTER_API_KEY=sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxx
```

### Ollama

1. Install Ollama from [ollama.com](https://ollama.com)
2. Pull a model:

```bash
ollama pull llama3.1
ollama pull mistral
```

3. Start Ollama:

```bash
ollama serve
```

4. Configure in `.env`:

```bash
OLLAMA_BASE_URL=http://localhost:11434
```

---

## ⚙️ Provider Configuration

### Basic Configuration

Create or edit `config/providers.toml`:

```toml
[providers]
# Primary provider used by default
default_provider = "anthropic"

# Fallback chain (tried in order if primary fails)
fallback_chain = ["openai", "openrouter", "ollama"]

# Preferred provider used for operator/control-plane guidance
control_plane_provider = "openrouter"

# Control-plane failover order
control_plane_fallback_chain = ["ollama", "anthropic"]

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"
temperature = 0.7
max_tokens = 4096

[providers.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"
use_responses_api = true  # Use new Responses API instead of Chat Completions
temperature = 0.7
max_tokens = 4096

[providers.openrouter]
api_key = "${OPENROUTER_API_KEY}"
model = "anthropic/claude-3.5-sonnet"
# OpenRouter requires these for TOS compliance
site_url = "https://yourdomain.com"
site_name = "Your App"

[providers.ollama]
base_url = "${OLLAMA_BASE_URL}"
model = "llama3.1"
```

### Control-Plane Provider Lane

OpenRustClaw can declare the operator/control-plane lane separately from the main task lane:

```toml
[providers]
default_provider = "anthropic"
fallback_chain = ["openai", "openrouter", "ollama"]

control_plane_provider = "openrouter"
control_plane_fallback_chain = ["ollama", "anthropic"]
```

Use this when you want runtime edits, onboarding, upgrade planning, and health guidance to evaluate a cheaper or more available provider separately from the main task model. The current runtime status and health surfaces report this lane separately and precompute failover recommendations when the preferred control-plane provider is degraded.

The shipped onboarding and `openrustclaw runtime switch-provider` flows now also persist this lane automatically when it is missing or still mirrors the primary task lane. They prefer a distinct low-cost/local control-plane provider where one is available, with `ollama` first, then other configured non-primary providers. `openrustclaw runtime status|upgrade-plan|self-update-plan|rollback-plan` now expose the resolved control-plane action provider/model that operator actions will use.

### Runtime Health Scans

`openrustclaw runtime health --refresh` and `/control/runtime/health|scan` now persist more than a basic healthy/unhealthy bit. The current runtime-health report records:

- whether the configured model still appears in the provider catalog
- whether the latest provider probe looks like auth/access failure, billing/quota failure, rate limiting, or a generic provider outage
- rate-limit header snapshots where providers expose them
- operator warnings when a previously healthy lane regresses or a configured model disappears

That lets operators catch removed models, broken credentials, and changed provider limit posture before the next runtime cutover or model switch.

### Per-Request Configuration

Override defaults for specific requests:

```rust
let request = CompletionRequest {
    model: Some("claude-opus-4-20250514".into()),  // Override default model
    temperature: Some(0.2),  // Lower for more deterministic output
    max_tokens: Some(8192),  // Larger for long outputs
    ..Default::default()
};

let response = provider.complete(request).await?;
```

---

## 🤖 Model Selection

### Model Comparison

| Model | Context | Strengths | Cost (per 1M tokens) |
|-------|---------|-----------|---------------------|
| Claude 3.5 Sonnet | 200K | Best tool use, reasoning | $3 / $15 |
| Claude 3 Haiku | 200K | Fast, cheap | $0.25 / $1.25 |
| GPT-4o | 128K | General purpose, vision | $2.50 / $10 |
| GPT-4o-mini | 128K | Very cheap | $0.15 / $0.60 |
| Llama 3.1 (Ollama) | 128K | Free, private | $0 |

### Selecting by Use Case

**Tool Use (Function Calling):**
```toml
# Best: Claude 3.5 Sonnet
model = "claude-sonnet-4-20250514"

# Alternative: GPT-4o
model = "gpt-4o"
```

**Quick Responses:**
```toml
# Fast and cheap: Claude Haiku
model = "claude-haiku-3-20240307"

# Or GPT-4o-mini
model = "gpt-4o-mini"
```

**Long Context:**
```toml
# 200K context: Claude models
model = "claude-sonnet-4-20250514"

# 128K context: GPT-4o
model = "gpt-4o"
```

**Local/Privacy:**
```toml
# Ollama local models
[providers.ollama]
model = "llama3.1:70b"  # Larger model
# or
model = "mistral"       # Smaller, faster
```

---

## 🔄 Fallback Configuration

### Automatic Fallback Chain

```rust
use openrustclaw_providers::{ProviderChain, AnthropicProvider, OpenAiProvider, OpenRouterProvider};

let chain = ProviderChain::builder()
    .add_provider(Arc::new(AnthropicProvider::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        "claude-sonnet-4-20250514".into(),
    )))
    .add_provider(Arc::new(OpenAiProvider::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o".into(),
    )))
    .add_provider(Arc::new(OpenRouterProvider::new(
        std::env::var("OPENROUTER_API_KEY")?,
        "anthropic/claude-3.5-sonnet".into(),
    )))
    .build();

// Automatically tries each provider on failure
let response = chain.complete(request).await?;
```

### Fallback Triggers

The chain automatically falls back on:

| Error Type | Behavior |
|------------|----------|
| Rate limit | Cooldown provider, try next |
| Auth failure | Permanent cooldown |
| Timeout | Try next immediately |
| 5xx error | Try next immediately |
| Context limit | Try next (might have larger context) |

### Cooldown Configuration

```toml
[providers.cooldowns]
# How long to wait before retrying after rate limit
rate_limit_seconds = 60

# How long to wait after auth failure
auth_failure_minutes = 60

# Max cooldown duration
max_cooldown_minutes = 120
```

---

## 📊 Provider-Specific Features

### Anthropic

```rust
// Strict tool mode (guaranteed schema compliance)
let request = CompletionRequest {
    tools: Some(tools),
    tool_choice: Some(ToolChoice::Any),  // Force tool use
    ..Default::default()
};

// Batch API for cost savings (50% off)
let batch_provider = anthropic_provider.as_batch_provider();
let responses = batch_provider.batch(requests).await?;
```

### OpenAI

```rust
// Use Responses API for better tool support
let provider = OpenAiProvider::new(api_key, model)
    .with_responses_api(true);

// JSON mode for structured output
let request = CompletionRequest {
    response_format: Some(ResponseFormat::JsonObject {
        schema: my_schema,
    }),
    ..Default::default()
};
```

### OpenRouter

```rust
// Auto-routing by price/quality/speed
let provider = OpenRouterProvider::new(api_key, "auto".into())
    .with_routing_strategy(RoutingStrategy::Price);

// Provider-specific parameters
let request = CompletionRequest {
    extra_body: Some(json!({
        "provider": {
            "order": ["Anthropic", "OpenAI"],
            "allow_fallbacks": false,
        }
    })),
    ..Default::default()
};
```

### Ollama

```rust
// Custom Ollama options
let provider = OllamaProvider::new("http://localhost:11434".into())
    .with_model("llama3.1")
    .with_options(OllamaOptions {
        temperature: 0.7,
        num_ctx: 32768,  // Context window
        num_gpu: 1,      // GPU layers
    });
```

---

## 🛠️ Troubleshooting

### "Rate limited" errors

```bash
# Check rate limits
curl -H "x-api-key: $ANTHROPIC_API_KEY" \
  https://api.anthropic.com/v1/rate_limits

# Solutions:
# 1. Configure multiple providers for fallback
# 2. Implement request batching
# 3. Upgrade your API plan
# 4. Add delays between requests
```

### "Authentication failed" errors

```bash
# Verify API key format
echo $ANTHROPIC_API_KEY | head -c 20
# Should start with "sk-ant-"

# Test the key
curl -H "x-api-key: $ANTHROPIC_API_KEY" \
  https://api.anthropic.com/v1/models
```

### "Model not found" errors

```bash
# List available models
openrustclaw models list --provider anthropic

# Check model name spelling
# Anthropic: claude-sonnet-4-20250514
# OpenAI: gpt-4o
# OpenRouter: anthropic/claude-3.5-sonnet
```

### Ollama connection issues

```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# If not running:
ollama serve

# Check model is pulled:
ollama list

# Pull if needed:
ollama pull llama3.1
```

### Slow responses

```toml
# Use faster models
[providers.anthropic]
model = "claude-haiku-3-20240307"  # Fastest Claude

[providers.openai]
model = "gpt-4o-mini"  # Fastest GPT-4

# Or use local models
[providers.ollama]
model = "phi3"  # Very fast on CPU
```

---

## 📈 Monitoring Provider Usage

### View Provider Statistics

```bash
# Show usage by provider
openrustclaw doctor --providers

# Output:
# Provider      Requests    Tokens    Errors    Avg Latency
# anthropic     1,234       5.2M      12        450ms
# openai        456         2.1M      3         380ms
# openrouter    89          0.4M      0         520ms
```

### Cost Tracking

```rust
// Responses include cost information
let response = provider.complete(request).await?;

if let Some(cost) = response.usage.cost_usd {
    tracing::info!("Request cost: ${:.4}", cost);
}
```

---

## 🎓 Best Practices

### 1. Configure Multiple Providers

Always have at least 2 providers configured for fallback:

```toml
[providers]
primary = "anthropic"
fallback_chain = ["openai", "ollama"]
control_plane_provider = "ollama"
control_plane_fallback_chain = ["openrouter"]
```

### 2. Use Appropriate Models

Match the model to the task:

- **Complex reasoning**: Claude 3.5 Sonnet, GPT-4o
- **Quick tasks**: Claude Haiku, GPT-4o-mini
- **Cost-sensitive**: GPT-4o-mini, local Ollama
- **Privacy**: Ollama local models

### 3. Set Reasonable Timeouts

```rust
let chain = ProviderChain::builder()
    .add_provider(anthropic)
    .timeout(Duration::from_secs(30))
    .build();
```

### 4. Monitor Fallback Frequency

High fallback frequency indicates:
- Need to upgrade rate limits
- Network issues
- Provider instability

### 5. Use Streaming for UX

```rust
let mut stream = provider.stream(request).await?;

while let Some(chunk) = stream.next().await {
    match chunk? {
        StreamChunk::ContentDelta { delta } => {
            print!("{}", delta);
            std::io::stdout().flush()?;
        }
        StreamChunk::Done { response } => {
            println!("\n[Complete: {} tokens]", response.usage.total_tokens);
        }
        _ => {}
    }
}
```
