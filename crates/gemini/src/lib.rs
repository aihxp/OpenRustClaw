//! Google Gemini SDK for Rust
//!
//! Complete async SDK for Google Gemini API with support for:
//! - Text generation
//! - Multi-turn conversations
//! - Vision (image understanding)
//! - Function calling
//! - Embeddings
//! - Streaming responses

pub mod chat;
pub mod client;
pub mod embedding;
pub mod error;
pub mod streaming;
pub mod types;
pub mod vision;

pub use chat::ChatSession;
pub use client::GeminiClient;
pub use error::GeminiError;
pub use types::*;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default API endpoint
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
