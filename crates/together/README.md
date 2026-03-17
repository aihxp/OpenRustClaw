# Together AI SDK for Rust

A native Rust SDK for [Together AI](https://www.together.ai/)'s inference API.

## Features

- **Chat Completions**: Full support for multi-turn chat with streaming and tool calling
- **Completions**: Legacy text completions API
- **Embeddings**: Generate embeddings for text using various models
- **Fine-tuning**: Create and manage fine-tuned models
- **Models**: List available models with their capabilities
- **Streaming**: Real-time streaming responses for chat and completions

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
together-ai = { path = "../crates/together" }
```

## Quick Start

```rust
use together_ai::{TogetherClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = TogetherClient::new(std::env::var("TOGETHER_API_KEY")?)?;

    // Create a chat request
    let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
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
| Llama 3 70B Chat | `meta-llama/Llama-3-70b-chat-hf` | Chat |
| Llama 3 8B Chat | `meta-llama/Llama-3-8b-chat-hf` | Chat |
| Mixtral 8x22B Instruct | `mistralai/Mixtral-8x22B-Instruct-v0.1` | Chat |
| WizardLM 2 8x22B | `microsoft/WizardLM-2-8x22B` | Chat |
| Gemma 7B Instruct | `google/gemma-7b-it` | Chat |
| BGE Large En v1.5 | `BAAI/bge-large-en-v1.5` | Embedding |

## Examples

### Chat Completions

```rust
use together_ai::{TogetherClient, ChatRequest, Role};

let client = TogetherClient::new("your-api-key")?;

let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
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

let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
    .user("Tell me a story.")
    .build();

let mut stream = client.chat().complete_stream(request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content());
    std::io::Write::flush(&mut std::io::stdout())?;
}
```

### Embeddings

```rust
use together_ai::embeddings::EmbeddingRequest;

let request = EmbeddingRequest::new(
    "BAAI/bge-large-en-v1.5",
    "The quick brown fox jumps over the lazy dog"
);

let response = client.embeddings().create(request).await?;
let embedding = response.first_embedding().unwrap();
println!("Embedding dimensions: {}", embedding.len());
```

### Fine-tuning

```rust
use together_ai::fine_tuning::FineTuneRequest;

// Upload training file
let file = client.fine_tuning()
    .upload_file("training_data.jsonl", "fine-tune")
    .await?;

// Create fine-tuning job
let request = FineTuneRequest::new("meta-llama/Llama-3-8b-chat-hf", &file.id)
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
use together_ai::ClientConfig;
use std::time::Duration;

let config = ClientConfig::new("your-api-key")
    .base_url("https://api.together.xyz")
    .timeout(Duration::from_secs(120))
    .max_retries(3);

let client = TogetherClient::with_config(config)?;
```

## Error Handling

The SDK uses a comprehensive error type:

```rust
use together_ai::TogetherError;

match result {
    Err(TogetherError::RateLimit { retry_after, .. }) => {
        println!("Rate limited! Retry after: {:?}", retry_after);
    }
    Err(TogetherError::Authentication { message }) => {
        println!("Auth failed: {}", message);
    }
    Err(e) => println!("Error: {}", e),
    Ok(response) => println!("Success!"),
}
```

## License

MIT
