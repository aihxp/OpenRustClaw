//! # Mistral AI SDK for Rust
//!
//! A native Rust SDK for Mistral AI's API (Chat Completions, Embeddings, Agents).
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming and tool calling
//! - **Embeddings API**: Generate text embeddings
//! - **Agents API**: Create and manage Mistral AI agents
//! - **Streaming support**: Real-time streaming for chat completions
//! - **Tool calling**: Support for function calling
//! - **All Mistral Models**: Support for all Mistral AI models
//!
//! ## Quick Start
//!
//! ```no_run
//! use mistral::{MistralClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = MistralClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("mistral-large-latest")
//!     .message(Role::User, "Hello, Mistral!")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or("No content"));
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use mistral::{MistralClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = MistralClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("mistral-large-latest")
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
//! ## Tool Calling Example
//!
//! ```no_run
//! use mistral::{MistralClient, ChatRequest, Function, Role};
//! use mistral::chat::ToolChoice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = MistralClient::new("your-api-key")?;
//!
//! let weather_function = Function::builder("get_weather", "Get the current weather")
//!     .string_property("location", "City name", true)
//!     .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
//!     .build();
//!
//! let request = ChatRequest::builder("mistral-large-latest")
//!     .message(Role::User, "What's the weather in Paris?")
//!     .tool(weather_function)
//!     .tool_choice(ToolChoice::auto())
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! if let Some(tool_calls) = response.tool_calls() {
//!     for call in tool_calls {
//!         println!("Function: {}", call.function.name);
//!         println!("Arguments: {}", call.function.arguments);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/mistral")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

pub mod chat;
pub mod embedding;
pub mod types;

pub use client::{MistralClient, ClientConfig, DEFAULT_BASE_URL, endpoints};
pub use error::{MistralError, Result};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, 
    Role, TokenUsage, Tool, ToolCall, MistralModel,
    Embedding, EmbeddingsResponse, EmbeddingUsage,
    FinishReason,
};

pub use chat::{ChatRequest, ChatRequestBuilder, AgentRequest, AgentRequestBuilder, ToolChoice, ResponseFormat};
pub use embedding::EmbeddingRequest;

// Streaming types
#[cfg(feature = "streaming")]
pub use chat::{
    ChatCompletionChunk, StreamChoice, StreamDelta, 
    StreamCollector, ToolCallDelta, FunctionDelta,
};

// Agents support
#[cfg(feature = "agents")]
pub use chat::Agents;

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
        assert_eq!(DEFAULT_BASE_URL, "https://api.mistral.ai/v1");
    }
}
