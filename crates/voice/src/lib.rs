//! Voice processing and Talk Mode for OpenRustClaw.
//!
//! This crate provides voice-based interaction capabilities including:
//! - Wake word detection
//! - Speech-to-text (STT)
//! - Text-to-speech (TTS)
//! - Talk Mode: Continuous voice conversation

pub mod error;
pub mod stt;
pub mod talk_mode;
pub mod tts;
pub mod types;
pub mod vad;
pub mod wake;

pub use error::{VoiceError, VoiceResult};
pub use stt::{Segment, SpeechToText, SttConfig, StreamingStt, Transcription};
pub use talk_mode::{TalkConfig, TalkEvent, TalkMode, TalkModeBuilder, TalkState, ConversationTurn};
pub use tts::{ElevenLabsVoiceSettings, TextToSpeech, TtsBackend, TtsConfig, VoiceInfo};
pub use types::{AudioFrame, AudioStream};
pub use vad::{VadConfig, VadState, VoiceActivityDetector, AdaptiveVad};
pub use wake::{WakeDetectionResult, WakeDetector, WakeWordConfig, SimpleWakeDetector, WakeDetectorFactory};
