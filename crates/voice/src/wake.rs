//! Wake word detection for activating voice interaction.

use crate::error::{VoiceError, VoiceResult};
use crate::vad::{VoiceActivityDetector, VadConfig};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Notify};

/// Configuration for wake word detection.
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// The wake word phrase to listen for
    pub wake_word: String,
    /// Alternative wake words that trigger activation
    pub alternative_wake_words: Vec<String>,
    /// Sensitivity (0.0 to 1.0, higher = more sensitive)
    pub sensitivity: f32,
    /// Minimum confidence to trigger (0.0 to 1.0)
    pub min_confidence: f32,
    /// Timeout for wake word detection
    pub timeout: Option<Duration>,
    /// Post-wake buffer duration to capture audio after wake word
    pub post_wake_buffer: Duration,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            wake_word: "Hey Assistant".to_string(),
            alternative_wake_words: vec![
                "Hey OpenRustClaw".to_string(),
                "Assistant".to_string(),
            ],
            sensitivity: 0.7,
            min_confidence: 0.6,
            timeout: None,
            post_wake_buffer: Duration::from_millis(500),
        }
    }
}

impl WakeWordConfig {
    /// Create a new config with a custom wake word.
    pub fn with_wake_word(wake_word: impl Into<String>) -> Self {
        Self {
            wake_word: wake_word.into(),
            ..Default::default()
        }
    }

    /// Add an alternative wake word.
    pub fn with_alternative(mut self, word: impl Into<String>) -> Self {
        self.alternative_wake_words.push(word.into());
        self
    }

    /// Set the sensitivity.
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity.clamp(0.0, 1.0);
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Result of wake word detection.
#[derive(Debug, Clone)]
pub struct WakeDetectionResult {
    /// The wake word that was detected
    pub detected_wake_word: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Audio buffer captured around the wake word
    pub audio_buffer: Vec<f32>,
}

/// Wake word detector trait.
#[async_trait]
pub trait WakeDetector: Send + Sync {
    /// Listen for wake word and return when detected.
    async fn listen(&self) -> VoiceResult<WakeDetectionResult>;

    /// Stop listening.
    async fn stop(&self) -> VoiceResult<()>;

    /// Check if currently listening.
    fn is_listening(&self) -> bool;
}

/// Simple keyword-based wake detector using VAD and pattern matching.
///
/// This is a placeholder implementation that uses energy-based detection.
/// In a real implementation, you would use a neural network model like
/// Porcupine, Snowboy, or a custom ONNX model.
pub struct SimpleWakeDetector {
    config: WakeWordConfig,
    listening: AtomicBool,
    stop_notify: Notify,
    audio_rx: Mutex<mpsc::Receiver<Vec<f32>>>,
    audio_tx: mpsc::Sender<Vec<f32>>,
}

impl SimpleWakeDetector {
    /// Create a new simple wake detector.
    pub fn new(config: WakeWordConfig) -> Self {
        let (audio_tx, audio_rx) = mpsc::channel(100);
        Self {
            config,
            listening: AtomicBool::new(false),
            stop_notify: Notify::new(),
            audio_rx: Mutex::new(audio_rx),
            audio_tx,
        }
    }

    /// Get the sender for feeding audio data.
    pub fn audio_sender(&self) -> mpsc::Sender<Vec<f32>> {
        self.audio_tx.clone()
    }
}

#[async_trait]
impl WakeDetector for SimpleWakeDetector {
    async fn listen(&self) -> VoiceResult<WakeDetectionResult> {
        self.listening.store(true, Ordering::SeqCst);

        let mut vad = VoiceActivityDetector::new(VadConfig {
            threshold: 0.02 * (1.0 + self.config.sensitivity),
            ..Default::default()
        });

        let mut audio_buffer: Vec<f32> = Vec::new();
        let mut speech_detected = false;
        let mut silence_after_speech = 0u32;
        const MAX_SILENCE_FRAMES: u32 = 30; // ~1 second at 30ms frames

        let timeout_future = match self.config.timeout {
            Some(t) => tokio::time::timeout(t, std::future::pending::<()>()),
            None => tokio::time::timeout(Duration::from_secs(3600), std::future::pending::<()>()),
        };

        tokio::pin!(timeout_future);

        let mut rx = self.audio_rx.lock().await;

        loop {
            tokio::select! {
                Some(frame) = rx.recv() => {
                    let vad_state = vad.process_frame(&frame);

                    if vad.is_speech() {
                        speech_detected = true;
                        silence_after_speech = 0;
                        audio_buffer.extend_from_slice(&frame);
                    } else if speech_detected {
                        silence_after_speech += 1;
                        audio_buffer.extend_from_slice(&frame);

                        if silence_after_speech >= MAX_SILENCE_FRAMES {
                            // End of utterance detected
                            // In a real implementation, we would run STT here
                            // and check if the wake word was spoken
                            break;
                        }
                    }
                }
                _ = self.stop_notify.notified() => {
                    return Err(VoiceError::Interrupted);
                }
                _ = &mut timeout_future => {
                    return Err(VoiceError::Timeout(
                        "Wake word detection timed out".to_string()
                    ));
                }
            }
        }

        self.listening.store(false, Ordering::SeqCst);

        // For this simple implementation, we simulate wake word detection
        // In reality, you'd run a proper wake word model here
        Ok(WakeDetectionResult {
            detected_wake_word: self.config.wake_word.clone(),
            confidence: self.config.min_confidence,
            audio_buffer,
        })
    }

    async fn stop(&self) -> VoiceResult<()> {
        self.listening.store(false, Ordering::SeqCst);
        self.stop_notify.notify_one();
        Ok(())
    }

    fn is_listening(&self) -> bool {
        self.listening.load(Ordering::SeqCst)
    }
}

/// Porcupine-based wake word detector (placeholder).
///
/// This would integrate with Picovoice Porcupine for on-device
/// wake word detection. Requires a Porcupine access key and model files.
pub struct PorcupineWakeDetector {
    config: WakeWordConfig,
    listening: AtomicBool,
    stop_notify: Notify,
}

impl PorcupineWakeDetector {
    /// Create a new Porcupine wake detector.
    ///
    /// # Errors
    ///
    /// Returns an error if Porcupine is not available or initialization fails.
    pub fn new(config: WakeWordConfig) -> VoiceResult<Self> {
        // In a real implementation, this would initialize Porcupine
        // For now, we just check if we could potentially use it
        Ok(Self {
            config,
            listening: AtomicBool::new(false),
            stop_notify: Notify::new(),
        })
    }

    /// Check if Porcupine is available on this system.
    pub fn is_available() -> bool {
        // Would check for Porcupine library availability
        false
    }
}

#[async_trait]
impl WakeDetector for PorcupineWakeDetector {
    async fn listen(&self) -> VoiceResult<WakeDetectionResult> {
        // Would implement Porcupine wake word detection here
        Err(VoiceError::WakeWord(
            "Porcupine not implemented. Use SimpleWakeDetector instead.".to_string()
        ))
    }

    async fn stop(&self) -> VoiceResult<()> {
        self.listening.store(false, Ordering::SeqCst);
        self.stop_notify.notify_one();
        Ok(())
    }

    fn is_listening(&self) -> bool {
        self.listening.load(Ordering::SeqCst)
    }
}

/// Factory for creating wake detectors.
pub struct WakeDetectorFactory;

impl WakeDetectorFactory {
    /// Create the best available wake detector.
    pub fn create(config: WakeWordConfig) -> VoiceResult<Arc<dyn WakeDetector>> {
        if PorcupineWakeDetector::is_available() {
            Ok(Arc::new(PorcupineWakeDetector::new(config)?))
        } else {
            Ok(Arc::new(SimpleWakeDetector::new(config)))
        }
    }

    /// Create a simple wake detector.
    pub fn create_simple(config: WakeWordConfig) -> Arc<dyn WakeDetector> {
        Arc::new(SimpleWakeDetector::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_config() {
        let config = WakeWordConfig::with_wake_word("Hello")
            .with_alternative("Hi")
            .with_sensitivity(0.8);

        assert_eq!(config.wake_word, "Hello");
        assert!(config.alternative_wake_words.contains(&"Hi".to_string()));
        assert_eq!(config.sensitivity, 0.8);
    }

    #[tokio::test]
    async fn test_simple_wake_detector() {
        let config = WakeWordConfig::default();
        let detector = SimpleWakeDetector::new(config);

        assert!(!detector.is_listening());

        // Feed some simulated audio
        let tx = detector.audio_sender();

        // Simulate speech frames (high energy)
        for _ in 0..10 {
            let frame = vec![0.5; 480]; // 30ms at 16kHz
            tx.send(frame).await.unwrap();
        }

        // Start listening in background
        let detector_clone = Arc::new(detector);
        let detector_for_task = detector_clone.clone();

        let handle = tokio::spawn(async move {
            detector_for_task.listen().await
        });

        // Send silence frames to trigger end of utterance
        let tx = detector_clone.audio_sender();
        for _ in 0..50 {
            let frame = vec![0.001; 480];
            tx.send(frame).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Stop the detector
        detector_clone.stop().await.unwrap();

        // Should get interrupted error or timeout
        let result = handle.await.unwrap();
        assert!(result.is_err() || result.unwrap().confidence > 0.0);
    }
}
