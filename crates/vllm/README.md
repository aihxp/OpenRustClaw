# vLLM SDK for Rust

A native Rust SDK for [vLLM](https://github.com/vllm-project/vllm) - a high-throughput and memory-efficient inference engine for LLMs.

## Features

- **Chat Completions**: OpenAI-compatible chat completions with streaming support
- **Completions**: Legacy text completions API
- **Embeddings**: Generate embeddings using embedding models
- **Tokenize/Detokenize**: Convert text to tokens and vice versa
- **Model Listing**: List and inspect available models
- **Health Check**: Check vLLM server health status
- **Metrics**: Prometheus-compatible metrics endpoint

## vLLM-Specific Features

- **PagedAttention**: Efficient memory management for attention key/value cache
- **Continuous Batching**: Maximize GPU utilization with dynamic batching
- **Tensor Parallelism**: Distributed inference across multiple GPUs
- **Pipeline Parallelism**: Support for large model sharding

## Quick Start

```rust
use vllm::{VllmClient, ChatRequest, Role};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = VllmClient::new("http://localhost:8000")?;

let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
    .message(Role::User, "Hello, vLLM!")
    .build();

let response = client.chat().complete(request).await?;
println!("{}", response.content());
# Ok(())
# }
```

## Streaming Example

```rust
use vllm::{VllmClient, ChatRequest, Role};
use futures::StreamExt;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = VllmClient::new("http://localhost:8000")?;

let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
    .message(Role::User, "Tell me a story")
    .build();

let mut stream = client.chat().complete_stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => print!("{}", chunk.content()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
# Ok(())
# }
```

## Tokenization Example

```rust
use vllm::VllmClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = VllmClient::new("http://localhost:8000")?;

// Tokenize text
let tokens = client.tokenize()
    .tokenize("meta-llama/Llama-3-8b-chat-hf", "Hello, world!")
    .await?;
println!("Token count: {}", tokens.len());

// Detokenize
let text = client.tokenize()
    .detokenize("meta-llama/Llama-3-8b-chat-hf", &tokens)
    .await?;
println!("Original text: {}", text);
# Ok(())
# }
```

## Health Check

```rust
use vllm::VllmClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = VllmClient::new("http://localhost:8000")?;

match client.health().await {
    Ok(()) => println!("vLLM server is healthy"),
    Err(e) => println!("Health check failed: {}", e),
}
# Ok(())
# }
```

## Metrics

```rust
use vllm::VllmClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = VllmClient::new("http://localhost:8000")?;

let metrics = client.metrics().get().await?;
println!("{}", metrics);
# Ok(())
# }
```

## License

MIT License - See [LICENSE](../../LICENSE) for details.
