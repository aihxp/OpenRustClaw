//! # async-openai
//!
//! A native Rust SDK for OpenAI's API (Chat Completions, Assistants, Batch, Files, Fine-tuning).
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming and tool calling
//! - **Assistants API**: Create and manage assistants, threads, and runs
//! - **Batch API**: Process multiple requests asynchronously at 50% cost
//! - **Files API**: Upload and manage files for fine-tuning and assistants
//! - **Fine-tuning API**: Create and manage fine-tuning jobs
//! - **Embeddings API**: Generate text embeddings
//! - **Streaming support**: Real-time streaming for chat completions
//! - **Tool calling**: Support for function calling
//!
//! ## Quick Start
//!
//! ```no_run
//! use async_openai::{OpenAIClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OpenAIClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("gpt-4o")
//!     .message(Role::User, "Hello, GPT!")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or_default());
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/async-openai")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod constants;
mod error;

pub mod chat;
pub mod embeddings;
pub mod streaming;
pub mod types;

#[cfg(feature = "assistants")]
pub mod assistants;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "files")]
pub mod files;
#[cfg(feature = "fine-tuning")]
pub mod fine_tuning;

pub use client::{ClientConfig, OpenAIClient};
pub use constants::*;
pub use error::{OpenAIError, Result};

// Re-export commonly used types
pub use types::{ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, Role, TokenUsage};

pub use chat::{ChatRequest, ChatRequestBuilder};
pub use embeddings::EmbeddingRequest;

#[cfg(feature = "streaming")]
pub use streaming::{ChatCompletionChunk, StreamChoice, StreamDelta};

#[cfg(feature = "assistants")]
pub use assistants::{Assistant, AssistantRequest, Run, Thread, ThreadMessage};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
