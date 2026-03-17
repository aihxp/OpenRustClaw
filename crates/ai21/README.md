# AI21 Labs SDK

Native Rust SDK for AI21 Labs API with support for Jamba and Jurassic models.

## Features

- **Chat API**: Conversational AI with Jamba models (jamba-1.5-large, jamba-1.5-mini, jamba-instruct)
- **Completions API**: Text generation with Jurassic models (j2-ultra, j2-mid, j2-light)
- **RAG (Contextual Answers)**: Retrieval-Augmented Generation for question answering
- **Tokenization**: Tokenize and detokenize text with AI21 models
- **Streaming support**: Real-time responses with async iterators
- **Built-in retry logic**: Exponential backoff with configurable policies

## Quick Start

```rust
use ai21::{Ai21Client, ChatRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = Ai21Client::new("your-api-key")?;

    // Chat with Jamba
    let request = ChatRequest::builder("jamba-1.5-large")
        .messages(vec![Message::user("What is the capital of France?")])
        .build();

    let response = client.chat().create(request).await?;
    println!("{}", response.choices[0].message.content);

    Ok(())
}
```

## Jurassic Completions

```rust
use ai21::{Ai21Client, CompletionRequest};

let client = Ai21Client::new("your-api-key")?;

let request = CompletionRequest::builder("j2-ultra")
    .prompt("The capital of France is")
    .max_tokens(50)
    .build();

let response = client.completions().create(request).await?;
println!("{}", response.completions[0].data.text);
```

## RAG with Contextual Answers

```rust
use ai21::{Ai21Client, ContextualAnswersRequest, Document};

let client = Ai21Client::new("your-api-key")?;

let documents = vec![
    Document::new("doc1", "The Eiffel Tower is in Paris."),
    Document::new("doc2", "Paris is the capital of France."),
];

let request = ContextualAnswersRequest::new(
    "Where is the Eiffel Tower?",
    documents,
);

let response = client.rag().contextual_answers(request).await?;
println!("{}", response.answer);
```

## Streaming Example

```rust
use ai21::{Ai21Client, ChatRequest};
use futures::StreamExt;

let client = Ai21Client::new("your-api-key")?;

let request = ChatRequest::builder("jamba-1.5-large")
    .messages(vec![Message::user("Tell me a story")])
    .build();

let mut stream = client.chat().stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk? {
        ai21::StreamEvent::ContentDelta { delta } => print!("{}", delta),
        ai21::StreamEvent::End { .. } => break,
        _ => {}
    }
}
```

## Models

### Jamba Models
- `jamba-1.5-large` - Most capable Jamba model
- `jamba-1.5-mini` - Faster, cost-effective Jamba model
- `jamba-instruct` - Instruction-tuned Jamba model

### Jurassic Models
- `j2-ultra` - Most capable Jurassic-2 model
- `j2-mid` - Balanced performance and cost
- `j2-light` - Fastest, most cost-effective

## License

MIT
