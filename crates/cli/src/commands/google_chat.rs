use anyhow::Result;
use openrustclaw_channels::GoogleChatChannel;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::OutgoingMessage;
use uuid::Uuid;

fn load_config(config_path: &str) -> AppConfig {
    AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default()
}

fn channel_from_config(config_path: &str) -> GoogleChatChannel {
    let config = load_config(config_path);
    GoogleChatChannel::new(config.channels.google_chat)
}

pub async fn status(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let google_chat = config.channels.google_chat;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "enabled": google_chat.enabled,
            "project_id": google_chat.project_id,
            "webhook_url": google_chat.webhook_url,
            "pubsub_subscription": google_chat.pubsub_subscription,
            "allowed_spaces": google_chat.allowed_spaces,
            "allowlist": google_chat.allowlist,
            "cards_enabled": google_chat.cards_enabled,
            "attachment_download_dir": google_chat.attachment_download_dir,
            "response_mode": google_chat.response_mode,
        }))?
    );
    Ok(())
}

pub async fn connect(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let mut channel = GoogleChatChannel::new(config.channels.google_chat.clone());
    channel.connect().await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "connected": true,
            "project_id": config.channels.google_chat.project_id,
            "response_mode": config.channels.google_chat.response_mode,
            "webhook_url": config.channels.google_chat.webhook_url,
            "pubsub_subscription": config.channels.google_chat.pubsub_subscription,
        }))?
    );
    channel.disconnect().await?;
    Ok(())
}

pub async fn send(
    config_path: &str,
    space: &str,
    message: &str,
    thread: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: message.to_string(),
            metadata: serde_json::json!({
                "google_chat_space": space,
                "google_chat_thread": thread,
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent Google Chat message to {}", space);
    Ok(())
}

pub async fn send_card(
    config_path: &str,
    space: &str,
    title: &str,
    content: &str,
    thread: Option<&str>,
    subtitle: Option<&str>,
    image_url: Option<&str>,
    section_header: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: String::new(),
            metadata: serde_json::json!({
                "google_chat_space": space,
                "google_chat_thread": thread,
                "cards": [{
                    "title": title,
                    "content": content,
                    "subtitle": subtitle,
                    "image_url": image_url,
                    "section_header": section_header,
                }],
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent Google Chat card to {}", space);
    Ok(())
}

pub async fn send_link_card(
    config_path: &str,
    space: &str,
    url: &str,
    title: Option<&str>,
    thread: Option<&str>,
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: String::new(),
            metadata: serde_json::json!({
                "google_chat_space": space,
                "google_chat_thread": thread,
                "file_references": [{
                    "url": url,
                    "title": title.unwrap_or("Open link"),
                }],
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent Google Chat link card to {}", space);
    Ok(())
}
