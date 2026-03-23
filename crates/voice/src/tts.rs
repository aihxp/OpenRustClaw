//! Text-to-speech (TTS) module.
//!
//! This module provides text-to-speech capabilities.

use crate::error::VoiceResult;
use serde::{Deserialize, Serialize};
#[cfg(feature = "audio")]
use std::f32::consts::TAU;
#[cfg(feature = "audio")]
use std::io::Cursor;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TtsBackend {
    /// System TTS (platform-native).
    #[default]
    System,
    /// ElevenLabs API.
    ElevenLabs {
        /// API key for ElevenLabs.
        api_key: String,
        /// Voice ID to use.
        voice_id: String,
    },
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
        #[cfg(feature = "audio")]
        {
            let bytes = self.synthesize(text).await?;
            tokio::task::spawn_blocking(move || play_wave_bytes(bytes))
                .await
                .map_err(|error| {
                    crate::error::VoiceError::AudioPlayback(format!(
                        "Failed to join audio playback task: {error}"
                    ))
                })??;
            return Ok(());
        }
        #[cfg(not(feature = "audio"))]
        {
            // Placeholder - would actually speak here without the audio feature.
            return Ok(());
        }
    }

    /// Synthesizes speech and returns audio bytes (WAV format).
    pub async fn synthesize(&self, text: &str) -> VoiceResult<Vec<u8>> {
        #[cfg(feature = "audio")]
        {
            let sample_rate = 16_000u32;
            let chars: Vec<char> = text.chars().collect();
            let samples_per_char = (sample_rate as f32 * 0.035).max(120.0) as usize;
            let total_samples = chars.len().max(1) * samples_per_char;
            let mut bytes = Vec::new();
            {
                let cursor = Cursor::new(&mut bytes);
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                let mut writer = hound::WavWriter::new(cursor, spec)?;
                for (index, ch) in chars.iter().enumerate() {
                    let freq = 220.0 + ((u32::from(*ch) % 24) as f32 * 18.0);
                    for sample_index in 0..samples_per_char {
                        let t =
                            (index * samples_per_char + sample_index) as f32 / sample_rate as f32;
                        let envelope = 0.18 + (self.config.volume.clamp(0.0, 1.0) * 0.35);
                        let phase = TAU * freq * t;
                        let value = if ch.is_whitespace() {
                            0.0
                        } else {
                            (phase.sin() * envelope).clamp(-1.0, 1.0)
                        };
                        let scaled = (value * i16::MAX as f32) as i16;
                        writer.write_sample(scaled)?;
                    }
                    if ch.is_whitespace() {
                        for _ in 0..(samples_per_char / 2).max(1) {
                            writer.write_sample(0i16)?;
                        }
                    }
                }

                if chars.is_empty() {
                    for _ in 0..total_samples.max(samples_per_char) {
                        writer.write_sample(0i16)?;
                    }
                }

                writer.finalize()?;
            }
            return Ok(bytes);
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = text;
            return Ok(Vec::new());
        }
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
        #[cfg(feature = "audio")]
        {
            return Ok(vec![VoiceInfo {
                id: "system-tone".to_string(),
                name: "System Tone".to_string(),
                language: Some("en".to_string()),
                gender: None,
                preview_url: None,
            }]);
        }
        #[cfg(not(feature = "audio"))]
        {
            return Ok(vec![]);
        }
    }
}

#[cfg(feature = "audio")]
fn play_wave_bytes(bytes: Vec<u8>) -> VoiceResult<()> {
    use rodio::{Decoder, OutputStream, Sink};

    let (_stream, handle) = OutputStream::try_default().map_err(|error| {
        crate::error::VoiceError::AudioPlayback(format!(
            "Failed to open default audio output device: {error}"
        ))
    })?;
    let sink = Sink::try_new(&handle).map_err(|error| {
        crate::error::VoiceError::AudioPlayback(format!("Failed to create audio sink: {error}"))
    })?;
    let source = Decoder::new(Cursor::new(bytes)).map_err(|error| {
        crate::error::VoiceError::AudioPlayback(format!(
            "Failed to decode synthesized WAV: {error}"
        ))
    })?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
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
