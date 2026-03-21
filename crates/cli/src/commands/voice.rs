use anyhow::Result;
use openrustclaw_core::config::AppConfig;

use super::voice_runtime::{
    self, VoicePrewarmRequest, VoiceSessionAppendRequest, VoiceSessionControlRequest,
    VoiceSessionEndRequest, VoiceSessionHealthRequest, VoiceSessionReapRequest,
    VoiceSessionReconnectRequest, VoiceSessionRespondRequest, VoiceSessionStartRequest,
    VoiceSynthesizeRequest, VoiceTranscribeRequest,
};

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

pub async fn sessions(config_path: &str) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::list_voice_sessions(&workspace_root).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn session_health(config_path: &str, stale_after_secs: Option<u64>) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::voice_session_health(
        &workspace_root,
        VoiceSessionHealthRequest { stale_after_secs },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn prewarm(
    config_path: &str,
    provider: Option<&str>,
    model: Option<&str>,
    voice: Option<&str>,
    format: Option<&str>,
    greeting: Option<&str>,
    output_path: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::prewarm_voice_runtime(
        &config,
        &workspace_root,
        VoicePrewarmRequest {
            provider: provider.map(ToString::to_string),
            model: model.map(ToString::to_string),
            voice: voice.map(ToString::to_string),
            format: format.map(ToString::to_string),
            greeting: greeting.map(ToString::to_string),
            output_path: output_path.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn reap_sessions(
    config_path: &str,
    stale_after_secs: Option<u64>,
    reason: Option<&str>,
) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::reap_voice_sessions(
        &workspace_root,
        VoiceSessionReapRequest {
            stale_after_secs,
            reason: reason.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn start_session(
    config_path: &str,
    session_id: Option<&str>,
    assistant_prompt: Option<&str>,
    voice: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::start_voice_session(
        &config,
        &workspace_root,
        VoiceSessionStartRequest {
            session_id: session_id.map(ToString::to_string),
            assistant_prompt: assistant_prompt.map(ToString::to_string),
            voice: voice.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn session_status(config_path: &str, session_id: &str) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::inspect_voice_session(&workspace_root, session_id).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn transcript(config_path: &str, session_id: &str) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::voice_session_transcript(&workspace_root, session_id).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn append_user(config_path: &str, session_id: &str, text: &str) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::append_voice_session_user(
        &workspace_root,
        session_id,
        VoiceSessionAppendRequest {
            text: text.to_string(),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn respond(
    config_path: &str,
    session_id: &str,
    text: &str,
    provider: Option<&str>,
    model: Option<&str>,
    voice: Option<&str>,
    format: Option<&str>,
    output_path: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::respond_voice_session(
        &config,
        &workspace_root,
        session_id,
        VoiceSessionRespondRequest {
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

pub async fn reconnect_session(
    config_path: &str,
    session_id: &str,
    assistant_prompt: Option<&str>,
    voice: Option<&str>,
    greeting: Option<&str>,
    format: Option<&str>,
    output_path: Option<&str>,
) -> Result<()> {
    let config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::reconnect_voice_session(
        &config,
        &workspace_root,
        session_id,
        VoiceSessionReconnectRequest {
            assistant_prompt: assistant_prompt.map(ToString::to_string),
            voice: voice.map(ToString::to_string),
            greeting: greeting.map(ToString::to_string),
            format: format.map(ToString::to_string),
            output_path: output_path.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn pause_session(
    config_path: &str,
    session_id: &str,
    reason: Option<&str>,
) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::pause_voice_session(
        &workspace_root,
        session_id,
        VoiceSessionControlRequest {
            reason: reason.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn resume_session(
    config_path: &str,
    session_id: &str,
    reason: Option<&str>,
) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::resume_voice_session(
        &workspace_root,
        session_id,
        VoiceSessionControlRequest {
            reason: reason.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn interrupt_session(
    config_path: &str,
    session_id: &str,
    reason: Option<&str>,
) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::interrupt_voice_session(
        &workspace_root,
        session_id,
        VoiceSessionControlRequest {
            reason: reason.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub async fn end_session(config_path: &str, session_id: &str, reason: Option<&str>) -> Result<()> {
    let _config = load_config(config_path);
    let workspace_root = std::env::current_dir()?;
    let result = voice_runtime::end_voice_session(
        &workspace_root,
        session_id,
        VoiceSessionEndRequest {
            reason: reason.map(ToString::to_string),
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
