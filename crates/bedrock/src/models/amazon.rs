//! Amazon Titan model-specific types.

use serde::{Deserialize, Serialize};

/// Amazon Titan text generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanTextRequest {
    /// The input text.
    pub input_text: String,
    /// Text generation configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_generation_config: Option<TitanTextGenerationConfig>,
}

/// Titan text generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanTextGenerationConfig {
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Maximum token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_token_count: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Amazon Titan text generation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanTextResponse {
    /// The input text token count.
    pub input_text_token_count: i32,
    /// The results.
    pub results: Vec<TitanTextResult>,
}

/// Titan text generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanTextResult {
    /// The token count.
    pub token_count: i32,
    /// The output text.
    pub output_text: String,
    /// The completion reason.
    pub completion_reason: String,
}

impl TitanTextRequest {
    /// Create a new Titan text request.
    pub fn new(input_text: impl Into<String>) -> Self {
        Self {
            input_text: input_text.into(),
            text_generation_config: None,
        }
    }

    /// Set text generation configuration.
    pub fn with_config(mut self, config: TitanTextGenerationConfig) -> Self {
        self.text_generation_config = Some(config);
        self
    }
}

impl TitanTextGenerationConfig {
    /// Create new configuration.
    pub fn new() -> Self {
        Self {
            temperature: None,
            top_p: None,
            max_token_count: None,
            stop_sequences: None,
        }
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_token_count = Some(max_tokens);
        self
    }
}

/// Amazon Titan embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanEmbeddingRequest {
    /// The input text.
    pub input_text: String,
}

/// Amazon Titan embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanEmbeddingResponse {
    /// The embedding.
    pub embedding: Vec<f32>,
    /// Input text token count.
    pub input_text_token_count: i32,
}

/// Amazon Titan image generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanImageRequest {
    /// The task type.
    pub task_type: String,
    /// The image generation configuration.
    pub image_generation_config: TitanImageGenerationConfig,
}

/// Titan image generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanImageGenerationConfig {
    /// The text prompt.
    pub text: String,
    /// Negative text prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_text: Option<String>,
    /// Number of images to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<i32>,
    /// Quality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// Height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// Width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// CFG scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    /// Seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
}

/// Amazon Titan image generation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitanImageResponse {
    /// The images generated.
    pub images: Vec<String>,
}
