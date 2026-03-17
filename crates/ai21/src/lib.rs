//! # ai21
//!
//! A native Rust SDK for AI21 Labs API with support for Jamba and Jurassic models.
//!
//! ## Features
//!
//! - **Chat API**: Conversational AI with Jamba models (jamba-1.5-large, jamba-1.5-mini, jamba-instruct)
//! - **Completions API**: Text generation with Jurassic models (j2-ultra, j2-mid, j2-light)
//! - **RAG (Contextual Answers)**: Retrieval-Augmented Generation for question answering
//! - **Tokenization**: Tokenize and detokenize text with AI21 models
//! - **Streaming support**: Real-time responses with async iterators
//! - **Built-in retry logic**: Exponential backoff with configurable policies
//!
//! ## Quick Start
//!
//! ```no_run
//! use ai21::{Ai21Client, ChatRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Ai21Client::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("jamba-1.5-large")
//!     .messages(vec![Message::user("What is the capital of France?")])
//!     .build();
//!
//! let response = client.chat().create(request).await?;
//! println!("{}", response.choices[0].message.content);
//! # Ok(())
//! # }
//! ```
//!
//! ## Jurassic Completions
//!
//! ```no_run
//! use ai21::{Ai21Client, CompletionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Ai21Client::new("your-api-key")?;
//!
//! let request = CompletionRequest::builder("j2-ultra")
//!     .prompt("The capital of France is")
//!     .max_tokens(50)
//!     .build();
//!
//! let response = client.completions().create(request).await?;
//! println!("{}", response.completions[0].data.text);
//! # Ok(())
//! # }
//! ```
//!
//! ## RAG with Contextual Answers
//!
//! ```no_run
//! use ai21::{Ai21Client, ContextualAnswersRequest, Document};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Ai21Client::new("your-api-key")?;
//!
//! let documents = vec![
//!     Document::new("doc1", "The Eiffel Tower is in Paris."),
//!     Document::new("doc2", "Paris is the capital of France."),
//! ];
//!
//! let request = ContextualAnswersRequest::new(
//!     "Where is the Eiffel Tower?",
//!     documents,
//! );
//!
//! let response = client.rag().contextual_answers(request).await?;
//! println!("{}", response.answer);
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use ai21::{Ai21Client, ChatRequest, Message};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Ai21Client::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("jamba-1.5-large")
//!     .messages(vec![Message::user("Tell me a story")])
//!     .build();
//!
//! let chat = client.chat();
//! let mut stream = chat.stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         ai21::StreamEvent::ContentDelta { delta } => print!("{}", delta),
//!         ai21::StreamEvent::End { .. } => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/ai21")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod constants;
mod error;

pub mod types;

#[cfg(feature = "streaming")]
pub mod streaming;

// API endpoint modules
#[cfg(feature = "chat")]
pub mod chat;
#[cfg(feature = "completions")]
pub mod completions;
#[cfg(feature = "rag")]
pub mod rag;
#[cfg(feature = "tokenize")]
pub mod tokenize;

pub use client::{Ai21Client, ClientConfig};
pub use constants::{
    DEFAULT_API_VERSION, DEFAULT_BASE_URL, JambaModel, JurassicModel, Model, endpoints, headers,
    retry,
};
pub use error::{Ai21Error, Result};

// Re-export commonly used types
pub use types::{
    Document, FinishReason, Message, MessageRole, Penalty, ResponseFormat, Tool, ToolCall,
    ToolCallFunction, ToolFunction, ToolResult, Usage,
};

#[cfg(feature = "chat")]
pub use chat::{
    ChatChoice, ChatEndpoint, ChatMessage, ChatRequest, ChatRequestBuilder, ChatResponse,
    ToolChoice,
};

#[cfg(feature = "completions")]
pub use completions::{
    Completion, CompletionData, CompletionRequest, CompletionRequestBuilder, CompletionResponse,
    CompletionsEndpoint, GeneratedToken, PromptInfo, TokenData, TokenInfo, TopToken,
};

#[cfg(feature = "rag")]
pub use rag::{
    ContextualAnswersRequest, ContextualAnswersRequestBuilder, ContextualAnswersResponse,
    RagEndpoint, Segment, SegmentsRequest, SegmentsResponse,
};

#[cfg(feature = "tokenize")]
pub use tokenize::{
    DetokenizeRequest, DetokenizeResponse, Token, TokenizeEndpoint, TokenizeRequest,
    TokenizeResponse,
};

#[cfg(feature = "streaming")]
pub use streaming::{StreamCollector, StreamEvent, StreamResult};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
