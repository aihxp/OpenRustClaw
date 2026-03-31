# anthropic-rust

A native Rust SDK for Anthropic's Messages API with full support for streaming, tools, vision, and batch processing.

## Features

- ✅ **Complete Messages API support**: All endpoints and features
- ✅ **Type-safe request/response types**: Full Rust type safety
- ✅ **Streaming support**: Real-time response streaming with async iterators
- ✅ **Tool calling**: Full support for function/tool calling
- ✅ **Vision**: Support for image inputs (with `vision` feature)
- ✅ **Batch API**: Submit multiple requests for async processing at 50% cost savings
- ✅ **Built-in retry logic**: Exponential backoff with configurable policies
- ✅ **Rate limit handling**: Automatic retry-after handling
- ✅ **Prompt caching**: Support for Anthropic's beta caching features

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
anthropic-rust = "1.4.1"
tokio = { version = "1", features = ["full"] }
```

### Simple Completion

```rust
use anthropic_rust::{AnthropicClient, MessageRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AnthropicClient::new(std::env::var("ANTHROPIC_API_KEY")?)?;

    let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
        .user("What is the capital of France?")
        .max_tokens(1024)
        .build();

    let response = client.messages().create(request).await?;
    println!("{}", response.text());

    Ok(())
}
```

### Streaming

```rust
use futures::StreamExt;

let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
    .user("Tell me a story")
    .stream(true)
    .build();

let mut stream = client.messages().stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk? {
        anthropic_rust::StreamEvent::ContentBlockDelta { delta, .. } => {
            print!("{}", delta.text());
        }
        anthropic_rust::StreamEvent::MessageStop => break,
        _ => {}
    }
}
```

### Tool Calling

```rust
use anthropic_rust::{Tool, ToolResult, ContentBlock};

let tool = Tool::builder("get_weather", "Get weather for a location")
    .string_property("location", "City name", true)
    .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
    .build();

let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
    .user("What's the weather in Tokyo?")
    .tool(tool)
    .build();

let response = client.messages().create(request).await?;

if response.has_tool_use() {
    for tool_use in response.tool_uses() {
        // Execute the tool...
        let result = execute_tool(tool_use);
        
        // Continue the conversation with the result
        let follow_up = MessageRequest::builder("claude-3-5-sonnet-20241022")
            .messages(vec![
                Message::user("What's the weather in Tokyo?"),
                Message::assistant(response.text()),
                Message::with_content(Role::User, vec![ContentBlock::ToolResult(result)]),
            ])
            .build();
    }
}
```

### Batch API (50% cheaper)

```rust
use anthropic_rust::batch::{BatchClient, BatchRequest};

let batch_client = client.batch();

let requests = vec![
    BatchRequest::from_request(MessageRequest::simple("model", "Question 1")),
    BatchRequest::from_request(MessageRequest::simple("model", "Question 2")),
];

let batch = batch_client.create(requests).await?;

// Poll for completion
let completed = batch_client
    .poll_until_complete(&batch.id, Duration::from_secs(10), Duration::from_secs(3600))
    .await?;

// Get results
let results = batch_client.results(&batch.id).await?;
```

## Configuration

```rust
use anthropic_rust::ClientConfig;
use std::time::Duration;

let config = ClientConfig::new("your-api-key")
    .base_url("https://custom.api.endpoint")
    .api_version("2023-06-01")
    .timeout(Duration::from_secs(120))
    .max_retries(5)
    .retry_delay(Duration::from_millis(1000));

let client = AnthropicClient::with_config(config)?;
```

## Features

- `streaming` - Enable streaming support (enabled by default)
- `batch` - Enable batch API support
- `vision` - Enable vision/image input support

## License

MIT
