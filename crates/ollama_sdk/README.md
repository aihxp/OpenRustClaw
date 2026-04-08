# Ollama SDK for Rust

A native Rust SDK for the Ollama API - Local LLM inference with comprehensive API support.

## Features

- **Chat Completions API**: Full support for conversational AI with streaming
- **Generate API**: Text completion and generation endpoint
- **Model Management**: List, pull, push, create, delete, and copy models
- **Embeddings API**: Generate embeddings for text
- **Multi-modal Support**: Vision capabilities with image inputs (llava models)
- **JSON Mode**: Structured JSON output support
- **Streaming Support**: Real-time streaming for chat, generate, pull, push, and create endpoints
- **Keep-alive Configuration**: Control model loading/unloading behavior
- **Custom Headers**: Support for additional HTTP headers
- **Tool Calling**: Function calling support in chat

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ollama-sdk = "1.4.2"
```

## Quick Start

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "Hello, Ollama!")
        .build();

    let response = client.chat().generate(request).await?;
    println!("{}", response.message.content.unwrap_or_default());
    
    Ok(())
}
```

## Streaming

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role};
use futures::StreamExt;

async fn streaming_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "Tell me a story")
        .build();

    let mut stream = client.chat().stream(request).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => print!("{}", chunk.message.content.unwrap_or_default()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Vision (Multi-modal)

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role, ImageInput};

async fn vision_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let image = ImageInput::from_path("./image.jpg").await?;

    let request = ChatRequest::builder("llava")
        .message_with_images(Role::User, "What's in this image?", vec![image])
        .build();

    let response = client.chat().generate(request).await?;
    println!("{}", response.message.content.unwrap_or_default());
    
    Ok(())
}
```

## JSON Mode

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role, FormatType};

async fn json_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let request = ChatRequest::builder("llama3.2")
        .system("You are a helpful assistant that always responds with valid JSON.")
        .message(Role::User, r#"Generate a JSON object with "name" and "age" fields"#)
        .format(FormatType::Json)
        .build();

    let response = client.chat().generate(request).await?;
    let json: serde_json::Value = serde_json::from_str(&response.message.content.unwrap_or_default())?;
    println!("Name: {}, Age: {}", json["name"], json["age"]);
    
    Ok(())
}
```

## Tool Calling

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role, Function, Tool};

async fn tool_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let weather_func = Function::builder("get_weather", "Get the current weather")
        .string_property("location", "The city name", true)
        .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
        .build();

    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "What's the weather in Paris?")
        .tool(Tool::function(weather_func))
        .build();

    let response = client.chat().generate(request).await?;

    if response.has_tool_calls() {
        for tool_call in response.tool_calls().unwrap() {
            println!("Function: {}", tool_call.function.name);
            println!("Arguments: {}", tool_call.function.arguments);
        }
    }
    
    Ok(())
}
```

## Model Management

```rust
use ollama_sdk::OllamaClient;
use futures::StreamExt;

async fn models_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    // List local models
    let models = client.models().list().await?;
    for model in models {
        println!("Model: {} (Size: {} MB)", model.name, model.size / 1_048_576);
    }

    // Pull a model with progress
    let mut stream = client.models().pull("llama3.2").await?;
    while let Some(status) = stream.next().await {
        match status {
            Ok(status) => {
                if let Some(progress) = status.progress() {
                    println!("Progress: {:.1}%", progress * 100.0);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    // Show model info
    let info = client.models().show("llama3.2").await?;
    println!("Modelfile: {:?}", info.modelfile);

    // Copy a model
    client.models().copy("llama3.2", "my-llama3.2").await?;

    // Delete a model
    client.models().delete("my-llama3.2").await?;

    // List running models
    let running = client.models().running().await?;
    for model in running {
        println!("Running: {} (VRAM: {} MB)", model.name, model.size_vram / 1_048_576);
    }

    // Get Ollama version
    let version = client.models().version().await?;
    println!("Ollama version: {}", version);
    
    Ok(())
}
```

## Create Custom Models

```rust
use ollama_sdk::OllamaClient;
use futures::StreamExt;

async fn create_model_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let modelfile = r#"
FROM llama3.2
SYSTEM You are a helpful assistant specialized in Rust programming.
PARAMETER temperature 0.7
"#;

    let mut stream = client.models().create("rust-assistant", modelfile).await?;
    while let Some(status) = stream.next().await {
        match status {
            Ok(status) => println!("Status: {}", status.status),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Embeddings

```rust
use ollama_sdk::OllamaClient;

async fn embeddings_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let response = client.embeddings()
        .generate("nomic-embed-text", "Hello, world!")
        .await?;

    println!("Embedding dimensions: {}", response.embedding.len());
    
    Ok(())
}
```

## Configuration

```rust
use ollama_sdk::{OllamaClient, ClientConfig, KeepAlive};
use std::time::Duration;

let config = ClientConfig::new("http://my-ollama-server:11434")
    .timeout(Duration::from_secs(120))
    .max_retries(5)
    .header("X-Custom-Header", "value")
    .unwrap();

let client = OllamaClient::with_config(config);
```

## Keep-alive Configuration

Control how long models stay loaded in memory:

```rust
use ollama_sdk::{OllamaClient, ChatRequest, Role, KeepAlive};

async fn keep_alive_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    // Keep model loaded for 5 minutes
    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "Hello!")
        .keep_alive(KeepAlive::duration("5m"))
        .build();

    // Keep model loaded indefinitely
    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "Hello!")
        .keep_alive(KeepAlive::indefinitely())
        .build();

    // Unload immediately after request
    let request = ChatRequest::builder("llama3.2")
        .message(Role::User, "Hello!")
        .keep_alive(KeepAlive::unload_immediately())
        .build();
    
    Ok(())
}
```

## Generate API (Legacy)

For single-turn completions without conversation history:

```rust
use ollama_sdk::{OllamaClient, GenerateRequest};

async fn generate_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    let request = GenerateRequest::builder("llama3.2")
        .prompt("Why is the sky blue?")
        .temperature(0.7)
        .max_tokens(1024)
        .build();

    let response = client.generate().text(request).await?;
    println!("{}", response.response);
    
    Ok(())
}
```

## Features

- `default`: Enables `rustls-tls`, `streaming`, `chat`, `generate`, `models`, `embeddings`
- `rustls-tls`: Use rustls for TLS (default)
- `native-tls`: Use native TLS
- `streaming`: Enable streaming support for all endpoints
- `chat`: Enable chat API
- `generate`: Enable generate API
- `models`: Enable model management API
- `embeddings`: Enable embeddings API
- `vision`: Enable multi-modal/vision support
- `json-mode`: Enable JSON mode support

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.
