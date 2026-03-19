use anyhow::Result;
use openrustclaw_channels::SignalChannel;
use openrustclaw_core::config::AppConfig;

fn channel_from_config(config_path: &str) -> Result<SignalChannel> {
    let config = AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default();
    Ok(SignalChannel::new(config.channels.signal))
}

pub async fn register(config_path: &str, voice: bool) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    channel.register(voice).await?;
    println!("Signal registration initiated");
    Ok(())
}

pub async fn verify(config_path: &str, code: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    channel.verify(code).await?;
    println!("Signal number verified");
    Ok(())
}

pub async fn link(config_path: &str, device_name: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let uri = channel.link_device(device_name).await?;
    println!("{}", uri);
    Ok(())
}

pub async fn list_groups(config_path: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let groups = channel.list_groups().await?;
    println!("{}", serde_json::to_string_pretty(&groups)?);
    Ok(())
}
