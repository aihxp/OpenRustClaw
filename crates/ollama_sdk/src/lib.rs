//! # Ollama SDK for Rust
//!
//! A native Rust SDK for the Ollama API - Local LLM inference with comprehensive API support.
//!
//! ## Features
//!
//! - **Chat Completions API**: Full support for conversational AI with streaming
//! - **Generate API**: Text completion and generation endpoint
//! - **Model Management**: List, pull, push, create, delete, and copy models
//! - **Embeddings API**: Generate embeddings for text
//! - **Multi-modal Support**: Vision capabilities with image inputs (llava models)
//! - **JSON Mode**: Structured JSON output support
//! - **Streaming Support**: Real-time streaming for chat and generate endpoints
//! - **Keep-alive Configuration**: Control model loading/unloading behavior
//! - **Custom Headers**: Support for additional HTTP headers
//!
//! ## Quick Start
//!
//! ```no_run
//! use ollama_sdk::{OllamaClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let request = ChatRequest::builder("llama3.2")
//!     .message(Role::User, "Hello, Ollama!")
//!     .build();
//!
//! let response = client.chat().generate(request).await?;
//! println!("{}", response.content());
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use ollama_sdk::{OllamaClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let request = ChatRequest::builder("llama3.2")
//!     .message(Role::User, "Tell me a story")
//!     .build();
//!
//! let mut stream = client.chat().stream(request).await?;
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
//! ## Vision (Multi-modal) Example
//!
//! ```no_run
//! use ollama_sdk::{OllamaClient, ChatRequest, Role, ImageInput};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let image = ImageInput::from_path("./image.jpg");
//!
//! let request = ChatRequest::builder("llava")
//!     .message_with_images(Role::User, "What's in this image?", vec![image])
//!     .build();
//!
//! let response = client.chat().generate(request).await?;
//! println!("{}", response.content());
//! # Ok(())
//! # }
//! ```
//!
//! ## JSON Mode Example
//!
//! ```no_run
//! use ollama_sdk::{OllamaClient, ChatRequest, Role, FormatType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let request = ChatRequest::builder("llama3.2")
//!     .system("You are a helpful assistant that always responds with valid JSON.")
//!     .message(Role::User, r#"Generate a JSON object with "name" and "age" fields"#)
//!     .format(FormatType::json())
//!     .build();
//!
//! let response = client.chat().generate(request).await?;
//! let json: serde_json::Value = serde_json::from_str(response.content())?;
//! println!("Name: {}, Age: {}", json["name"], json["age"]);
//! # Ok(())
//! # }
//! ```
//!
//! ## Model Management
//!
//! ```no_run
//! use ollama_sdk::OllamaClient;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! // List local models
//! let models = client.models().list().await?;
//! for model in models {
//!     println!("Model: {} (Size: {} MB)", model.name, model.size / 1_048_576);
//! }
//!
//! // Pull a model
//! let mut stream = client.models().pull("llama3.2").await?;
//! while let Some(status) = stream.next().await {
//!     match status {
//!         Ok(status) => println!("Pull status: {:?}", status),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//!
//! // Delete a model
//! client.models().delete("old-model").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Embeddings
//!
//! ```no_run
//! use ollama_sdk::OllamaClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let response = client.embeddings()
//!     .generate("nomic-embed-text", "Hello, world!")
//!     .await?;
//!
//! println!("Embedding dimensions: {}", response.embedding.len());
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/ollama-sdk")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

pub mod chat;
pub mod generate;
pub mod models;
pub mod embeddings;
pub mod types;

pub use client::{OllamaClient, ClientConfig, DEFAULT_BASE_URL};
pub use error::{OllamaError, Result};

// Re-export commonly used types
pub use types::{
    ChatMessage, ChatResponse, GenerateResponse, Role, Tool, ToolCall,
    Function, FunctionCall, MessageRole, ModelInfo, ListModelsResponse,
    PullStatus, PushStatus, ProgressStatus, EmbeddingResponse,
    RunningModel, RunningModelsResponse, VersionResponse, FormatType,
    KeepAlive, ModelDetails, CreateModelRequest, CreateModelStatus,
    CopyModelRequest, DeleteModelRequest, ShowModelResponse, ShowModelRequest,
    Options, ToolCallFunction, ImageInput, ToolResult,
};

pub use chat::{ChatRequest, ChatRequestBuilder};
pub use generate::{GenerateRequest, GenerateRequestBuilder};

// Streaming types
#[cfg(feature = "streaming")]
pub use chat::{
    ChatStream, ChatStreamChunk,
};

#[cfg(feature = "streaming")]
pub use generate::{
    GenerateStream, GenerateStreamChunk,
};

#[cfg(feature = "streaming")]
pub use models::{
    PullStream, PushStream, CreateModelStream,
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
        assert_eq!(DEFAULT_BASE_URL, "http://localhost:11434");
    }
}
