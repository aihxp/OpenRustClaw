//! Speech-to-text (STT) module.
//!
//! This module provides speech-to-text capabilities.

use crate::error::{VoiceError, VoiceResult};
use serde::{Deserialize, Serialize};

/// Configuration for speech-to-text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Language code (e.g., "en", "es", "fr").
    #[serde(default = "default_language")]
    pub language: String,
    /// Path to the model file (if applicable).
    pub model_path: Option<String>,
    /// Whether to use GPU acceleration if available.
    #[serde(default = "default_true")]
    pub use_gpu: bool,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            model_path: None,
            use_gpu: default_true(),
        }
    }
}

fn default_language() -> String {
    "en".to_string()
}

fn default_true() -> bool {
    true
}

/// A single segment of transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Speaker ID if diarization is enabled.
    pub speaker: Option<String>,
}

/// Transcription result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    /// Full transcribed text.
    pub text: String,
    /// Overall confidence score.
    pub confidence: f32,
    /// Individual segments.
    pub segments: Vec<Segment>,
    /// Language detected/provided.
    pub language: String,
    /// Duration of audio in seconds.
    pub duration_secs: f64,
    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

/// Speech-to-text engine.
pub struct SpeechToText {
    config: SttConfig,
}

impl SpeechToText {
    /// Creates a new speech-to-text engine.
    pub fn new(config: SttConfig) -> VoiceResult<Self> {
        Ok(Self { config })
    }

    /// Transcribes audio samples to text.
    ///
    /// The audio should be 16kHz mono f32 samples normalized to [-1.0, 1.0].
    pub fn transcribe(&self, audio: &[f32]) -> VoiceResult<Transcription> {
        if audio.is_empty() {
            return Err(VoiceError::InvalidFormat("Empty audio buffer".to_string()));
        }

        // Placeholder implementation
        Ok(Transcription {
            text: "Placeholder transcription".to_string(),
            confidence: 0.95,
            segments: vec![],
            language: self.config.language.clone(),
            duration_secs: audio.len() as f64 / 16_000.0,
            processing_time_ms: 100,
        })
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &SttConfig {
        &self.config
    }
}

/// Streaming STT for real-time transcription.
pub struct StreamingStt {
    stt: SpeechToText,
    buffer: Vec<f32>,
}

impl StreamingStt {
    /// Creates a new streaming STT instance.
    pub fn new(stt: SpeechToText) -> Self {
        Self {
            stt,
            buffer: Vec::new(),
        }
    }

    /// Adds audio samples to the buffer.
    pub fn feed(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
    }

    /// Attempts to transcribe if enough audio is buffered.
    ///
    /// Returns `Ok(None)` if not enough audio yet.
    pub fn try_transcribe(&self) -> VoiceResult<Option<Transcription>> {
        if self.buffer.len() < 16000 {
            return Ok(None);
        }

        let result = self.stt.transcribe(&self.buffer)?;
        Ok(Some(result))
    }

    /// Clears the audio buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Returns the current buffer duration in seconds.
    pub fn buffer_duration_secs(&self) -> f64 {
        self.buffer.len() as f64 / 16_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stt_config_default() {
        let config = SttConfig::default();
        assert_eq!(config.language, "en");
        assert!(config.use_gpu);
    }

    #[test]
    fn test_transcribe_empty() {
        let stt = SpeechToText::new(SttConfig::default()).unwrap();
        let result = stt.transcribe(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_transcribe_placeholder() {
        let stt = SpeechToText::new(SttConfig::default()).unwrap();
        let audio = vec![0.0f32; 16000];
        let result = stt.transcribe(&audio).unwrap();
        assert!(!result.text.is_empty());
        assert_eq!(result.language, "en");
    }
}
