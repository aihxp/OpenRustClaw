# llama.cpp SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/llama-cpp)](https://crates.io/crates/llama-cpp)
[![Documentation](https://docs.rs/llama-cpp/badge.svg)](https://docs.rs/llama-cpp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A native Rust SDK for [llama.cpp](https://github.com/ggerganov/llama.cpp) server - Local LLM inference with no API keys required.

## Features

- **Chat Completions API** - Complete support with streaming
- **Text Completions API** - Raw text completion with streaming
- **Tokenization** - Convert text to token IDs
- **Embeddings** - Generate embeddings for text
- **Health Check** - Monitor server status
- **Slots Info** - Monitor concurrent request slots
- **All GGUF Models** - Support for all llama.cpp compatible models:
  - Llama 2/3
  - Mistral/Mixtral
  - CodeLlama
  - Phi
  - Qwen
  - Gemma
  - And all other GGUF models

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llama-cpp = "1.4.0"
```

### Feature Flags

- `streaming` - Enable streaming support (enabled by default)
- `chat` - Enable chat completions API (enabled by default)
- `completion` - Enable text completions API (enabled by default)
- `tokenize` - Enable tokenization API (enabled by default)
- `embeddings` - Enable embeddings API (enabled by default)
- `health` - Enable health check API (enabled by default)
- `slots` - Enable slots API (enabled by default)

## Quick Start

```rust
use llama_cpp::{LlamaCppClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client connecting to local llama.cpp server
    let client = LlamaCppClient::new()?;

    // Create a chat request
    let request = ChatRequest::builder()
        .message(Role::System, "You are a helpful assistant.")
        .message(Role::User, "Hello, llama!")
        .build();

    // Get completion
    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or("No content"));

    Ok(())
}
```

## Usage Examples

### Chat Completions

```rust
use llama_cpp::{LlamaCppClient, ChatRequest, Role};

let client = LlamaCppClient::new()?;

let request = ChatRequest::builder()
    .system("You are a helpful assistant.")
    .user("What is the capital of France?")
    .temperature(0.7)
    .max_tokens(512)
    .build();

let response = client.chat().complete(request).await?;
println!("{}", response.content().unwrap_or_default());
```

### Streaming Chat

```rust
use llama_cpp::{LlamaCppClient, ChatRequest, Role};
use futures::StreamExt;

let client = LlamaCppClient::new()?;

let request = ChatRequest::builder()
    .user("Tell me a story")
    .build();

let mut stream = client.chat().stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => {
            if let Some(content) = chunk.content() {
                print!("{}", content);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Text Completions

```rust
use llama_cpp::{LlamaCppClient, CompletionRequest};

let client = LlamaCppClient::new()?;

let request = CompletionRequest::builder("Once upon a time")
    .max_tokens(100)
    .temperature(0.8)
    .build();

let response = client.completion().complete(request).await?;
println!("{}", response.content().unwrap_or_default());
```

### Fill-in-the-Middle (FIM)

```rust
use llama_cpp::{LlamaCppClient, CompletionRequest};

let client = LlamaCppClient::new()?;

let request = CompletionRequest::builder("def fibonacci(n):")
    .suffix("return result")
    .max_tokens(256)
    .build();

let response = client.completion().complete(request).await?;
println!("{}", response.content().unwrap_or_default());
```

### Tokenization

```rust
use llama_cpp::LlamaCppClient;

let client = LlamaCppClient::new()?;

// Tokenize text
let tokens = client.tokenize().tokenize_text("Hello world").await?;
println!("Tokens: {:?}", tokens.tokens);

// Or just count tokens
let count = client.tokenize().count_tokens("Hello world").await?;
println!("Token count: {}", count);
```

### Embeddings

```rust
use llama_cpp::LlamaCppClient;

let client = LlamaCppClient::new()?;

// Generate embeddings
let response = client.embeddings().embed_text("Hello world").await?;
println!("Embedding dimensions: {}", response.embedding.len());

// With normalization
use llama_cpp::embeddings::EmbeddingRequest;

let request = EmbeddingRequest::builder("Hello world")
    .normalize(true)
    .build();

let response = client.embeddings().embed(request).await?;
```

### Health Check

```rust
use llama_cpp::LlamaCppClient;

let client = LlamaCppClient::new()?;

// Check health
let health = client.health().check().await?;
println!("Status: {}", health.status);
println!("Idle slots: {:?}", health.slots_idle);
println!("Processing slots: {:?}", health.slots_processing);

// Quick boolean check
if client.health().is_healthy().await {
    println!("Server is healthy!");
}

// Wait for server to be healthy
let health = client.health().wait_for_healthy(std::time::Duration::from_secs(30)).await?;
```

### Slots Monitoring

```rust
use llama_cpp::LlamaCppClient;

let client = LlamaCppClient::new()?;

// List all slots
let slots = client.slots().list().await?;
for slot in &slots.slots {
    println!("Slot {}: {:?}", slot.id, slot.state);
}

// Get counts
let total = client.slots().count().await?;
let idle = client.slots().idle_count().await?;
let processing = client.slots().processing_count().await?;

println!("Total: {}, Idle: {}, Processing: {}", total, idle, processing);

// Wait for available slot
let slot = client.slots().wait_for_available(std::time::Duration::from_secs(30)).await?;
```

### Custom Configuration

```rust
use llama_cpp::{LlamaCppClient, ClientConfig};
use std::time::Duration;

let config = ClientConfig::new()
    .base_url("http://localhost:8081")
    .timeout(Duration::from_secs(300))
    .max_retries(5);

let client = LlamaCppClient::with_config(config)?;
```

### JSON Mode

```rust
use llama_cpp::{LlamaCppClient, ChatRequest, Role};

let client = LlamaCppClient::new()?;

let request = ChatRequest::builder()
    .system("You are a helpful assistant that outputs JSON.")
    .user("Give me information about Paris in JSON format")
    .json_mode(true)
    .build();

let response = client.chat().complete(request).await?;
let json: serde_json::Value = serde_json::from_str(response.content().unwrap_or_default())?;
```

## Server Endpoints

The SDK interacts with the following llama.cpp server endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/completion` | POST | Text completion |
| `/chat/completion` | POST | Chat completion |
| `/tokenize` | POST | Tokenize text |
| `/embedding` | POST | Generate embeddings |
| `/health` | GET | Health check |
| `/slots` | GET | Get slot information |

## Running llama.cpp Server

To use this SDK, you need to run the llama.cpp server:

```bash
# Build llama.cpp server
make server

# Run server with a model
./server -m path/to/model.gguf -c 4096

# Or with specific host/port
./server -m path/to/model.gguf --host 0.0.0.0 --port 8080
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
