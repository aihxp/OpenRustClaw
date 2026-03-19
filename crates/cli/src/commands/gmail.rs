use anyhow::Result;
use openrustclaw_channels::gmail_pubsub::{EmailAction, GmailNotification, GmailPubSub};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::OutgoingMessage;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

fn load_config(config_path: &str) -> AppConfig {
    AppConfig::load_from(config_path)
        .or_else(|_| AppConfig::load())
        .unwrap_or_default()
}

fn channel_from_config(config_path: &str) -> GmailPubSub {
    let config = load_config(config_path);
    GmailPubSub::new(config.channels.gmail_pubsub)
}

async fn read_input(path: &str) -> Result<String> {
    if path == "-" {
        let mut input = String::new();
        tokio::io::stdin().read_to_string(&mut input).await?;
        Ok(input)
    } else {
        Ok(tokio::fs::read_to_string(path).await?)
    }
}

fn file_reference_values(paths: &[String]) -> Vec<serde_json::Value> {
    paths.iter()
        .map(|path| {
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("attachment");
            serde_json::json!({
                "local_path": path,
                "name": name,
            })
        })
        .collect()
}

pub async fn status(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let gmail = config.channels.gmail_pubsub;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "enabled": gmail.enabled,
            "project_id": gmail.project_id,
            "subscription_name": gmail.subscription_name,
            "topic_name": gmail.topic_name,
            "user_email": gmail.user_email,
            "label_filters": gmail.label_filters,
            "query_filter": gmail.query_filter,
            "auto_reply": gmail.auto_reply,
            "max_history_fetch": gmail.max_history_fetch,
        }))?
    );
    Ok(())
}

pub async fn connect(config_path: &str) -> Result<()> {
    let config = load_config(config_path);
    let mut channel = GmailPubSub::new(config.channels.gmail_pubsub.clone());
    channel.connect().await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "connected": true,
            "project_id": config.channels.gmail_pubsub.project_id,
            "subscription_name": config.channels.gmail_pubsub.subscription_name,
            "user_email": config.channels.gmail_pubsub.user_email,
        }))?
    );
    channel.disconnect().await?;
    Ok(())
}

pub async fn process_notification(config_path: &str, input: &str) -> Result<()> {
    let raw = read_input(input).await?;
    let notification: GmailNotification = serde_json::from_str(&raw)?;
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel.process_notification(notification).await?;
    channel.disconnect().await?;
    println!("processed Gmail notification");
    Ok(())
}

pub async fn send(
    config_path: &str,
    to: &[String],
    cc: &[String],
    bcc: &[String],
    subject: &str,
    body: &str,
    thread_id: Option<&str>,
    files: &[String],
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: body.to_string(),
            metadata: serde_json::json!({
                "gmail_to": to,
                "gmail_cc": cc,
                "gmail_bcc": bcc,
                "gmail_subject": subject,
                "gmail_thread_id": thread_id,
                "file_references": file_reference_values(files),
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("sent Gmail message");
    Ok(())
}

pub async fn reply(
    config_path: &str,
    message_id: &str,
    body: &str,
    files: &[String],
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: body.to_string(),
            metadata: serde_json::json!({
                "gmail_message_id": message_id,
                "gmail_action": "reply",
                "file_references": file_reference_values(files),
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("replied to Gmail message {}", message_id);
    Ok(())
}

pub async fn label(
    config_path: &str,
    message_id: &str,
    add: &[String],
    remove: &[String],
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .take_action(
            message_id,
            EmailAction::Label {
                add: add.to_vec(),
                remove: remove.to_vec(),
            },
        )
        .await?;
    channel.disconnect().await?;
    println!("updated Gmail labels for {}", message_id);
    Ok(())
}

pub async fn archive(config_path: &str, message_id: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel.take_action(message_id, EmailAction::Archive).await?;
    channel.disconnect().await?;
    println!("archived Gmail message {}", message_id);
    Ok(())
}

pub async fn delete(config_path: &str, message_id: &str) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel.take_action(message_id, EmailAction::Delete).await?;
    channel.disconnect().await?;
    println!("deleted Gmail message {}", message_id);
    Ok(())
}

pub async fn forward(
    config_path: &str,
    message_id: &str,
    to: &str,
    body: &str,
    files: &[String],
) -> Result<()> {
    let mut channel = channel_from_config(config_path);
    channel.connect().await?;
    channel
        .send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: body.to_string(),
            metadata: serde_json::json!({
                "gmail_message_id": message_id,
                "gmail_action": "forward",
                "gmail_forward_to": to,
                "file_references": file_reference_values(files),
            }),
        })
        .await?;
    channel.disconnect().await?;
    println!("forwarded Gmail message {}", message_id);
    Ok(())
}
