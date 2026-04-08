# DeepSeek SDK for Rust

A native Rust SDK for the DeepSeek API - Cost-effective LLM inference with reasoning support.

## Features

- **Chat Completions API**: Complete support with streaming and tool calling
- **Reasoning Mode (DeepSeek-R1)**: Access chain-of-thought reasoning content
- **JSON Mode**: Structured JSON output support
- **All DeepSeek Models**:
  - `deepseek-chat` (DeepSeek-V3) - General purpose chat model
  - `deepseek-reasoner` (DeepSeek-R1) - Reasoning model with chain-of-thought
  - `deepseek-coder` - Code generation model
- **Streaming support**: Real-time streaming for chat completions
- **Tool calling**: Support for function calling

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
deepseek = "1.4.2"
```

## Quick Start

```rust
use deepseek::{DeepSeekClient, ChatRequest, Role};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let client = DeepSeekClient::new("your-api-key")?;

    let request = ChatRequest::builder("deepseek-chat")
        .message(Role::User, "Hello, DeepSeek!")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or("No content"));
    
    Ok(())
}
```

## Reasoning Mode (DeepSeek-R1)

DeepSeek-R1 provides chain-of-thought reasoning in a separate field:

```rust
use deepseek::{DeepSeekClient, ChatRequest, Role};

async fn reasoning_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = DeepSeekClient::new("your-api-key")?;

    let request = ChatRequest::builder("deepseek-reasoner")
        .message(Role::User, "Solve step by step: What is 15 * 27?")
        .build();

    let response = client.chat().complete(request).await?;

    // Access the reasoning content (chain-of-thought)
    if let Some(reasoning) = response.reasoning_content() {
        println!("Reasoning: {}", reasoning);
    }

    // Access the final answer
    if let Some(content) = response.content() {
        println!("Answer: {}", content);
    }
    
    Ok(())
}
```

## Streaming

```rust
use deepseek::{DeepSeekClient, ChatRequest, Role};
use futures::StreamExt;

async fn streaming_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = DeepSeekClient::new("your-api-key")?;

    let request = ChatRequest::builder("deepseek-chat")
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

## JSON Mode

```rust
use deepseek::{DeepSeekClient, ChatRequest, Role};

async fn json_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = DeepSeekClient::new("your-api-key")?;

    let request = ChatRequest::builder("deepseek-chat")
        .system("You are a helpful assistant that always responds with valid JSON.")
        .message(Role::User, r#"Generate a JSON object with "name" and "age" fields"#)
        .json_mode()
        .build();

    let response = client.chat().complete(request).await?;
    if let Some(content) = response.content() {
        let json: serde_json::Value = serde_json::from_str(content)?;
        println!("Name: {}, Age: {}", json["name"], json["age"]);
    }
    
    Ok(())
}
```

## Tool Calling

```rust
use deepseek::{DeepSeekClient, ChatRequest, Role, Function, ToolChoice};

async fn tool_example() -> Result<(), Box<dyn std::error::Error>> {
    let client = DeepSeekClient::new("your-api-key")?;

    let weather_func = Function::builder("get_weather", "Get the current weather")
        .string_property("location", "The city and state", true)
        .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
        .build();

    let request = ChatRequest::builder("deepseek-chat")
        .message(Role::User, "What's the weather in Paris?")
        .tool(weather_func)
        .tool_choice(ToolChoice::auto())
        .build();

    let response = client.chat().complete(request).await?;

    if response.has_tool_calls() {
        for tool_call in response.tool_calls().unwrap() {
            println!("Function: {}", tool_call.function.name);
            println!("Arguments: {}", tool_call.function.arguments);
        }
    }
    
    Ok(())
}
```

## Configuration

```rust
use deepseek::{DeepSeekClient, ClientConfig};
use std::time::Duration;

let config = ClientConfig::new("your-api-key")
    .base_url("https://custom.api.endpoint")
    .timeout(Duration::from_secs(60))
    .max_retries(5);

let client = DeepSeekClient::with_config(config)?;
```

## Examples

See the `examples/` directory for more usage examples:

- `chat_completion` - Basic chat completion
- `streaming_chat` - Streaming responses
- `reasoning` - DeepSeek-R1 reasoning mode
- `json_mode` - Structured JSON output

Run examples with:

```bash
export DEEPSEEK_API_KEY=your-api-key
cargo run --example chat_completion
cargo run --example streaming_chat
cargo run --example reasoning
cargo run --example json_mode
```

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.
