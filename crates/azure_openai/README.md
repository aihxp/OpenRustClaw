# Azure OpenAI SDK for Rust

An enterprise-grade Azure OpenAI Service SDK for Rust with comprehensive support for Azure AD authentication, regional endpoints, and content filtering.

## Features

- **Chat Completions API**: Complete support with streaming and tool calling
- **Completions API**: Legacy completions support
- **Embeddings API**: Generate text embeddings with Azure integration
- **DALL-E Images API**: Generate and edit images
- **Audio API**: Whisper transcription and Text-to-Speech (TTS)
- **Assistants API**: Create and manage assistants, threads, and runs
- **Batch API**: Process multiple requests asynchronously at 50% lower cost
- **Fine-tuning API**: Create and manage fine-tuning jobs
- **Files API**: Upload and manage files for fine-tuning and assistants
- **Azure AD Authentication**: Token-based authentication support
- **Managed Identity**: Azure Managed Identity support for Azure resources
- **Regional Endpoints**: Support for Azure regional deployments
- **Content Filtering**: Azure Content Safety integration

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
azure-openai = { path = "../crates/azure_openai" }
tokio = { version = "1", features = ["full"] }
```

### API Key Authentication

```rust
use azure_openai::{AzureOpenAIClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-deployment-name",
        "your-api-key",
    )?;

    let request = ChatRequest::builder()
        .system("You are a helpful assistant.")
        .user("What is Rust?")
        .temperature(0.7)
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or_default());

    Ok(())
}
```

### Azure AD Authentication

```rust
use azure_openai::{AzureOpenAIClient, AzureConfig, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AzureConfig::azure_ad_token("your-aad-token")
        .resource_name("your-resource-name")
        .deployment_name("your-deployment-name");

    let client = AzureOpenAIClient::with_config(config)?;

    let request = ChatRequest::builder()
        .user("Hello, Azure AD!")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or_default());

    Ok(())
}
```

### Managed Identity Authentication

```rust
use azure_openai::{AzureOpenAIClient, AzureConfig, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For system-assigned managed identity
    let config = AzureConfig::managed_identity()
        .resource_name("your-resource-name")
        .deployment_name("your-deployment-name");

    // Or for user-assigned managed identity
    let config = AzureConfig::managed_identity_with_client_id("your-client-id")
        .resource_name("your-resource-name")
        .deployment_name("your-deployment-name");

    let client = AzureOpenAIClient::with_config(config)?;

    let request = ChatRequest::builder()
        .user("Hello from managed identity!")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or_default());

    Ok(())
}
```

## Examples

### Streaming Chat Completions

```rust
use azure_openai::{AzureOpenAIClient, ChatRequest, Role};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-deployment-name",
        "your-api-key",
    )?;

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

    Ok(())
}
```

### Embeddings

```rust
use azure_openai::{AzureOpenAIClient, EmbeddingRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-embedding-deployment",
        "your-api-key",
    )?;

    // Single embedding
    let embedding = client.embeddings().embed("Hello, world!").await?;
    println!("Dimensions: {}", embedding.len());

    // Multiple embeddings
    let inputs = vec![
        "First text".to_string(),
        "Second text".to_string(),
    ];
    let response = client.embeddings().create_many(inputs).await?;

    Ok(())
}
```

### DALL-E Image Generation

```rust
use azure_openai::{AzureOpenAIClient, ImageRequest, ImageSize, ImageQuality, ImageStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-dalle-deployment",
        "your-api-key",
    )?;

    let request = ImageRequest::new("A cute cat wearing a hat")
        .size(ImageSize::Size1024x1024)
        .quality(ImageQuality::Hd)
        .style(ImageStyle::Vivid)
        .n(1);

    let response = client.images().generate(request).await?;

    for image in response.data {
        if let Some(url) = image.url {
            println!("Image URL: {}", url);
        }
    }

    Ok(())
}
```

### Audio Transcription (Whisper)

```rust
use azure_openai::{AzureOpenAIClient, TranscriptionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-whisper-deployment",
        "your-api-key",
    )?;

    let request = TranscriptionRequest::new("audio.mp3")
        .language("en")
        .temperature(0.5);

    let response = client.audio().transcribe(request).await?;
    println!("Transcription: {}", response.text);

    Ok(())
}
```

### Text-to-Speech

```rust
use azure_openai::{AzureOpenAIClient, TtsRequest, TtsVoice, TtsResponseFormat};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-tts-deployment",
        "your-api-key",
    )?;

    let request = TtsRequest::new("Hello, world!", TtsVoice::Alloy)
        .response_format(TtsResponseFormat::Mp3)
        .speed(1.0);

    let audio_bytes = client.audio().speech(request).await?;
    std::fs::write("output.mp3", audio_bytes)?;

    Ok(())
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | `rustls-tls`, `streaming`, `chat`, `embeddings`, `azure-ad` |
| `rustls-tls` | Use rustls for TLS |
| `native-tls` | Use native TLS |
| `streaming` | Enable streaming support |
| `chat` | Enable chat completions API |
| `completions` | Enable legacy completions API |
| `embeddings` | Enable embeddings API |
| `images` | Enable DALL-E images API |
| `audio` | Enable Whisper/TTS audio API |
| `assistants` | Enable Assistants API |
| `batch` | Enable Batch API |
| `files` | Enable Files API |
| `fine-tuning` | Enable fine-tuning API |
| `azure-ad` | Enable Azure AD authentication |
| `managed-identity` | Enable Managed Identity authentication |
| `content-safety` | Enable Content Safety integration |

## Azure Regions

The SDK supports all Azure regions where Azure OpenAI is available:

- `EastUS`, `EastUS2`, `NorthCentralUS`, `SouthCentralUS`, `WestUS`, `WestUS2`, `WestUS3`
- `NorthEurope`, `WestEurope`, `UKSouth`, `UKWest`, `FranceCentral`, `GermanyWestCentral`
- `EastAsia`, `SoutheastAsia`, `JapanEast`, `JapanWest`, `KoreaCentral`, `AustraliaEast`
- And more...

## Content Safety

The SDK provides integration with Azure Content Safety for analyzing text:

```rust
use azure_openai::{AzureOpenAIClient, content_safety::{TextAnalysisRequest, SafetyThresholds}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AzureOpenAIClient::new(
        "your-resource-name",
        "your-deployment",
        "your-api-key",
    )?;

    let request = TextAnalysisRequest::new("Text to analyze");
    let response = client.content_safety().analyze_text(request).await?;

    let thresholds = SafetyThresholds::moderate();
    let is_safe = ContentSafety::is_content_safe(&response, &thresholds);

    Ok(())
}
```

## Error Handling

The SDK provides detailed error types for handling various failure scenarios:

```rust
use azure_openai::error::AzureOpenAIError;

match result {
    Err(AzureOpenAIError::RateLimit { retry_after, .. }) => {
        println!("Rate limited, retry after: {:?}", retry_after);
    }
    Err(AzureOpenAIError::ContentFiltered { filter_results, .. }) => {
        println!("Content was filtered");
    }
    Err(AzureOpenAIError::Authentication { message }) => {
        println!("Authentication failed: {}", message);
    }
    _ => {}
}
```

## License

MIT
