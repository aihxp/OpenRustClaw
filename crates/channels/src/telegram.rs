//! Telegram Bot API integration for OpenRustClaw.
//!
//! Features:
//! - Text messages
//! - Commands (/start, /help, etc.)
//! - Inline queries
//! - File uploads/downloads
//! - Group chats
//! - Reply keyboards
//! - Webhook or polling mode

use std::sync::Arc;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::{TelegramConfig, TelegramMode};
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// Telegram channel implementation.
pub struct TelegramChannel {
    config: TelegramConfig,
    client: Client,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: RwLock<bool>,
    polling_task: Mutex<Option<JoinHandle<()>>>,
    bot_username: Arc<RwLock<Option<String>>>,
}

impl TelegramChannel {
    /// Create a new Telegram channel with the given configuration.
    pub fn new(config: TelegramConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(30).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            config,
            client: Client::new(),
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter,
            is_connected: RwLock::new(false),
            polling_task: Mutex::new(None),
            bot_username: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a user is allowed to interact with the bot.
    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: i64) -> bool {
        if self.config.allowed_users.is_empty() {
            return true;
        }
        let user_id_str = user_id.to_string();
        self.config.allowed_users.contains(&user_id_str)
    }

    /// Parse inline keyboard from metadata.
    fn parse_inline_keyboard(metadata: &serde_json::Value) -> Option<serde_json::Value> {
        metadata.get("inline_keyboard").cloned()
    }

    fn api_base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://api.telegram.org".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn build_file_references(message: &serde_json::Value) -> Vec<serde_json::Value> {
        fn push_file_ref(
            refs: &mut Vec<serde_json::Value>,
            message_id: Option<i64>,
            media_type: &str,
            item: &serde_json::Value,
        ) {
            let Some(file_id) = item.get("file_id").and_then(|value| value.as_str()) else {
                return;
            };
            let mut file_ref = serde_json::Map::new();
            file_ref.insert("id".into(), serde_json::json!(file_id));
            file_ref.insert("platform_ref".into(), serde_json::json!(file_id));
            file_ref.insert("media_type".into(), serde_json::json!(media_type));
            if let Some(message_id) = message_id {
                file_ref.insert("message_id".into(), serde_json::json!(message_id));
            }
            if let Some(name) = item.get("file_name").and_then(|value| value.as_str()) {
                file_ref.insert("name".into(), serde_json::json!(name));
            }
            if let Some(mime) = item.get("mime_type").and_then(|value| value.as_str()) {
                file_ref.insert("mime_type".into(), serde_json::json!(mime));
            }
            if let Some(size) = item.get("file_size").and_then(|value| value.as_u64()) {
                file_ref.insert("size".into(), serde_json::json!(size));
            }
            refs.push(serde_json::Value::Object(file_ref));
        }

        let mut refs = Vec::new();
        let message_id = message.get("message_id").and_then(|value| value.as_i64());
        if let Some(photo) = message.get("photo").and_then(|value| value.as_array()) {
            for item in photo {
                push_file_ref(&mut refs, message_id, "image", item);
            }
        } else if let Some(document) = message.get("document") {
            push_file_ref(&mut refs, message_id, "document", document);
        } else if let Some(audio) = message.get("audio") {
            push_file_ref(&mut refs, message_id, "audio", audio);
        } else if let Some(voice) = message.get("voice") {
            push_file_ref(&mut refs, message_id, "voice", voice);
        } else if let Some(video) = message.get("video") {
            push_file_ref(&mut refs, message_id, "video", video);
        }
        refs
    }

    fn normalize_incoming_message(
        message: &serde_json::Value,
        allowed_users: &[String],
        bot_username: Option<&str>,
    ) -> Option<IncomingMessage> {
        let from = message.get("from")?;
        let user_id = from
            .get("id")
            .and_then(|value| value.as_i64())
            .map(|value| value.to_string())
            .unwrap_or_default();

        if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
            debug!(user_id = %user_id, "Skipping Telegram user outside allowlist");
            return None;
        }

        let content = message
            .get("text")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .or_else(|| {
                message
                    .get("caption")
                    .and_then(|value| value.as_str())
                    .map(ToString::to_string)
            })
            .or_else(|| {
                message.get("poll").map(|poll| {
                    let question = poll
                        .get("question")
                        .and_then(|value| value.as_str())
                        .unwrap_or("Poll");
                    let options = poll
                        .get("options")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|option| {
                                    option.get("text").and_then(|value| value.as_str())
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    if options.is_empty() {
                        format!("[Poll] {question}")
                    } else {
                        format!("[Poll] {question} Options: {options}")
                    }
                })
            })
            .or_else(|| {
                if message.get("photo").is_some() {
                    Some("[Image]".to_string())
                } else if message.get("document").is_some() {
                    Some("[Document]".to_string())
                } else if message.get("audio").is_some() {
                    Some("[Audio]".to_string())
                } else if message.get("voice").is_some() {
                    Some("[Voice message]".to_string())
                } else if message.get("video").is_some() {
                    Some("[Video]".to_string())
                } else if let Some(topic_created) = message.get("forum_topic_created") {
                    let name = topic_created
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("topic");
                    Some(format!("[Forum topic created] {name}"))
                } else if message.get("forum_topic_closed").is_some() {
                    Some("[Forum topic closed]".to_string())
                } else if message.get("forum_topic_reopened").is_some() {
                    Some("[Forum topic reopened]".to_string())
                } else if let Some(topic_edited) = message.get("forum_topic_edited") {
                    let name = topic_edited
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("topic");
                    Some(format!("[Forum topic edited] {name}"))
                } else {
                    None
                }
            })?;

        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(|value| value.as_i64())
            .map(|value| value.to_string())
            .unwrap_or_default();

        let mut metadata = serde_json::json!({
            "telegram_chat_id": chat_id,
            "telegram_chat_type": message
                .get("chat")
                .and_then(|chat| chat.get("type"))
                .and_then(|value| value.as_str())
                .unwrap_or("private"),
            "telegram_is_group": message
                .get("chat")
                .and_then(|chat| chat.get("type"))
                .and_then(|value| value.as_str())
                .map(|value| matches!(value, "group" | "supergroup"))
                .unwrap_or(false),
        });
        if let Some(message_id) = message.get("message_id").and_then(|value| value.as_i64()) {
            metadata["telegram_message_id"] = serde_json::json!(message_id);
        }
        if let Some(username) = from.get("username").and_then(|value| value.as_str()) {
            metadata["telegram_username"] = serde_json::json!(username);
        }
        if let Some(reply_to) = message
            .get("reply_to_message")
            .and_then(|value| value.get("message_id"))
            .and_then(|value| value.as_i64())
        {
            metadata["telegram_reply_to_message_id"] = serde_json::json!(reply_to);
        }
        if let Some(thread_id) = message
            .get("message_thread_id")
            .and_then(|value| value.as_i64())
        {
            metadata["telegram_message_thread_id"] = serde_json::json!(thread_id);
        }
        if message
            .get("is_topic_message")
            .and_then(|value| value.as_bool())
            == Some(true)
        {
            metadata["telegram_is_topic_message"] = serde_json::json!(true);
        }
        if let Some(topic_created) = message.get("forum_topic_created") {
            metadata["telegram_forum_topic_created"] = topic_created.clone();
        }
        if message.get("forum_topic_closed").is_some() {
            metadata["telegram_forum_topic_closed"] = serde_json::json!(true);
        }
        if message.get("forum_topic_reopened").is_some() {
            metadata["telegram_forum_topic_reopened"] = serde_json::json!(true);
        }
        if let Some(topic_edited) = message.get("forum_topic_edited") {
            metadata["telegram_forum_topic_edited"] = topic_edited.clone();
        }
        if let Some(poll) = message.get("poll") {
            metadata["telegram_poll"] = poll.clone();
        }

        let file_refs = Self::build_file_references(message);
        if !file_refs.is_empty() {
            metadata["file_references"] = serde_json::json!(file_refs);
            metadata["telegram_file_references"] = serde_json::json!(file_refs);
        }
        if message.get("photo").is_some() {
            metadata["telegram_media_type"] = serde_json::json!("image");
        } else if message.get("document").is_some() {
            metadata["telegram_media_type"] = serde_json::json!("document");
        } else if message.get("audio").is_some() {
            metadata["telegram_media_type"] = serde_json::json!("audio");
        } else if message.get("voice").is_some() {
            metadata["telegram_media_type"] = serde_json::json!("voice");
        } else if message.get("video").is_some() {
            metadata["telegram_media_type"] = serde_json::json!("video");
        }

        if let Some(bot_username) = bot_username {
            metadata["telegram_bot_mentioned"] =
                serde_json::json!(content.contains(&format!("@{bot_username}")));
        }

        Some(IncomingMessage {
            session_id: Uuid::new_v4(),
            user_id,
            content,
            platform: Platform::Telegram,
            metadata,
        })
    }

    fn chat_id_from_metadata(metadata: &serde_json::Value) -> Result<String> {
        if let Some(value) = metadata.get("telegram_chat_id") {
            if let Some(chat_id) = value.as_str() {
                return Ok(chat_id.to_string());
            }
            if let Some(chat_id) = value.as_i64() {
                return Ok(chat_id.to_string());
            }
            if let Some(chat_id) = value.as_u64() {
                return Ok(chat_id.to_string());
            }
        }

        Err(ChannelError::InvalidFormat {
            platform: "telegram".to_string(),
            message: "Missing chat_id in metadata".to_string(),
        }
        .into())
    }

    async fn spawn_polling_task(&self) -> Result<()> {
        let mut task_guard = self.polling_task.lock().await;
        if task_guard.is_some() {
            return Ok(());
        }

        let client = self.client.clone();
        let incoming_tx = self.incoming_tx.clone();
        let token = self.config.token.clone();
        let api_base_url = self.api_base_url();
        let allowed_users = self.config.allowed_users.clone();
        let bot_username = self.bot_username.read().await.clone();

        *task_guard = Some(tokio::spawn(async move {
            let mut next_offset: i64 = 0;

            loop {
                let response = client
                    .get(format!("{}/bot{}/getUpdates", api_base_url, token))
                    .query(&[
                        ("timeout", "30"),
                        ("offset", &next_offset.to_string()),
                        ("allowed_updates", "[\"message\"]"),
                    ])
                    .send()
                    .await;

                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        warn!(error = %error, "Telegram polling request failed");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let status = response.status();
                let body: serde_json::Value = match response.json().await {
                    Ok(body) => body,
                    Err(error) => {
                        warn!(error = %error, "Failed to parse Telegram polling response");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                    warn!(status = %status, body = %body, "Telegram polling returned failure");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                let updates = body
                    .get("result")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default();

                for update in updates {
                    if let Some(update_id) =
                        update.get("update_id").and_then(|value| value.as_i64())
                    {
                        next_offset = update_id + 1;
                    }

                    let Some(message) = update.get("message") else {
                        continue;
                    };

                    let Some(incoming) = TelegramChannel::normalize_incoming_message(
                        message,
                        &allowed_users,
                        bot_username.as_deref(),
                    ) else {
                        continue;
                    };

                    if let Err(error) = incoming_tx.send(incoming).await {
                        error!(error = %error, "Failed to enqueue Telegram incoming message");
                        return;
                    }
                }
            }
        }));

        Ok(())
    }

    async fn set_webhook(&self, webhook_url: &str) -> Result<()> {
        let response = self
            .client
            .post(format!(
                "{}/bot{}/setWebhook",
                self.api_base_url(),
                self.config.token
            ))
            .json(&serde_json::json!({
                "url": webhook_url,
                "allowed_updates": ["message"],
            }))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "telegram".to_string(),
                message: format!("Failed to configure Telegram webhook: {}", e),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram setWebhook failed")
                    .to_string(),
            }
            .into());
        }

        Ok(())
    }

    async fn delete_webhook(&self) -> Result<()> {
        let response = self
            .client
            .post(format!(
                "{}/bot{}/deleteWebhook",
                self.api_base_url(),
                self.config.token
            ))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "telegram".to_string(),
                message: format!("Failed to delete Telegram webhook: {}", e),
            })?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram deleteWebhook failed")
                    .to_string(),
            }
            .into());
        }

        Ok(())
    }

    pub fn webhook_handler(&self) -> TelegramWebhookHandler {
        TelegramWebhookHandler::new(
            self.config.allowed_users.clone(),
            self.incoming_tx.clone(),
            self.bot_username.clone(),
        )
    }
}

#[derive(Clone)]
pub struct TelegramWebhookHandler {
    allowed_users: Vec<String>,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    bot_username: Arc<RwLock<Option<String>>>,
}

impl TelegramWebhookHandler {
    fn new(
        allowed_users: Vec<String>,
        incoming_tx: mpsc::Sender<IncomingMessage>,
        bot_username: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            allowed_users,
            incoming_tx,
            bot_username,
        }
    }

    pub async fn handle_event(&self, body: &[u8]) -> Result<()> {
        let update: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "telegram".to_string(),
                message: format!("Failed to parse Telegram webhook payload: {}", e),
            })?;
        let Some(message) = update.get("message") else {
            return Ok(());
        };
        let bot_username = self.bot_username.read().await.clone();
        let Some(incoming) = TelegramChannel::normalize_incoming_message(
            message,
            &self.allowed_users,
            bot_username.as_deref(),
        ) else {
            return Ok(());
        };
        self.incoming_tx
            .send(incoming)
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "telegram".to_string(),
                message: format!("Failed to enqueue Telegram webhook message: {}", e),
            })?;
        Ok(())
    }
}

impl TelegramChannel {
    fn media_reference(metadata: &serde_json::Value) -> Option<serde_json::Value> {
        metadata
            .get("telegram_media")
            .cloned()
            .or_else(|| metadata.get("telegram_media_url").cloned())
            .or_else(|| metadata.get("telegram_media_path").cloned())
            .or_else(|| {
                metadata
                    .get("file_references")
                    .and_then(|value| value.as_array())
                    .and_then(|values| values.first())
                    .cloned()
            })
    }

    fn media_reference_string(reference: &serde_json::Value) -> Option<String> {
        if let Some(value) = reference.as_str() {
            return Some(value.to_string());
        }
        reference
            .get("url")
            .or_else(|| reference.get("platform_ref"))
            .or_else(|| reference.get("id"))
            .or_else(|| reference.get("telegram_file_id"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    }

    fn media_reference_local_path(reference: &serde_json::Value) -> Option<String> {
        if let Some(value) = reference.get("local_path").and_then(|value| value.as_str()) {
            return Some(value.to_string());
        }
        reference
            .as_str()
            .filter(|value| std::path::Path::new(value).exists())
            .map(ToString::to_string)
    }

    async fn post_telegram_payload(
        &self,
        url: String,
        payload: serde_json::Value,
        upload: Option<(&str, String, Option<String>)>,
    ) -> Result<reqwest::Response> {
        let request = if let Some((field, local_path, filename)) = upload {
            let bytes =
                tokio::fs::read(&local_path)
                    .await
                    .map_err(|e| ChannelError::SendFailed {
                        platform: "telegram".to_string(),
                        message: format!("Failed to read Telegram upload '{}': {}", local_path, e),
                    })?;
            let mut form = Form::new();
            if let Some(object) = payload.as_object() {
                for (key, value) in object {
                    if value.is_null() {
                        continue;
                    }
                    let text = match value {
                        serde_json::Value::String(text) => text.clone(),
                        _ => value.to_string(),
                    };
                    form = form.text(key.clone(), text);
                }
            }
            let mut part = Part::bytes(bytes);
            if let Some(filename) = filename.or_else(|| {
                std::path::Path::new(&local_path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(ToString::to_string)
            }) {
                part = part.file_name(filename);
            }
            self.client
                .post(url)
                .multipart(form.part(field.to_string(), part))
        } else {
            self.client.post(url).json(&payload)
        };

        Ok(request.send().await.map_err(|e| ChannelError::SendFailed {
            platform: "telegram".to_string(),
            message: e.to_string(),
        })?)
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn platform(&self) -> Platform {
        Platform::Telegram
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "telegram".to_string(),
            }
            .into());
        }

        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract chat ID from session metadata
        let chat_id = Self::chat_id_from_metadata(&msg.metadata)?;

        // Parse inline keyboard from metadata
        let reply_markup = Self::parse_inline_keyboard(&msg.metadata);
        let admin_action = msg
            .metadata
            .get("telegram_forum_action")
            .or_else(|| msg.metadata.get("telegram_admin_action"))
            .and_then(|value| value.as_str());
        let media_type = msg
            .metadata
            .get("telegram_media_type")
            .and_then(|value| value.as_str());
        let media_ref = Self::media_reference(&msg.metadata);
        let is_poll = msg
            .metadata
            .get("telegram_poll_options")
            .and_then(|value| value.as_array());
        let reaction = msg
            .metadata
            .get("telegram_reaction")
            .and_then(|value| value.as_str());
        let reaction_target = msg
            .metadata
            .get("telegram_reaction_message_id")
            .or_else(|| msg.metadata.get("telegram_reply_to_message_id"))
            .and_then(|value| value.as_i64());
        let endpoint = if let Some(action) = admin_action {
            match action {
                "create_topic" => "createForumTopic",
                "edit_topic" => "editForumTopic",
                "close_topic" => "closeForumTopic",
                "reopen_topic" => "reopenForumTopic",
                "delete_topic" => "deleteForumTopic",
                "unpin_all_topic_messages" => "unpinAllForumTopicMessages",
                "pin_message" => "pinChatMessage",
                "unpin_message" => "unpinChatMessage",
                _ => {
                    return Err(ChannelError::InvalidFormat {
                        platform: "telegram".to_string(),
                        message: format!("Unsupported telegram forum/admin action: {}", action),
                    }
                    .into());
                }
            }
        } else if reaction.is_some() && reaction_target.is_some() {
            "setMessageReaction"
        } else if is_poll.is_some() {
            "sendPoll"
        } else {
            match media_type {
                Some("image") => "sendPhoto",
                Some("document") => "sendDocument",
                Some("audio") => "sendAudio",
                Some("voice") => "sendVoice",
                Some("video") => "sendVideo",
                _ => "sendMessage",
            }
        };
        let url = format!(
            "{}/bot{}/{}",
            self.api_base_url(),
            self.config.token,
            endpoint
        );
        let mut payload = serde_json::json!({
            "chat_id": chat_id,
        });
        if let Some(action) = admin_action {
            match action {
                "create_topic" => {
                    let topic_name = msg
                        .metadata
                        .get("telegram_topic_name")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .unwrap_or(msg.content.as_str());
                    if topic_name.is_empty() {
                        return Err(ChannelError::InvalidFormat {
                            platform: "telegram".to_string(),
                            message: "telegram_topic_name or message content is required for create_topic".to_string(),
                        }
                        .into());
                    }
                    payload["name"] = serde_json::json!(topic_name);
                    if let Some(icon_color) = msg
                        .metadata
                        .get("telegram_topic_icon_color")
                        .and_then(|value| value.as_u64())
                    {
                        payload["icon_color"] = serde_json::json!(icon_color);
                    }
                    if let Some(icon_emoji) = msg
                        .metadata
                        .get("telegram_topic_icon_custom_emoji_id")
                        .and_then(|value| value.as_str())
                    {
                        payload["icon_custom_emoji_id"] = serde_json::json!(icon_emoji);
                    }
                }
                "edit_topic" => {
                    let thread_id = msg
                        .metadata
                        .get("telegram_message_thread_id")
                        .and_then(|value| value.as_i64())
                        .ok_or_else(|| ChannelError::InvalidFormat {
                            platform: "telegram".to_string(),
                            message: "telegram_message_thread_id is required for edit_topic"
                                .to_string(),
                        })?;
                    payload["message_thread_id"] = serde_json::json!(thread_id);
                    if let Some(topic_name) = msg
                        .metadata
                        .get("telegram_topic_name")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .or_else(|| (!msg.content.is_empty()).then_some(msg.content.as_str()))
                    {
                        payload["name"] = serde_json::json!(topic_name);
                    }
                    if let Some(icon_emoji) = msg
                        .metadata
                        .get("telegram_topic_icon_custom_emoji_id")
                        .and_then(|value| value.as_str())
                    {
                        payload["icon_custom_emoji_id"] = serde_json::json!(icon_emoji);
                    }
                }
                "close_topic" | "reopen_topic" | "delete_topic" | "unpin_all_topic_messages" => {
                    let thread_id = msg
                        .metadata
                        .get("telegram_message_thread_id")
                        .and_then(|value| value.as_i64())
                        .ok_or_else(|| ChannelError::InvalidFormat {
                            platform: "telegram".to_string(),
                            message:
                                "telegram_message_thread_id is required for topic admin actions"
                                    .to_string(),
                        })?;
                    payload["message_thread_id"] = serde_json::json!(thread_id);
                }
                "pin_message" | "unpin_message" => {
                    let message_id = msg
                        .metadata
                        .get("telegram_pin_message_id")
                        .or_else(|| msg.metadata.get("telegram_reply_to_message_id"))
                        .and_then(|value| value.as_i64())
                        .ok_or_else(|| ChannelError::InvalidFormat {
                            platform: "telegram".to_string(),
                            message:
                                "telegram_pin_message_id or telegram_reply_to_message_id is required for pin/unpin".to_string(),
                        })?;
                    payload["message_id"] = serde_json::json!(message_id);
                }
                _ => {}
            }
        } else if let (Some(reaction), Some(message_id)) = (reaction, reaction_target) {
            payload["message_id"] = serde_json::json!(message_id);
            payload["reaction"] = serde_json::json!([{
                "type": "emoji",
                "emoji": reaction,
            }]);
        } else if let Some(options) = is_poll {
            payload["question"] = serde_json::json!(msg.content);
            payload["options"] = serde_json::json!(options);
        } else if let (Some(kind), Some(reference)) = (media_type, media_ref.as_ref()) {
            let field = match kind {
                "image" => "photo",
                "document" => "document",
                "audio" => "audio",
                "voice" => "voice",
                "video" => "video",
                _ => "text",
            };
            if let Some(reference) = Self::media_reference_string(reference) {
                payload[field] = serde_json::json!(reference);
            }
            if !msg.content.is_empty() {
                payload["caption"] = serde_json::json!(msg.content);
            }
        } else {
            payload["text"] = serde_json::json!(msg.content);
        }
        if let Some(reply_to) = msg
            .metadata
            .get("telegram_reply_to_message_id")
            .and_then(|value| value.as_i64())
        {
            payload["reply_to_message_id"] = serde_json::json!(reply_to);
        }
        if let Some(thread_id) = msg
            .metadata
            .get("telegram_message_thread_id")
            .and_then(|value| value.as_i64())
        {
            payload["message_thread_id"] = serde_json::json!(thread_id);
        }
        if let Some(reply_markup) = reply_markup {
            payload["reply_markup"] = reply_markup;
        }

        let upload = if admin_action.is_none() && reaction.is_none() && is_poll.is_none() {
            media_type
                .zip(media_ref.as_ref())
                .and_then(|(kind, reference)| {
                    let field = match kind {
                        "image" => Some("photo"),
                        "document" => Some("document"),
                        "audio" => Some("audio"),
                        "voice" => Some("voice"),
                        "video" => Some("video"),
                        _ => None,
                    }?;
                    let local_path = Self::media_reference_local_path(reference)?;
                    let filename = reference
                        .get("name")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string);
                    Some((field, local_path, filename))
                })
        } else {
            None
        };

        let response = self.post_telegram_payload(url, payload, upload).await?;

        let status = response.status();
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "telegram".to_string(),
                    message: format!("failed to parse Telegram response: {}", e),
                })?;

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = body
                .get("parameters")
                .and_then(|v| v.get("retry_after"))
                .and_then(|v| v.as_u64());
            return Err(ChannelError::RateLimited {
                platform: "telegram".to_string(),
                retry_after_secs: retry_after,
            }
            .into());
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::SendFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram send failed")
                    .to_string(),
            }
            .into());
        }

        debug!("Telegram message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "telegram".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Telegram Bot API...");

        // Validate token
        if self.config.token.is_empty() {
            return Err(ChannelError::Config {
                platform: "telegram".to_string(),
                message: "Telegram bot token is required".to_string(),
            }
            .into());
        }

        let url = format!("{}/bot{}/getMe", self.api_base_url(), self.config.token);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "telegram".to_string(),
                message: e.to_string(),
            })?;
        let status = response.status();
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "telegram".to_string(),
                    message: format!("failed to parse Telegram auth response: {}", e),
                })?;

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unauthorized")
                    .to_string(),
            }
            .into());
        }

        if !status.is_success() || body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(ChannelError::Connection {
                platform: "telegram".to_string(),
                message: body
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Telegram authentication probe failed")
                    .to_string(),
            }
            .into());
        }

        if let Some(username) = body
            .get("result")
            .and_then(|value| value.get("username"))
            .and_then(|value| value.as_str())
        {
            *self.bot_username.write().await = Some(username.to_string());
        }

        match self.config.mode {
            TelegramMode::Polling => {
                self.spawn_polling_task().await?;
            }
            TelegramMode::Webhook => {
                let webhook_url =
                    self.config
                        .webhook_url
                        .as_deref()
                        .ok_or_else(|| ChannelError::Config {
                            platform: "telegram".to_string(),
                            message: "telegram.webhook_url is required for webhook mode"
                                .to_string(),
                        })?;
                self.set_webhook(webhook_url).await?;
            }
        }

        *self.is_connected.write().await = true;
        info!(mode = ?self.config.mode, "Telegram channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Telegram...");

        *self.is_connected.write().await = false;
        if let Some(task) = self.polling_task.lock().await.take() {
            task.abort();
        }
        if self.config.mode == TelegramMode::Webhook && self.config.webhook_url.is_some() {
            self.delete_webhook().await?;
        }

        info!("Telegram channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "test".to_string(),
            api_base_url: None,
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let channel = TelegramChannel::new(config);
        assert!(channel.is_user_allowed(12345));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "test".to_string(),
            api_base_url: None,
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec!["12345".to_string(), "67890".to_string()],
            rate_limit_per_second: 30,
        };
        let channel = TelegramChannel::new(config);
        assert!(channel.is_user_allowed(12345));
        assert!(channel.is_user_allowed(67890));
        assert!(!channel.is_user_allowed(11111));
    }

    #[tokio::test]
    async fn test_connect_and_send_via_bot_api() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/sendMessage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"message_id": 42}
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "hello".to_string(),
                metadata: serde_json::json!({"telegram_chat_id": "123"}),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_reaction_via_bot_api() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/setMessageReaction"))
            .and(body_partial_json(serde_json::json!({
                "chat_id": "123",
                "message_id": 42,
                "reaction": [{"type": "emoji", "emoji": "👍"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": true
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "".to_string(),
                metadata: serde_json::json!({
                    "telegram_chat_id": "123",
                    "telegram_reaction": "👍",
                    "telegram_reaction_message_id": 42
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_forum_topic_via_bot_api() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/createForumTopic"))
            .and(body_partial_json(serde_json::json!({
                "chat_id": "-100123",
                "name": "Ops Topic",
                "icon_color": 7322096u64
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"message_thread_id": 77, "name": "Ops Topic"}
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "Ops Topic".to_string(),
                metadata: serde_json::json!({
                    "telegram_chat_id": "-100123",
                    "telegram_forum_action": "create_topic",
                    "telegram_topic_icon_color": 7322096u64
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_close_forum_topic_via_bot_api() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/closeForumTopic"))
            .and(body_partial_json(serde_json::json!({
                "chat_id": "-100123",
                "message_thread_id": 77
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": true
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: String::new(),
                metadata: serde_json::json!({
                    "telegram_chat_id": "-100123",
                    "telegram_forum_action": "close_topic",
                    "telegram_message_thread_id": 77
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pin_message_via_bot_api() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/pinChatMessage"))
            .and(body_partial_json(serde_json::json!({
                "chat_id": "-100123",
                "message_id": 42
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": true
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: String::new(),
                metadata: serde_json::json!({
                    "telegram_chat_id": "-100123",
                    "telegram_forum_action": "pin_message",
                    "telegram_pin_message_id": 42
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_polling_receive_enqueues_message() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getUpdates"))
            .and(query_param("timeout", "30"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": [{
                    "update_id": 100,
                    "message": {
                        "message_id": 7,
                        "text": "hello from telegram",
                        "chat": {"id": 12345},
                        "from": {"id": 999, "username": "alice"}
                    }
                }]
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        let incoming = tokio::time::timeout(tokio::time::Duration::from_secs(2), channel.receive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(incoming.content, "hello from telegram");
        assert_eq!(incoming.user_id, "999");
        assert_eq!(incoming.metadata["telegram_chat_id"], "12345");
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_connect_webhook_sets_and_deletes_webhook() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/setWebhook"))
            .and(body_partial_json(serde_json::json!({
                "url": "https://example.com/webhooks/telegram/events"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": true
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/deleteWebhook"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": true
            })))
            .mount(&server)
            .await;

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Webhook,
            webhook_url: Some("https://example.com/webhooks/telegram/events".to_string()),
            webhook_port: Some(8080),
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_send_local_photo_via_multipart_upload() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bottoken/getMe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "username": "test_bot"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/bottoken/sendPhoto"))
            .and(body_string_contains("name=\"photo\""))
            .and(body_string_contains("local-photo.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {"message_id": 42}
            })))
            .mount(&server)
            .await;

        let upload_path = std::env::temp_dir().join("local-photo.txt");
        tokio::fs::write(&upload_path, b"telegram local upload")
            .await
            .unwrap();

        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: Some(server.uri()),
            mode: openrustclaw_core::config::TelegramMode::Polling,
            webhook_url: None,
            webhook_port: None,
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let mut channel = TelegramChannel::new(config);
        channel.connect().await.unwrap();
        channel
            .send(OutgoingMessage {
                session_id: uuid::Uuid::new_v4(),
                content: "caption".to_string(),
                metadata: serde_json::json!({
                    "telegram_chat_id": "123",
                    "telegram_media_type": "image",
                    "file_references": [{
                        "local_path": upload_path.to_string_lossy().to_string(),
                        "name": "local-photo.txt"
                    }]
                }),
            })
            .await
            .unwrap();
        channel.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_webhook_handler_enqueues_message() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            api_base_url: None,
            mode: openrustclaw_core::config::TelegramMode::Webhook,
            webhook_url: Some("https://example.com/webhooks/telegram/events".to_string()),
            webhook_port: Some(8080),
            allowed_users: vec![],
            rate_limit_per_second: 30,
        };
        let channel = TelegramChannel::new(config);
        *channel.bot_username.write().await = Some("test_bot".to_string());
        let handler = channel.webhook_handler();

        handler
            .handle_event(
                br#"{
                    "update_id": 200,
                    "message": {
                        "message_id": 7,
                        "text": "hello @test_bot",
                        "chat": {"id": 12345, "type": "supergroup"},
                        "from": {"id": 999, "username": "alice"}
                    }
                }"#,
            )
            .await
            .unwrap();

        let incoming = channel.receive().await.unwrap();
        assert_eq!(incoming.content, "hello @test_bot");
        assert_eq!(incoming.metadata["telegram_chat_id"], "12345");
        assert_eq!(incoming.metadata["telegram_bot_mentioned"], true);
    }
}
