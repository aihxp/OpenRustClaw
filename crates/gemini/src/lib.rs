//! Google Gemini SDK for Rust
//!
//! Complete async SDK for Google Gemini API with support for:
//! - Text generation
//! - Multi-turn conversations
//! - Vision (image understanding)
//! - Function calling
//! - Embeddings
//! - Streaming responses

pub mod client;
pub mod types;
pub mod error;
pub mod chat;
pub mod embedding;
pub mod vision;
pub mod streaming;

pub use client::GeminiClient;
pub use types::*;
pub use error::GeminiError;
pub use chat::ChatSession;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default API endpoint
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
