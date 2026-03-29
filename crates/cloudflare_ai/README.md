# Cloudflare Workers AI SDK for Rust

A native Rust SDK for Cloudflare Workers AI - Run AI models on Cloudflare's global network.

## Features

- **Text Generation**: Support for all Cloudflare text generation models (Llama, Mistral, Phi, Qwen, TinyLlama)
- **Embeddings**: Generate text embeddings for semantic search
- **Translation**: Translate text between languages (M2M100)
- **Summarization**: Summarize long texts (BART)
- **Image Classification**: Classify images with pre-trained models (ResNet)
- **Text-to-Image**: Generate images from text descriptions (Stable Diffusion XL)
- **Speech Recognition**: Transcribe audio using Whisper
- **Streaming support**: Real-time streaming for text generation
- **Retry logic**: Built-in exponential backoff for rate limiting

## Supported Models

### Text Generation
| Model | Description |
|-------|-------------|
| `@cf/meta/llama-3-8b-instruct` | Meta Llama 3 8B Instruct |
| `@cf/meta/llama-3-8b-instruct-awq` | Meta Llama 3 8B Instruct (AWQ quantized) |
| `@cf/mistral/mistral-7b-instruct-v0.1` | Mistral 7B Instruct v0.1 |
| `@cf/microsoft/phi-2` | Microsoft Phi-2 |
| `@cf/qwen/qwen1.5-7b-chat-awq` | Qwen 1.5 7B Chat AWQ |
| `@cf/tinyllama/tinyllama-1.1b-chat-v1.0` | TinyLlama 1.1B Chat |

### Speech Recognition
| Model | Description |
|-------|-------------|
| `@cf/openai/whisper` | OpenAI Whisper (multilingual) |

### Text-to-Image
| Model | Description |
|-------|-------------|
| `@cf/stabilityai/stable-diffusion-xl-base-1.0` | Stable Diffusion XL Base 1.0 |

## Authentication

This SDK requires a Cloudflare Account ID and API Token. You can create an API token in the Cloudflare dashboard:

1. Go to "My Profile" → "API Tokens"
2. Click "Create Token"
3. Use the "Workers AI" template or create a custom token with `Workers AI:Read` permission

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cloudflare-ai = "1.4.0"
```

## Quick Start

```rust
use cloudflare_ai::{CloudflareAiClient, TextGenerationRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CloudflareAiClient::new(
        "your-account-id",
        "your-api-token",
    )?;

    let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
        .prompt("What is the capital of France?")
        .build();

    let response = client.text().generate(request).await?;
    println!("{}", response.text());
    
    Ok(())
}
```

## Examples

### Chat Completion with Messages

```rust
use cloudflare_ai::{CloudflareAiClient, ChatMessage, Role};

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let messages = vec![
    ChatMessage::system("You are a helpful assistant."),
    ChatMessage::user("What is the capital of France?"),
];

let response = client.text()
    .chat("@cf/meta/llama-3-8b-instruct", messages)
    .await?;
println!("{}", response.text());
```

### Text Generation with Streaming

```rust
use cloudflare_ai::{CloudflareAiClient, TextGenerationRequest};
use futures::StreamExt;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
    .prompt("Tell me a story")
    .build();

let mut stream = client.text().stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => print!("{}", chunk.text()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Embeddings

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

// Single embedding
let embedding = client.embeddings()
    .embed("@cf/baai/bge-base-en-v1.5", "Hello world")
    .await?;

// Multiple embeddings
let texts = vec!["Hello".to_string(), "World".to_string()];
let embeddings = client.embeddings()
    .embed_many("@cf/baai/bge-base-en-v1.5", texts)
    .await?;
```

### Translation

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

// Auto-detect source language
let response = client.translation()
    .auto_translate("Bonjour le monde", "english")
    .await?;

// Specify source language
let response = client.translation()
    .from_to("Hello world", "english", "spanish")
    .await?;
```

### Summarization

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let response = client.summarization()
    .summarize_text("Long text to summarize...")
    .await?;

if let Some(summary) = response.summary() {
    println!("Summary: {}", summary);
}
```

### Text-to-Image

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let response = client.text_to_image()
    .generate_prompt("A beautiful sunset over mountains")
    .await?;

if let Ok(bytes) = response.decode_image() {
    std::fs::write("generated.png", bytes)?;
}
```

### Speech Recognition

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let response = client.speech()
    .transcribe_file("audio.mp3")
    .await?;

if let Some(text) = response.text() {
    println!("Transcription: {}", text);
}

// With language hint
let response = client.speech()
    .transcribe_with_language("audio.mp3", "en")
    .await?;
```

### Image Classification

```rust
use cloudflare_ai::CloudflareAiClient;

let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;

let response = client.image_classification()
    .classify_file("image.jpg")
    .await?;

if let Some(top) = response.top_prediction() {
    println!("Class: {}, Score: {}", top.label, top.score);
}
```

## Client Configuration

```rust
use cloudflare_ai::{CloudflareAiClient, ClientConfig};
use std::time::Duration;

let config = ClientConfig::new("your-account-id", "your-api-token")
    .timeout(Duration::from_secs(60))
    .max_retries(5)
    .retry_delay(Duration::from_millis(500));

let client = CloudflareAiClient::with_config(config)?;
```

## Feature Flags

The crate supports the following feature flags:

- `text` - Text generation API (enabled by default)
- `embeddings` - Embeddings API (enabled by default)
- `translation` - Translation API (enabled by default)
- `summarization` - Summarization API (enabled by default)
- `image` - Image classification and text-to-image APIs (enabled by default)
- `speech` - Speech recognition API (enabled by default)
- `streaming` - Streaming support for text generation (enabled by default)
- `rustls-tls` - Use rustls for TLS (enabled by default)
- `native-tls` - Use native-tls for TLS

To disable default features and enable only specific ones:

```toml
[dependencies]
cloudflare-ai = { version = "1.4.0", default-features = false, features = ["text", "embeddings"] }
```

## Error Handling

The SDK uses a custom error type `CloudflareAiError` that covers various failure scenarios:

- `Authentication` - Invalid API token
- `RateLimit` - Rate limit exceeded (includes retry-after info)
- `ModelNotFound` - Model doesn't exist
- `InvalidRequest` - Bad parameters
- `Http` - Network/HTTP errors
- `Json` - Serialization errors
- And more...

```rust
use cloudflare_ai::CloudflareAiError;

match client.text().generate(request).await {
    Ok(response) => println!("{}", response.text()),
    Err(CloudflareAiError::RateLimit { retry_after, .. }) => {
        eprintln!("Rate limited. Retry after: {:?}", retry_after);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
