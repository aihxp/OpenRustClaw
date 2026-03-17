//! # Replicate SDK for Rust
//!
//! A native Rust SDK for the Replicate API - Run ML models in the cloud.
//!
//! ## Features
//!
//! - **Predictions**: Create and manage predictions with full lifecycle support
//! - **Official Models**: Run official models like Llama, Flux, Stable Diffusion
//! - **Community Models**: Run any model on Replicate with version IDs
//! - **Deployments**: Create and manage model deployments
//! - **Webhooks**: Receive real-time updates with webhook support and signature verification
//! - **Streaming**: Stream prediction output in real-time with Server-Sent Events
//! - **Trainings**: Fine-tune models with custom training jobs
//! - **Async/Long-running Jobs**: Full support for asynchronous predictions
//!
//! ## Supported Models
//!
//! ### Text Generation
//! - `meta/meta-llama-3-70b-instruct` - Meta's Llama 3 70B
//! - `meta/meta-llama-3-8b-instruct` - Meta's Llama 3 8B
//! - `mistralai/mixtral-8x7b-instruct-v0.1` - Mistral's Mixtral 8x7B
//!
//! ### Image Generation
//! - `black-forest-labs/flux-schnell` - Fastest Flux image generation
//! - `black-forest-labs/flux-dev` - Flux development model
//! - `stability-ai/stable-diffusion-3` - Stable Diffusion 3
//! - `stability-ai/sdxl` - Stable Diffusion XL
//!
//! ### Audio
//! - `openai/whisper` - Speech-to-text transcription
//! - `meta/musicgen` - Music generation
//!
//! ## Quick Start
//!
//! ```no_run
//! use replicate::{ReplicateClient, PredictionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ReplicateClient::new("your-api-token")?;
//!
//! // Run an official model
//! let request = PredictionRequest::new()
//!     .model("black-forest-labs/flux-schnell")
//!     .input("prompt", "Astronaut riding a horse");
//!
//! let prediction = client.predictions().create(request).await?;
//! println!("Prediction ID: {}", prediction.id);
//!
//! // Poll for completion
//! let output = client.predictions().wait_for_completion(&prediction.id).await?;
//! println!("Output: {:?}", output);
//! # Ok(())
//! # }
//! ```
//!
//! ## Async Predictions with Webhooks
//!
//! ```no_run
//! use replicate::{ReplicateClient, PredictionRequest, WebhookEvents};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ReplicateClient::new("your-api-token")?;
//!
//! let request = PredictionRequest::new()
//!     .model("meta/meta-llama-3-70b-instruct")
//!     .input("prompt", "Tell me a story")
//!     .webhook("https://myapp.com/webhooks/replicate")
//!     .webhook_events(vec![WebhookEvents::Completed, WebhookEvents::Output]);
//!
//! let prediction = client.predictions().create(request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Output
//!
//! ```no_run
//! use replicate::{ReplicateClient, PredictionRequest};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ReplicateClient::new("your-api-token")?;
//!
//! let prediction = client.predictions()
//!     .create(PredictionRequest::new()
//!         .model("meta/meta-llama-3-70b-instruct")
//!         .input("prompt", "Count to 100"))
//!     .await?;
//!
//! let mut stream = client.streaming().stream_output(&prediction.id).await?;
//! while let Some(event) = stream.next().await {
//!     match event {
//!         Ok(event) => println!("Event: {:?}", event),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Webhook Verification
//!
//! ```no_run
//! use replicate::webhooks::WebhookVerifier;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let verifier = WebhookVerifier::new("your-webhook-secret")?;
//!
//! // In your webhook handler
//! let signature = "t=1234567890,v1=abc123...";
//! let body = b"{\"id\": \"pred_123\", ...}";
//!
//! match verifier.verify(signature, body) {
//!     Ok(()) => println!("Webhook verified!"),
//!     Err(e) => println!("Invalid webhook: {}", e),
//! }
//! # Ok(())
//! # }
//! ```

#![doc(html_root_url = "https://docs.rs/replicate")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;
mod types;

#[cfg(feature = "deployments")]
pub mod deployments;
#[cfg(feature = "models")]
pub mod models;
#[cfg(feature = "predictions")]
pub mod predictions;
#[cfg(feature = "streaming")]
pub mod streaming;
#[cfg(feature = "trainings")]
pub mod trainings;
#[cfg(feature = "webhooks")]
pub mod webhooks;

// Re-export main types
pub use client::{ClientConfig, DEFAULT_BASE_URL, ReplicateClient};
pub use error::{ReplicateError, Result};
pub use types::*;

// Re-export prediction types
#[cfg(feature = "predictions")]
pub use predictions::{PredictionRequest, PredictionRequestBuilder, Predictions};

// Re-export model types
#[cfg(feature = "models")]
pub use models::{ModelInfo, Models};

// Re-export deployment types
#[cfg(feature = "deployments")]
pub use deployments::{DeploymentConfig, Deployments};

// Re-export training types
#[cfg(feature = "trainings")]
pub use trainings::{TrainingRequest, Trainings};

// Re-export webhook types
#[cfg(feature = "webhooks")]
pub use webhooks::{WebhookEvent, WebhookVerifier};

// Re-export streaming types
#[cfg(feature = "streaming")]
pub use streaming::{StreamEvent, Streaming};

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Model identifiers for popular models
pub mod models_list {
    /// Meta Llama 3 70B Instruct
    pub const META_LLAMA_3_70B_INSTRUCT: &str = "meta/meta-llama-3-70b-instruct";
    /// Meta Llama 3 8B Instruct
    pub const META_LLAMA_3_8B_INSTRUCT: &str = "meta/meta-llama-3-8b-instruct";
    /// Mistral Mixtral 8x7B Instruct
    pub const MISTRAL_MIXTRAL_8X7B_INSTRUCT: &str = "mistralai/mixtral-8x7b-instruct-v0.1";
    /// Stability AI Stable Diffusion 3
    pub const STABILITY_SD3: &str = "stability-ai/stable-diffusion-3";
    /// Stability AI SDXL
    pub const STABILITY_SDXL: &str = "stability-ai/stable-diffusion-xl-base-1.0";
    /// Black Forest Labs Flux Schnell
    pub const FLUX_SCHNELL: &str = "black-forest-labs/flux-schnell";
    /// Black Forest Labs Flux Dev
    pub const FLUX_DEV: &str = "black-forest-labs/flux-dev";
    /// Black Forest Labs Flux Pro
    pub const FLUX_PRO: &str = "black-forest-labs/flux-1.1-pro";
    /// OpenAI Whisper
    pub const OPENAI_WHISPER: &str = "openai/whisper";
    /// Meta MusicGen
    pub const META_MUSICGEN: &str = "meta/musicgen";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.replicate.com/v1");
    }

    #[test]
    fn test_model_constants() {
        assert_eq!(models_list::FLUX_SCHNELL, "black-forest-labs/flux-schnell");
        assert_eq!(
            models_list::META_LLAMA_3_70B_INSTRUCT,
            "meta/meta-llama-3-70b-instruct"
        );
    }
}
