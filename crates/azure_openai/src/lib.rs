//! # azure-openai
//!
//! An enterprise-grade Azure OpenAI Service SDK for Rust.
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming and tool calling
//! - **Completions API**: Legacy completions support
//! - **Embeddings API**: Generate text embeddings with Azure integration
//! - **DALL-E Images API**: Generate and edit images
//! - **Audio API**: Whisper transcription and Text-to-Speech (TTS)
//! - **Assistants API**: Create and manage assistants, threads, and runs
//! - **Batch API**: Process multiple requests asynchronously at 50% cost
//! - **Fine-tuning API**: Create and manage fine-tuning jobs
//! - **Files API**: Upload and manage files for fine-tuning and assistants
//! - **Azure AD Authentication**: Token-based authentication support
//! - **Managed Identity**: Azure Managed Identity support
//! - **Regional Endpoints**: Support for Azure regional deployments
//! - **Content Filtering**: Azure Content Safety integration
//!
//! ## Quick Start
//!
//! ```no_run
//! use azure_openai::{AzureOpenAIClient, ChatRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = AzureOpenAIClient::new(
//!     "your-resource-name",
//!     "your-deployment-name",
//!     "your-api-key",
//! )?;
//!
//! let request = ChatRequest::builder()
//!     .user("Hello, Azure OpenAI!")
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content().unwrap_or_default());
//! # Ok(())
//! # }
//! ```
//!
//! ## Azure AD Authentication
//!
//! ```no_run
//! use azure_openai::{AzureOpenAIClient, AzureConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = AzureOpenAIClient::with_azure_ad_token(
//!     "your-resource-name",
//!     "your-deployment-name",
//!     "your-aad-token",
//! )?;
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/azure-openai")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod config;
mod constants;
mod error;

pub mod auth;
pub mod chat;
pub mod completions;
pub mod embeddings;
pub mod streaming;
pub mod types;

#[cfg(feature = "assistants")]
pub mod assistants;
#[cfg(feature = "audio")]
pub mod audio;
#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "content-safety")]
pub mod content_safety;
#[cfg(feature = "files")]
pub mod files;
#[cfg(feature = "fine-tuning")]
pub mod fine_tuning;
#[cfg(feature = "images")]
pub mod images;

pub use client::AzureOpenAIClient;
pub use config::{AzureConfig, AzureCredential};
pub use constants::AzureRegion;
pub use constants::*;
pub use error::{AzureOpenAIError, ContentFilterResults, Result};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, FinishReason, Function, FunctionCall, Role, TokenUsage,
    Tool, ToolCall,
};

pub use chat::{ChatRequest, ChatRequestBuilder};
pub use embeddings::EmbeddingRequest;

#[cfg(feature = "streaming")]
pub use streaming::{ChatCompletionChunk, StreamChoice, StreamDelta};

#[cfg(feature = "assistants")]
pub use assistants::{Assistant, AssistantRequest, Run, Thread, ThreadMessage};

#[cfg(feature = "images")]
pub use images::{ImageQuality, ImageRequest, ImageResponse, ImageSize, ImageStyle};

#[cfg(feature = "audio")]
pub use audio::{AudioResponseFormat, TranscriptionRequest, TtsRequest, TtsVoice};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export auth types
pub use auth::{AzureADToken, ManagedIdentityCredential, TokenCredential};
