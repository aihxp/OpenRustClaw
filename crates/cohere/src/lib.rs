//! # cohere
//!
//! A native Rust SDK for Cohere's API.
//!
//! ## Features
//!
//! - **Chat API**: Conversational AI with Command R, Command R+, and Command models
//! - **Generate API**: Legacy text generation (Command models)
//! - **Embeddings**: Text embeddings with embed-english-v3 and embed-multilingual-v3
//! - **Rerank**: Semantic reranking of documents
//! - **Classify**: Text classification
//! - **Summarize**: Text summarization
//! - **Tokenize/Detokenize**: Token-level operations
//! - **Streaming support**: Real-time responses with async iterators
//! - **Built-in retry logic**: Exponential backoff with configurable policies
//!
//! ## Quick Start
//!
//! ```no_run
//! use cohere::{CohereClient, ChatRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CohereClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("command-r")
//!     .message("What is the capital of France?")
//!     .build();
//!
//! let response = client.chat().create(request).await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use cohere::{CohereClient, ChatRequest};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CohereClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("command-r")
//!     .message("Tell me a story")
//!     .build();
//!
//! let chat = client.chat();
//! let mut stream = chat.stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         cohere::StreamEvent::TextGeneration { text } => print!("{}", text),
//!         cohere::StreamEvent::StreamEnd { .. } => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/cohere")]
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
#[cfg(feature = "classify")]
pub mod classify;
#[cfg(feature = "embeddings")]
pub mod embeddings;
#[cfg(feature = "generate")]
pub mod generate;
#[cfg(feature = "rerank")]
pub mod rerank;
#[cfg(feature = "summarize")]
pub mod summarize;
#[cfg(feature = "tokenize")]
pub mod tokenize;

pub use client::{ClientConfig, CohereClient};
pub use constants::{
    DEFAULT_API_VERSION, DEFAULT_BASE_URL, EmbeddingModel, Model, endpoints, headers, retry,
};
pub use error::{CohereError, Result};

// Re-export commonly used types
pub use types::{ApiMeta, BilledUnits, Document, FinishReason, Message, MessageRole};

#[cfg(feature = "chat")]
pub use chat::{
    ChatRequest, ChatRequestBuilder, ChatResponse, ChatStreamResponse, Tool, ToolCall, ToolResult,
};

#[cfg(feature = "generate")]
pub use generate::{GenerateRequest, GenerateRequestBuilder, GenerateResponse, Generation};

#[cfg(feature = "embeddings")]
pub use embeddings::{
    EmbedRequest, EmbedRequestBuilder, EmbedResponse, Embedding, EmbeddingType, InputType,
    TruncateMode,
};

#[cfg(feature = "rerank")]
pub use rerank::{RerankRequest, RerankRequestBuilder, RerankResponse, RerankResult};

#[cfg(feature = "classify")]
pub use classify::{
    Classification, ClassifyRequest, ClassifyRequestBuilder, ClassifyResponse, Example,
};

#[cfg(feature = "summarize")]
pub use summarize::{SummarizeRequest, SummarizeRequestBuilder, SummarizeResponse};

#[cfg(feature = "tokenize")]
pub use tokenize::{DetokenizeRequest, DetokenizeResponse, TokenizeRequest, TokenizeResponse};

#[cfg(feature = "streaming")]
pub use streaming::{StreamCollector, StreamEvent, StreamResult};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
