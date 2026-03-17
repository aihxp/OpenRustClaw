# Fireworks AI SDK for Rust

A native Rust SDK for [Fireworks AI](https://fireworks.ai/)'s fast inference API.

## Features

- **Chat Completions**: Full support for multi-turn chat with streaming and tool calling
- **Completions**: Legacy text completions API
- **Embeddings**: Generate embeddings for text using various models
- **Fine-tuning**: Create and manage fine-tuned models
- **Image Generation**: Generate images using state-of-the-art diffusion models (SDXL, Flux, etc.)
- **Models**: List available models with their capabilities
- **Streaming**: Real-time streaming responses for chat and completions
- **Function Calling**: Fireworks-specific function calling support with firefunction-v2

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
fireworks-ai = { path = "../crates/fireworks" }
```

## Quick Start

```rust
use fireworks_ai::{FireworksClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = FireworksClient::new(std::env::var("FIREWORKS_API_KEY")?)?;

    // Create a chat request
    let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
        .message(Role::User, "What is the capital of France?")
        .build();

    // Send the request
    let response = client.chat().complete(request).await?;
    println!("{}", response.content());

    Ok(())
}
```

## Popular Models

| Model | ID | Type |
|-------|-----|------|
| Llama 3.1 405B Instruct | `accounts/fireworks/models/llama-v3p1-405b-instruct` | Chat |
| Llama 3.1 70B Instruct | `accounts/fireworks/models/llama-v3p1-70b-instruct` | Chat |
| Llama 3.1 8B Instruct | `accounts/fireworks/models/llama-v3p1-8b-instruct` | Chat |
| Mixtral 8x22B Instruct | `accounts/fireworks/models/mixtral-8x22b-instruct` | Chat |
| Firefunction V2 | `accounts/fireworks/models/firefunction-v2` | Function Calling |
| SDXL | `accounts/fireworks/models/sdxl` | Image Generation |
| Flux.1 Dev | `accounts/fireworks/models/flux-1-dev` | Image Generation |
| Nomic Embed Text v1.5 | `accounts/fireworks/models/nomic-embed-text-v1-5` | Embedding |

## Examples

### Chat Completions

```rust
use fireworks_ai::{FireworksClient, ChatRequest, Role};

let client = FireworksClient::new("your-api-key")?;

let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
    .system("You are a helpful assistant.")
    .user("Explain quantum computing in simple terms.")
    .max_tokens(500)
    .temperature(0.7)
    .build();

let response = client.chat().complete(request).await?;
println!("{}", response.content());
```

### Streaming Chat

```rust
use futures::StreamExt;

let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
    .user("Tell me a story.")
    .build();

let mut stream = client.chat().complete_stream(request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = chunk.content() {
        print!("{}", content);
        std::io::Write::flush(&mut std::io::stdout())?;
    }
}
```

### Function Calling

```rust
use fireworks_ai::{FireworksClient, ChatRequest, Role, Tool};
use serde_json::json;

let client = FireworksClient::new("your-api-key")?;

// Define a tool
let weather_tool = Tool::function(
    "get_weather",
    "Get the current weather for a location",
    json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "The city and state"
            }
        },
        "required": ["location"]
    })
);

let request = ChatRequest::builder("accounts/fireworks/models/firefunction-v2")
    .message(Role::User, "What's the weather in San Francisco?")
    .tool(weather_tool)
    .build();

let response = client.chat().complete(request).await?;
if response.has_tool_calls() {
    println!("Tool calls: {:?}", response.tool_calls());
}
```

### Embeddings

```rust
use fireworks_ai::embeddings::EmbeddingRequest;

let request = EmbeddingRequest::new(
    "accounts/fireworks/models/nomic-embed-text-v1-5",
    "The quick brown fox jumps over the lazy dog"
);

let response = client.embeddings().create(request).await?;
let embedding = response.first_embedding().unwrap();
println!("Embedding dimensions: {}", embedding.len());
```

### Image Generation

```rust
use fireworks_ai::image_generation::{ImageGenerationRequest, ImageSize};

let request = ImageGenerationRequest::builder(
    "accounts/fireworks/models/flux-1-dev",
    "A serene mountain landscape at sunset"
)
    .size(ImageSize::S1024x1024)
    .n(1)
    .build();

let response = client.images().generate(request).await?;
for image in &response.data {
    if let Some(url) = &image.url {
        println!("Generated image: {}", url);
    }
}
```

### Fine-tuning

```rust
use fireworks_ai::fine_tuning::FineTuneRequest;

// Upload training file
let file = client.fine_tuning()
    .upload_file("training_data.jsonl", "fine-tune")
    .await?;

// Create fine-tuning job
let request = FineTuneRequest::new(
    "accounts/fireworks/models/llama-v3p1-8b-instruct",
    &file.id
)
    .n_epochs(3)
    .learning_rate(1e-5)
    .suffix("my-custom-model");

let job = client.fine_tuning().create_job(request).await?;
println!("Job created: {}", job.id);

// Check job status
let job = client.fine_tuning().get_job(&job.id).await?;
println!("Status: {}", job.status);
```

### List Models

```rust
let models = client.models().list().await?;

for model in &models.data {
    println!("{} - {}", 
        model.id, 
        model.description.as_deref().unwrap_or("No description")
    );
}
```

## Configuration

```rust
use fireworks_ai::ClientConfig;
use std::time::Duration;

let config = ClientConfig::new("your-api-key")
    .base_url("https://api.fireworks.ai/inference/v1")
    .timeout(Duration::from_secs(120))
    .max_retries(3);

let client = FireworksClient::with_config(config)?;
```

## Error Handling

The SDK uses a comprehensive error type:

```rust
use fireworks_ai::FireworksError;

match result {
    Err(FireworksError::RateLimit { retry_after, .. }) => {
        println!("Rate limited! Retry after: {:?}", retry_after);
    }
    Err(FireworksError::Authentication { message }) => {
        println!("Auth failed: {}", message);
    }
    Err(e) => println!("Error: {}", e),
    Ok(response) => println!("Success!"),
}
```

## License

MIT
