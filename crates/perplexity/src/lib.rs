//! # perplexity
//!
//! A native Rust SDK for Perplexity AI's API with built-in search and grounding capabilities.
//!
//! ## Features
//!
//! - **Chat Completions**: Conversational AI with Sonar models
//! - **Citations**: Built-in source citations for search-grounded responses
//! - **Search Recency Filters**: Control how recent search results should be
//! - **Related Questions**: Get AI-suggested follow-up questions
//! - **Image Returns**: Request images in responses (online models)
//! - **Streaming Support**: Real-time responses with async iterators
//! - **Built-in Retry Logic**: Exponential backoff with configurable policies
//!
//! ## Models
//!
//! Perplexity offers Sonar models with different capabilities:
//!
//! | Model | Search | Context | Best For |
//! |-------|--------|---------|----------|
//! | `llama-3.1-sonar-small-128k-online` | ✅ | 128k | Fast, cost-effective search |
//! | `llama-3.1-sonar-large-128k-online` | ✅ | 128k | Balanced performance |
//! | `llama-3.1-sonar-huge-128k-online` | ✅ | 128k | Maximum capability |
//! | `llama-3.1-sonar-small-128k-chat` | ❌ | 128k | Fast chat without search |
//! | `llama-3.1-sonar-large-128k-chat` | ❌ | 128k | Powerful chat without search |
//!
//! ## Quick Start
//!
//! ```no_run
//! use perplexity::{PerplexityClient, ChatRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = PerplexityClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
//!     .user("What are the latest developments in Rust?")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or("No content"));
//!
//! // Print citations
//! for citation in response.get_citations() {
//!     println!("[{}] {}", citation.index, citation.url);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Search with Recency Filter
//!
//! ```no_run
//! use perplexity::{PerplexityClient, ChatRequest, SearchRecencyFilter};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = PerplexityClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
//!     .user("What happened in AI this week?")
//!     .search_recency_filter(SearchRecencyFilter::Week)
//!     .return_related_questions(true)
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or("No content"));
//!
//! // Print related questions
//! for question in response.get_related_questions() {
//!     println!("Related: {}", question.question);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use perplexity::{PerplexityClient, ChatRequest};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = PerplexityClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
//!     .user("Tell me about Rust's ownership system")
//!     .build();
//!
//! let mut stream = client.chat().complete_stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         chunk if chunk.is_final() => {
//!             println!(); // End of stream
//!             // Print citations from final chunk
//!             for citation in chunk.citations() {
//!                 println!("[{}] {}", citation.index, citation.url);
//!             }
//!             break;
//!         }
//!         chunk => {
//!             if let Some(content) = chunk.delta_content() {
//!                 print!("{}", content);
//!             }
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/perplexity")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod constants;
mod error;
mod types;

#[cfg(feature = "streaming")]
pub mod streaming;

// API endpoint modules
#[cfg(feature = "chat")]
pub mod chat;

pub use client::{ClientConfig, PerplexityClient};
pub use constants::{
    endpoints, headers, retry, Model, SearchRecencyFilter, DEFAULT_API_VERSION, DEFAULT_BASE_URL,
};
pub use error::{PerplexityError, Result};

// Re-export commonly used types
pub use types::{
    Citation, FinishReason, Function, FunctionCall, Message, RelatedQuestion, ResponseFormat,
    Role, TokenUsage, Tool, ToolCall,
};

#[cfg(feature = "chat")]
pub use chat::{
    ChatEndpoint, ChatRequest, ChatRequestBuilder, ChatResponse, Choice, ResponseMessage,
};

#[cfg(feature = "streaming")]
pub use streaming::{
    ChatCompletionChunk, ChatCompletionStream, StreamChoice, StreamCollector, StreamDelta,
};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
