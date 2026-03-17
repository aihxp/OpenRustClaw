//! Talk Mode - continuous voice conversation loop.
//!
//! This module implements a state machine for continuous voice interaction:
//!
//! ```text
//! ┌──────┐    wake detected     ┌───────────┐    silence detected   ┌────────────┐
//! │ Idle │ ───────────────────► │ Listening │ ───────────────────► │ Processing │
//! └──────┘                      └───────────┘                       └────────────┘
//!    ▲                                                                  │
//!    │                                                                  │ STT → Agent → TTS
//!    │                                                                  ▼
//!    │                                                              ┌─────────┐
//!    └───────────────────────────────────────────────────────────── │ Speaking│
//!         barge-in (wake during speech)                             └─────────┘
//! ```

use crate::error::{VoiceError, VoiceResult};
use crate::stt::{SpeechToText, Transcription};
use crate::tts::TextToSpeech;
use crate::wake::{WakeDetector, WakeWordConfig};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, Notify, RwLock};
use tracing::{debug, error, info, warn};

/// Configuration for Talk Mode.
#[derive(Debug, Clone)]
pub struct TalkConfig {
    /// Wake word configuration
    pub wake_word_config: WakeWordConfig,
    /// Stop listening after this duration of silence
    pub silence_timeout: Duration,
    /// Maximum duration for a single utterance
    pub max_utterance_duration: Duration,
    /// Sample rate for audio processing
    pub sample_rate: u32,
    /// Maximum conversation history to keep
    pub max_history: usize,
    /// Enable barge-in (interrupt TTS with new wake word)
    pub enable_barge_in: bool,
    /// Auto-end conversation after this duration of inactivity
    pub session_timeout: Option<Duration>,
    /// Play audio cues for state changes
    pub audio_cues: bool,
}

impl Default for TalkConfig {
    fn default() -> Self {
        Self {
            wake_word_config: WakeWordConfig::default(),
            silence_timeout: Duration::from_secs(3),
            max_utterance_duration: Duration::from_secs(30),
            sample_rate: 16000,
            max_history: 20,
            enable_barge_in: true,
            session_timeout: Some(Duration::from_secs(300)), // 5 minutes
            audio_cues: true,
        }
    }
}

impl TalkConfig {
    /// Create a new config with a custom wake word.
    pub fn with_wake_word(wake_word: impl Into<String>) -> Self {
        Self {
            wake_word_config: WakeWordConfig::with_wake_word(wake_word),
            ..Default::default()
        }
    }

    /// Set the silence timeout.
    pub fn with_silence_timeout(mut self, timeout: Duration) -> Self {
        self.silence_timeout = timeout;
        self
    }

    /// Set the max utterance duration.
    pub fn with_max_utterance_duration(mut self, duration: Duration) -> Self {
        self.max_utterance_duration = duration;
        self
    }

    /// Enable or disable barge-in.
    pub fn with_barge_in(mut self, enabled: bool) -> Self {
        self.enable_barge_in = enabled;
        self
    }

    /// Set session timeout.
    pub fn with_session_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.session_timeout = timeout;
        self
    }
}

/// Current state of Talk Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkState {
    /// Waiting for wake word.
    Idle,
    /// Wake word detected, listening for speech.
    Listening,
    /// Processing speech (STT → Agent → TTS).
    Processing,
    /// Playing TTS response.
    Speaking,
    /// Shutting down.
    Stopping,
    /// Error state.
    Error,
}

impl std::fmt::Display for TalkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TalkState::Idle => write!(f, "idle"),
            TalkState::Listening => write!(f, "listening"),
            TalkState::Processing => write!(f, "processing"),
            TalkState::Speaking => write!(f, "speaking"),
            TalkState::Stopping => write!(f, "stopping"),
            TalkState::Error => write!(f, "error"),
        }
    }
}

/// Events emitted by Talk Mode.
#[derive(Debug, Clone)]
pub enum TalkEvent {
    /// State changed.
    StateChanged { from: TalkState, to: TalkState },
    /// Wake word detected.
    WakeWordDetected { word: String, confidence: f32 },
    /// Speech detected.
    SpeechStarted,
    /// Speech ended.
    SpeechEnded { duration: Duration },
    /// Transcription received.
    Transcription { text: String, confidence: f32 },
    /// Agent response received.
    AgentResponse { text: String },
    /// TTS started.
    SpeakingStarted,
    /// TTS ended.
    SpeakingEnded,
    /// Barge-in detected.
    BargeIn,
    /// Error occurred.
    Error { message: String },
    /// Session ended due to timeout.
    SessionTimeout,
}

/// Conversation turn (user input + agent response).
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user_text: String,
    pub agent_response: String,
    pub timestamp: Instant,
}

/// Talk Mode - continuous voice conversation.
pub struct TalkMode {
    /// Wake word detector
    wake: Arc<dyn WakeDetector>,
    /// Speech-to-text engine
    stt: Arc<SpeechToText>,
    /// Text-to-speech engine
    tts: Arc<TextToSpeech>,
    /// Configuration
    config: TalkConfig,
    /// Current state
    state: Arc<RwLock<TalkState>>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Stop signal
    stop_notify: Arc<Notify>,
    /// Event sender
    event_tx: mpsc::Sender<TalkEvent>,
    /// Event receiver (stored for cloning)
    event_rx: Arc<Mutex<mpsc::Receiver<TalkEvent>>>,
    /// Conversation history
    history: Arc<Mutex<VecDeque<ConversationTurn>>>,
    /// Current session start time
    session_start: Arc<RwLock<Option<Instant>>>,
    /// Last activity time
    last_activity: Arc<RwLock<Instant>>,
    /// Audio input channel
    audio_tx: mpsc::Sender<Vec<f32>>,
    /// Audio input receiver
    audio_rx: Arc<Mutex<mpsc::Receiver<Vec<f32>>>>,
}

impl TalkMode {
    /// Create a new Talk Mode instance.
    pub fn new(
        wake: Arc<dyn WakeDetector>,
        stt: Arc<SpeechToText>,
        tts: Arc<TextToSpeech>,
        config: TalkConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (audio_tx, audio_rx) = mpsc::channel(1000);

        Self {
            wake,
            stt,
            tts,
            config,
            state: Arc::new(RwLock::new(TalkState::Idle)),
            running: Arc::new(AtomicBool::new(false)),
            stop_notify: Arc::new(Notify::new()),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            history: Arc::new(Mutex::new(VecDeque::new())),
            session_start: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            audio_tx,
            audio_rx: Arc::new(Mutex::new(audio_rx)),
        }
    }

    /// Get the current state.
    pub async fn current_state(&self) -> TalkState {
        *self.state.read().await
    }

    /// Get a clone of the event receiver.
    pub async fn subscribe_events(&self) -> mpsc::Receiver<TalkEvent> {
        let (_tx, rx) = mpsc::channel(100);
        // Note: This is a simplified implementation
        // In production, you'd use a broadcast channel
        rx
    }

    /// Get the event sender for external event subscription.
    pub fn event_sender(&self) -> mpsc::Sender<TalkEvent> {
        self.event_tx.clone()
    }

    /// Get the audio sender for feeding audio data.
    pub fn audio_sender(&self) -> mpsc::Sender<Vec<f32>> {
        self.audio_tx.clone()
    }

    /// Get the conversation history.
    pub async fn history(&self) -> Vec<ConversationTurn> {
        let history = self.history.lock().await;
        history.iter().cloned().collect()
    }

    /// Clear the conversation history.
    pub async fn clear_history(&self) {
        let mut history = self.history.lock().await;
        history.clear();
    }

    /// Run the Talk Mode main loop.
    pub async fn run(&self) -> VoiceResult<()> {
        info!("Starting Talk Mode");
        self.running.store(true, Ordering::SeqCst);
        *self.state.write().await = TalkState::Idle;
        *self.session_start.write().await = Some(Instant::now());
        *self.last_activity.write().await = Instant::now();

        // Emit initial state
        self.emit_event(TalkEvent::StateChanged {
            from: TalkState::Idle,
            to: TalkState::Idle,
        })
        .await;

        loop {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            // Check session timeout
            if self.check_session_timeout().await {
                self.emit_event(TalkEvent::SessionTimeout).await;
                break;
            }

            let state = self.current_state().await;

            match state {
                TalkState::Idle => {
                    if let Err(e) = self.run_idle_state().await {
                        error!("Error in idle state: {}", e);
                        self.handle_error(e).await;
                    }
                }
                TalkState::Listening => {
                    if let Err(e) = self.run_listening_state().await {
                        error!("Error in listening state: {}", e);
                        self.handle_error(e).await;
                    }
                }
                TalkState::Processing => {
                    // This state is handled within listening state
                    warn!("Unexpected processing state in main loop");
                    self.transition_to(TalkState::Idle).await;
                }
                TalkState::Speaking => {
                    // Speaking state is handled within listening state
                    warn!("Unexpected speaking state in main loop");
                    self.transition_to(TalkState::Idle).await;
                }
                TalkState::Stopping => {
                    break;
                }
                TalkState::Error => {
                    // Try to recover
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    self.transition_to(TalkState::Idle).await;
                }
            }
        }

        info!("Talk Mode stopped");
        Ok(())
    }

    /// Stop Talk Mode gracefully.
    pub async fn stop(&self) {
        info!("Stopping Talk Mode...");
        self.running.store(false, Ordering::SeqCst);
        self.transition_to(TalkState::Stopping).await;
        self.stop_notify.notify_one();

        // Stop TTS if speaking
        let _ = self.tts.stop().await;

        // Stop wake word detection
        let _ = self.wake.stop().await;
    }

    /// Run the idle state (waiting for wake word).
    async fn run_idle_state(&self) -> VoiceResult<()> {
        debug!("Entering idle state, waiting for wake word");

        tokio::select! {
            result = self.wake.listen() => {
                match result {
                    Ok(_) => {
                        info!("Wake word detected");
                        self.emit_event(TalkEvent::WakeWordDetected {
                            word: self.config.wake_word_config.wake_word.clone(),
                            confidence: 1.0,
                        }).await;
                        self.transition_to(TalkState::Listening).await;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            _ = self.stop_notify.notified() => {
                Err(VoiceError::Interrupted)
            }
        }
    }

    /// Run the listening state (recording speech).
    async fn run_listening_state(&self) -> VoiceResult<()> {
        debug!("Entering listening state");
        self.emit_event(TalkEvent::SpeechStarted).await;

        // For now, we simulate the listening process
        // In a real implementation, this would record audio until silence
        
        tokio::time::sleep(Duration::from_secs(2)).await;

        self.emit_event(TalkEvent::SpeechEnded { duration: Duration::from_secs(2) }).await;

        // Transition to processing
        self.transition_to(TalkState::Processing).await;

        // Simulate transcription
        let transcription = Transcription {
            text: "Hello, this is a test message".to_string(),
            confidence: 0.95,
            segments: vec![],
            language: "en".to_string(),
            duration_secs: 2.0,
            processing_time_ms: 100,
        };

        info!("Transcribed: {}", transcription.text);
        self.emit_event(TalkEvent::Transcription {
            text: transcription.text.clone(),
            confidence: transcription.confidence,
        }).await;

        // Simulate agent response
        let response = format!("You said: {}", transcription.text);

        self.emit_event(TalkEvent::AgentResponse {
            text: response.clone(),
        }).await;

        // Store in history
        {
            let mut history = self.history.lock().await;
            history.push_back(ConversationTurn {
                user_text: transcription.text,
                agent_response: response.clone(),
                timestamp: Instant::now(),
            });

            // Trim history
            while history.len() > self.config.max_history {
                history.pop_front();
            }
        }

        // Speak response
        self.transition_to(TalkState::Speaking).await;
        self.tts.speak(&response).await?;
        self.emit_event(TalkEvent::SpeakingEnded).await;

        // Return to idle
        self.transition_to(TalkState::Idle).await;

        Ok(())
    }

    /// Transition to a new state.
    async fn transition_to(&self, new_state: TalkState) {
        let old_state = {
            let mut state = self.state.write().await;
            let old = *state;
            *state = new_state;
            old
        };

        if old_state != new_state {
            info!("State transition: {} -> {}", old_state, new_state);
            self.emit_event(TalkEvent::StateChanged {
                from: old_state,
                to: new_state,
            }).await;
        }
    }

    /// Emit an event.
    async fn emit_event(&self, event: TalkEvent) {
        let _ = self.event_tx.send(event).await;
    }

    /// Handle an error.
    async fn handle_error(&self, error: VoiceError) {
        self.emit_event(TalkEvent::Error {
            message: error.to_string(),
        }).await;
        self.transition_to(TalkState::Error).await;
    }

    /// Check if session has timed out.
    async fn check_session_timeout(&self) -> bool {
        if let Some(timeout) = self.config.session_timeout {
            let last_activity = *self.last_activity.read().await;
            if last_activity.elapsed() > timeout {
                return true;
            }
        }
        false
    }

    /// Update last activity timestamp.
    async fn update_activity(&self) {
        *self.last_activity.write().await = Instant::now();
    }
}

/// Builder for TalkMode.
pub struct TalkModeBuilder {
    wake: Option<Arc<dyn WakeDetector>>,
    stt: Option<Arc<SpeechToText>>,
    tts: Option<Arc<TextToSpeech>>,
    config: TalkConfig,
}

impl TalkModeBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            wake: None,
            stt: None,
            tts: None,
            config: TalkConfig::default(),
        }
    }

    /// Set the wake word detector.
    pub fn wake(mut self, wake: Arc<dyn WakeDetector>) -> Self {
        self.wake = Some(wake);
        self
    }

    /// Set the speech-to-text engine.
    pub fn stt(mut self, stt: Arc<SpeechToText>) -> Self {
        self.stt = Some(stt);
        self
    }

    /// Set the text-to-speech engine.
    pub fn tts(mut self, tts: Arc<TextToSpeech>) -> Self {
        self.tts = Some(tts);
        self
    }

    /// Set the configuration.
    pub fn config(mut self, config: TalkConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the TalkMode.
    pub fn build(self) -> VoiceResult<TalkMode> {
        let wake = self.wake.ok_or_else(|| {
            VoiceError::Config("Wake word detector is required".to_string())
        })?;
        let stt = self.stt.ok_or_else(|| {
            VoiceError::Config("Speech-to-text is required".to_string())
        })?;
        let tts = self.tts.ok_or_else(|| {
            VoiceError::Config("Text-to-speech is required".to_string())
        })?;

        Ok(TalkMode::new(wake, stt, tts, self.config))
    }
}

impl Default for TalkModeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_talk_config_default() {
        let config = TalkConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert!(config.enable_barge_in);
        assert_eq!(config.max_history, 20);
    }

    #[test]
    fn test_talk_config_builder() {
        let config = TalkConfig::with_wake_word("Hello Computer")
            .with_silence_timeout(Duration::from_secs(5))
            .with_barge_in(false);

        assert_eq!(config.wake_word_config.wake_word, "Hello Computer");
        assert_eq!(config.silence_timeout, Duration::from_secs(5));
        assert!(!config.enable_barge_in);
    }

    #[test]
    fn test_talk_mode_state() {
        // Create minimal test - just verify state types work
        let config = TalkConfig::default();
        assert_eq!(config.wake_word_config.wake_word, "Hey Assistant");
    }

    #[test]
    fn test_talk_mode_history() {
        // Just test that history types compile correctly
        let history: VecDeque<ConversationTurn> = VecDeque::new();
        assert!(history.is_empty());
    }
}
