//! Common types for voice processing.

use crate::error::VoiceResult;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// A single frame of audio data.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// Raw audio samples (typically f32, -1.0 to 1.0)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Timestamp when the frame was captured
    pub timestamp: std::time::Instant,
}

impl AudioFrame {
    /// Create a new audio frame.
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Calculate the duration of this frame.
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.samples.len() as f32 / self.sample_rate as f32)
    }

    /// Calculate RMS (root mean square) energy of the frame.
    pub fn rms_energy(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = self.samples.iter().map(|s| s * s).sum();
        (sum_squares / self.samples.len() as f32).sqrt()
    }
}

/// A stream of audio frames.
#[derive(Debug)]
pub struct AudioStream {
    frames: Vec<AudioFrame>,
    sample_rate: u32,
}

impl AudioStream {
    /// Create a new empty audio stream.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            frames: Vec::new(),
            sample_rate,
        }
    }

    /// Add a frame to the stream.
    pub fn push_frame(&mut self, frame: AudioFrame) {
        self.frames.push(frame);
    }

    /// Get all frames as a single continuous buffer.
    pub fn to_buffer(&self) -> Vec<f32> {
        self.frames
            .iter()
            .flat_map(|f| f.samples.clone())
            .collect()
    }

    /// Get the total duration of the stream.
    pub fn duration(&self) -> Duration {
        self.frames.iter().map(|f| f.duration()).sum()
    }

    /// Clear all frames.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Check if the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Result of a speech-to-text transcription.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Language detected (ISO 639-1 code)
    pub language: Option<String>,
    /// Duration of the audio that was transcribed
    pub duration: Duration,
    /// Whether the transcription is final (not interim)
    pub is_final: bool,
}

impl TranscriptionResult {
    /// Create a new transcription result.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            confidence: 1.0,
            language: None,
            duration: Duration::default(),
            is_final: true,
        }
    }

    /// Check if the transcription is empty or contains only whitespace.
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Trait for speech-to-text engines.
#[async_trait]
pub trait SpeechToText: Send + Sync {
    /// Transcribe audio data to text.
    async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> VoiceResult<TranscriptionResult>;

    /// Check if the STT engine is available.
    fn is_available(&self) -> bool;

    /// Get the supported sample rate.
    fn sample_rate(&self) -> u32;
}

/// Trait for text-to-speech engines.
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    /// Synthesize text to speech and play it.
    async fn speak(&self, text: &str) -> VoiceResult<()>;

    /// Synthesize text to audio buffer.
    async fn synthesize(&self, text: &str) -> VoiceResult<Vec<f32>>;

    /// Stop any ongoing speech.
    async fn stop(&self) -> VoiceResult<()>;

    /// Check if currently speaking.
    fn is_speaking(&self) -> bool;

    /// Set the speech rate (1.0 = normal).
    fn set_rate(&mut self, rate: f32);

    /// Set the voice to use.
    fn set_voice(&mut self, voice_id: &str) -> VoiceResult<()>;
}

/// Trait for wake word detection.
#[async_trait]
pub trait VoiceWake: Send + Sync {
    /// Wait for the wake word to be detected.
    ///
    /// Returns true if wake word was detected, false if interrupted.
    async fn wait_for_wake(&self) -> VoiceResult<bool>;

    /// Check if wake word detection is active.
    fn is_listening(&self) -> bool;

    /// Stop listening for wake word.
    async fn stop(&self) -> VoiceResult<()>;
}

/// Mock STT implementation for testing.
pub struct MockSpeechToText;

#[async_trait]
impl SpeechToText for MockSpeechToText {
    async fn transcribe(&self, _audio: &[f32], _sample_rate: u32) -> VoiceResult<TranscriptionResult> {
        Ok(TranscriptionResult::new("mock transcription"))
    }

    fn is_available(&self) -> bool {
        true
    }

    fn sample_rate(&self) -> u32 {
        16000
    }
}

/// Mock TTS implementation for testing.
pub struct MockTextToSpeech {
    speaking: AtomicBool,
    rate_ms_per_word: AtomicU32,
}

impl MockTextToSpeech {
    /// Create a new mock TTS.
    pub fn new() -> Self {
        Self {
            speaking: AtomicBool::new(false),
            rate_ms_per_word: AtomicU32::new(300), // 300ms per word at 1.0 rate
        }
    }
}

impl Default for MockTextToSpeech {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextToSpeech for MockTextToSpeech {
    async fn speak(&self, text: &str) -> VoiceResult<()> {
        tracing::info!("MockTTS: Speaking: {}", text);
        self.speaking.store(true, Ordering::SeqCst);
        // Simulate speaking time
        let word_count = text.split_whitespace().count();
        let ms_per_word = self.rate_ms_per_word.load(Ordering::SeqCst) as u64;
        let duration = Duration::from_millis(word_count as u64 * ms_per_word);
        tokio::time::sleep(duration).await;
        self.speaking.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn synthesize(&self, _text: &str) -> VoiceResult<Vec<f32>> {
        Ok(vec![0.0; 16000]) // 1 second of silence at 16kHz
    }

    async fn stop(&self) -> VoiceResult<()> {
        self.speaking.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_speaking(&self) -> bool {
        self.speaking.load(Ordering::SeqCst)
    }

    fn set_rate(&mut self, rate: f32) {
        let ms_per_word = (300.0 / rate) as u32;
        self.rate_ms_per_word.store(ms_per_word, Ordering::SeqCst);
    }

    fn set_voice(&mut self, _voice_id: &str) -> VoiceResult<()> {
        Ok(())
    }
}

/// Mock wake word detector for testing.
pub struct MockWakeDetector {
    listening: AtomicBool,
}

impl MockWakeDetector {
    /// Create a new mock wake detector.
    pub fn new() -> Self {
        Self {
            listening: AtomicBool::new(false),
        }
    }
}

impl Default for MockWakeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VoiceWake for MockWakeDetector {
    async fn wait_for_wake(&self) -> VoiceResult<bool> {
        self.listening.store(true, Ordering::SeqCst);
        // In a real implementation, this would listen for audio
        // For mock, we just wait a bit then return true
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.listening.store(false, Ordering::SeqCst);
        Ok(true)
    }

    fn is_listening(&self) -> bool {
        self.listening.load(Ordering::SeqCst)
    }

    async fn stop(&self) -> VoiceResult<()> {
        self.listening.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_frame_rms() {
        let silence = vec![0.0; 100];
        let frame = AudioFrame::new(silence, 16000);
        assert!(frame.rms_energy() < 0.001);

        let full_scale = vec![1.0; 100];
        let frame = AudioFrame::new(full_scale, 16000);
        assert!((frame.rms_energy() - 1.0).abs() < 0.001);

        let half_scale = vec![0.5; 100];
        let frame = AudioFrame::new(half_scale, 16000);
        assert!((frame.rms_energy() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_audio_stream() {
        let mut stream = AudioStream::new(16000);
        assert!(stream.is_empty());

        stream.push_frame(AudioFrame::new(vec![0.1; 1600], 16000));
        assert!(!stream.is_empty());
        assert_eq!(stream.sample_rate(), 16000);

        let buffer = stream.to_buffer();
        assert_eq!(buffer.len(), 1600);
    }

    #[test]
    fn test_transcription_result() {
        let result = TranscriptionResult::new("Hello world");
        assert_eq!(result.text, "Hello world");
        assert!(!result.is_empty());
        assert_eq!(result.confidence, 1.0);
    }
}
