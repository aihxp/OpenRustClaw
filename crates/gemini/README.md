# Google Gemini SDK for Rust

Complete async SDK for Google Gemini API with support for:
- Text generation
- Multi-turn conversations
- Vision (image understanding)
- Function calling
- Embeddings
- Streaming responses

## Quick Start

```rust
use google_gemini::{GeminiClient, GeminiModel, Content, GenerateContentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GeminiClient::new(std::env::var("GEMINI_API_KEY")?)
        .with_model(GeminiModel::Gemini15Pro);
    
    // Simple text generation
    let request = GenerateContentRequest {
        contents: vec![Content::user("Hello, Gemini!")],
        system_instruction: None,
        generation_config: None,
        tools: None,
        tool_config: None,
        safety_settings: None,
    };
    
    let response = client.generate_content(request).await?;
    println!("{:?}", response);
    
    Ok(())
}
```

## Features

### Chat Sessions

```rust
let mut chat = client.chat()
    .with_system_instruction("You are a helpful assistant.");

let response = chat.send_message("What is Rust?").await?;
```

### Vision

```rust
use google_gemini::vision::VisionExt;

let image_data = std::fs::read("image.png")?;
let description = client.vision().describe("image/png", image_data).await?;
```

### Embeddings

```rust
use google_gemini::embedding::{EmbeddingExt, cosine_similarity};

let embedding = client.embed_text_simple("Hello").await?;
```

## Supported Models

- `GeminiModel::Gemini15Pro` - Gemini 1.5 Pro
- `GeminiModel::Gemini15Flash` - Gemini 1.5 Flash
- `GeminiModel::Gemini10Pro` - Gemini 1.0 Pro
- `GeminiModel::Gemini10ProVision` - Gemini 1.0 Pro Vision
- `GeminiModel::Gemini10Ultra` - Gemini 1.0 Ultra
- `GeminiModel::Embedding004` - Embedding model

## License

MIT
