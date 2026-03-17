//! Text-to-speech (TTS) module.
//!
//! This module provides text-to-speech capabilities.

use crate::error::{VoiceError, VoiceResult};
use serde::{Deserialize, Serialize};

/// Configuration for text-to-speech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// The TTS backend to use.
    #[serde(default)]
    pub backend: TtsBackend,
    /// Default voice to use (backend-specific).
    pub voice: Option<String>,
    /// Speech rate/speed (1.0 = normal).
    #[serde(default = "default_rate")]
    pub rate: f32,
    /// Speech volume (0.0 - 1.0).
    #[serde(default = "default_volume")]
    pub volume: f32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            backend: TtsBackend::default(),
            voice: None,
            rate: default_rate(),
            volume: default_volume(),
        }
    }
}

fn default_rate() -> f32 {
    1.0
}

fn default_volume() -> f32 {
    1.0
}

/// TTS backend selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TtsBackend {
    /// System TTS (platform-native).
    System,
    /// ElevenLabs API.
    ElevenLabs {
        /// API key for ElevenLabs.
        api_key: String,
        /// Voice ID to use.
        voice_id: String,
    },
}

impl Default for TtsBackend {
    fn default() -> Self {
        TtsBackend::System
    }
}

/// ElevenLabs voice settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsVoiceSettings {
    /// Stability (0.0 - 1.0).
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// Similarity boost (0.0 - 1.0).
    #[serde(default = "default_similarity")]
    pub similarity_boost: f32,
}

impl Default for ElevenLabsVoiceSettings {
    fn default() -> Self {
        Self {
            stability: default_stability(),
            similarity_boost: default_similarity(),
        }
    }
}

fn default_stability() -> f32 {
    0.5
}

fn default_similarity() -> f32 {
    0.75
}

/// Information about a voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Voice ID.
    pub id: String,
    /// Voice name.
    pub name: String,
    /// Supported language.
    pub language: Option<String>,
    /// Voice gender.
    pub gender: Option<String>,
    /// URL to preview audio.
    pub preview_url: Option<String>,
}

/// Text-to-speech engine.
#[derive(Clone)]
pub struct TextToSpeech {
    config: TtsConfig,
}

impl TextToSpeech {
    /// Creates a new TTS engine.
    pub async fn new(config: TtsConfig) -> VoiceResult<Self> {
        Ok(Self { config })
    }

    /// Synthesizes speech and plays it immediately.
    pub async fn speak(&self, text: &str) -> VoiceResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        tracing::info!("TTS: {}", text);
        // Placeholder - would actually speak here
        Ok(())
    }

    /// Synthesizes speech and returns audio bytes (WAV format).
    pub async fn synthesize(&self, _text: &str) -> VoiceResult<Vec<u8>> {
        // Placeholder
        Ok(vec![])
    }

    /// Stops any ongoing speech.
    pub async fn stop(&self) -> VoiceResult<()> {
        Ok(())
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &TtsConfig {
        &self.config
    }

    /// Returns available voices for the current backend.
    pub async fn available_voices(&self) -> VoiceResult<Vec<VoiceInfo>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_config_default() {
        let config = TtsConfig::default();
        assert!((config.rate - 1.0).abs() < f32::EPSILON);
        assert!((config.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tts_backend_default() {
        let backend = TtsBackend::default();
        assert!(matches!(backend, TtsBackend::System));
    }

    #[test]
    fn test_elevenlabs_voice_settings_default() {
        let settings = ElevenLabsVoiceSettings::default();
        assert!((settings.stability - 0.5).abs() < f32::EPSILON);
        assert!((settings.similarity_boost - 0.75).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_voice_info_serialization() {
        let info = VoiceInfo {
            id: "abc123".to_string(),
            name: "Test Voice".to_string(),
            language: Some("en".to_string()),
            gender: Some("female".to_string()),
            preview_url: Some("http://example.com/preview.mp3".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("Test Voice"));
    }
}
