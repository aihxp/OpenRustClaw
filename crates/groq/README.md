# Groq SDK for Rust

A native Rust SDK for Groq API - Ultra-fast LLM inference.

## Features

- **Chat Completions API**: Complete support with streaming and tool calling
- **Audio Transcription (Whisper)**: Convert audio to text
- **Audio Translations**: Translate audio to English text
- **All Groq Models**: Support for all Groq-hosted models:
  - `llama3-8b-8192` - Fast, efficient Llama 3 8B
  - `llama3-70b-8192` - Powerful Llama 3 70B
  - `llama-3.1-8b-instant` - Instant responses with Llama 3.1 8B (131K context)
  - `llama-3.1-70b-versatile` - Versatile Llama 3.1 70B (131K context)
  - `llama-3.1-405b-reasoning` - Advanced reasoning with Llama 3.1 405B
  - `mixtral-8x7b-32768` - Mixtral 8x7B with 32K context
  - `gemma-7b-it` - Google's Gemma 7B Instruct
  - `gemma2-9b-it` - Google's Gemma 2 9B Instruct
- **Streaming support**: Real-time streaming for chat completions
- **Tool calling**: Support for function calling with parallel tool calls
- **JSON mode**: Structured JSON outputs

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
groq = { path = "../crates/groq" }
```

## Quick Start

```rust
use groq::{GroqClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let request = ChatRequest::builder("llama3-8b-8192")
        .message(Role::User, "Hello, Groq!")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or("No content"));
    
    Ok(())
}
```

## Streaming Example

```rust
use groq::{GroqClient, ChatRequest, Role};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let request = ChatRequest::builder("llama3-8b-8192")
        .message(Role::User, "Tell me a story")
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

## Tool Calling Example

```rust
use groq::{GroqClient, ChatRequest, Function, Role};
use groq::chat::ToolChoice;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let weather_function = Function::builder("get_weather", "Get the current weather")
        .string_property("location", "City name", true)
        .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
        .build();

    let request = ChatRequest::builder("llama3-70b-8192")
        .message(Role::User, "What's the weather in Paris?")
        .tool(weather_function)
        .tool_choice(ToolChoice::auto())
        .build();

    let response = client.chat().complete(request).await?;
    if let Some(tool_calls) = response.tool_calls() {
        for call in tool_calls {
            println!("Function: {}", call.function.name);
            println!("Arguments: {}", call.function.arguments);
        }
    }
    
    Ok(())
}
```

## Audio Transcription Example

```rust
use groq::{GroqClient, AudioTranscriptionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let request = AudioTranscriptionRequest::builder()
        .file_path("/path/to/audio.mp3").await?
        .model("whisper-large-v3")
        .language("en")
        .build()?;

    let response = client.audio().transcribe(request).await?;
    println!("Transcription: {}", response.text);
    
    Ok(())
}
```

## Audio Translation Example

```rust
use groq::{GroqClient, AudioTranslationRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let request = AudioTranslationRequest::builder()
        .file_path("/path/to/audio.mp3").await?
        .model("whisper-large-v3")
        .build()?;

    let response = client.audio().translate(request).await?;
    println!("Translation: {}", response.text);
    
    Ok(())
}
```

## JSON Mode

```rust
use groq::{GroqClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GroqClient::new("your-api-key")?;

    let request = ChatRequest::builder("llama3-8b-8192")
        .system("You are a helpful assistant that outputs JSON.")
        .user("List 3 colors")
        .json_mode()
        .build();

    let response = client.chat().complete(request).await?;
    if let Some(content) = response.content() {
        let json: serde_json::Value = serde_json::from_str(content)?;
        println!("{}", serde_json::to_string_pretty(&json)?);
    }
    
    Ok(())
}
```

## Configuration

```rust
use groq::{GroqClient, ClientConfig};
use std::time::Duration;

let config = ClientConfig::new("your-api-key")
    .base_url("https://api.groq.com/openai/v1")
    .timeout(Duration::from_secs(60))
    .max_retries(5);

let client = GroqClient::with_config(config)?;
```

## License

MIT
