//! # openrouter-api
//!
//! A native Rust SDK for OpenRouter's API with routing strategies and fallback support.
//!
//! ## Features
//!
//! - **Chat Completions**: Full support with streaming and tool calling
//! - **Model Listing**: Get models with pricing information
//! - **Routing Strategies**: Price, quality, and throughput optimization
//! - **Fallback Support**: Automatic fallback between providers
//! - **Usage Tracking**: Token and cost tracking
//!
//! ## Quick Start
//!
//! ```no_run
//! use openrouter_api::{OpenRouterClient, ChatRequest, Role, RouteStrategy};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OpenRouterClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("anthropic/claude-3.5-sonnet")
//!     .message(Role::User, "Hello!")
//!     .route_strategy(RouteStrategy::Quality)
//!     .build();
//!
//! let response = client.chat().complete(request).await?;
//! println!("{}", response.content);
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/openrouter-api")]
#![warn(missing_docs)]

mod client;
mod constants;
mod error;

pub mod chat;
pub mod models;
pub mod routing;
pub mod streaming;
pub mod types;

pub use client::{ClientConfig, OpenRouterClient};
pub use constants::*;
pub use error::{OpenRouterError, Result};

// Re-export commonly used types
pub use chat::{ChatRequest, ChatRequestBuilder};
pub use models::ModelInfo;
pub use routing::RouteStrategy;
pub use types::{ChatChoice, ChatMessage, ChatResponse, Role, TokenUsage, Tool, ToolCall};

#[cfg(feature = "streaming")]
pub use streaming::{ChatCompletionChunk, StreamChoice};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
