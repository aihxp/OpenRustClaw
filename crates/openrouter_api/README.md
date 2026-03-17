# openrouter-api

A native Rust SDK for OpenRouter's API with routing strategies and fallback support.

## Features

- ✅ **Chat Completions**: Full streaming support
- ✅ **Model Listing**: With pricing information
- ✅ **Routing Strategies**: Price, quality, throughput, and web search
- ✅ **Provider Fallback**: Automatic fallback between providers
- ✅ **Usage Tracking**: Token and cost tracking

## Quick Start

```rust
use openrouter_api::{OpenRouterClient, ChatRequest, RouteStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenRouterClient::new(std::env::var("OPENROUTER_API_KEY")?)?;

    let request = ChatRequest::builder("anthropic/claude-3.5-sonnet")
        .user("What is the capital of France?")
        .route_strategy(RouteStrategy::Quality)
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content());

    Ok(())
}
```

## Routing Strategies

```rust
use openrouter_api::RouteStrategy;

// Optimize for price (adds :floor suffix)
let request = ChatRequest::builder("openai/gpt-4o")
    .route_strategy(RouteStrategy::Price)
    .user("Hello")
    .build();

// Optimize for throughput (adds :nitro suffix)
let request = ChatRequest::builder("openai/gpt-4o")
    .route_strategy(RouteStrategy::Throughput)
    .user("Hello")
    .build();

// Include web search results (adds :online suffix)
let request = ChatRequest::builder("openai/gpt-4o")
    .route_strategy(RouteStrategy::Online)
    .user("What's the latest news?")
    .build();
```

## List Models

```rust
let models = client.models().list().await?;
for model in models.data {
    println!("{} - ${}/1M tokens", 
        model.id, 
        model.pricing.prompt * 1_000_000.0
    );
}
```

## Features

- `streaming` - Enable streaming support
- `routing` - Enable routing strategies
- `fallback` - Enable fallback support
- `models` - Enable model listing

## License

MIT
