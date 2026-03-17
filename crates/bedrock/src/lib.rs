//! # AWS Bedrock SDK for Rust
//!
//! A comprehensive Rust SDK for AWS Bedrock, supporting:
//! - **Converse API**: Unified conversation interface for all Bedrock models
//! - **InvokeModel API**: Direct model invocation with native request/response formats
//! - **InvokeModelWithResponseStream**: Streaming responses for real-time output
//! - **All Bedrock Models**: Claude, Titan, Llama, Mistral, Cohere, AI21, Stability AI
//! - **AWS SigV4**: Full authentication with credential chain support
//! - **Cross-Region Inference**: Route requests to optimal regions
//! - **Guardrails**: Apply content filtering to model responses
//! - **Agents**: Invoke Bedrock Agents for complex workflows
//! - **Knowledge Bases**: Retrieve context from enterprise data
//!
//! ## Quick Start
//!
//! ```no_run
//! use aws_bedrock::{BedrockClient, ConverseRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = BedrockClient::new("us-east-1").await?;
//!
//! let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
//!     .message(Message::user_text("Hello, Claude!"))
//!     .build();
//!
//! let response = client.converse().invoke(request).await?;
//! println!("{}", response.output.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Example
//!
//! ```no_run
//! use aws_bedrock::{BedrockClient, ConverseRequest, Message};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = BedrockClient::new("us-east-1").await?;
//!
//! let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
//!     .message(Message::user_text("Tell me a story"))
//!     .build();
//!
//! let mut stream = client.converse().stream(request).await?;
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         aws_bedrock::StreamEvent::ContentBlockDelta { delta, .. } => {
//!             print!("{}", delta.text().unwrap_or_default());
//!         }
//!         aws_bedrock::StreamEvent::MessageStop { stop_reason: _ } => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/aws-bedrock")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod constants;
mod error;

pub mod auth;
pub mod converse;
pub mod models;
pub mod streaming;
pub mod types;

#[cfg(feature = "invoke-model")]
pub mod invoke;

#[cfg(feature = "agents")]
pub mod agents;

#[cfg(feature = "guardrails")]
pub mod guardrails;

#[cfg(feature = "knowledge-bases")]
pub mod knowledge;

pub use client::{BedrockClient, ClientConfig};
pub use constants::*;
pub use error::{BedrockError, Result};

// Re-export commonly used types
pub use types::{
    ContentBlock, ContentBlockDelta, ConversationRole as Role, ImageBlock, ImageFormat,
    InferenceConfiguration, Message, StopReason, TokenUsage, Tool, ToolChoice, ToolInputSchema,
    ToolResultBlock, ToolSpecification, ToolUseBlock, VideoBlock, VideoFormat,
};

#[cfg(feature = "converse")]
pub use converse::{ConverseRequest, ConverseResponse, ConverseStreamResponse};

#[cfg(feature = "streaming")]
pub use streaming::{StreamEvent, StreamResult};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
