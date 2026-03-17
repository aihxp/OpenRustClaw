//! # Cloudflare Workers AI SDK for Rust
//!
//! A native Rust SDK for Cloudflare Workers AI - Run AI models on Cloudflare's global network.
//!
//! ## Features
//!
//! - **Text Generation**: Support for all Cloudflare text generation models
//! - **Embeddings**: Generate text embeddings for semantic search
//! - **Translation**: Translate text between languages
//! - **Summarization**: Summarize long texts
//! - **Image Classification**: Classify images with pre-trained models
//! - **Text-to-Image**: Generate images from text descriptions
//! - **Speech Recognition**: Transcribe audio using Whisper
//! - **Streaming support**: Real-time streaming for text generation
//!
//! ## Supported Models
//!
//! ### Text Generation
//! - `@cf/meta/llama-3-8b-instruct`
//! - `@cf/meta/llama-3-8b-instruct-awq`
//! - `@cf/mistral/mistral-7b-instruct-v0.1`
//! - `@cf/microsoft/phi-2`
//! - `@cf/qwen/qwen1.5-7b-chat-awq`
//! - `@cf/tinyllama/tinyllama-1.1b-chat-v1.0`
//!
//! ### Speech Recognition
//! - `@cf/openai/whisper`
//!
//! ### Text-to-Image
//! - `@cf/stabilityai/stable-diffusion-xl-base-1.0`
//!
//! ## Authentication
//!
//! This SDK requires a Cloudflare Account ID and API Token. You can create an API token
//! in the Cloudflare dashboard under "My Profile" → "API Tokens" with the "Workers AI" permission.
//!
//! ## Quick Start
//!
//! ```no_run
//! use cloudflare_ai::{CloudflareAiClient, TextGenerationRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new(
//!     "your-account-id",
//!     "your-api-token",
//! )?;
//!
//! let request = TextGenerationRequest::builder("@cf/meta/llama-3-8b-instruct")
//!     .prompt("What is the capital of France?")
//!     .build();
//!
//! let response = client.text().generate(request).await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Chat Example
//!
//! ```no_run
//! use cloudflare_ai::{CloudflareAiClient, ChatMessage, Role};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let messages = vec![
//!     ChatMessage::system("You are a helpful assistant."),
//!     ChatMessage::user("What is the capital of France?"),
//! ];
//!
//! let response = client.text()
//!     .chat("@cf/meta/llama-3-8b-instruct", messages)
//!     .await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Embeddings Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let embedding = client.embeddings()
//!     .embed("@cf/baai/bge-base-en-v1.5", "Hello world")
//!     .await?;
//! println!("Embedding dimension: {}", embedding.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Translation Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let response = client.translation()
//!     .auto_translate("Bonjour le monde", "english")
//!     .await?;
//!
//! if let Some(text) = response.text() {
//!     println!("Translated: {}", text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Summarization Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let response = client.summarization()
//!     .summarize_text("Long text to summarize...")
//!     .await?;
//!
//! if let Some(summary) = response.summary() {
//!     println!("Summary: {}", summary);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Text-to-Image Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let response = client.text_to_image()
//!     .generate_prompt("A beautiful sunset over mountains")
//!     .await?;
//!
//! if let Ok(bytes) = response.decode_image() {
//!     std::fs::write("generated.png", bytes)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Speech Recognition Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let response = client.speech()
//!     .transcribe_file("audio.mp3")
//!     .await?;
//!
//! if let Some(text) = response.text() {
//!     println!("Transcription: {}", text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Image Classification Example
//!
//! ```no_run
//! use cloudflare_ai::CloudflareAiClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
//!
//! let response = client.image_classification()
//!     .classify_file("image.jpg")
//!     .await?;
//!
//! if let Some(top) = response.top_prediction() {
//!     println!("Class: {}, Score: {}", top.label, top.score);
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/cloudflare-ai")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;

#[cfg(feature = "embeddings")]
pub mod embeddings;
#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "speech")]
pub mod speech;
#[cfg(feature = "summarization")]
pub mod summarization;
#[cfg(feature = "text")]
pub mod text;
#[cfg(feature = "translation")]
pub mod translation;

pub mod types;

pub use client::{ClientConfig, CloudflareAiClient, DEFAULT_BASE_URL, endpoints};
pub use error::{CloudflareAiError, Result};

// Re-export commonly used types
pub use types::{
    ChatMessage, ClassificationResult, CloudflareModel, Embedding, EmbeddingsResponse,
    ImageClassificationResponse, ModelCategory, Role, SpeechRecognitionResponse,
    SummarizationResponse, TextGenerationResponse, TextGenerationStreamResponse,
    TextToImageResponse, TranslationResponse,
};

// Text generation types
#[cfg(feature = "text")]
pub use text::{TextGenerationRequest, TextGenerationRequestBuilder};

// Embeddings types
#[cfg(feature = "embeddings")]
pub use embeddings::{EmbeddingsBody, EmbeddingsRequest, EmbeddingsRequestBuilder};

// Translation types
#[cfg(feature = "translation")]
pub use translation::{
    TranslationRequest, TranslationRequestBuilder, languages as translation_languages,
};

// Summarization types
#[cfg(feature = "summarization")]
pub use summarization::{SummarizationRequest, SummarizationRequestBuilder};

// Image types
#[cfg(feature = "image")]
pub use image::{
    ImageClassificationRequest, ImageClassificationRequestBuilder, TextToImageRequest,
    TextToImageRequestBuilder,
};

// Speech types
#[cfg(feature = "speech")]
pub use speech::{
    SpeechRecognitionRequest, SpeechRecognitionRequestBuilder, formats as audio_formats,
    languages as speech_languages,
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
        assert_eq!(
            DEFAULT_BASE_URL,
            "https://api.cloudflare.com/client/v4/accounts"
        );
    }

    #[test]
    fn test_model_enum() {
        let model = CloudflareModel::Llama3_8bInstruct;
        assert_eq!(model.as_str(), "@cf/meta/llama-3-8b-instruct");
        assert!(model.is_text_generation());
        assert!(!model.is_speech());
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::System.to_string(), "system");
    }
}
