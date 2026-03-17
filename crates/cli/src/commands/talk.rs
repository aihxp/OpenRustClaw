//! Talk Mode command - continuous voice conversation.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

use openrustclaw_voice::{
    SimpleWakeDetector, SpeechToText, SttConfig, TalkConfig, TalkEvent, TalkModeBuilder, TalkState,
    TextToSpeech, TtsConfig, WakeWordConfig,
};

/// Run the Talk Mode voice conversation.
pub async fn run(
    _provider: &str,
    _model: Option<&str>,
    wake_word: &str,
    silence_timeout_secs: u64,
    max_utterance_secs: u64,
    enable_barge_in: bool,
) -> Result<()> {
    print_banner();

    println!("Wake word: \"{}\"", wake_word);
    println!("Silence timeout: {}s", silence_timeout_secs);
    println!("Max utterance: {}s", max_utterance_secs);
    println!(
        "Barge-in: {}",
        if enable_barge_in {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    // Create voice components
    let wake = Arc::new(SimpleWakeDetector::new(WakeWordConfig::with_wake_word(
        wake_word,
    )));
    let stt = Arc::new(
        SpeechToText::new(SttConfig::default())
            .map_err(|e| anyhow::anyhow!("Failed to create STT: {}", e))?,
    );
    let tts = Arc::new(
        TextToSpeech::new(TtsConfig::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create TTS: {}", e))?,
    );

    // Create Talk Mode configuration
    let config = TalkConfig::with_wake_word(wake_word)
        .with_silence_timeout(Duration::from_secs(silence_timeout_secs))
        .with_max_utterance_duration(Duration::from_secs(max_utterance_secs))
        .with_barge_in(enable_barge_in)
        .with_session_timeout(Some(Duration::from_secs(300)));

    // Build Talk Mode
    let talk_mode = TalkModeBuilder::new()
        .wake(wake)
        .stt(stt)
        .tts(tts)
        .config(config)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build Talk Mode: {}", e))?;

    // Subscribe to events
    let (_event_tx, event_rx) = mpsc::channel(100);

    // Start event display task
    let _event_handle = tokio::spawn(async move {
        display_events(event_rx).await;
    });

    println!("Talk Mode started!");
    println!("Say \"{}\" to start a conversation.", wake_word);
    println!();
    println!("Commands:");
    println!("  Ctrl+C - Stop Talk Mode");
    println!();

    // Setup Ctrl+C handler
    let talk_mode_for_ctrlc = talk_mode.audio_sender();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Ctrl+C received, stopping...");
        let _ = talk_mode_for_ctrlc.send(vec![]).await;
    });

    // Run Talk Mode
    match talk_mode.run().await {
        Ok(()) => {
            println!("\nTalk Mode ended.");
            Ok(())
        }
        Err(e) => {
            error!("Talk Mode error: {}", e);
            Err(anyhow::anyhow!("Talk Mode failed: {}", e))
        }
    }
}

/// Display events from Talk Mode.
async fn display_events(event_rx: mpsc::Receiver<TalkEvent>) {
    let mut event_rx = event_rx;
    while let Some(event) = event_rx.recv().await {
        match event {
            TalkEvent::StateChanged { from, to } => {
                let icon = match to {
                    TalkState::Idle => "🔴",
                    TalkState::Listening => "🎤",
                    TalkState::Processing => "⚙️ ",
                    TalkState::Speaking => "🔊",
                    TalkState::Stopping => "🛑",
                    TalkState::Error => "❌",
                };
                println!("{} State: {} → {}", icon, from, to);
            }
            TalkEvent::WakeWordDetected { word, confidence } => {
                println!(
                    "👋 Wake word detected: \"{}\" (confidence: {:.2})",
                    word, confidence
                );
            }
            TalkEvent::SpeechStarted => {
                println!("💬 Listening...");
            }
            TalkEvent::SpeechEnded { duration } => {
                println!(
                    "✓ Speech ended ({}.{:03}s)",
                    duration.as_secs(),
                    duration.subsec_millis()
                );
            }
            TalkEvent::Transcription { text, confidence } => {
                println!("📝 You said: \"{}\" (confidence: {:.2})", text, confidence);
            }
            TalkEvent::AgentResponse { text } => {
                println!("🤖 Agent: {}", text.chars().take(100).collect::<String>());
                if text.len() > 100 {
                    println!("   ... (truncated)")
                }
            }
            TalkEvent::SpeakingStarted => {
                println!("🔊 Speaking...");
            }
            TalkEvent::SpeakingEnded => {
                println!("✓ Done speaking");
            }
            TalkEvent::BargeIn => {
                println!("⏹️  Barge-in detected, stopping speech");
            }
            TalkEvent::Error { message } => {
                println!("❌ Error: {}", message);
            }
            TalkEvent::SessionTimeout => {
                println!("⏰ Session timed out due to inactivity");
            }
        }
    }
}

/// Print the Talk Mode banner.
fn print_banner() {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           OpenRustClaw Talk Mode                         ║");
    println!("║           Continuous Voice Conversation                  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
}
