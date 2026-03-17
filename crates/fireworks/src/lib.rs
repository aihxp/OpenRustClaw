//! # Fireworks AI SDK for Rust
//!
//! A native Rust SDK for [Fireworks AI](https://fireworks.ai/)'s fast inference API.
//!
//! ## Features
//!
//! - **Chat Completions**: Full support for multi-turn chat with streaming and tool calling
//! - **Completions**: Legacy text completions API
//! - **Embeddings**: Generate embeddings for text using various models
//! - **Fine-tuning**: Create and manage fine-tuned models
//! - **Image Generation**: Generate images using state-of-the-art diffusion models
//! - **Models**: List available models with their capabilities
//! - **Streaming**: Real-time streaming responses for chat and completions
//! - **Function Calling**: Fireworks-specific function calling support with firefunction-v2
//!
//! ## Quick Start
//!
//! ```no_run
//! use fireworks_ai::{FireworksClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FireworksClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
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
//! - `accounts/fireworks/models/llama-v3p1-405b-instruct` - Llama 3.1 405B Instruct
//! - `accounts/fireworks/models/llama-v3p1-70b-instruct` - Llama 3.1 70B Instruct
//! - `accounts/fireworks/models/llama-v3p1-8b-instruct` - Llama 3.1 8B Instruct
//! - `accounts/fireworks/models/mixtral-8x22b-instruct` - Mixtral 8x22B Instruct
//! - `accounts/fireworks/models/firefunction-v2` - Firefunction V2 (function calling)
//!
//! ## Streaming Example
//!
//! ```no_run
//! use fireworks_ai::{FireworksClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FireworksClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
//!     .message(Role::User, "Tell me a story")
//!     .build();
//!
//! let mut stream = client.chat().complete_stream(request).await?;
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
//! ## Function Calling Example
//!
//! ```no_run
//! use fireworks_ai::{FireworksClient, ChatRequest, Role, Tool};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FireworksClient::new("your-api-key")?;
//!
//! // Define a tool
//! let weather_tool = Tool::function(
//!     "get_weather",
//!     "Get the current weather for a location",
//!     json!({
//!         "type": "object",
//!         "properties": {
//!             "location": {
//!                 "type": "string",
//!                 "description": "The city and state"
//!             }
//!         },
//!         "required": ["location"]
//!     })
//! );
//!
//! let request = ChatRequest::builder("accounts/fireworks/models/firefunction-v2")
//!     .message(Role::User, "What's the weather in San Francisco?")
//!     .tool(weather_tool)
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! if response.has_tool_calls() {
//!     println!("Tool calls: {:?}", response.tool_calls());
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/fireworks-ai")]
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
#[cfg(feature = "image-generation")]
pub mod image_generation;
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "streaming")]
pub mod streaming;
pub mod types;

pub use client::{ClientConfig, FireworksClient};
pub use constants::*;
pub use error::{FireworksError, Result};

// Re-export commonly used types
pub use types::{ChatChoice, ChatMessage, ChatResponse, Role, TokenUsage, Tool, ToolCall};

#[cfg(feature = "chat")]
pub use chat::{ChatRequest, ChatRequestBuilder};

#[cfg(feature = "completions")]
pub use completions::{CompletionRequest, CompletionRequestBuilder};

#[cfg(feature = "embeddings")]
pub use embeddings::{EmbeddingRequest, EmbeddingResponse};

#[cfg(feature = "image-generation")]
pub use image_generation::ImageGenerationRequest;

#[cfg(feature = "image-generation")]
pub use types::ImageGenerationResponse;

#[cfg(feature = "models")]
pub use models::ModelInfo;

#[cfg(feature = "streaming")]
pub use streaming::{ChatCompletionChunk, CompletionChunk, StreamChoice, StreamDelta};

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
        assert_eq!(
            client::DEFAULT_BASE_URL,
            "https://api.fireworks.ai/inference/v1"
        );
    }
}
