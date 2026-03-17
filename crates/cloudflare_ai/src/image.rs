//! Image API for Cloudflare Workers AI (Classification and Text-to-Image).

use std::path::Path;

use crate::client::CloudflareAiClient;
use crate::error::{CloudflareAiError, Result};
use crate::types::{ImageClassificationResponse, TextToImageResponse};

/// Client for the image classification API.
#[derive(Debug)]
pub struct ImageClassification<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> ImageClassification<'a> {
    /// Create a new image classification client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Classify an image.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, ImageClassificationRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// // Using image bytes
    /// let image_bytes = std::fs::read("image.jpg")?;
    /// let request = ImageClassificationRequest::new(image_bytes);
    /// let response = client.image_classification().classify(request).await?;
    ///
    /// if let Some(top) = response.top_prediction() {
    ///     println!("Class: {}, Score: {}", top.label, top.score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify(
        &self,
        request: ImageClassificationRequest,
    ) -> Result<ImageClassificationResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: ImageClassificationResponse =
                        client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Classify an image from a file path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.image_classification()
    ///     .classify_file("image.jpg")
    ///     .await?;
    ///
    /// for result in &response.results {
    ///     println!("{}: {:.2}", result.label, result.score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ImageClassificationResponse> {
        let bytes = tokio::fs::read(path).await?;
        let request = ImageClassificationRequest::new(bytes);
        self.classify(request).await
    }
}

/// Client for the text-to-image API.
#[derive(Debug)]
pub struct TextToImage<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> TextToImage<'a> {
    /// Create a new text-to-image client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Generate an image from text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, TextToImageRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let request = TextToImageRequest::new("A beautiful sunset over mountains");
    /// let response = client.text_to_image().generate(request).await?;
    ///
    /// // Save the generated image
    /// if let Some(base64) = response.base64_image() {
    ///     let bytes = response.decode_image()?;
    ///     std::fs::write("generated.png", bytes)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, request: TextToImageRequest) -> Result<TextToImageResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: TextToImageResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Generate an image from a simple prompt.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.text_to_image()
    ///     .generate_prompt("A cat in space")
    ///     .await?;
    ///
    /// if let Ok(bytes) = response.decode_image() {
    ///     std::fs::write("cat_in_space.png", bytes)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_prompt(
        &self,
        prompt: impl Into<String>,
    ) -> Result<TextToImageResponse> {
        let request = TextToImageRequest::new(prompt);
        self.generate(request).await
    }

    /// Generate an image with specific dimensions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.text_to_image()
    ///     .generate_with_size("A landscape", 1024, 768)
    ///     .await?;
    ///
    /// if let Ok(bytes) = response.decode_image() {
    ///     std::fs::write("landscape.png", bytes)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_with_size(
        &self,
        prompt: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<TextToImageResponse> {
        let request = TextToImageRequest::builder()
            .prompt(prompt)
            .width(width)
            .height(height)
            .build();
        self.generate(request).await
    }
}

/// An image classification request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageClassificationRequest {
    /// ID of the model to use (default: "@cf/microsoft/resnet-50").
    #[serde(skip_serializing)]
    pub model: String,
    /// The image data as bytes (will be base64 encoded).
    #[serde(with = "base64_serde")]
    pub image: Vec<u8>,
}

impl ImageClassificationRequest {
    /// Create a new image classification request.
    ///
    /// # Arguments
    ///
    /// * `image` - The image data as bytes
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::ImageClassificationRequest;
    ///
    /// let image_bytes = vec![/* image data */];
    /// let request = ImageClassificationRequest::new(image_bytes);
    /// ```
    pub fn new(image: Vec<u8>) -> Self {
        Self {
            model: "@cf/microsoft/resnet-50".to_string(),
            image,
        }
    }

    /// Create a request from a file path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::ImageClassificationRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = ImageClassificationRequest::from_file("image.jpg").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        Ok(Self::new(bytes))
    }

    /// Create a builder for image classification requests.
    pub fn builder() -> ImageClassificationRequestBuilder {
        ImageClassificationRequestBuilder::new()
    }

    /// Set a custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// Builder for image classification requests.
#[derive(Debug, Clone, Default)]
pub struct ImageClassificationRequestBuilder {
    model: String,
    image: Option<Vec<u8>>,
}

impl ImageClassificationRequestBuilder {
    /// Create a new image classification request builder.
    pub fn new() -> Self {
        Self {
            model: "@cf/microsoft/resnet-50".to_string(),
            image: None,
        }
    }

    /// Set the image data.
    pub fn image(mut self, image: Vec<u8>) -> Self {
        self.image = Some(image);
        self
    }

    /// Set the image from a file path.
    pub async fn image_file(self, path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        Ok(self.image(bytes))
    }

    /// Set a custom model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the request.
    pub fn build(self) -> Result<ImageClassificationRequest> {
        let image = self.image.ok_or_else(|| CloudflareAiError::Image {
            message: "Image is required".to_string(),
        })?;

        Ok(ImageClassificationRequest {
            model: self.model,
            image,
        })
    }
}

/// A text-to-image request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextToImageRequest {
    /// ID of the model to use (default: "@cf/stabilityai/stable-diffusion-xl-base-1.0").
    #[serde(skip_serializing)]
    pub model: String,
    /// The text prompt.
    pub prompt: String,
    /// Negative prompt (what to avoid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Height of the generated image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Width of the generated image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Number of inference steps (higher = better quality but slower).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_steps: Option<u32>,
    /// Guidance scale (how closely to follow the prompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<f32>,
    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl TextToImageRequest {
    /// Create a new text-to-image request.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The text description of the image to generate
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::TextToImageRequest;
    ///
    /// let request = TextToImageRequest::new("A beautiful sunset");
    /// ```
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            model: "@cf/stabilityai/stable-diffusion-xl-base-1.0".to_string(),
            prompt: prompt.into(),
            negative_prompt: None,
            height: None,
            width: None,
            num_steps: None,
            guidance: None,
            seed: None,
        }
    }

    /// Create a builder for text-to-image requests.
    pub fn builder() -> TextToImageRequestBuilder {
        TextToImageRequestBuilder::new()
    }

    /// Set a custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the negative prompt.
    pub fn negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(prompt.into());
        self
    }

    /// Set the dimensions.
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the number of inference steps.
    pub fn num_steps(mut self, steps: u32) -> Self {
        self.num_steps = Some(steps);
        self
    }

    /// Set the guidance scale.
    pub fn guidance(mut self, guidance: f32) -> Self {
        self.guidance = Some(guidance);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Builder for text-to-image requests.
#[derive(Debug, Clone, Default)]
pub struct TextToImageRequestBuilder {
    model: String,
    prompt: Option<String>,
    negative_prompt: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
    num_steps: Option<u32>,
    guidance: Option<f32>,
    seed: Option<i64>,
}

impl TextToImageRequestBuilder {
    /// Create a new text-to-image request builder.
    pub fn new() -> Self {
        Self {
            model: "@cf/stabilityai/stable-diffusion-xl-base-1.0".to_string(),
            prompt: None,
            negative_prompt: None,
            height: None,
            width: None,
            num_steps: None,
            guidance: None,
            seed: None,
        }
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the negative prompt.
    pub fn negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(prompt.into());
        self
    }

    /// Set the height.
    pub fn height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the width.
    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the dimensions.
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the number of inference steps.
    pub fn num_steps(mut self, steps: u32) -> Self {
        self.num_steps = Some(steps);
        self
    }

    /// Set the guidance scale.
    pub fn guidance(mut self, guidance: f32) -> Self {
        self.guidance = Some(guidance);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set a custom model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the request.
    pub fn build(self) -> TextToImageRequest {
        TextToImageRequest {
            model: self.model,
            prompt: self.prompt.unwrap_or_default(),
            negative_prompt: self.negative_prompt,
            height: self.height,
            width: self.width,
            num_steps: self.num_steps,
            guidance: self.guidance,
            seed: self.seed,
        }
    }
}

/// Module for base64 serialization of image bytes.
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_classification_request() {
        let bytes = vec![1, 2, 3, 4];
        let req = ImageClassificationRequest::new(bytes);
        assert_eq!(req.model, "@cf/microsoft/resnet-50");
        assert_eq!(req.image, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_image_classification_with_model() {
        let req = ImageClassificationRequest::new(vec![1, 2, 3])
            .with_model("@cf/custom/model");
        assert_eq!(req.model, "@cf/custom/model");
    }

    #[test]
    fn test_text_to_image_request() {
        let req = TextToImageRequest::new("A beautiful sunset");
        assert_eq!(req.prompt, "A beautiful sunset");
        assert_eq!(req.model, "@cf/stabilityai/stable-diffusion-xl-base-1.0");
        assert_eq!(req.height, None);
        assert_eq!(req.width, None);
    }

    #[test]
    fn test_text_to_image_request_with_options() {
        let req = TextToImageRequest::new("A cat")
            .negative_prompt("blurry")
            .dimensions(1024, 768)
            .num_steps(30)
            .guidance(7.5)
            .seed(42);

        assert_eq!(req.negative_prompt, Some("blurry".to_string()));
        assert_eq!(req.width, Some(1024));
        assert_eq!(req.height, Some(768));
        assert_eq!(req.num_steps, Some(30));
        assert_eq!(req.guidance, Some(7.5));
        assert_eq!(req.seed, Some(42));
    }

    #[test]
    fn test_text_to_image_builder() {
        let req = TextToImageRequest::builder()
            .prompt("A landscape")
            .negative_prompt("blurry, low quality")
            .width(512)
            .height(512)
            .num_steps(20)
            .guidance(8.0)
            .seed(123)
            .build();

        assert_eq!(req.prompt, "A landscape");
        assert_eq!(req.negative_prompt, Some("blurry, low quality".to_string()));
        assert_eq!(req.width, Some(512));
        assert_eq!(req.height, Some(512));
        assert_eq!(req.num_steps, Some(20));
        assert_eq!(req.guidance, Some(8.0));
        assert_eq!(req.seed, Some(123));
    }

    #[test]
    fn test_image_classification_response() {
        let response: ImageClassificationResponse = serde_json::from_str(
            r#"{
                "results": [
                    {"label": "cat", "score": 0.95},
                    {"label": "dog", "score": 0.04},
                    {"label": "bird", "score": 0.01}
                ]
            }"#
        ).unwrap();

        assert_eq!(response.results.len(), 3);
        let top = response.top_prediction().unwrap();
        assert_eq!(top.label, "cat");
        assert!((top.score - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_text_to_image_response() {
        let response = TextToImageResponse {
            image: Some("aGVsbG8=".to_string()), // base64 encoded "hello"
        };

        assert_eq!(response.base64_image(), Some("aGVsbG8="));
        let decoded = response.decode_image().unwrap();
        assert_eq!(decoded, b"hello");
    }
}
