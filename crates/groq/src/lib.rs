//! # Groq SDK for Rust
//!
//! A native Rust SDK for Groq API - Ultra-fast LLM inference.
//!
//! ## Features
//!
//! - **Chat Completions API**: Complete support with streaming and tool calling
//! - **Audio Transcription (Whisper)**: Convert audio to text
//! - **Audio Translations**: Translate audio to English text
//! - **All Groq Models**: Support for all Groq-hosted models including:
//!   - llama3-8b-8192
//!   - llama3-70b-8192
//!   - llama-3.1-8b-instant
//!   - llama-3.1-70b-versatile
//!   - llama-3.1-405b-reasoning
//!   - mixtral-8x7b-32768
//!   - gemma-7b-it
//!   - gemma2-9b-it
//! - **Streaming support**: Real-time streaming for chat completions
//! - **Tool calling**: Support for function calling
//!
//! ## Quick Start
//!
//! ```no_run
//! use groq::{GroqClient, ChatRequest, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GroqClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("llama3-8b-8192")
//!     .message(Role::User, "Hello, Groq!")
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
//! use groq::{GroqClient, ChatRequest, Role};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GroqClient::new("your-api-key")?;
//!
//! let request = ChatRequest::builder("llama3-8b-8192")
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
//! ## Audio Transcription Example
//!
//! ```no_run
//! use groq::{GroqClient, AudioTranscriptionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GroqClient::new("your-api-key")?;
//!
//! let request = AudioTranscriptionRequest::builder()
//!     .file_path("/path/to/audio.mp3").await?
//!     .model("whisper-large-v3")
//!     .build()?;
//!
//! let response = client.audio().transcribe(request).await?;
//! println!("Transcription: {}", response.text);
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/groq")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

pub mod chat;
pub mod audio;
pub mod types;

pub use client::{GroqClient, ClientConfig, DEFAULT_BASE_URL, endpoints};
pub use error::{GroqError, Result};

// Re-export commonly used types
pub use types::{
    ChatChoice, ChatMessage, ChatResponse, Function, FunctionCall, 
    Role, TokenUsage, Tool, ToolCall, GroqModel,
    FinishReason,
};

pub use chat::{ChatRequest, ChatRequestBuilder, ToolChoice, ResponseFormat};

// Audio types
#[cfg(feature = "audio")]
pub use audio::{
    AudioTranscriptionRequest, AudioTranscriptionRequestBuilder,
    AudioTranslationRequest, AudioTranslationRequestBuilder,
    TranscriptionResponse, TranslationResponse,
    AudioSegment, TranscriptionVerboseResponse,
};

// Streaming types
#[cfg(feature = "streaming")]
pub use chat::{
    ChatCompletionChunk, StreamChoice, StreamDelta, 
    StreamCollector, ToolCallDelta, FunctionDelta,
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
        assert_eq!(DEFAULT_BASE_URL, "https://api.groq.com/openai/v1");
    }
}
