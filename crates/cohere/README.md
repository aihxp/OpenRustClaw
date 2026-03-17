# Cohere Rust SDK

A native Rust SDK for Cohere's API with full support for chat, embeddings, rerank, classify, summarize, tokenize, and more.

## Features

- **Chat API**: Conversational AI with Command R, Command R+, Command, and other models
- **Generate API**: Legacy text generation with Command models
- **Embeddings**: Text embeddings with embed-english-v3 and embed-multilingual-v3
- **Rerank**: Semantic reranking of documents for improved search
- **Classify**: Text classification with few-shot and zero-shot learning
- **Summarize**: Text summarization with configurable length and format
- **Tokenize/Detokenize**: Token-level operations
- **Streaming support**: Real-time responses with async iterators
- **Built-in retry logic**: Exponential backoff with configurable policies
- **Type-safe**: Full API coverage with Rust type safety

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cohere = { path = "../crates/cohere" }
```

## Quick Start

```rust
use cohere::{CohereClient, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let request = ChatRequest::builder("command-r")
        .message("What is the capital of France?")
        .build();

    let response = client.chat().create(request).await?;
    println!("{}", response.text());

    Ok(())
}
```

## Examples

### Chat with Streaming

```rust
use cohere::{CohereClient, ChatRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let request = ChatRequest::builder("command-r")
        .message("Tell me a story about Rust")
        .build();

    let mut stream = client.chat().stream(request).await?;
    while let Some(chunk) = stream.next().await {
        match chunk? {
            cohere::StreamEvent::TextGeneration { text } => print!("{}", text),
            cohere::StreamEvent::StreamEnd { .. } => break,
            _ => {}
        }
    }

    Ok(())
}
```

### Chat with Documents (RAG)

```rust
use cohere::{CohereClient, ChatRequest, Document};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let documents = vec![
        Document::new("doc1", "Rust is a systems programming language.")
            .with_title("About Rust"),
        Document::new("doc2", "Cargo is Rust's package manager.")
            .with_title("About Cargo"),
    ];

    let request = ChatRequest::builder("command-r")
        .message("What is Rust?")
        .documents(documents)
        .build();

    let response = client.chat().create(request).await?;
    println!("{}", response.text());

    Ok(())
}
```

### Embeddings

```rust
use cohere::{CohereClient, EmbedRequest, InputType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let request = EmbedRequest::builder()
        .model("embed-english-v3.0")
        .add_text("Hello world")
        .add_text("Goodbye world")
        .input_type(InputType::SearchDocument)
        .build();

    let response = client.embed().create(request).await?;
    
    for (i, embedding) in response.get_embeddings().iter().enumerate() {
        println!("Embedding {}: {:?}...", i, &embedding[..5]);
    }

    Ok(())
}
```

### Rerank

```rust
use cohere::CohereClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let documents = vec![
        "Carson City is the capital city of the American state of Nevada.".to_string(),
        "The Commonwealth of the Northern Mariana Islands is a group of islands in the Pacific Ocean. Its capital is Saipan.".to_string(),
        "Capitalization or capitalisation in English grammar is the use of a capital letter at the start of a word.".to_string(),
    ];

    let request = cohere::RerankRequest::builder()
        .model("rerank-english-v3.0")
        .query("What is the capital of the United States?")
        .documents(documents)
        .top_n(3)
        .build();

    let response = client.rerank().create(request).await?;
    
    for result in response.results {
        println!(
            "Index: {}, Score: {:.4}",
            result.index, result.relevance_score
        );
    }

    Ok(())
}
```

### Classify

```rust
use cohere::{CohereClient, ClassifyRequest, Example};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let examples = vec![
        Example::new("I love this product!", "positive"),
        Example::new("This is amazing!", "positive"),
        Example::new("I hate this product.", "negative"),
        Example::new("This is terrible.", "negative"),
    ];

    let request = ClassifyRequest::few_shot(
        vec!["This product is okay.".to_string()],
        examples,
    );

    let response = client.classify().create(request).await?;
    
    for classification in response.classifications {
        println!(
            "Input: {}, Label: {}, Confidence: {:.2}",
            classification.input,
            classification.label(),
            classification.confidence()
        );
    }

    Ok(())
}
```

### Summarize

```rust
use cohere::{CohereClient, SummarizeRequest, SummaryLength, SummaryFormat};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    let long_text = "Your long text here that needs summarization...";

    let request = SummarizeRequest::builder()
        .text(long_text)
        .length(SummaryLength::Medium)
        .format(SummaryFormat::Paragraph)
        .build();

    let response = client.summarize().create(request).await?;
    println!("Summary: {}", response.text());

    Ok(())
}
```

### Tokenize

```rust
use cohere::CohereClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CohereClient::new("your-api-key")?;

    // Tokenize
    let tokens = client.tokenize().encode("command-r", "Hello world").await?;
    println!("Tokens: {:?}", tokens);

    // Detokenize
    let text = client.tokenize().decode("command-r", tokens).await?;
    println!("Text: {}", text);

    Ok(())
}
```

## Models

### Chat Models

| Model | Description | Context | Max Output |
|-------|-------------|---------|------------|
| `command-r` | Fast, efficient conversational model | 128K | 4,096 |
| `command-r-plus` | Advanced conversational model | 128K | 4,096 |
| `command` | General purpose generation | 4,096 | 4,096 |
| `command-nightly` | Latest experimental version | 8,192 | 4,096 |
| `c4ai-aya-23` | Multilingual model | 8,192 | 4,096 |

### Embedding Models

| Model | Dimensions | Description |
|-------|------------|-------------|
| `embed-english-v3.0` | 1,024 | English embedding model |
| `embed-multilingual-v3.0` | 1,024 | Multilingual embedding model |
| `embed-english-light-v3.0` | 384 | Light English embedding model |
| `embed-multilingual-light-v3.0` | 384 | Light multilingual embedding model |

## Features

- `chat` (default): Chat API support
- `generate`: Generate API (legacy) support
- `embeddings` (default): Embeddings API support
- `rerank`: Rerank API support
- `classify`: Classify API support
- `summarize`: Summarize API support
- `tokenize`: Tokenize/Detokenize API support
- `streaming` (default): Streaming support for chat
- `full`: Enable all features

## License

MIT
