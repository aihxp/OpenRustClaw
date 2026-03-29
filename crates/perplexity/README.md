# perplexity

A native Rust SDK for Perplexity AI's API with built-in search and grounding capabilities.

[![crates.io](https://img.shields.io/crates/v/perplexity.svg)](https://crates.io/crates/perplexity)
[![docs.rs](https://docs.rs/perplexity/badge.svg)](https://docs.rs/perplexity)

## Features

- **Chat Completions**: Conversational AI with Sonar models
- **Citations**: Built-in source citations for search-grounded responses
- **Search Recency Filters**: Control how recent search results should be
- **Related Questions**: Get AI-suggested follow-up questions
- **Image Returns**: Request images in responses (online models)
- **Streaming Support**: Real-time responses with async iterators
- **Built-in Retry Logic**: Exponential backoff with configurable policies

## Installation

```toml
[dependencies]
perplexity = "1.4.0"
```

## Quick Start

```rust
use perplexity::{PerplexityClient, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PerplexityClient::new("your-api-key")?;

    let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
        .user("What are the latest developments in Rust?")
        .build();

    let response = client.chat().complete(request).await?;
    println!("{}", response.content().unwrap_or("No content"));

    // Print citations
    for citation in response.get_citations() {
        println!("[{}] {}", citation.index, citation.url);
    }

    Ok(())
}
```

## Available Models

| Model | Search | Context | Best For |
|-------|--------|---------|----------|
| `llama-3.1-sonar-small-128k-online` | ✅ | 128k | Fast, cost-effective search |
| `llama-3.1-sonar-large-128k-online` | ✅ | 128k | Balanced performance |
| `llama-3.1-sonar-huge-128k-online` | ✅ | 128k | Maximum capability |
| `llama-3.1-sonar-small-128k-chat` | ❌ | 128k | Fast chat without search |
| `llama-3.1-sonar-large-128k-chat` | ❌ | 128k | Powerful chat without search |

## Using Search Features

### Search Recency Filter

Control how recent the search results should be:

```rust
use perplexity::{PerplexityClient, ChatRequest, SearchRecencyFilter};

let client = PerplexityClient::new("your-api-key")?;

let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
    .user("What happened in AI this week?")
    .search_recency_filter(SearchRecencyFilter::Week)
    .build();

let response = client.chat().complete(request).await?;
```

Available filters: `Hour`, `Day`, `Week`, `Month`, `Year`

### Related Questions

Get AI-suggested follow-up questions:

```rust
let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
    .user("What is machine learning?")
    .return_related_questions(true)
    .build();

let response = client.chat().complete(request).await?;

for question in response.get_related_questions() {
    println!("Related: {}", question.question);
}
```

### Return Images

Request images to be included in responses:

```rust
let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
    .user("Show me pictures of the Eiffel Tower")
    .return_images(true)
    .build();
```

## Streaming

For real-time responses:

```rust
use perplexity::{PerplexityClient, ChatRequest};
use futures::StreamExt;

let client = PerplexityClient::new("your-api-key")?;

let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
    .user("Tell me about Rust's ownership system")
    .build();

let mut stream = client.chat().complete_stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk? {
        chunk if chunk.is_final() => {
            println!();
            // Citations appear in the final chunk
            for citation in chunk.citations() {
                println!("[{}] {}", citation.index, citation.url);
            }
            break;
        }
        chunk => {
            if let Some(content) = chunk.delta_content() {
                print!("{}", content);
            }
        }
    }
}
```

## Advanced Usage

### System Messages

```rust
let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
    .system("You are a helpful coding assistant.")
    .user("How do I use async/await in Rust?")
    .temperature(0.7)
    .max_tokens(2048)
    .build();
```

### JSON Mode

```rust
let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
    .user("List 3 Rust web frameworks as JSON")
    .json_mode()
    .build();
```

### Custom Configuration

```rust
use std::time::Duration;
use perplexity::{PerplexityClient, ClientConfig};

let config = ClientConfig::new("your-api-key")
    .timeout(Duration::from_secs(60))
    .max_retries(5);

let client = PerplexityClient::with_config(config)?;
```

## Error Handling

The SDK provides detailed error types:

```rust
use perplexity::{PerplexityClient, ChatRequest, PerplexityError};

match client.chat().complete(request).await {
    Ok(response) => println!("{}", response.content().unwrap_or("")),
    Err(PerplexityError::RateLimit { retry_after, .. }) => {
        println!("Rate limited. Retry after: {:?}", retry_after);
    }
    Err(PerplexityError::Authentication { message }) => {
        println!("Auth error: {}", message);
    }
    Err(e) => println!("Error: {}", e),
}
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
