//! Voice Activity Detection (VAD) for speech detection.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Configuration for voice activity detection.
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// RMS energy threshold for speech detection (0.0 to 1.0)
    pub threshold: f32,
    /// Number of consecutive speech frames required to trigger speech start
    pub speech_frames_threshold: u32,
    /// Number of consecutive silence frames required to trigger speech end
    pub silence_frames_threshold: u32,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u32,
    /// Pre-buffer duration to capture before speech starts
    pub pre_buffer_duration: Duration,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold: 0.02,
            speech_frames_threshold: 3,
            silence_frames_threshold: 10,
            frame_duration_ms: 30,
            pre_buffer_duration: Duration::from_millis(200),
        }
    }
}

/// Voice Activity Detector.
///
/// Detects speech in audio frames using RMS energy thresholding
/// with hysteresis to avoid rapid state changes.
#[derive(Debug)]
pub struct VoiceActivityDetector {
    config: VadConfig,
    state: VadState,
    speech_frame_count: u32,
    silence_frame_count: u32,
    /// Pre-buffer to capture audio before speech starts
    pre_buffer: VecDeque<Vec<f32>>,
    /// Maximum pre-buffer size in frames
    max_pre_buffer_frames: usize,
    speech_start_time: Option<Instant>,
}

/// Current state of voice activity detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// Waiting for speech to start.
    Silent,
    /// Speech is currently detected.
    Speaking,
    /// In the process of ending speech (silence detected but not enough to end yet).
    Trailing,
}

impl VoiceActivityDetector {
    /// Create a new VAD with the given configuration.
    pub fn new(config: VadConfig) -> Self {
        let max_pre_buffer_frames =
            (config.pre_buffer_duration.as_millis() / config.frame_duration_ms as u128) as usize;

        Self {
            config,
            state: VadState::Silent,
            speech_frame_count: 0,
            silence_frame_count: 0,
            pre_buffer: VecDeque::with_capacity(max_pre_buffer_frames),
            max_pre_buffer_frames,
            speech_start_time: None,
        }
    }

    /// Process an audio frame and update VAD state.
    ///
    /// Returns the current VAD state after processing.
    pub fn process_frame(&mut self, frame: &[f32]) -> VadState {
        let energy = calculate_rms_energy(frame);
        let is_speech = energy > self.config.threshold;

        match self.state {
            VadState::Silent => {
                if is_speech {
                    self.speech_frame_count += 1;
                    if self.speech_frame_count >= self.config.speech_frames_threshold {
                        self.state = VadState::Speaking;
                        self.speech_frame_count = 0;
                        self.silence_frame_count = 0;
                        self.speech_start_time = Some(Instant::now());
                    }
                } else {
                    // Keep pre-buffer of recent audio
                    self.update_pre_buffer(frame);
                    self.speech_frame_count = 0;
                }
            }
            VadState::Speaking => {
                if is_speech {
                    self.silence_frame_count = 0;
                } else {
                    self.silence_frame_count += 1;
                    if self.silence_frame_count >= self.config.silence_frames_threshold {
                        self.state = VadState::Silent;
                        self.silence_frame_count = 0;
                        self.speech_start_time = None;
                        self.pre_buffer.clear();
                    } else {
                        self.state = VadState::Trailing;
                    }
                }
            }
            VadState::Trailing => {
                if is_speech {
                    self.state = VadState::Speaking;
                    self.silence_frame_count = 0;
                } else {
                    self.silence_frame_count += 1;
                    if self.silence_frame_count >= self.config.silence_frames_threshold {
                        self.state = VadState::Silent;
                        self.silence_frame_count = 0;
                        self.speech_start_time = None;
                        self.pre_buffer.clear();
                    }
                }
            }
        }

        self.state
    }

    /// Check if the current state indicates speech is present.
    pub fn is_speech(&self) -> bool {
        matches!(self.state, VadState::Speaking | VadState::Trailing)
    }

    /// Get the current VAD state.
    pub fn state(&self) -> VadState {
        self.state
    }

    /// Reset the VAD to initial state.
    pub fn reset(&mut self) {
        self.state = VadState::Silent;
        self.speech_frame_count = 0;
        self.silence_frame_count = 0;
        self.pre_buffer.clear();
        self.speech_start_time = None;
    }

    /// Get the pre-buffer containing audio before speech started.
    pub fn pre_buffer(&self) -> &VecDeque<Vec<f32>> {
        &self.pre_buffer
    }

    /// Get the duration of current speech segment.
    pub fn speech_duration(&self) -> Option<Duration> {
        self.speech_start_time.map(|t| t.elapsed())
    }

    /// Update the pre-buffer with a new frame.
    fn update_pre_buffer(&mut self, frame: &[f32]) {
        if self.max_pre_buffer_frames > 0 {
            if self.pre_buffer.len() >= self.max_pre_buffer_frames {
                self.pre_buffer.pop_front();
            }
            self.pre_buffer.push_back(frame.to_vec());
        }
    }

    /// Calculate RMS energy of a frame.
    pub fn frame_energy(frame: &[f32]) -> f32 {
        calculate_rms_energy(frame)
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

/// Calculate RMS (Root Mean Square) energy of an audio buffer.
fn calculate_rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f64 = samples
        .iter()
        .map(|&s| {
            let s = s.clamp(-1.0, 1.0) as f64;
            s * s
        })
        .sum();

    ((sum_squares / samples.len() as f64).sqrt()) as f32
}

/// Adaptive VAD that adjusts threshold based on ambient noise.
#[derive(Debug)]
pub struct AdaptiveVad {
    base_vad: VoiceActivityDetector,
    /// Running average of background noise level
    noise_floor: f32,
    /// Alpha for exponential moving average
    alpha: f32,
    /// Minimum threshold to prevent over-sensitivity
    min_threshold: f32,
    /// Maximum threshold to prevent under-sensitivity
    max_threshold: f32,
}

impl AdaptiveVad {
    /// Create a new adaptive VAD.
    pub fn new(mut config: VadConfig) -> Self {
        let base_threshold = config.threshold;
        config.threshold = base_threshold; // Will be updated adaptively

        Self {
            base_vad: VoiceActivityDetector::new(config),
            noise_floor: base_threshold * 0.5,
            alpha: 0.95,
            min_threshold: 0.005,
            max_threshold: 0.1,
        }
    }

    /// Process a frame with adaptive threshold.
    pub fn process_frame(&mut self, frame: &[f32]) -> VadState {
        let energy = calculate_rms_energy(frame);

        // Update noise floor during silence
        if !self.base_vad.is_speech() && energy < self.base_vad.config.threshold * 2.0 {
            self.noise_floor = self.alpha * self.noise_floor + (1.0 - self.alpha) * energy;

            // Update threshold based on noise floor
            let new_threshold = (self.noise_floor * 3.0).clamp(self.min_threshold, self.max_threshold);
            self.base_vad.config.threshold = new_threshold;
        }

        self.base_vad.process_frame(frame)
    }

    /// Check if speech is detected.
    pub fn is_speech(&self) -> bool {
        self.base_vad.is_speech()
    }

    /// Get current threshold.
    pub fn current_threshold(&self) -> f32 {
        self.base_vad.config.threshold
    }

    /// Get noise floor estimate.
    pub fn noise_floor(&self) -> f32 {
        self.noise_floor
    }

    /// Reset the adaptive VAD.
    pub fn reset(&mut self) {
        self.base_vad.reset();
        self.noise_floor = self.base_vad.config.threshold * 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_energy() {
        let silence = vec![0.0; 100];
        assert!(calculate_rms_energy(&silence) < 0.001);

        let full_scale = vec![1.0; 100];
        assert!((calculate_rms_energy(&full_scale) - 1.0).abs() < 0.001);

        let half_scale = vec![0.5; 100];
        assert!((calculate_rms_energy(&half_scale) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_vad_state_transitions() {
        let config = VadConfig {
            threshold: 0.1,
            speech_frames_threshold: 2,
            silence_frames_threshold: 2,
            frame_duration_ms: 30,
            pre_buffer_duration: Duration::from_millis(0),
        };

        let mut vad = VoiceActivityDetector::new(config);

        // Start silent
        assert_eq!(vad.state(), VadState::Silent);

        // Low energy frames should keep it silent
        let silence = vec![0.01; 100];
        assert_eq!(vad.process_frame(&silence), VadState::Silent);
        assert_eq!(vad.process_frame(&silence), VadState::Silent);

        // Need consecutive speech frames to trigger
        let speech = vec![0.5; 100];
        assert_eq!(vad.process_frame(&speech), VadState::Silent);
        assert_eq!(vad.process_frame(&speech), VadState::Speaking);

        // Single silence frame shouldn't end speech
        assert_eq!(vad.process_frame(&silence), VadState::Trailing);

        // Back to speech
        assert_eq!(vad.process_frame(&speech), VadState::Speaking);

        // Consecutive silence frames should end speech
        assert_eq!(vad.process_frame(&silence), VadState::Trailing);
        assert_eq!(vad.process_frame(&silence), VadState::Silent);
    }

    #[test]
    fn test_adaptive_vad() {
        let config = VadConfig::default();
        let mut vad = AdaptiveVad::new(config);

        // Simulate background noise
        let noise = vec![0.01; 1000];
        for _ in 0..100 {
            vad.process_frame(&noise);
        }

        // Noise floor should adapt
        assert!(vad.noise_floor() > 0.0);
        assert!(vad.current_threshold() >= vad.min_threshold);
    }
}
