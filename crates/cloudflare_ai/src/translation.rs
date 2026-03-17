//! Translation API for Cloudflare Workers AI.

use crate::client::CloudflareAiClient;
use crate::error::Result;
use crate::types::TranslationResponse;

/// Client for the translation API.
#[derive(Debug)]
pub struct Translation<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> Translation<'a> {
    /// Create a new translation client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Translate text from one language to another.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, TranslationRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let request = TranslationRequest::new("Hello world", "english", "french");
    /// let response = client.translation().translate(request).await?;
    /// 
    /// if let Some(text) = response.text() {
    ///     println!("Translated: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn translate(&self, request: TranslationRequest) -> Result<TranslationResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: TranslationResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Translate text with automatic source language detection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.translation()
    ///     .auto_translate("Bonjour le monde", "english")
    ///     .await?;
    /// 
    /// if let Some(text) = response.text() {
    ///     println!("Translated: {}", text);
    /// }
    /// if let Some(lang) = response.detected_language {
    ///     println!("Detected language: {}", lang);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn auto_translate(
        &self,
        text: impl Into<String>,
        target_language: impl Into<String>,
    ) -> Result<TranslationResponse> {
        let request = TranslationRequest::auto(text, target_language);
        self.translate(request).await
    }

    /// Translate text from a specific source language.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.translation()
    ///     .from_to("Hello world", "english", "spanish")
    ///     .await?;
    /// 
    /// if let Some(text) = response.text() {
    ///     println!("Translated: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_to(
        &self,
        text: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
    ) -> Result<TranslationResponse> {
        let request = TranslationRequest::new(text, source_language, target_language);
        self.translate(request).await
    }
}

/// A translation request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslationRequest {
    /// ID of the model to use (default: "@cf/meta/m2m100-1.2b").
    #[serde(skip_serializing)]
    pub model: String,
    /// The text to translate.
    pub text: String,
    /// The source language (e.g., "english", "french", "german").
    /// If not provided, the model will auto-detect the language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lang: Option<String>,
    /// The target language (e.g., "english", "french", "german").
    pub target_lang: String,
}

impl TranslationRequest {
    /// Create a new translation request.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to translate
    /// * `source_language` - The source language name or code
    /// * `target_language` - The target language name or code
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::TranslationRequest;
    ///
    /// let request = TranslationRequest::new("Hello world", "english", "french");
    /// ```
    pub fn new(
        text: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
    ) -> Self {
        Self {
            model: "@cf/meta/m2m100-1.2b".to_string(),
            text: text.into(),
            source_lang: Some(source_language.into()),
            target_lang: target_language.into(),
        }
    }

    /// Create a new translation request with automatic source language detection.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to translate
    /// * `target_language` - The target language name or code
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::TranslationRequest;
    ///
    /// let request = TranslationRequest::auto("Bonjour le monde", "english");
    /// ```
    pub fn auto(text: impl Into<String>, target_language: impl Into<String>) -> Self {
        Self {
            model: "@cf/meta/m2m100-1.2b".to_string(),
            text: text.into(),
            source_lang: None,
            target_lang: target_language.into(),
        }
    }

    /// Create a builder for translation requests.
    pub fn builder() -> TranslationRequestBuilder {
        TranslationRequestBuilder::new()
    }

    /// Set a custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// Builder for translation requests.
#[derive(Debug, Clone, Default)]
pub struct TranslationRequestBuilder {
    model: String,
    text: Option<String>,
    source_lang: Option<String>,
    target_lang: Option<String>,
}

impl TranslationRequestBuilder {
    /// Create a new translation request builder.
    pub fn new() -> Self {
        Self {
            model: "@cf/meta/m2m100-1.2b".to_string(),
            text: None,
            source_lang: None,
            target_lang: None,
        }
    }

    /// Set the text to translate.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the source language.
    pub fn source_language(mut self, lang: impl Into<String>) -> Self {
        self.source_lang = Some(lang.into());
        self
    }

    /// Set the target language.
    pub fn target_language(mut self, lang: impl Into<String>) -> Self {
        self.target_lang = Some(lang.into());
        self
    }

    /// Set a custom model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the request.
    pub fn build(self) -> Result<TranslationRequest> {
        let text = self.text.ok_or_else(|| crate::error::CloudflareAiError::Config {
            message: "Text is required".to_string(),
        })?;
        let target_lang = self.target_lang.ok_or_else(|| crate::error::CloudflareAiError::Config {
            message: "Target language is required".to_string(),
        })?;

        Ok(TranslationRequest {
            model: self.model,
            text,
            source_lang: self.source_lang,
            target_lang,
        })
    }
}

/// Common language codes for translation.
pub mod languages {
    /// English
    pub const ENGLISH: &str = "english";
    /// French
    pub const FRENCH: &str = "french";
    /// German
    pub const GERMAN: &str = "german";
    /// Spanish
    pub const SPANISH: &str = "spanish";
    /// Italian
    pub const ITALIAN: &str = "italian";
    /// Portuguese
    pub const PORTUGUESE: &str = "portuguese";
    /// Dutch
    pub const DUTCH: &str = "dutch";
    /// Russian
    pub const RUSSIAN: &str = "russian";
    /// Chinese
    pub const CHINESE: &str = "chinese";
    /// Japanese
    pub const JAPANESE: &str = "japanese";
    /// Korean
    pub const KOREAN: &str = "korean";
    /// Arabic
    pub const ARABIC: &str = "arabic";
    /// Hindi
    pub const HINDI: &str = "hindi";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_request_new() {
        let req = TranslationRequest::new("Hello", "english", "french");
        assert_eq!(req.text, "Hello");
        assert_eq!(req.source_lang, Some("english".to_string()));
        assert_eq!(req.target_lang, "french");
        assert_eq!(req.model, "@cf/meta/m2m100-1.2b");
    }

    #[test]
    fn test_translation_request_auto() {
        let req = TranslationRequest::auto("Bonjour", "english");
        assert_eq!(req.text, "Bonjour");
        assert_eq!(req.source_lang, None);
        assert_eq!(req.target_lang, "english");
    }

    #[test]
    fn test_translation_request_with_model() {
        let req = TranslationRequest::new("Hello", "english", "french")
            .with_model("@cf/custom/model");
        assert_eq!(req.model, "@cf/custom/model");
    }

    #[test]
    fn test_builder() {
        let req = TranslationRequest::builder()
            .text("Hello world")
            .source_language("english")
            .target_language("spanish")
            .build()
            .unwrap();

        assert_eq!(req.text, "Hello world");
        assert_eq!(req.source_lang, Some("english".to_string()));
        assert_eq!(req.target_lang, "spanish");
    }

    #[test]
    fn test_builder_missing_text() {
        let result = TranslationRequest::builder()
            .target_language("spanish")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_translation_response() {
        let response: TranslationResponse = serde_json::from_str(
            r#"{
                "translated_text": "Bonjour le monde",
                "detected_language": "english"
            }"#
        ).unwrap();

        assert_eq!(response.text(), Some("Bonjour le monde"));
        assert_eq!(response.detected_language, Some("english".to_string()));
    }

    #[test]
    fn test_language_constants() {
        assert_eq!(languages::ENGLISH, "english");
        assert_eq!(languages::FRENCH, "french");
        assert_eq!(languages::SPANISH, "spanish");
    }
}
