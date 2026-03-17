//! Stability AI model-specific types for image generation.

use serde::{Deserialize, Serialize};

/// Stability AI SDXL request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StabilitySDXLRequest {
    /// The text prompts.
    pub text_prompts: Vec<StabilityTextPrompt>,
    /// Height of the generated image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// Width of the generated image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// Number of images to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<i32>,
    /// Number of diffusion steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<i32>,
    /// Scale for classifier-free guidance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Sampler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampler: Option<String>,
}

/// Stability text prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityTextPrompt {
    /// The prompt text.
    pub text: String,
    /// Weight of the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
}

impl StabilitySDXLRequest {
    /// Create a new SDXL request.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text_prompts: vec![StabilityTextPrompt {
                text: text.into(),
                weight: Some(1.0),
            }],
            height: None,
            width: None,
            samples: None,
            steps: None,
            cfg_scale: None,
            seed: None,
            sampler: None,
        }
    }

    /// Add a negative prompt.
    pub fn negative_prompt(mut self, text: impl Into<String>) -> Self {
        self.text_prompts.push(StabilityTextPrompt {
            text: text.into(),
            weight: Some(-1.0),
        });
        self
    }

    /// Set image dimensions.
    pub fn dimensions(mut self, width: i32, height: i32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the number of samples.
    pub fn samples(mut self, samples: i32) -> Self {
        self.samples = Some(samples);
        self
    }

    /// Set the seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Stability AI SDXL response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StabilitySDXLResponse {
    /// The result.
    pub result: String,
    /// The generated images.
    pub artifacts: Vec<StabilityArtifact>,
}

/// Stability artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StabilityArtifact {
    /// The seed used.
    pub seed: i64,
    /// The base64-encoded image.
    pub base64: String,
    /// Finish reason.
    pub finish_reason: String,
}

impl StabilitySDXLResponse {
    /// Get the first image as base64.
    pub fn image_base64(&self) -> Option<&str> {
        self.artifacts.first().map(|a| a.base64.as_str())
    }

    /// Get all images as base64.
    pub fn images_base64(&self) -> Vec<&str> {
        self.artifacts.iter().map(|a| a.base64.as_str()).collect()
    }
}

/// Stability AI SD3 request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilitySD3Request {
    /// The prompt.
    pub prompt: String,
    /// Negative prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Aspect ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    /// Image format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Strength (for image-to-image).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f32>,
    /// Mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl StabilitySD3Request {
    /// Create a new SD3 request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            negative_prompt: None,
            aspect_ratio: None,
            output_format: None,
            seed: None,
            model: None,
            strength: None,
            mode: None,
        }
    }

    /// Set negative prompt.
    pub fn negative_prompt(mut self, negative: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative.into());
        self
    }

    /// Set aspect ratio.
    pub fn aspect_ratio(mut self, ratio: impl Into<String>) -> Self {
        self.aspect_ratio = Some(ratio.into());
        self
    }

    /// Set output format.
    pub fn output_format(mut self, format: impl Into<String>) -> Self {
        self.output_format = Some(format.into());
        self
    }

    /// Set seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Stability AI SD3 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilitySD3Response {
    /// The base64-encoded image.
    pub image: String,
    /// Content type.
    #[serde(rename = "contentType")]
    pub content_type: String,
    /// Seed used.
    pub seed: i64,
    /// Finish reason.
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
}

/// Stability AI Core/Ultra request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityCoreRequest {
    /// The prompt.
    pub prompt: String,
    /// Negative prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Aspect ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    /// Output format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// Seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl StabilityCoreRequest {
    /// Create a new Core/Ultra request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            negative_prompt: None,
            aspect_ratio: None,
            output_format: None,
            seed: None,
        }
    }
}
