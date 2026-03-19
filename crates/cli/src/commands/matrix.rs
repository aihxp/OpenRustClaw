use anyhow::Result;
use openrustclaw_channels::ChannelFactory;
use openrustclaw_channels::MatrixChannel;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;

async fn channel_from_config(config_path: &str) -> Result<MatrixChannel> {
    let config = AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default();
    let mut channel = ChannelFactory::create_matrix(config.channels.matrix)?;
    channel.connect().await?;
    Ok(channel)
}

pub async fn join(config_path: &str, room: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.join_room(room).await?;
    println!("joined {}", room);
    channel.disconnect().await?;
    Ok(())
}

pub async fn leave(config_path: &str, room: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.leave_room(room).await?;
    println!("left {}", room);
    channel.disconnect().await?;
    Ok(())
}

pub async fn rooms(config_path: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    let rooms = channel.joined_rooms().await?;
    println!("{}", serde_json::to_string_pretty(&rooms)?);
    channel.disconnect().await?;
    Ok(())
}

pub async fn send_formatted(config_path: &str, room: &str, text: &str, html: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.send_formatted(room, text, html).await?;
    channel.disconnect().await?;
    Ok(())
}

pub async fn react(config_path: &str, room: &str, event_id: &str, emoji: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.send_reaction(room, event_id, emoji).await?;
    channel.disconnect().await?;
    Ok(())
}

pub async fn send_file(
    config_path: &str,
    room: &str,
    file_path: &str,
    filename: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.send_file(room, file_path, filename).await?;
    channel.disconnect().await?;
    Ok(())
}

pub async fn typing(config_path: &str, room: &str, typing: bool) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.send_typing_indicator(room, typing).await?;
    channel.disconnect().await?;
    Ok(())
}

pub async fn redact(
    config_path: &str,
    room: &str,
    event_id: &str,
    reason: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path).await?;
    channel.redact_message(room, event_id, reason).await?;
    channel.disconnect().await?;
    Ok(())
}
