use anyhow::Result;
use openrustclaw_channels::WhatsAppChannel;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::OutgoingMessage;
use tokio::time::Duration;
use uuid::Uuid;

fn channel_from_config(config_path: &str) -> Result<WhatsAppChannel> {
    let config = AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default();
    Ok(WhatsAppChannel::new(config.channels.whatsapp))
}

pub async fn status(config_path: &str) -> Result<()> {
    let channel = channel_from_config(config_path)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "state": channel.connection_state().await.to_string(),
            "latest_qr_code": channel.latest_qr_code().await,
            "latest_pairing_code": channel.latest_pairing_code().await,
        }))?
    );
    Ok(())
}

pub async fn connect(config_path: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    println!("{}", channel.connection_state().await);
    channel.disconnect().await?;
    Ok(())
}

pub async fn pair(config_path: &str, timeout_secs: u64) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    let artifact = channel
        .wait_for_pairing_artifact(Duration::from_secs(timeout_secs))
        .await?;
    println!("{}", artifact);
    channel.disconnect().await?;
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
                "whatsapp_chat_id": to,
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent WhatsApp message to {}", to);
    Ok(())
}

pub async fn send_media(
    config_path: &str,
    to: &str,
    media_type: &str,
    path: &str,
    caption: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path)?;
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: caption.unwrap_or_default().to_string(),
            metadata: serde_json::json!({
                "whatsapp_chat_id": to,
                "whatsapp_media_type": media_type,
                "file_references": [{
                    "local_path": path
                }],
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent WhatsApp media to {}", to);
    Ok(())
}
