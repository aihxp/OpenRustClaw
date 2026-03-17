//! Audio API for Azure OpenAI (Whisper and TTS).

use secrecy::ExposeSecret;

use crate::client::AzureOpenAIClient;
use crate::error::{AzureOpenAIError, Result};

/// Client for the audio API.
#[derive(Debug)]
pub struct Audio<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Audio<'a> {
    /// Create a new audio client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Transcribe audio to text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, TranscriptionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-whisper-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = TranscriptionRequest::new("audio.mp3");
    /// let response = client.audio().transcribe(request).await?;
    /// println!("{}", response.text);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionResponse> {
        let deployment = self.client.deployment_name().to_string();
        let path = format!("/openai/deployments/{}/audio/transcriptions", &deployment);

        let form = self.build_form(&request).await?;

        let response = self.client.post_multipart(&path, form).await?;
        let body = self.client.handle_response(response).await?;
        let transcription_response: TranscriptionResponse = serde_json::from_value(body)?;
        Ok(transcription_response)
    }

    /// Translate audio to English.
    pub async fn translate(&self, request: TranscriptionRequest) -> Result<TranscriptionResponse> {
        let deployment = self.client.deployment_name().to_string();
        let path = format!("/openai/deployments/{}/audio/translations", &deployment);

        let form = self.build_form(&request).await?;

        let response = self.client.post_multipart(&path, form).await?;
        let body = self.client.handle_response(response).await?;
        let translation_response: TranscriptionResponse = serde_json::from_value(body)?;
        Ok(translation_response)
    }

    /// Generate speech from text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, TtsRequest, TtsVoice};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-tts-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = TtsRequest::new("Hello, world!", TtsVoice::Alloy);
    /// let audio_bytes = client.audio().speech(request).await?;
    ///
    /// // Save to file
    /// std::fs::write("output.mp3", audio_bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn speech(&self, request: TtsRequest) -> Result<bytes::Bytes> {
        let body = serde_json::to_value(&request)?;
        let deployment = self.client.deployment_name().to_string();
        let path = format!("/openai/deployments/{}/audio/speech", &deployment);

        let mut http_request = self.client.http().post(self.client.build_url(&path));

        // Add authentication headers
        if self.client.is_azure_ad() {
            let auth_header = self.client.config().authorization_header().await?;
            http_request = http_request.header("Authorization", auth_header);
        } else {
            if let crate::AzureCredential::ApiKey(key) = &self.client.config().credential {
                http_request = http_request.header("api-key", key.expose_secret());
            }
        }

        let response = http_request
            .json(&body)
            .send()
            .await
            .map_err(AzureOpenAIError::from)?;

        if !response.status().is_success() {
            return Err(AzureOpenAIError::from_response(response).await);
        }

        response.bytes().await.map_err(AzureOpenAIError::from)
    }

    /// Build multipart form for audio upload.
    async fn build_form(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();

        // Read the file
        let file_bytes = tokio::fs::read(&request.file_path)
            .await
            .map_err(|e| AzureOpenAIError::Config {
                message: format!("Failed to read audio file: {e}"),
            })?;

        let file_name = std::path::Path::new(&request.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3");

        let mime_type = request
            .file_format
            .as_ref()
            .map(|f| match f.as_str() {
                "mp3" => "audio/mpeg",
                "mp4" => "audio/mp4",
                "mpeg" => "audio/mpeg",
                "mpga" => "audio/mpeg",
                "m4a" => "audio/mp4",
                "wav" => "audio/wav",
                "webm" => "audio/webm",
                "ogg" => "audio/ogg",
                _ => "audio/mpeg",
            })
            .unwrap_or("audio/mpeg");

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str(mime_type)
            .map_err(|e| AzureOpenAIError::Config {
                message: format!("Failed to set MIME type: {e}"),
            })?;

        form = form.part("file", part);

        if let Some(model) = &request.model {
            form = form.text("model", model.clone());
        }

        if let Some(language) = &request.language {
            form = form.text("language", language.clone());
        }

        if let Some(prompt) = &request.prompt {
            form = form.text("prompt", prompt.clone());
        }

        if let Some(format) = &request.response_format {
            form = form.text("response_format", format.clone());
        }

        if let Some(temperature) = request.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        if let Some(granularities) = &request.timestamp_granularities {
            for g in granularities {
                form = form.text("timestamp_granularities[]", g.clone());
            }
        }

        Ok(form)
    }
}

/// A transcription request.
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// The path to the audio file.
    pub file_path: String,
    /// The format of the audio file.
    pub file_format: Option<String>,
    /// The model to use.
    pub model: Option<String>,
    /// The language of the audio.
    pub language: Option<String>,
    /// An optional prompt to guide the model.
    pub prompt: Option<String>,
    /// The format of the response.
    pub response_format: Option<String>,
    /// The sampling temperature.
    pub temperature: Option<f32>,
    /// The timestamp granularities.
    pub timestamp_granularities: Option<Vec<String>>,
}

impl TranscriptionRequest {
    /// Create a new transcription request.
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            file_format: None,
            model: None,
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
            timestamp_granularities: None,
        }
    }

    /// Set the file format.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.file_format = Some(format.into());
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the language (ISO-639-1 code).
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
    pub fn response_format(mut self, format: AudioResponseFormat) -> Self {
        self.response_format = Some(format.as_str().to_string());
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    /// Add timestamp granularity.
    pub fn timestamp_granularity(mut self, granularity: TimestampGranularity) -> Self {
        self.timestamp_granularities
            .get_or_insert_with(Vec::new)
            .push(granularity.as_str().to_string());
        self
    }
}

/// Audio response format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioResponseFormat {
    /// JSON format.
    Json,
    /// Text format.
    Text,
    /// SRT subtitle format.
    Srt,
    /// Verbose JSON format.
    VerboseJson,
    /// VTT subtitle format.
    Vtt,
}

impl AudioResponseFormat {
    /// Get the format string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioResponseFormat::Json => "json",
            AudioResponseFormat::Text => "text",
            AudioResponseFormat::Srt => "srt",
            AudioResponseFormat::VerboseJson => "verbose_json",
            AudioResponseFormat::Vtt => "vtt",
        }
    }
}

/// Timestamp granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimestampGranularity {
    /// Word-level timestamps.
    Word,
    /// Segment-level timestamps.
    Segment,
}

impl TimestampGranularity {
    /// Get the granularity string.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimestampGranularity::Word => "word",
            TimestampGranularity::Segment => "segment",
        }
    }
}

/// A transcription response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionResponse {
    /// The transcribed text.
    pub text: String,
    /// The task type (transcribe or translate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// The language detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The duration of the audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Word-level timestamps (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<WordTimestamp>>,
    /// Segment-level timestamps (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<Segment>>,
}

/// A word timestamp.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WordTimestamp {
    /// The word.
    pub word: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

/// A transcription segment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Segment {
    /// Segment ID.
    pub id: i32,
    /// Seek offset.
    pub seek: i32,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// The transcribed text.
    pub text: String,
    /// Token IDs.
    pub tokens: Vec<i32>,
    /// Temperature used.
    pub temperature: f32,
    /// Average log probability.
    pub avg_logprob: f64,
    /// Compression ratio.
    pub compression_ratio: f64,
    /// No speech probability.
    pub no_speech_prob: f64,
}

/// A TTS request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TtsRequest {
    /// The text to convert to speech.
    pub input: String,
    /// The voice to use.
    pub voice: String,
    /// The response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// The speed of the speech.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// The model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl TtsRequest {
    /// Create a new TTS request.
    pub fn new(input: impl Into<String>, voice: TtsVoice) -> Self {
        Self {
            input: input.into(),
            voice: voice.as_str().to_string(),
            response_format: None,
            speed: None,
            model: None,
        }
    }

    /// Set the response format.
    pub fn response_format(mut self, format: TtsResponseFormat) -> Self {
        self.response_format = Some(format.as_str().to_string());
        self
    }

    /// Set the speed.
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed.clamp(0.25, 4.0));
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// TTS voice options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TtsVoice {
    /// Alloy (neutral, versatile).
    Alloy,
    /// Echo (neutral, versatile).
    Echo,
    /// Fable (neutral, versatile).
    Fable,
    /// Onyx (neutral, versatile).
    Onyx,
    /// Nova (neutral, versatile).
    Nova,
    /// Shimmer (neutral, versatile).
    Shimmer,
}

impl TtsVoice {
    /// Get the voice string.
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsVoice::Alloy => "alloy",
            TtsVoice::Echo => "echo",
            TtsVoice::Fable => "fable",
            TtsVoice::Onyx => "onyx",
            TtsVoice::Nova => "nova",
            TtsVoice::Shimmer => "shimmer",
        }
    }
}

/// TTS response format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TtsResponseFormat {
    /// MP3 format.
    Mp3,
    /// Opus format.
    Opus,
    /// AAC format.
    Aac,
    /// FLAC format.
    Flac,
    /// WAV format.
    Wav,
    /// PCM format.
    Pcm,
}

impl TtsResponseFormat {
    /// Get the format string.
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsResponseFormat::Mp3 => "mp3",
            TtsResponseFormat::Opus => "opus",
            TtsResponseFormat::Aac => "aac",
            TtsResponseFormat::Flac => "flac",
            TtsResponseFormat::Wav => "wav",
            TtsResponseFormat::Pcm => "pcm",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_request_builder() {
        let request = TranscriptionRequest::new("audio.mp3")
            .format("mp3")
            .language("en")
            .temperature(0.5)
            .response_format(AudioResponseFormat::VerboseJson)
            .timestamp_granularity(TimestampGranularity::Word);

        assert_eq!(request.file_path, "audio.mp3");
        assert_eq!(request.file_format, Some("mp3".to_string()));
        assert_eq!(request.language, Some("en".to_string()));
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.response_format, Some("verbose_json".to_string()));
    }

    #[test]
    fn test_tts_request_builder() {
        let request = TtsRequest::new("Hello, world!", TtsVoice::Alloy)
            .response_format(TtsResponseFormat::Mp3)
            .speed(1.0);

        assert_eq!(request.input, "Hello, world!");
        assert_eq!(request.voice, "alloy");
        assert_eq!(request.response_format, Some("mp3".to_string()));
    }

    #[test]
    fn test_audio_response_format() {
        assert_eq!(AudioResponseFormat::Json.as_str(), "json");
        assert_eq!(AudioResponseFormat::VerboseJson.as_str(), "verbose_json");
    }

    #[test]
    fn test_tts_voice() {
        assert_eq!(TtsVoice::Alloy.as_str(), "alloy");
        assert_eq!(TtsVoice::Nova.as_str(), "nova");
    }

    #[test]
    fn test_tts_response_format() {
        assert_eq!(TtsResponseFormat::Mp3.as_str(), "mp3");
        assert_eq!(TtsResponseFormat::Flac.as_str(), "flac");
    }
}
