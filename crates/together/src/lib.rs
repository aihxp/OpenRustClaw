//! # together-ai
//!
//! A native Rust SDK for Together AI's inference API.
//!
//! ## Features
//!
//! - **Chat Completions**: Full support for multi-turn chat with streaming
//! - **Completions**: Legacy text completions API
//! - **Embeddings**: Generate embeddings for text using various models
//! - **Fine-tuning**: Create and manage fine-tuned models
//! - **Models**: List available models with their capabilities
//! - **Streaming**: Real-time streaming responses for chat and completions
//!
//! ## Quick Start
//!
//! ```no_run
//! use together_ai::{TogetherClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TogetherClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
//!     .message(Role::User, "What is the capital of France?")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content());
//! # Ok(())
//! # }
//! ```
//!
//! ## Popular Models
//!
//! - `meta-llama/Llama-3-70b-chat-hf` - Llama 3 70B Chat
//! - `meta-llama/Llama-3-8b-chat-hf` - Llama 3 8B Chat
//! - `mistralai/Mixtral-8x22B-Instruct-v0.1` - Mixtral 8x22B Instruct
//! - `microsoft/WizardLM-2-8x22B` - WizardLM 2 8x22B
//! - `google/gemma-7b-it` - Gemma 7B Instruct

#![doc(html_root_url = "https://docs.rs/together-ai")]
#![warn(missing_docs)]

mod client;
mod constants;
mod error;

#[cfg(feature = "chat")]
pub mod chat;
#[cfg(feature = "completions")]
pub mod completions;
#[cfg(feature = "embeddings")]
pub mod embeddings;
#[cfg(feature = "fine-tuning")]
pub mod fine_tuning;
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "streaming")]
pub mod streaming;
pub mod types;

pub use client::{ClientConfig, TogetherClient};
pub use constants::*;
pub use error::{Result, TogetherError};

// Re-export commonly used types
pub use types::{ChatChoice, ChatMessage, ChatResponse, Role, TokenUsage};

#[cfg(feature = "chat")]
pub use chat::{ChatRequest, ChatRequestBuilder};

#[cfg(feature = "completions")]
pub use completions::{CompletionRequest, CompletionRequestBuilder};

#[cfg(feature = "embeddings")]
pub use embeddings::{EmbeddingRequest, EmbeddingResponse};

#[cfg(feature = "models")]
pub use models::ModelInfo;

#[cfg(feature = "streaming")]
pub use streaming::{ChatCompletionChunk, CompletionChunk, StreamChoice, StreamDelta};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
