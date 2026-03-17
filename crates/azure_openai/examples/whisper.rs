//! Whisper Audio Transcription example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_WHISPER_DEPLOYMENT=your-whisper-deployment \
//! AUDIO_FILE=audio.mp3 \
//! cargo run --example whisper
//! ```

use azure_openai::{AzureOpenAIClient, TranscriptionRequest, TtsRequest, TtsVoice, TtsResponseFormat};
use azure_openai::audio::{AudioResponseFormat, TimestampGranularity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let whisper_deployment = std::env::var("AZURE_OPENAI_WHISPER_DEPLOYMENT")
        .expect("AZURE_OPENAI_WHISPER_DEPLOYMENT environment variable not set");

    let audio_file = std::env::var("AUDIO_FILE").unwrap_or_else(|_| "audio.mp3".to_string());

    // Create the client for Whisper
    let client = AzureOpenAIClient::new(
        &resource_name,
        &whisper_deployment,
        &api_key,
    )?;

    println!("Azure OpenAI Whisper Audio Example\n");

    // Check if audio file exists
    if !std::path::Path::new(&audio_file).exists() {
        println!("Audio file '{}' not found.", audio_file);
        println!("Please provide an audio file using the AUDIO_FILE environment variable.");
        println!();
        println!("If you don't have an audio file, you can:");
        println!("1. Create a TTS audio file first using the text-to-speech example");
        println!("2. Or use any MP3, WAV, or other supported audio file");
        return Ok(());
    }

    // Basic transcription
    println!("1. Basic Transcription:");
    let request = TranscriptionRequest::new(&audio_file)
        .response_format(AudioResponseFormat::Json);

    let response = client.audio().transcribe(request).await?;
    println!("   Text: {}", response.text);
    if let Some(language) = response.language {
        println!("   Detected language: {}", language);
    }
    if let Some(duration) = response.duration {
        println!("   Duration: {:.2}s", duration);
    }

    // Transcription with timestamps
    println!("\n2. Transcription with Word Timestamps:");
    let request_timestamps = TranscriptionRequest::new(&audio_file)
        .response_format(AudioResponseFormat::VerboseJson)
        .timestamp_granularity(TimestampGranularity::Word);

    let response_ts = client.audio().transcribe(request_timestamps).await?;
    
    if let Some(words) = response_ts.words {
        println!("   Words detected: {}", words.len());
        for word in words.iter().take(10) {
            println!("   [{:.2}s - {:.2}s] {}", word.start, word.end, word.word);
        }
        if words.len() > 10 {
            println!("   ... and {} more words", words.len() - 10);
        }
    }

    // Translation (if audio is in another language)
    println!("\n3. Translation to English:");
    let request_translate = TranscriptionRequest::new(&audio_file);
    let response_translate = client.audio().translate(request_translate).await?;
    println!("   English translation: {}", response_translate.text);

    // Try TTS if we have a TTS deployment
    if let Ok(tts_deployment) = std::env::var("AZURE_OPENAI_TTS_DEPLOYMENT") {
        println!("\n4. Text-to-Speech:");
        
        let tts_client = AzureOpenAIClient::new(
            &resource_name,
            &tts_deployment,
            &api_key,
        )?;

        let tts_request = TtsRequest::new(
            "Hello! This is a test of Azure OpenAI text to speech.",
            TtsVoice::Alloy
        )
        .response_format(TtsResponseFormat::Mp3)
        .speed(1.0);

        let audio_bytes = tts_client.audio().speech(tts_request).await?;
        
        let output_file = "tts_output.mp3";
        tokio::fs::write(output_file, audio_bytes).await?;
        println!("   Generated speech saved to: {}", output_file);
        println!("   Voice: Alloy");
    }

    println!("\nDone!");
    Ok(())
}
