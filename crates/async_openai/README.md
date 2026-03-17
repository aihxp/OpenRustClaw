# async-openai

A native Rust SDK for OpenAI's API (Chat Completions, Assistants, Batch, Files, Fine-tuning, Embeddings).

## Features

- ✅ **Chat Completions API**: Streaming and non-streaming completions
- ✅ **Assistants API**: Create and manage assistants, threads, and runs
- ✅ **Batch API**: Process multiple requests asynchronously
- ✅ **Files API**: Upload and manage files
- ✅ **Fine-tuning API**: Create and manage fine-tuning jobs
- ✅ **Embeddings API**: Generate text embeddings
- ✅ **Tool calling**: Function calling support
- ✅ **Built-in retry logic**: Exponential backoff

## Quick Start

```rust
use async_openai::{OpenAIClient, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::new(std::env::var("OPENAI_API_KEY")?)?;

    let request = ChatRequest::builder("gpt-4o")
        .user("What is the capital of France?")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or("No response"));

    Ok(())
}
```

## Streaming Example

```rust
use futures::StreamExt;

let request = ChatRequest::builder("gpt-4o")
    .user("Tell me a story")
    .stream(true)
    .build();

let mut stream = client.chat().stream(request).await?;
while let Some(chunk) = stream.next().await {
    if let Some(content) = chunk?.content() {
        print!("{}", content);
    }
}
```

## Embeddings

```rust
let embeddings = client.embeddings()
    .embed("text-embedding-3-small", "Hello world")
    .await?;
```

## Features

- `streaming` - Enable streaming support (enabled by default)
- `chat` - Chat completions API
- `assistants` - Assistants API
- `batch` - Batch API
- `files` - Files API
- `embeddings` - Embeddings API
- `fine-tuning` - Fine-tuning API

## License

MIT
