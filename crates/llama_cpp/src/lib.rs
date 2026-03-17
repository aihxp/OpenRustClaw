//! # llama.cpp SDK for Rust
//!
//! A native Rust SDK for llama.cpp server - Local LLM inference.
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming
//! - **Text Completions API**: Raw text completion with streaming
//! - **Tokenization**: Convert text to token IDs
//! - **Embeddings**: Generate embeddings for text
//! - **Health Check**: Monitor server status
//! - **Slots Info**: Monitor concurrent request slots
//! - **All GGUF Models**: Support for all llama.cpp models including:
//!   - Llama 2/3
//!   - Mistral/Mixtral
//!   - CodeLlama
//!   - Phi
//!   - Qwen
//!   - Gemma
//!   - And all other GGUF models
//!
//! ## Quick Start
//!
//! ```no_run
//! use llama_cpp::{LlamaCppClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client connecting to local llama.cpp server
//! let client = LlamaCppClient::new()?;
//!
//! // Create a chat request
//! let request = ChatRequest::builder()
//!     .message(Role::System, "You are a helpful assistant.")
//!     .message(Role::User, "Hello, llama!")
//!     .build();
//!
//! // Get completion
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or("No content"));
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Server URL
//!
//! ```no_run
//! use llama_cpp::LlamaCppClient;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::with_base_url("http://localhost:8081")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use llama_cpp::{LlamaCppClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! let request = ChatRequest::builder()
//!     .message(Role::User, "Tell me a story")
//!     .build();
//!
//! let chat = client.chat();
//! let mut stream = chat.stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk {
//!         Ok(chunk) => {
//!             if let Some(content) = chunk.content() {
//!                 print!("{}", content);
//!             }
//!         }
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Text Completion Example
//!
//! ```no_run
//! use llama_cpp::{LlamaCppClient, CompletionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! let request = CompletionRequest::builder("Once upon a time")
//!     .max_tokens(100)
//!     .temperature(0.8)
//!     .build();
//!
//! let response = client.completion().complete(request).await?;
//! println!("{}", response.content().unwrap_or("No content"));
//! # Ok(())
//! # }
//! ```
//!
//! ## Tokenization Example
//!
//! ```no_run
//! use llama_cpp::LlamaCppClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! // Tokenize text
//! let tokens = client.tokenize().tokenize_text("Hello world").await?;
//! println!("Token count: {}", tokens.tokens.len());
//!
//! // Or just count tokens
//! let count = client.tokenize().count_tokens("Hello world").await?;
//! println!("Token count: {}", count);
//! # Ok(())
//! # }
//! ```
//!
//! ## Embeddings Example
//!
//! ```no_run
//! use llama_cpp::LlamaCppClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! let response = client.embeddings().embed_text("Hello world").await?;
//! println!("Embedding dimensions: {}", response.embedding.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Health Check Example
//!
//! ```no_run
//! use llama_cpp::LlamaCppClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! // Check health
//! let health = client.health().check().await?;
//! println!("Server status: {}", health.status);
//!
//! // Quick boolean check
//! if client.health().is_healthy().await {
//!     println!("Server is healthy!");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Slots Monitoring Example
//!
//! ```no_run
//! use llama_cpp::LlamaCppClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LlamaCppClient::new()?;
//!
//! let slots = client.slots().list().await?;
//! println!("Total slots: {}", slots.slots.len());
//!
//! let idle = client.slots().idle_count().await?;
//! let processing = client.slots().processing_count().await?;
//! println!("Idle: {}, Processing: {}", idle, processing);
//! # Ok(())
//! # }
//! ```
//!
//! ## Server Endpoints
//!
//! - `POST /completion` - Text completion
//! - `POST /chat/completion` - Chat completion
//! - `POST /tokenize` - Tokenize text
//! - `POST /embedding` - Generate embeddings
//! - `GET /health` - Health check
//! - `GET /slots` - Get slot information

#![doc(html_root_url = "https://docs.rs/llama-cpp")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

pub mod types;

#[cfg(feature = "chat")]
pub mod chat;
#[cfg(feature = "completion")]
pub mod completion;
#[cfg(feature = "tokenize")]
pub mod tokenize;
#[cfg(feature = "embeddings")]
pub mod embeddings;
#[cfg(feature = "health")]
pub mod health;
#[cfg(feature = "slots")]
pub mod slots;

pub use client::{LlamaCppClient, ClientConfig, DEFAULT_BASE_URL, endpoints};
pub use error::{LlamaCppError, Result};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, 
    Role, TokenUsage, FinishReason,
    CompletionChoice, CompletionResponse,
    EmbeddingResponse, TokenizeResponse,
    HealthResponse, SlotInfo, SlotState, SlotsResponse,
    GgufArchitecture, SamplingParams,
};

// Chat types
#[cfg(feature = "chat")]
pub use chat::{ChatRequest, ChatRequestBuilder, ChatCompletionChunk, StreamChoice, StreamDelta, StreamCollector};

// Completion types
#[cfg(feature = "completion")]
pub use completion::{CompletionRequest, CompletionRequestBuilder, CompletionChunk, CompletionStreamChoice, StreamCollector as CompletionStreamCollector};

// Tokenize types
#[cfg(feature = "tokenize")]
pub use tokenize::{TokenizeRequest, TokenizeRequestBuilder};

// Embeddings types
#[cfg(feature = "embeddings")]
pub use embeddings::{EmbeddingRequest, EmbeddingRequestBuilder, TruncationDirection};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default port for llama.cpp server.
pub const DEFAULT_PORT: u16 = 8080;

/// Check if the default server URL is reachable.
pub async fn is_server_available() -> bool {
    match LlamaCppClient::new() {
        Ok(client) => client.health().is_healthy().await,
        Err(_) => false,
    }
}

/// Create a client with the given base URL.
pub fn client(base_url: impl Into<String>) -> Result<LlamaCppClient> {
    LlamaCppClient::with_base_url(base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:8080");
    }

    #[test]
    fn test_default_port() {
        assert_eq!(DEFAULT_PORT, 8080);
    }

    #[test]
    fn test_client_creation() {
        let client = LlamaCppClient::new().unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_client_with_base_url() {
        let client = client("http://localhost:8081").unwrap();
        assert_eq!(client.base_url(), "http://localhost:8081");
    }
}
