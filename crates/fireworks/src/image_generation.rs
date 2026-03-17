//! Image generation API for Fireworks AI.

use crate::client::FireworksClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ImageGenerationResponse;

/// Client for image generation.
#[derive(Debug)]
pub struct Images<'a> {
    client: &'a FireworksClient,
}

impl<'a> Images<'a> {
    /// Create a new image generation client.
    pub fn new(client: &'a FireworksClient) -> Self {
        Self { client }
    }

    /// Generate an image from a prompt.
    pub async fn generate(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::IMAGE_GENERATION, body).await?;
                    let body = client.handle_response(response).await?;
                    let image_response: ImageGenerationResponse = serde_json::from_value(body)?;
                    Ok(image_response)
                })
            })
            .await
    }
}

/// Image size options.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSize {
    /// 256x256
    #[serde(rename = "256x256")]
    S256x256,
    /// 512x512
    #[serde(rename = "512x512")]
    S512x512,
    /// 768x768
    #[serde(rename = "768x768")]
    S768x768,
    /// 1024x1024
    #[serde(rename = "1024x1024")]
    S1024x1024,
    /// 1024x576 (16:9)
    #[serde(rename = "1024x576")]
    S1024x576,
    /// 576x1024 (9:16)
    #[serde(rename = "576x1024")]
    S576x1024,
    /// 1280x768
    #[serde(rename = "1280x768")]
    S1280x768,
    /// 768x1280
    #[serde(rename = "768x1280")]
    S768x1280,
}

impl ImageSize {
    /// Get the size as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageSize::S256x256 => "256x256",
            ImageSize::S512x512 => "512x512",
            ImageSize::S768x768 => "768x768",
            ImageSize::S1024x1024 => "1024x1024",
            ImageSize::S1024x576 => "1024x576",
            ImageSize::S576x1024 => "576x1024",
            ImageSize::S1280x768 => "1280x768",
            ImageSize::S768x1280 => "768x1280",
        }
    }
}

/// Image response format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageResponseFormat {
    /// URL to the generated image.
    Url,
    /// Base64-encoded image data.
    B64Json,
}

/// An image generation request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageGenerationRequest {
    /// Model ID.
    pub model: String,
    /// Prompt for image generation.
    pub prompt: String,
    /// Negative prompt (what to avoid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Image size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,
    /// Number of images to generate (1-4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ImageResponseFormat>,
    /// Guidance scale (how closely to follow the prompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    /// Number of inference steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_inference_steps: Option<usize>,
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// CFG scale (Classifier-Free Guidance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    /// Strength for img2img generation (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f32>,
    /// Safety check level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_check: Option<String>,
}

impl ImageGenerationRequest {
    /// Create a new builder.
    pub fn builder(
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ImageGenerationRequestBuilder {
        ImageGenerationRequestBuilder::new(model, prompt)
    }

    /// Create a simple request.
    pub fn simple(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self::builder(model, prompt).build()
    }
}

/// Builder for image generation requests.
#[derive(Debug, Clone)]
pub struct ImageGenerationRequestBuilder {
    model: String,
    prompt: String,
    negative_prompt: Option<String>,
    size: Option<ImageSize>,
    n: Option<usize>,
    response_format: Option<ImageResponseFormat>,
    guidance_scale: Option<f32>,
    num_inference_steps: Option<usize>,
    seed: Option<i64>,
    cfg_scale: Option<f32>,
    strength: Option<f32>,
    safety_check: Option<String>,
}

impl ImageGenerationRequestBuilder {
    /// Create a new builder.
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            negative_prompt: None,
            size: None,
            n: None,
            response_format: None,
            guidance_scale: None,
            num_inference_steps: None,
            seed: None,
            cfg_scale: None,
            strength: None,
            safety_check: None,
        }
    }

    /// Set the negative prompt.
    pub fn negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(prompt.into());
        self
    }

    /// Set the image size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the number of images.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n.clamp(1, 4));
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ImageResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set base64 response format.
    pub fn base64(mut self) -> Self {
        self.response_format = Some(ImageResponseFormat::B64Json);
        self
    }

    /// Set URL response format.
    pub fn url(mut self) -> Self {
        self.response_format = Some(ImageResponseFormat::Url);
        self
    }

    /// Set the guidance scale.
    pub fn guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = Some(scale);
        self
    }

    /// Set the number of inference steps.
    pub fn num_inference_steps(mut self, steps: usize) -> Self {
        self.num_inference_steps = Some(steps);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the CFG scale.
    pub fn cfg_scale(mut self, scale: f32) -> Self {
        self.cfg_scale = Some(scale);
        self
    }

    /// Set the strength for img2img.
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = Some(strength.clamp(0.0, 1.0));
        self
    }

    /// Set the safety check level.
    pub fn safety_check(mut self, level: impl Into<String>) -> Self {
        self.safety_check = Some(level.into());
        self
    }

    /// Build the request.
    pub fn build(self) -> ImageGenerationRequest {
        ImageGenerationRequest {
            model: self.model,
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            size: self.size,
            n: self.n,
            response_format: self.response_format,
            guidance_scale: self.guidance_scale,
            num_inference_steps: self.num_inference_steps,
            seed: self.seed,
            cfg_scale: self.cfg_scale,
            strength: self.strength,
            safety_check: self.safety_check,
        }
    }
}

use serde::{Deserialize, Serialize};
