use anyhow::Result;
use openrustclaw_channels::IMessageChannel;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::OutgoingMessage;
use uuid::Uuid;

fn channel_from_config(config_path: &str) -> Result<IMessageChannel> {
    let config = AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default();
    Ok(IMessageChannel::new(config.channels.imessage))
}

pub async fn ping(config_path: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let value = channel.ping().await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub async fn server(config_path: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let value = channel.server_info().await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub async fn chats(config_path: &str, limit: usize, offset: usize) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let value = channel.list_chats(limit, offset).await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub async fn contacts(config_path: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    let value = channel.list_contacts().await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub async fn send(config_path: &str, to: &str, message: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: message.to_string(),
            metadata: serde_json::json!({
                "imessage_recipient": to
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent iMessage to {}", to);
    Ok(())
}

pub async fn send_file(
    config_path: &str,
    to: &str,
    file_path: &str,
    filename: Option<&str>,
    message: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: message.unwrap_or_default().to_string(),
            metadata: serde_json::json!({
                "imessage_recipient": to,
                "file_references": [{
                    "local_path": file_path,
                    "name": filename,
                }]
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent iMessage attachment to {}", to);
    Ok(())
}

pub async fn tapback(
    config_path: &str,
    chat_guid: &str,
    message_guid: &str,
    reaction: &str,
) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: String::new(),
            metadata: serde_json::json!({
                "imessage_chat_guid": chat_guid,
                "imessage_message_guid": message_guid,
                "imessage_tapback": reaction
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent iMessage tapback");
    Ok(())
}
