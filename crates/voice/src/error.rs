//! Error types for voice processing.

use thiserror::Error;

/// Voice processing errors.
#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("Audio capture error: {0}")]
    AudioCapture(String),

    #[error("Audio playback error: {0}")]
    AudioPlayback(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Speech-to-text error: {0}")]
    SpeechToText(String),

    #[error("STT error: {0}")]
    Stt(String),

    #[error("Text-to-speech error: {0}")]
    TextToSpeech(String),

    #[error("TTS error: {0}")]
    Tts(String),

    #[error("Wake word detection error: {0}")]
    WakeWord(String),

    #[error("Voice activity detection error: {0}")]
    Vad(String),

    #[error("Invalid audio format: {0}")]
    InvalidAudioFormat(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Conversation interrupted")]
    Interrupted,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<hound::Error> for VoiceError {
    fn from(err: hound::Error) -> Self {
        Self::InvalidFormat(format!("WAV error: {err}"))
    }
}

/// Convenient Result type alias for voice operations.
pub type VoiceResult<T> = std::result::Result<T, VoiceError>;

/// Alternative Result type alias.
pub type Result<T> = VoiceResult<T>;
