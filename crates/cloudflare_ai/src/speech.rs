//! Speech recognition API for Cloudflare Workers AI.

use std::path::Path;

use crate::client::CloudflareAiClient;
use crate::error::{CloudflareAiError, Result};
use crate::types::SpeechRecognitionResponse;

/// Client for the speech recognition API.
#[derive(Debug)]
pub struct Speech<'a> {
    client: &'a CloudflareAiClient,
}

impl<'a> Speech<'a> {
    /// Create a new speech recognition client.
    pub fn new(client: &'a CloudflareAiClient) -> Self {
        Self { client }
    }

    /// Transcribe audio to text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::{CloudflareAiClient, SpeechRecognitionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// // Using audio bytes
    /// let audio_bytes = std::fs::read("audio.mp3")?;
    /// let request = SpeechRecognitionRequest::new(audio_bytes);
    /// let response = client.speech().transcribe(request).await?;
    ///
    /// if let Some(text) = response.text() {
    ///     println!("Transcription: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transcribe(
        &self,
        request: SpeechRecognitionRequest,
    ) -> Result<SpeechRecognitionResponse> {
        let model = request.model.clone();
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let model = model.clone();
                Box::pin(async move {
                    let response = client.post(&model, body).await?;
                    let result: SpeechRecognitionResponse = client.handle_response(response).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Transcribe audio from a file path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.speech()
    ///     .transcribe_file("audio.mp3")
    ///     .await?;
    ///
    /// if let Some(text) = response.text() {
    ///     println!("Transcription: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transcribe_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<SpeechRecognitionResponse> {
        let bytes = tokio::fs::read(path).await?;
        let request = SpeechRecognitionRequest::new(bytes);
        self.transcribe(request).await
    }

    /// Transcribe audio with a specific language hint.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::CloudflareAiClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CloudflareAiClient::new("your-account-id", "your-api-token")?;
    ///
    /// let response = client.speech()
    ///     .transcribe_with_language("audio.mp3", "en")
    ///     .await?;
    ///
    /// if let Some(text) = response.text() {
    ///     println!("Transcription: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transcribe_with_language(
        &self,
        path: impl AsRef<Path>,
        language: impl Into<String>,
    ) -> Result<SpeechRecognitionResponse> {
        let bytes = tokio::fs::read(path).await?;
        let request = SpeechRecognitionRequest::new(bytes).with_language(language);
        self.transcribe(request).await
    }
}

/// A speech recognition request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeechRecognitionRequest {
    /// ID of the model to use (default: "@cf/openai/whisper").
    #[serde(skip_serializing)]
    pub model: String,
    /// The audio data as bytes (will be base64 encoded).
    #[serde(with = "base64_serde")]
    pub audio: Vec<u8>,
    /// Optional language code (e.g., "en", "es", "fr").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Optional task type ("transcribe" or "translate").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
}

impl SpeechRecognitionRequest {
    /// Create a new speech recognition request.
    ///
    /// # Arguments
    ///
    /// * `audio` - The audio data as bytes
    ///
    /// # Example
    ///
    /// ```
    /// use cloudflare_ai::SpeechRecognitionRequest;
    ///
    /// let audio_bytes = vec![/* audio data */];
    /// let request = SpeechRecognitionRequest::new(audio_bytes);
    /// ```
    pub fn new(audio: Vec<u8>) -> Self {
        Self {
            model: "@cf/openai/whisper".to_string(),
            audio,
            language: None,
            task: None,
        }
    }

    /// Create a request from a file path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cloudflare_ai::SpeechRecognitionRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = SpeechRecognitionRequest::from_file("audio.mp3").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        Ok(Self::new(bytes))
    }

    /// Create a builder for speech recognition requests.
    pub fn builder() -> SpeechRecognitionRequestBuilder {
        SpeechRecognitionRequestBuilder::new()
    }

    /// Set a custom model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the language code.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the task to translation (translates to English).
    pub fn translate_to_english(mut self) -> Self {
        self.task = Some("translate".to_string());
        self
    }
}

/// Builder for speech recognition requests.
#[derive(Debug, Clone, Default)]
pub struct SpeechRecognitionRequestBuilder {
    model: String,
    audio: Option<Vec<u8>>,
    language: Option<String>,
    task: Option<String>,
}

impl SpeechRecognitionRequestBuilder {
    /// Create a new speech recognition request builder.
    pub fn new() -> Self {
        Self {
            model: "@cf/openai/whisper".to_string(),
            audio: None,
            language: None,
            task: None,
        }
    }

    /// Set the audio data.
    pub fn audio(mut self, audio: Vec<u8>) -> Self {
        self.audio = Some(audio);
        self
    }

    /// Set the audio from a file path.
    pub async fn audio_file(self, path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        Ok(self.audio(bytes))
    }

    /// Set the language code.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the task to transcription.
    pub fn transcribe(mut self) -> Self {
        self.task = Some("transcribe".to_string());
        self
    }

    /// Set the task to translation (to English).
    pub fn translate(mut self) -> Self {
        self.task = Some("translate".to_string());
        self
    }

    /// Set a custom model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the request.
    pub fn build(self) -> Result<SpeechRecognitionRequest> {
        let audio = self.audio.ok_or_else(|| CloudflareAiError::Audio {
            message: "Audio is required".to_string(),
        })?;

        Ok(SpeechRecognitionRequest {
            model: self.model,
            audio,
            language: self.language,
            task: self.task,
        })
    }
}

/// Supported audio formats.
pub mod formats {
    /// MP3
    pub const MP3: &str = "mp3";
    /// MP4
    pub const MP4: &str = "mp4";
    /// MPEG
    pub const MPEG: &str = "mpeg";
    /// MPGA
    pub const MPGA: &str = "mpga";
    /// M4A
    pub const M4A: &str = "m4a";
    /// WAV
    pub const WAV: &str = "wav";
    /// WEBM
    pub const WEBM: &str = "webm";
    /// OGG
    pub const OGG: &str = "ogg";
    /// FLAC
    pub const FLAC: &str = "flac";
}

/// Common language codes.
pub mod languages {
    /// English
    pub const ENGLISH: &str = "en";
    /// Spanish
    pub const SPANISH: &str = "es";
    /// French
    pub const FRENCH: &str = "fr";
    /// German
    pub const GERMAN: &str = "de";
    /// Italian
    pub const ITALIAN: &str = "it";
    /// Portuguese
    pub const PORTUGUESE: &str = "pt";
    /// Dutch
    pub const DUTCH: &str = "nl";
    /// Russian
    pub const RUSSIAN: &str = "ru";
    /// Chinese
    pub const CHINESE: &str = "zh";
    /// Japanese
    pub const JAPANESE: &str = "ja";
    /// Korean
    pub const KOREAN: &str = "ko";
    /// Arabic
    pub const ARABIC: &str = "ar";
    /// Hindi
    pub const HINDI: &str = "hi";
}

/// Module for base64 serialization of audio bytes.
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
    fn test_speech_recognition_request() {
        let bytes = vec![1, 2, 3, 4];
        let req = SpeechRecognitionRequest::new(bytes);
        assert_eq!(req.model, "@cf/openai/whisper");
        assert_eq!(req.audio, vec![1, 2, 3, 4]);
        assert_eq!(req.language, None);
        assert_eq!(req.task, None);
    }

    #[test]
    fn test_speech_recognition_with_options() {
        let req = SpeechRecognitionRequest::new(vec![1, 2, 3])
            .with_language("es")
            .translate_to_english();

        assert_eq!(req.language, Some("es".to_string()));
        assert_eq!(req.task, Some("translate".to_string()));
    }

    #[test]
    fn test_speech_recognition_with_model() {
        let req = SpeechRecognitionRequest::new(vec![1, 2, 3])
            .with_model("@cf/custom/model");
        assert_eq!(req.model, "@cf/custom/model");
    }

    #[test]
    fn test_builder() {
        let req = SpeechRecognitionRequest::builder()
            .audio(vec![1, 2, 3, 4])
            .language("en")
            .transcribe()
            .build()
            .unwrap();

        assert_eq!(req.audio, vec![1, 2, 3, 4]);
        assert_eq!(req.language, Some("en".to_string()));
        assert_eq!(req.task, Some("transcribe".to_string()));
    }

    #[test]
    fn test_builder_translate() {
        let req = SpeechRecognitionRequest::builder()
            .audio(vec![1, 2, 3, 4])
            .language("fr")
            .translate()
            .build()
            .unwrap();

        assert_eq!(req.task, Some("translate".to_string()));
    }

    #[test]
    fn test_builder_missing_audio() {
        let result = SpeechRecognitionRequest::builder()
            .language("en")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_speech_recognition_response() {
        let response: SpeechRecognitionResponse = serde_json::from_str(
            r#"{
                "text": "Hello world",
                "word_count": 2,
                "language": "en",
                "duration": 3.5
            }"#
        ).unwrap();

        assert_eq!(response.text(), Some("Hello world"));
        assert_eq!(response.word_count, Some(2));
        assert_eq!(response.language, Some("en".to_string()));
        assert_eq!(response.duration, Some(3.5));
    }

    #[test]
    fn test_language_constants() {
        assert_eq!(languages::ENGLISH, "en");
        assert_eq!(languages::SPANISH, "es");
        assert_eq!(languages::FRENCH, "fr");
    }

    #[test]
    fn test_format_constants() {
        assert_eq!(formats::MP3, "mp3");
        assert_eq!(formats::WAV, "wav");
        assert_eq!(formats::FLAC, "flac");
    }
}
