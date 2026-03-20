use anyhow::Result;
use openrustclaw_core::config::AppConfig;

use super::voice_runtime::{self, VoiceSynthesizeRequest, VoiceTranscribeRequest};

fn load_config(config_path: &str) -> AppConfig {
    AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default()
}

pub async fn status(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let status = voice_runtime::voice_status(&config, &workspace_root);
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

pub async fn providers(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let providers = voice_runtime::voice_provider_catalog(&config);
    println!("{}", serde_json::to_string_pretty(&providers)?);
    Ok(())
}

pub async fn transcribe(
    config_path: &str,
    path: Option<&str>,
    url: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::transcribe_with_config(
        &config,
        &workspace_root,
        VoiceTranscribeRequest {
            path: path.map(ToString::to_string),
            url: url.map(ToString::to_string),
            provider: provider.map(ToString::to_string),
            model: model.map(ToString::to_string),
            language: language.map(ToString::to_string),
            prompt: prompt.map(ToString::to_string),
            max_audio_bytes: None,
            timeout_secs: None,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn voices(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::list_voices_with_config(&config, &workspace_root)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn synthesize(
    config_path: &str,
    text: &str,
    provider: Option<&str>,
    model: Option<&str>,
    voice: Option<&str>,
    format: Option<&str>,
    output_path: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::synthesize_with_config(
        &config,
        &workspace_root,
        VoiceSynthesizeRequest {
            text: text.to_string(),
            provider: provider.map(ToString::to_string),
            model: model.map(ToString::to_string),
            voice: voice.map(ToString::to_string),
            format: format.map(ToString::to_string),
            output_path: output_path.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
