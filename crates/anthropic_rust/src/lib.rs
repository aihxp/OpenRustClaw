//! # anthropic-rust
//!
//! A native Rust SDK for Anthropic's Messages API.
//!
//! ## Features
//!
//! - **Complete Messages API support**: Streaming, tools, vision, and batch processing
//! - **Type-safe request/response types**: Full API coverage with Rust type safety
//! - **Built-in retry logic**: Exponential backoff with configurable policies
//! - **Rate limit handling**: Automatic retry-after handling
//! - **Streaming support**: Async iterators for real-time responses
//! - **Batch API**: Submit multiple requests for async processing (50% cheaper)
//!
//! ## Quick Start
//!
//! ```no_run
//! use anthropic_rust::{AnthropicClient, MessageRequest, Message, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = AnthropicClient::new("your-api-key")?;
//!
//! let request = MessageRequest::builder("claude-sonnet-4-20250514")
//!     .max_tokens(1024)
//!     .message(Role::User, "Hello, Claude!")
//!     .build();
//!
//! let response = client.messages().create(request).await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use anthropic_rust::{AnthropicClient, MessageRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = AnthropicClient::new("your-api-key")?;
//!
//! let request = MessageRequest::builder("claude-sonnet-4-20250514")
//!     .message(Role::User, "Tell me a story")
//!     .stream(true)
//!     .build();
//!
//! let messages = client.messages();
//! let mut stream = messages.stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         anthropic_rust::StreamEvent::ContentBlockDelta { delta, .. } => {
//!             print!("{}", delta.text());
//!         }
//!         anthropic_rust::StreamEvent::MessageStop => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/anthropic-rust")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod constants;
mod error;

pub mod streaming;
pub mod types;

#[cfg(feature = "batch")]
pub mod batch;

pub use client::{AnthropicClient, ClientConfig, Messages};
pub use constants::*;
pub use error::{AnthropicError, Result};

// Re-export commonly used types
pub use types::{
    ContentBlock, ContentBlockDelta, ImageContent, ImageSource, Message, MessageRequest,
    MessageResponse, MessageRole as Role, TextBlock, TextDelta, Tool, ToolChoice, ToolResult,
    ToolUse, Usage,
};

#[cfg(feature = "streaming")]
pub use streaming::{StreamEvent, StreamResult};

#[cfg(feature = "batch")]
pub use batch::{Batch, BatchClient, BatchRequest};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
