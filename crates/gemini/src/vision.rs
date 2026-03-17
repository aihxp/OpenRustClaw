//! Vision capabilities for image understanding

use crate::{Content, GeminiClient, GeminiError, GenerateContentRequest, GenerationConfig, Part};
use async_trait::async_trait;

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Heic,
    Heif,
}

impl ImageFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Heic => "image/heic",
            ImageFormat::Heif => "image/heif",
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::Webp),
            "heic" => Some(ImageFormat::Heic),
            "heif" => Some(ImageFormat::Heif),
            _ => None,
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        // Check magic bytes
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            Some(ImageFormat::Png)
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageFormat::Jpeg)
        } else if data.starts_with(b"RIFF") && data.len() > 8 && &data[8..12] == b"WEBP" {
            Some(ImageFormat::Webp)
        } else if data.starts_with(b"heic") || data.starts_with(b"heix") {
            Some(ImageFormat::Heic)
        } else {
            None
        }
    }
}

/// Video format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    Mp4,
    Mpeg,
    Mov,
    Avi,
    Flv,
    Wmv,
}

impl VideoFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "video/mp4",
            VideoFormat::Mpeg => "video/mpeg",
            VideoFormat::Mov => "video/quicktime",
            VideoFormat::Avi => "video/x-msvideo",
            VideoFormat::Flv => "video/x-flv",
            VideoFormat::Wmv => "video/x-ms-wmv",
        }
    }
}

/// Builder for vision requests
pub struct VisionBuilder<'a> {
    client: &'a GeminiClient,
    generation_config: Option<GenerationConfig>,
}

impl<'a> VisionBuilder<'a> {
    /// Create new vision builder
    pub fn new(client: &'a GeminiClient) -> Self {
        Self {
            client,
            generation_config: None,
        }
    }

    /// Set generation config
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    /// Analyze image from bytes
    pub async fn analyze_image(
        &self,
        prompt: impl Into<String>,
        mime_type: &str,
        image_data: Vec<u8>,
    ) -> Result<String, GeminiError> {
        let content = Content::user(prompt).with_image(mime_type, image_data);

        let request = GenerateContentRequest {
            contents: vec![content],
            system_instruction: None,
            generation_config: self.generation_config.clone(),
            tools: None,
            tool_config: None,
            safety_settings: None,
        };

        let response = self.client.generate_content(request).await?;

        Ok(response
            .candidates
            .into_iter()
            .filter_map(|c| {
                c.content
                    .parts
                    .into_iter()
                    .filter_map(|p| match p {
                        Part::Text { text } => Some(text),
                        _ => None,
                    })
                    .next()
            })
            .collect::<Vec<_>>()
            .join(""))
    }

    /// Analyze image from file path (reads file)
    pub async fn analyze_image_file(
        &self,
        prompt: impl Into<String>,
        path: &std::path::Path,
    ) -> Result<String, GeminiError> {
        use std::fs;

        let data = fs::read(path).map_err(|e| {
            GeminiError::InvalidRequest(format!("Failed to read image file: {}", e))
        })?;

        let format = ImageFormat::from_path(path.to_str().unwrap_or(""))
            .or_else(|| ImageFormat::from_bytes(&data))
            .ok_or_else(|| GeminiError::InvalidRequest("Unknown image format".to_string()))?;

        self.analyze_image(prompt, format.mime_type(), data).await
    }

    /// Analyze multiple images
    pub async fn analyze_multiple_images(
        &self,
        prompt: impl Into<String>,
        images: Vec<(&str, Vec<u8>)>,
    ) -> Result<String, GeminiError> {
        let mut content = Content::user(prompt);

        for (mime_type, data) in images {
            content = content.with_image(mime_type, data);
        }

        let request = GenerateContentRequest {
            contents: vec![content],
            system_instruction: None,
            generation_config: self.generation_config.clone(),
            tools: None,
            tool_config: None,
            safety_settings: None,
        };

        let response = self.client.generate_content(request).await?;

        Ok(response
            .candidates
            .into_iter()
            .filter_map(|c| {
                c.content
                    .parts
                    .into_iter()
                    .filter_map(|p| match p {
                        Part::Text { text } => Some(text),
                        _ => None,
                    })
                    .next()
            })
            .collect::<Vec<_>>()
            .join(""))
    }

    /// OCR - Extract text from image
    pub async fn ocr(&self, mime_type: &str, image_data: Vec<u8>) -> Result<String, GeminiError> {
        self.analyze_image(
            "Extract all text visible in this image. Return only the text, no explanations.",
            mime_type,
            image_data,
        )
        .await
    }

    /// Describe image
    pub async fn describe(
        &self,
        mime_type: &str,
        image_data: Vec<u8>,
    ) -> Result<String, GeminiError> {
        self.analyze_image("Describe this image in detail.", mime_type, image_data)
            .await
    }
}

/// Extension trait for GeminiClient to add vision helpers
#[async_trait]
pub trait VisionExt {
    /// Create vision builder
    fn vision(&self) -> VisionBuilder<'_>;

    /// Quick image analysis
    async fn analyze_image(
        &self,
        prompt: impl Into<String> + Send,
        mime_type: &str,
        image_data: Vec<u8>,
    ) -> Result<String, GeminiError>;

    /// OCR shortcut
    async fn ocr(&self, mime_type: &str, image_data: Vec<u8>) -> Result<String, GeminiError>;
}

#[async_trait]
impl VisionExt for GeminiClient {
    fn vision(&self) -> VisionBuilder<'_> {
        VisionBuilder::new(self)
    }

    async fn analyze_image(
        &self,
        prompt: impl Into<String> + Send,
        mime_type: &str,
        image_data: Vec<u8>,
    ) -> Result<String, GeminiError> {
        VisionBuilder::new(self)
            .analyze_image(prompt, mime_type, image_data)
            .await
    }

    async fn ocr(&self, mime_type: &str, image_data: Vec<u8>) -> Result<String, GeminiError> {
        VisionBuilder::new(self).ocr(mime_type, image_data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_from_path() {
        assert_eq!(ImageFormat::from_path("image.png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_path("image.jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(
            ImageFormat::from_path("image.jpeg"),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_path("image.webp"),
            Some(ImageFormat::Webp)
        );
        assert_eq!(ImageFormat::from_path("image.txt"), None);
    }

    #[test]
    fn test_image_format_from_bytes() {
        let png_bytes = vec![0x89, 0x50, 0x4E, 0x47];
        assert_eq!(ImageFormat::from_bytes(&png_bytes), Some(ImageFormat::Png));

        let jpeg_bytes = vec![0xFF, 0xD8, 0xFF];
        assert_eq!(
            ImageFormat::from_bytes(&jpeg_bytes),
            Some(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
    }
}
