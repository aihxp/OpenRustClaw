//! Audio API for Groq (Whisper transcription and translation).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::client::GroqClient;
use crate::client::endpoints;
use crate::error::{GroqError, Result};

/// Client for the audio API.
#[derive(Debug)]
pub struct Audio<'a> {
    client: &'a GroqClient,
}

impl<'a> Audio<'a> {
    /// Create a new audio client.
    pub fn new(client: &'a GroqClient) -> Self {
        Self { client }
    }

    /// Transcribe audio to text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use groq::{GroqClient, AudioTranscriptionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GroqClient::new("your-api-key")?;
    ///
    /// let request = AudioTranscriptionRequest::builder()
    ///     .file_path("/path/to/audio.mp3").await?
    ///     .model("whisper-large-v3")
    ///     .build()?;
    ///
    /// let response = client.audio().transcribe(request).await?;
    /// println!("Transcription: {}", response.text);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transcribe(&self, request: AudioTranscriptionRequest) -> Result<TranscriptionResponse> {
        let form = request.build_form()?;
        
        let response = self.client.post_multipart(endpoints::AUDIO_TRANSCRIPTIONS, form).await?;
        
        if response.status().is_success() {
            let body = response.text().await.map_err(GroqError::from)?;
            let transcription: TranscriptionResponse = serde_json::from_str(&body)?;
            Ok(transcription)
        } else {
            Err(GroqError::from_response(response).await)
        }
    }

    /// Transcribe audio with verbose output (includes segments).
    pub async fn transcribe_verbose(&self, mut request: AudioTranscriptionRequest) -> Result<TranscriptionVerboseResponse> {
        request.response_format = Some("verbose_json".to_string());
        let form = request.build_form()?;
        
        let response = self.client.post_multipart(endpoints::AUDIO_TRANSCRIPTIONS, form).await?;
        
        if response.status().is_success() {
            let body = response.text().await.map_err(GroqError::from)?;
            let transcription: TranscriptionVerboseResponse = serde_json::from_str(&body)?;
            Ok(transcription)
        } else {
            Err(GroqError::from_response(response).await)
        }
    }

    /// Translate audio to English text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use groq::{GroqClient, AudioTranslationRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GroqClient::new("your-api-key")?;
    ///
    /// let request = AudioTranslationRequest::builder()
    ///     .file_path("/path/to/audio.mp3").await?
    ///     .model("whisper-large-v3")
    ///     .build()?;
    ///
    /// let response = client.audio().translate(request).await?;
    /// println!("Translation: {}", response.text);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn translate(&self, request: AudioTranslationRequest) -> Result<TranslationResponse> {
        let form = request.build_form()?;
        
        let response = self.client.post_multipart(endpoints::AUDIO_TRANSLATIONS, form).await?;
        
        if response.status().is_success() {
            let body = response.text().await.map_err(GroqError::from)?;
            let translation: TranslationResponse = serde_json::from_str(&body)?;
            Ok(translation)
        } else {
            Err(GroqError::from_response(response).await)
        }
    }
}

/// An audio transcription request.
#[derive(Debug, Clone)]
pub struct AudioTranscriptionRequest {
    /// The audio file to transcribe.
    pub file: Vec<u8>,
    /// The filename of the audio file.
    pub filename: String,
    /// ID of the model to use (whisper-large-v3).
    pub model: String,
    /// The language of the input audio (ISO-639-1 format).
    pub language: Option<String>,
    /// An optional text to guide the model's style or continue a previous audio segment.
    pub prompt: Option<String>,
    /// The format of the transcript output (json, text, srt, verbose_json, or vtt).
    pub response_format: Option<String>,
    /// The sampling temperature.
    pub temperature: Option<f32>,
    /// The timestamp granularities to populate (word or segment).
    pub timestamp_granularities: Option<Vec<String>>,
}

impl AudioTranscriptionRequest {
    /// Create a new transcription request builder.
    pub fn builder() -> AudioTranscriptionRequestBuilder {
        AudioTranscriptionRequestBuilder::new()
    }

    /// Build the multipart form for the request.
    pub(crate) fn build_form(&self) -> Result<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();

        // Add the file
        let file_part = reqwest::multipart::Part::bytes(self.file.clone())
            .file_name(self.filename.clone())
            .mime_str(&guess_mime_type(&self.filename))?;
        form = form.part("file", file_part);

        // Add required model field
        form = form.text("model", self.model.clone());

        // Add optional fields
        if let Some(language) = &self.language {
            form = form.text("language", language.clone());
        }
        if let Some(prompt) = &self.prompt {
            form = form.text("prompt", prompt.clone());
        }
        if let Some(response_format) = &self.response_format {
            form = form.text("response_format", response_format.clone());
        }
        if let Some(temperature) = self.temperature {
            form = form.text("temperature", temperature.to_string());
        }
        if let Some(granularities) = &self.timestamp_granularities {
            for granularity in granularities {
                form = form.text("timestamp_granularities[]", granularity.clone());
            }
        }

        Ok(form)
    }
}

/// Builder for audio transcription requests.
#[derive(Debug, Clone)]
pub struct AudioTranscriptionRequestBuilder {
    file: Option<Vec<u8>>,
    filename: Option<String>,
    model: String,
    language: Option<String>,
    prompt: Option<String>,
    response_format: Option<String>,
    temperature: Option<f32>,
    timestamp_granularities: Option<Vec<String>>,
}

impl AudioTranscriptionRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            file: None,
            filename: None,
            model: "whisper-large-v3".to_string(),
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
            timestamp_granularities: None,
        }
    }

    /// Set the audio file from bytes.
    pub fn file(mut self, bytes: Vec<u8>, filename: impl Into<String>) -> Self {
        self.file = Some(bytes);
        self.filename = Some(filename.into());
        self
    }

    /// Set the audio file from a path.
    pub async fn file_path(self, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = tokio::fs::read(path).await?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3")
            .to_string();
        Ok(self.file(bytes, filename))
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the language (ISO-639-1 format).
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: impl Into<String>) -> Self {
        self.response_format = Some(format.into());
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    /// Add a timestamp granularity (word or segment).
    pub fn timestamp_granularity(mut self, granularity: impl Into<String>) -> Self {
        self.timestamp_granularities
            .get_or_insert_with(Vec::new)
            .push(granularity.into());
        self
    }

    /// Build the request.
    pub fn build(self) -> Result<AudioTranscriptionRequest> {
        let file = self.file.ok_or_else(|| GroqError::Audio {
            message: "Audio file is required".to_string(),
        })?;
        let filename = self.filename.ok_or_else(|| GroqError::Audio {
            message: "Filename is required".to_string(),
        })?;

        Ok(AudioTranscriptionRequest {
            file,
            filename,
            model: self.model,
            language: self.language,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
            timestamp_granularities: self.timestamp_granularities,
        })
    }
}

impl Default for AudioTranscriptionRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An audio translation request.
#[derive(Debug, Clone)]
pub struct AudioTranslationRequest {
    /// The audio file to translate.
    pub file: Vec<u8>,
    /// The filename of the audio file.
    pub filename: String,
    /// ID of the model to use (whisper-large-v3).
    pub model: String,
    /// An optional text to guide the model's style or continue a previous audio segment.
    pub prompt: Option<String>,
    /// The format of the transcript output (json, text, srt, verbose_json, or vtt).
    pub response_format: Option<String>,
    /// The sampling temperature.
    pub temperature: Option<f32>,
}

impl AudioTranslationRequest {
    /// Create a new translation request builder.
    pub fn builder() -> AudioTranslationRequestBuilder {
        AudioTranslationRequestBuilder::new()
    }

    /// Build the multipart form for the request.
    pub(crate) fn build_form(&self) -> Result<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();

        // Add the file
        let file_part = reqwest::multipart::Part::bytes(self.file.clone())
            .file_name(self.filename.clone())
            .mime_str(&guess_mime_type(&self.filename))?;
        form = form.part("file", file_part);

        // Add required model field
        form = form.text("model", self.model.clone());

        // Add optional fields
        if let Some(prompt) = &self.prompt {
            form = form.text("prompt", prompt.clone());
        }
        if let Some(response_format) = &self.response_format {
            form = form.text("response_format", response_format.clone());
        }
        if let Some(temperature) = self.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        Ok(form)
    }
}

/// Builder for audio translation requests.
#[derive(Debug, Clone)]
pub struct AudioTranslationRequestBuilder {
    file: Option<Vec<u8>>,
    filename: Option<String>,
    model: String,
    prompt: Option<String>,
    response_format: Option<String>,
    temperature: Option<f32>,
}

impl AudioTranslationRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            file: None,
            filename: None,
            model: "whisper-large-v3".to_string(),
            prompt: None,
            response_format: None,
            temperature: None,
        }
    }

    /// Set the audio file from bytes.
    pub fn file(mut self, bytes: Vec<u8>, filename: impl Into<String>) -> Self {
        self.file = Some(bytes);
        self.filename = Some(filename.into());
        self
    }

    /// Set the audio file from a path.
    pub async fn file_path(self, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = tokio::fs::read(path).await?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3")
            .to_string();
        Ok(self.file(bytes, filename))
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the prompt.
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: impl Into<String>) -> Self {
        self.response_format = Some(format.into());
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    /// Build the request.
    pub fn build(self) -> Result<AudioTranslationRequest> {
        let file = self.file.ok_or_else(|| GroqError::Audio {
            message: "Audio file is required".to_string(),
        })?;
        let filename = self.filename.ok_or_else(|| GroqError::Audio {
            message: "Filename is required".to_string(),
        })?;

        Ok(AudioTranslationRequest {
            file,
            filename,
            model: self.model,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
        })
    }
}

impl Default for AudioTranslationRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from a transcription request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    /// The transcribed text.
    pub text: String,
}

/// Response from a transcription request with verbose output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionVerboseResponse {
    /// The transcribed text.
    pub text: String,
    /// The language detected.
    pub language: String,
    /// The duration of the audio in seconds.
    pub duration: f64,
    /// The segments of the transcription.
    pub segments: Vec<AudioSegment>,
    /// Words with timestamps (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<WordSegment>>,
}

/// A segment of audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSegment {
    /// Unique identifier for the segment.
    pub id: i64,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// The transcribed text.
    pub text: String,
    /// Token IDs (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<i64>>,
    /// Temperature used (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Average logprob (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprob: Option<f64>,
    /// Compression ratio (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,
    /// No speech probability (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_speech_prob: Option<f64>,
}

/// A word segment with timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordSegment {
    /// The word.
    pub word: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

/// Response from a translation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResponse {
    /// The translated text.
    pub text: String,
}

/// Guess the MIME type from a filename.
fn guess_mime_type(filename: &str) -> String {
    let ext = filename
        .split('.')
        .last()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "mp3" => "audio/mpeg",
        "mp4" => "audio/mp4",
        "mpeg" => "audio/mpeg",
        "mpga" => "audio/mpeg",
        "m4a" => "audio/m4a",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "amr" => "audio/amr",
        _ => "audio/mpeg",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type("audio.mp3"), "audio/mpeg");
        assert_eq!(guess_mime_type("audio.wav"), "audio/wav");
        assert_eq!(guess_mime_type("audio.ogg"), "audio/ogg");
        assert_eq!(guess_mime_type("audio.flac"), "audio/flac");
        assert_eq!(guess_mime_type("audio.unknown"), "audio/mpeg");
    }

    #[test]
    fn test_transcription_request_builder() {
        let request = AudioTranscriptionRequest::builder()
            .file(vec![1, 2, 3], "audio.mp3")
            .model("whisper-large-v3")
            .language("en")
            .temperature(0.5)
            .build()
            .unwrap();

        assert_eq!(request.filename, "audio.mp3");
        assert_eq!(request.model, "whisper-large-v3");
        assert_eq!(request.language, Some("en".to_string()));
        assert_eq!(request.temperature, Some(0.5));
    }

    #[test]
    fn test_translation_request_builder() {
        let request = AudioTranslationRequest::builder()
            .file(vec![1, 2, 3], "audio.mp3")
            .model("whisper-large-v3")
            .response_format("json")
            .build()
            .unwrap();

        assert_eq!(request.filename, "audio.mp3");
        assert_eq!(request.model, "whisper-large-v3");
        assert_eq!(request.response_format, Some("json".to_string()));
    }

    #[test]
    fn test_transcription_response() {
        let response: TranscriptionResponse = serde_json::from_str(r#"{"text": "Hello world"}"#).unwrap();
        assert_eq!(response.text, "Hello world");
    }

    #[test]
    fn test_verbose_transcription_response() {
        let json = r#"{
            "text": "Hello world",
            "language": "en",
            "duration": 2.5,
            "segments": [
                {
                    "id": 0,
                    "start": 0.0,
                    "end": 2.5,
                    "text": "Hello world"
                }
            ]
        }"#;
        
        let response: TranscriptionVerboseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.text, "Hello world");
        assert_eq!(response.language, "en");
        assert_eq!(response.duration, 2.5);
        assert_eq!(response.segments.len(), 1);
    }
}
