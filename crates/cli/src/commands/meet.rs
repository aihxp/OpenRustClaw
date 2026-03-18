use anyhow::{Context, Result};
use openrustclaw_channels::ChannelFactory;
use openrustclaw_channels::GoogleMeetClient;
use openrustclaw_core::config::AppConfig;

async fn client_from_config(config_path: &str) -> Result<GoogleMeetClient> {
    let config = AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default();
    let client = ChannelFactory::create_google_meet(config.channels.google_meet)?;
    client.connect().await?;
    Ok(client)
}

pub async fn create_space(config_path: &str) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.create_space().await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn get_space(config_path: &str, name: &str) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.get_space(name).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn end_space(config_path: &str, name: &str) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.end_active_conference(name).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn list_records(config_path: &str, limit: usize) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.list_conference_records(limit).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn list_participants(
    config_path: &str,
    conference_record: &str,
    limit: usize,
) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.list_participants(conference_record, limit).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn list_recordings(
    config_path: &str,
    conference_record: &str,
    limit: usize,
) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.list_recordings(conference_record, limit).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn list_transcripts(
    config_path: &str,
    conference_record: &str,
    limit: usize,
) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.list_transcripts(conference_record, limit).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn show_transcript(config_path: &str, transcript: &str, limit: usize) -> Result<()> {
    let client = client_from_config(config_path).await?;
    let response = client.list_transcript_entries(transcript, limit).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    client.disconnect().await?;
    Ok(())
}

pub async fn decode_event(config_path: &str, input: &str) -> Result<()> {
    let body = if input == "-" {
        use tokio::io::AsyncReadExt;
        let mut stdin = tokio::io::stdin();
        let mut bytes = Vec::new();
        stdin.read_to_end(&mut bytes).await?;
        bytes
    } else {
        tokio::fs::read(input)
            .await
            .with_context(|| format!("Failed to read '{}'", input))?
    };
    let client = client_from_config(config_path).await?;
    let event = client.webhook_handler().decode_push(&body).await?;
    println!("{}", serde_json::to_string_pretty(&event)?);
    client.disconnect().await?;
    Ok(())
}
