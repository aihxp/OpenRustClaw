//! # DeepSeek SDK for Rust
//!
//! A native Rust SDK for DeepSeek API - Cost-effective LLM inference with reasoning support.
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming and tool calling
//! - **Reasoning Mode (DeepSeek-R1)**: Access chain-of-thought reasoning content
//! - **JSON Mode**: Structured JSON output support
//! - **All DeepSeek Models**:
//!   - `deepseek-chat` (DeepSeek-V3) - General purpose chat model
//!   - `deepseek-reasoner` (DeepSeek-R1) - Reasoning model with chain-of-thought
//!   - `deepseek-coder` - Code generation model
//! - **Streaming support**: Real-time streaming for chat completions
//! - **Tool calling**: Support for function calling
//!
//! ## Quick Start
//!
//! ```no_run
//! use deepseek::{DeepSeekClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DeepSeekClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("deepseek-chat")
//!     .message(Role::User, "Hello, DeepSeek!")
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
//! use deepseek::{DeepSeekClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DeepSeekClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("deepseek-chat")
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
//! ## Reasoning Mode Example (DeepSeek-R1)
//!
//! ```no_run
//! use deepseek::{DeepSeekClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DeepSeekClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("deepseek-reasoner")
//!     .message(Role::User, "Solve this step by step: What is 15 * 27?")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//!
//! // Access the reasoning content (chain-of-thought)
//! if let Some(reasoning) = response.reasoning_content() {
//!     println!("Reasoning process: {}", reasoning);
//! }
//!
//! // Access the final answer
//! if let Some(content) = response.content() {
//!     println!("Final answer: {}", content);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## JSON Mode Example
//!
//! ```no_run
//! use deepseek::{DeepSeekClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DeepSeekClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("deepseek-chat")
//!     .system("You are a helpful assistant that always responds with valid JSON.")
//!     .message(Role::User, r#"Generate a JSON object with "name" and "age" fields"#)
//!     .json_mode()
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! if let Some(content) = response.content() {
//!     let json: serde_json::Value = serde_json::from_str(content)?;
//!     println!("Name: {}, Age: {}", json["name"], json["age"]);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Tool Calling Example
//!
//! ```no_run
//! use deepseek::{DeepSeekClient, ChatRequest, Role, Function, ToolChoice};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DeepSeekClient::new("your-api-key")?;
//!
//! // Define a function
//! let weather_func = Function::builder("get_weather", "Get the current weather")
//!     .string_property("location", "The city and state", true)
//!     .enum_property("unit", "Temperature unit", vec!["celsius", "fahrenheit"], false)
//!     .build();
//!
//! let request = ChatRequest::builder("deepseek-chat")
//!     .message(Role::User, "What's the weather in Paris?")
//!     .tool(weather_func)
//!     .tool_choice(ToolChoice::auto())
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//!
//! if response.has_tool_calls() {
//!     for tool_call in response.tool_calls().unwrap() {
//!         println!("Function: {}", tool_call.function.name);
//!         println!("Arguments: {}", tool_call.function.arguments);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/deepseek")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

pub mod chat;
pub mod models;
pub mod types;

pub use client::{DeepSeekClient, ClientConfig, DEFAULT_BASE_URL, endpoints};
pub use error::{DeepSeekError, Result};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, 
    Role, TokenUsage, Tool, ToolCall, DeepSeekModel,
    FinishReason, Model, ListModelsResponse,
};

pub use chat::{ChatRequest, ChatRequestBuilder, ToolChoice, ResponseFormat};

// Streaming types
#[cfg(feature = "streaming")]
pub use chat::{
    ChatCompletionChunk, StreamChoice, StreamDelta, 
    StreamCollector, ToolCallDelta, FunctionDelta, ChatStream,
};

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
        assert_eq!(DEFAULT_BASE_URL, "https://api.deepseek.com");
    }
}
