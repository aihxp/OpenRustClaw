//! DALL-E Images API for Azure OpenAI.

use crate::client::AzureOpenAIClient;
use crate::error::Result;

/// Client for the images API.
#[derive(Debug)]
pub struct Images<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Images<'a> {
    /// Create a new images client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Generate an image.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, ImageRequest, ImageSize, ImageQuality};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-dalle-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = ImageRequest::new("A cute cat wearing a hat")
    ///     .size(ImageSize::Size1024x1024)
    ///     .quality(ImageQuality::Hd)
    ///     .n(1);
    ///
    /// let response = client.images().generate(request).await?;
    ///
    /// for image in response.data {
    ///     if let Some(url) = image.url {
    ///         println!("Image URL: {}", url);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, request: ImageRequest) -> Result<ImageResponse> {
        let body = serde_json::to_value(&request)?;
        let deployment = self.client.deployment_name().to_string();

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = format!("/openai/deployments/{}/images/generations", &deployment);
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let image_response: ImageResponse = serde_json::from_value(body)?;
                    Ok(image_response)
                })
            })
            .await
    }

    /// Generate a simple image with default settings.
    pub async fn generate_simple(&self, prompt: impl Into<String>) -> Result<ImageResponse> {
        let request = ImageRequest::new(prompt);
        self.generate(request).await
    }
}

/// An image generation request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageRequest {
    /// A text description of the desired image.
    pub prompt: String,

    /// The model to use for image generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// The number of images to generate (1-10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,

    /// The quality of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    /// The format of the generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// The size of the generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// The style of the generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl ImageRequest {
    /// Create a new image request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            n: None,
            quality: None,
            response_format: None,
            size: None,
            style: None,
            user: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the number of images.
    pub fn n(mut self, n: usize) -> Self {
        self.n = Some(n.clamp(1, 10));
        self
    }

    /// Set the quality.
    pub fn quality(mut self, quality: ImageQuality) -> Self {
        self.quality = Some(quality.as_str().to_string());
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ImageResponseFormat) -> Self {
        self.response_format = Some(format.as_str().to_string());
        self
    }

    /// Set the size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size.as_str().to_string());
        self
    }

    /// Set the style.
    pub fn style(mut self, style: ImageStyle) -> Self {
        self.style = Some(style.as_str().to_string());
        self
    }

    /// Set the user.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// Image quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageQuality {
    /// Standard quality (faster, cheaper).
    Standard,
    /// HD quality (higher detail).
    Hd,
}

impl ImageQuality {
    /// Get the quality string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageQuality::Standard => "standard",
            ImageQuality::Hd => "hd",
        }
    }
}

/// Image response format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageResponseFormat {
    /// URL format.
    Url,
    /// Base64 JSON format.
    B64Json,
}

impl ImageResponseFormat {
    /// Get the format string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageResponseFormat::Url => "url",
            ImageResponseFormat::B64Json => "b64_json",
        }
    }
}

/// Image size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageSize {
    /// 256x256 (DALL-E 2 only).
    Size256x256,
    /// 512x512 (DALL-E 2 only).
    Size512x512,
    /// 1024x1024.
    Size1024x1024,
    /// 1792x1024 (DALL-E 3 only).
    Size1792x1024,
    /// 1024x1792 (DALL-E 3 only).
    Size1024x1792,
}

impl ImageSize {
    /// Get the size string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageSize::Size256x256 => "256x256",
            ImageSize::Size512x512 => "512x512",
            ImageSize::Size1024x1024 => "1024x1024",
            ImageSize::Size1792x1024 => "1792x1024",
            ImageSize::Size1024x1792 => "1024x1792",
        }
    }
}

/// Image style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageStyle {
    /// Vivid style (tends to generate hyper-real and dramatic images).
    Vivid,
    /// Natural style (less hyper-real).
    Natural,
}

impl ImageStyle {
    /// Get the style string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageStyle::Vivid => "vivid",
            ImageStyle::Natural => "natural",
        }
    }
}

/// An image response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageResponse {
    /// The Unix timestamp when the image was created.
    pub created: i64,
    /// The list of generated images.
    pub data: Vec<GeneratedImage>,
}

/// A generated image.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratedImage {
    /// The URL of the generated image (if response_format is "url").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The base64-encoded JSON of the generated image (if response_format is "b64_json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    /// The prompt that was used to generate the image (may be revised).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_request_builder() {
        let request = ImageRequest::new("A cute cat")
            .n(2)
            .quality(ImageQuality::Hd)
            .size(ImageSize::Size1024x1024)
            .style(ImageStyle::Vivid);

        assert_eq!(request.prompt, "A cute cat");
        assert_eq!(request.n, Some(2));
        assert_eq!(request.quality, Some("hd".to_string()));
        assert_eq!(request.size, Some("1024x1024".to_string()));
        assert_eq!(request.style, Some("vivid".to_string()));
    }

    #[test]
    fn test_image_quality() {
        assert_eq!(ImageQuality::Standard.as_str(), "standard");
        assert_eq!(ImageQuality::Hd.as_str(), "hd");
    }

    #[test]
    fn test_image_size() {
        assert_eq!(ImageSize::Size256x256.as_str(), "256x256");
        assert_eq!(ImageSize::Size1024x1024.as_str(), "1024x1024");
        assert_eq!(ImageSize::Size1792x1024.as_str(), "1792x1024");
    }

    #[test]
    fn test_image_style() {
        assert_eq!(ImageStyle::Vivid.as_str(), "vivid");
        assert_eq!(ImageStyle::Natural.as_str(), "natural");
    }

    #[test]
    fn test_image_response_format() {
        assert_eq!(ImageResponseFormat::Url.as_str(), "url");
        assert_eq!(ImageResponseFormat::B64Json.as_str(), "b64_json");
    }
}
