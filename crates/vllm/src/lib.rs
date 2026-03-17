//! # vLLM SDK for Rust
//!
//! A native Rust SDK for vLLM - a high-throughput and memory-efficient inference engine for LLMs.
//!
//! ## Features
//!
//! - **Chat Completions**: OpenAI-compatible chat completions with streaming support
//! - **Completions**: Legacy text completions API
//! - **Embeddings**: Generate embeddings using embedding models
//! - **Tokenize/Detokenize**: Convert text to tokens and vice versa
//! - **Model Listing**: List and inspect available models
//! - **Health Check**: Check vLLM server health status
//! - **Metrics**: Prometheus-compatible metrics endpoint
//!
//! ## vLLM-Specific Features
//!
//! - **PagedAttention**: Efficient memory management for attention key/value cache
//! - **Continuous Batching**: Maximize GPU utilization with dynamic batching
//! - **Tensor Parallelism**: Distributed inference across multiple GPUs
//! - **Pipeline Parallelism**: Support for large model sharding
//!
//! ## Quick Start
//!
//! ```no_run
//! use vllm::{VllmClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = VllmClient::new("http://localhost:8000")?;
//!
//! let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
//!     .message(Role::User, "Hello, vLLM!")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content());
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use vllm::{VllmClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = VllmClient::new("http://localhost:8000")?;
//!
//! let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
//!     .message(Role::User, "Tell me a story")
//!     .build();
//!
//! let mut stream = client.chat().complete_stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk {
//!         Ok(chunk) => print!("{}", chunk.content()),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Tokenization Example
//!
//! ```no_run
//! use vllm::VllmClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = VllmClient::new("http://localhost:8000")?;
//!
//! // Tokenize text
//! let tokens = client.tokenize()
//!     .tokenize("meta-llama/Llama-3-8b-chat-hf", "Hello, world!")
//!     .await?;
//! println!("Token count: {}", tokens.len());
//!
//! // Detokenize
//! let text = client.tokenize()
//!     .detokenize("meta-llama/Llama-3-8b-chat-hf", &tokens)
//!     .await?;
//! println!("Original text: {}", text);
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/vllm")]
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
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "streaming")]
pub mod streaming;
#[cfg(feature = "tokenize")]
pub mod tokenize;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod types;

pub use client::{ClientConfig, VllmClient};
pub use constants::*;
pub use error::{Result, VllmError};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, CompletionChoice, CompletionResponse, Embedding,
    EmbeddingUsage, Function, FunctionCall, LogitBias, Role, TokenUsage, Tool, ToolCall,
};

#[cfg(feature = "chat")]
pub use chat::{ChatRequest, ChatRequestBuilder};

#[cfg(feature = "completions")]
pub use completions::{CompletionRequest, CompletionRequestBuilder};

#[cfg(feature = "embeddings")]
pub use embeddings::{EmbeddingInput, EmbeddingRequest, EmbeddingResponse};

#[cfg(feature = "tokenize")]
pub use tokenize::{DetokenizeRequest, TokenizeRequest, TokenizeResponse};

#[cfg(feature = "models")]
pub use models::{ModelInfo, ModelsResponse};

#[cfg(feature = "streaming")]
pub use streaming::{
    ChatCompletionChunk, CompletionChunk, StreamChoice, StreamDelta, ChatStream, CompletionStream,
    collect_chat_stream, collect_completion_stream,
};

#[cfg(feature = "metrics")]
pub use metrics::MetricsInfo;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:8000");
    }
}
