//! WeChat Channel Integration
//!
//! Supports both WeChat Work (Enterprise) and WeChat Official Accounts APIs.
//!
//! WeChat Work API: https://developer.work.weixin.qq.com/
//! WeChat Official Accounts API: https://developers.weixin.qq.com/

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::WeChatConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// WeChat channel implementation.
pub struct WeChatChannel {
    config: WeChatConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    http: reqwest::Client,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: RwLock<bool>,
    access_token: RwLock<Option<String>>,
    token_expires_at: RwLock<Option<u64>>,
}

/// WeChat API mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeChatMode {
    Work,
    OfficialAccount,
}

/// WeChat Work webhook payload.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeChatWorkWebhook {
    #[serde(rename = "ToUserName")]
    pub to_user_name: String,
    #[serde(rename = "FromUserName")]
    pub from_user_name: String,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "MsgType")]
    pub msg_type: String,
    #[serde(rename = "Content")]
    pub content: Option<String>,
    #[serde(rename = "MsgId")]
    pub msg_id: Option<String>,
    #[serde(rename = "MsgDataId")]
    pub msg_data_id: Option<String>,
    #[serde(rename = "Idx")]
    pub idx: Option<i32>,
    // Event fields
    #[serde(rename = "Event")]
    pub event: Option<String>,
    #[serde(rename = "EventKey")]
    pub event_key: Option<String>,
    // Media fields
    #[serde(rename = "MediaId")]
    pub media_id: Option<String>,
    #[serde(rename = "PicUrl")]
    pub pic_url: Option<String>,
    #[serde(rename = "Format")]
    pub format: Option<String>,
    #[serde(rename = "Recognition")]
    pub recognition: Option<String>,
    // Location fields
    #[serde(rename = "Location_X")]
    pub location_x: Option<f64>,
    #[serde(rename = "Location_Y")]
    pub location_y: Option<f64>,
    #[serde(rename = "Scale")]
    pub scale: Option<i32>,
    #[serde(rename = "Label")]
    pub label: Option<String>,
}

/// WeChat Official Account webhook payload (similar structure).
pub type WeChatOAMessage = WeChatWorkWebhook;

/// WeChat access token response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeChatAccessTokenResponse {
    #[serde(rename = "access_token")]
    pub access_token: Option<String>,
    #[serde(rename = "expires_in")]
    pub expires_in: Option<i64>,
    #[serde(rename = "errcode")]
    pub err_code: Option<i32>,
    #[serde(rename = "errmsg")]
    pub err_msg: Option<String>,
}

/// WeChat API response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeChatApiResponse {
    #[serde(rename = "errcode")]
    pub err_code: i32,
    #[serde(rename = "errmsg")]
    pub err_msg: String,
}

/// WeChat user info.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeChatUserInfo {
    #[serde(rename = "UserID")]
    pub user_id: Option<String>,
    #[serde(rename = "OpenID")]
    pub open_id: Option<String>,
    #[serde(rename = "DeviceID")]
    pub device_id: Option<String>,
    #[serde(rename = "user_ticket")]
    pub user_ticket: Option<String>,
    #[serde(rename = "external_userid")]
    pub external_user_id: Option<String>,
}

/// WeChat Work message payload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeChatWorkMessage {
    #[serde(rename = "touser")]
    pub to_user: String,
    #[serde(rename = "msgtype")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<WeChatTextContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<WeChatMarkdownContent>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeChatTextContent {
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WeChatMarkdownContent {
    pub content: String,
}

impl WeChatChannel {
    /// Create a new WeChat channel with the given configuration.
    pub fn new(config: WeChatConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (WeChat: 20 requests/second)
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(20).unwrap()),
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            http,
            rate_limiter,
            is_connected: RwLock::new(false),
            access_token: RwLock::new(None),
            token_expires_at: RwLock::new(None),
        }
    }

    /// Check if a user is allowed to interact with the bot.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true;
        }
        self.config.allowlist.contains(&user_id.to_string())
    }

    /// Get the current mode (Work or Official Account).
    fn mode(&self) -> WeChatMode {
        if self.config.app_type == "work" {
            WeChatMode::Work
        } else {
            WeChatMode::OfficialAccount
        }
    }

    /// Get or refresh access token.
    async fn get_access_token(&self) -> Result<String> {
        // Check if we have a valid token
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(expires) = *self.token_expires_at.read().await
            && expires > now + 60
        {
            // Refresh 1 minute before expiry
            if let Some(token) = self.access_token.read().await.clone() {
                return Ok(token);
            }
        }

        // Need to fetch new token
        let token = match self.mode() {
            WeChatMode::Work => self.fetch_work_access_token().await?,
            WeChatMode::OfficialAccount => self.fetch_oa_access_token().await?,
        };

        // Store token with expiry
        let expires = now + 7200 - 300; // WeChat tokens valid for 2 hours, refresh 5 min early
        *self.access_token.write().await = Some(token.clone());
        *self.token_expires_at.write().await = Some(expires);

        Ok(token)
    }

    /// Fetch WeChat Work access token.
    async fn fetch_work_access_token(&self) -> Result<String> {
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
            self.config.corp_id, self.config.corp_secret
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "wechat".to_string(),
                message: e.to_string(),
            })?;

        let result: WeChatAccessTokenResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat".to_string(),
                    message: e.to_string(),
                })?;

        if let Some(err_code) = result.err_code
            && err_code != 0
        {
            return Err(ChannelError::AuthFailed {
                platform: "wechat_work".to_string(),
                message: result.err_msg.unwrap_or_default(),
            }
            .into());
        }

        result.access_token.ok_or_else(|| {
            ChannelError::AuthFailed {
                platform: "wechat_work".to_string(),
                message: "No access token in response".to_string(),
            }
            .into()
        })
    }

    /// Fetch WeChat Official Account access token.
    async fn fetch_oa_access_token(&self) -> Result<String> {
        let url = format!(
            "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
            self.config.app_id, self.config.app_secret
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "wechat".to_string(),
                message: e.to_string(),
            })?;

        let result: WeChatAccessTokenResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat".to_string(),
                    message: e.to_string(),
                })?;

        if let Some(err_code) = result.err_code
            && err_code != 0
        {
            return Err(ChannelError::AuthFailed {
                platform: "wechat_oa".to_string(),
                message: result.err_msg.unwrap_or_default(),
            }
            .into());
        }

        result.access_token.ok_or_else(|| {
            ChannelError::AuthFailed {
                platform: "wechat_oa".to_string(),
                message: "No access token in response".to_string(),
            }
            .into()
        })
    }

    /// Verify WeChat webhook signature.
    pub fn verify_signature(
        &self,
        timestamp: &str,
        nonce: &str,
        body: &str,
        signature: &str,
    ) -> Result<()> {
        use sha1::{Digest, Sha1};

        // Sort token, timestamp, nonce and body
        let mut params = [
            self.config.token.clone(),
            timestamp.to_string(),
            nonce.to_string(),
            body.to_string(),
        ];
        params.sort();

        let concatenated = params.join("");
        let mut hasher = Sha1::new();
        hasher.update(concatenated.as_bytes());
        let expected = hex::encode(hasher.finalize());

        if signature != expected {
            return Err(ChannelError::AuthFailed {
                platform: "wechat".to_string(),
                message: "Invalid webhook signature".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Handle incoming WeChat Work webhook message.
    pub async fn handle_work_webhook(&self, msg: WeChatWorkWebhook) -> Result<()> {
        // Check allowlist
        if !self.is_user_allowed(&msg.from_user_name) {
            warn!(user_id = %msg.from_user_name, "User not in allowlist, ignoring message");
            return Ok(());
        }

        let content = match msg.msg_type.as_str() {
            "text" => msg.content.clone().unwrap_or_default(),
            "image" => "[Image]".to_string(),
            "voice" => format!("[Voice: {}]", msg.recognition.clone().unwrap_or_default()),
            "video" => "[Video]".to_string(),
            "file" => "[File]".to_string(),
            "location" => format!("[Location: {}]", msg.label.clone().unwrap_or_default()),
            "link" => "[Link]".to_string(),
            "event" => format!("[Event: {}]", msg.event.clone().unwrap_or_default()),
            _ => "[Unknown message type]".to_string(),
        };

        let session_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "wechat_agent_id": self.config.agent_id,
            "wechat_msg_id": msg.msg_id,
            "wechat_msg_data_id": msg.msg_data_id,
            "wechat_msg_type": msg.msg_type,
            "wechat_event": msg.event,
            "wechat_event_key": msg.event_key,
        });

        let incoming = IncomingMessage {
            session_id,
            user_id: msg.from_user_name,
            content,
            platform: Platform::WeChat,
            metadata,
        };

        if let Err(e) = self.incoming_tx.send(incoming).await {
            error!("Failed to send incoming message: {}", e);
        }

        Ok(())
    }

    /// Send message via WeChat Work API.
    async fn send_work_message(&self, user_id: &str, content: &str, msg_type: &str) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let token = self.get_access_token().await?;
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );

        let message = match msg_type {
            "markdown" => WeChatWorkMessage {
                to_user: user_id.to_string(),
                msg_type: "markdown".to_string(),
                text: None,
                markdown: Some(WeChatMarkdownContent {
                    content: content.to_string(),
                }),
            },
            _ => WeChatWorkMessage {
                to_user: user_id.to_string(),
                msg_type: "text".to_string(),
                text: Some(WeChatTextContent {
                    content: content.to_string(),
                }),
                markdown: None,
            },
        };

        let response = self
            .http
            .post(&url)
            .json(&message)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "wechat_work".to_string(),
                message: e.to_string(),
            })?;

        let result: WeChatApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat_work".to_string(),
                    message: e.to_string(),
                })?;

        if result.err_code != 0 {
            return Err(ChannelError::SendFailed {
                platform: "wechat_work".to_string(),
                message: format!("{} (errcode: {})", result.err_msg, result.err_code),
            }
            .into());
        }

        Ok(())
    }

    /// Send message via WeChat Official Account API.
    async fn send_oa_message(&self, user_id: &str, content: &str) -> Result<()> {
        self.rate_limiter.until_ready().await;

        let token = self.get_access_token().await?;
        let url = format!(
            "https://api.weixin.qq.com/cgi-bin/message/custom/send?access_token={}",
            token
        );

        let payload = serde_json::json!({
            "touser": user_id,
            "msgtype": "text",
            "text": {
                "content": content
            }
        });

        let response = self
            .http
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "wechat_oa".to_string(),
                message: e.to_string(),
            })?;

        let result: WeChatApiResponse =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat_oa".to_string(),
                    message: e.to_string(),
                })?;

        if result.err_code != 0 {
            return Err(ChannelError::SendFailed {
                platform: "wechat_oa".to_string(),
                message: format!("{} (errcode: {})", result.err_msg, result.err_code),
            }
            .into());
        }

        Ok(())
    }

    /// Get WeChat Work user info.
    pub async fn get_work_user_info(&self, code: &str) -> Result<WeChatUserInfo> {
        let token = self.get_access_token().await?;
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/user/getuserinfo?access_token={}&code={}",
            token, code
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "wechat_work".to_string(),
                message: e.to_string(),
            })?;

        let result: WeChatUserInfo =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat_work".to_string(),
                    message: e.to_string(),
                })?;

        Ok(result)
    }

    /// Upload media to WeChat (for sending images, files, etc).
    pub async fn upload_media(
        &self,
        media_type: &str,
        data: Vec<u8>,
        filename: &str,
    ) -> Result<String> {
        self.rate_limiter.until_ready().await;

        let token = self.get_access_token().await?;

        let url = match self.mode() {
            WeChatMode::Work => format!(
                "https://qyapi.weixin.qq.com/cgi-bin/media/upload?access_token={}&type={}",
                token, media_type
            ),
            WeChatMode::OfficialAccount => format!(
                "https://api.weixin.qq.com/cgi-bin/media/upload?access_token={}&type={}",
                token, media_type
            ),
        };

        let part = reqwest::multipart::Part::bytes(data).file_name(filename.to_string());

        let form = reqwest::multipart::Form::new().part("media", part);

        let response = self
            .http
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "wechat".to_string(),
                message: e.to_string(),
            })?;

        let result: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "wechat".to_string(),
                    message: e.to_string(),
                })?;

        if let Some(err_code) = result.get("errcode").and_then(|v| v.as_i64())
            && err_code != 0
        {
            return Err(ChannelError::SendFailed {
                platform: "wechat".to_string(),
                message: result
                    .get("errmsg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
            }
            .into());
        }

        result
            .get("media_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| {
                ChannelError::InvalidFormat {
                    platform: "wechat".to_string(),
                    message: "No media_id in response".to_string(),
                }
                .into()
            })
    }
}

#[async_trait]
impl Channel for WeChatChannel {
    fn platform(&self) -> Platform {
        Platform::WeChat
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        // Extract recipient ID from metadata
        let recipient_id = msg
            .metadata
            .get("wechat_user_id")
            .and_then(|v| v.as_str())
            .or_else(|| msg.metadata.get("recipient_id").and_then(|v| v.as_str()))
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "wechat".to_string(),
                message: "Missing recipient_id in metadata".to_string(),
            })?;

        // Determine message type
        let msg_type = msg
            .metadata
            .get("msg_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        // Send based on mode
        match self.mode() {
            WeChatMode::Work => {
                self.send_work_message(recipient_id, &msg.content, msg_type)
                    .await
            }
            WeChatMode::OfficialAccount => self.send_oa_message(recipient_id, &msg.content).await,
        }
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "wechat".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!(mode = ?self.mode(), "Connecting to WeChat API...");

        // Validate credentials based on mode
        match self.mode() {
            WeChatMode::Work => {
                if self.config.corp_id.is_empty() || self.config.corp_secret.is_empty() {
                    return Err(ChannelError::Config {
                        platform: "wechat_work".to_string(),
                        message: "WeChat Work corp_id and corp_secret are required".to_string(),
                    }
                    .into());
                }
            }
            WeChatMode::OfficialAccount => {
                if self.config.app_id.is_empty() || self.config.app_secret.is_empty() {
                    return Err(ChannelError::Config {
                        platform: "wechat_oa".to_string(),
                        message: "WeChat Official Account app_id and app_secret are required"
                            .to_string(),
                    }
                    .into());
                }
                if self.config.token.is_empty() {
                    return Err(ChannelError::Config {
                        platform: "wechat_oa".to_string(),
                        message: "WeChat Official Account token is required for webhook validation"
                            .to_string(),
                    }
                    .into());
                }
            }
        }

        // Fetch access token to verify credentials
        let token = self.get_access_token().await?;
        info!(token_preview = %format!("{}...", &token[..10.min(token.len())]), "Connected to WeChat API");

        *self.is_connected.write().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from WeChat...");

        // Clear access token
        *self.access_token.write().await = None;
        *self.token_expires_at.write().await = None;
        *self.is_connected.write().await = false;

        info!("WeChat channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_work_config() -> WeChatConfig {
        WeChatConfig {
            enabled: true,
            app_type: "work".to_string(),
            corp_id: "test_corp".to_string(),
            corp_secret: "test_secret".to_string(),
            agent_id: "1000002".to_string(),
            app_id: String::new(),
            app_secret: String::new(),
            token: "test_token".to_string(),
            encoding_aes_key: None,
            webhook_path: "/webhook/wechat".to_string(),
            allowlist: vec![],
            rate_limit_per_second: 20,
            enable_encryption: false,
        }
    }

    fn create_oa_config() -> WeChatConfig {
        WeChatConfig {
            enabled: true,
            app_type: "official_account".to_string(),
            corp_id: String::new(),
            corp_secret: String::new(),
            agent_id: String::new(),
            app_id: "test_appid".to_string(),
            app_secret: "test_secret".to_string(),
            token: "test_token".to_string(),
            encoding_aes_key: None,
            webhook_path: "/webhook/wechat".to_string(),
            allowlist: vec![],
            rate_limit_per_second: 20,
            enable_encryption: false,
        }
    }

    #[test]
    fn test_user_allowed_empty_list() {
        let config = create_work_config();
        let channel = WeChatChannel::new(config);
        assert!(channel.is_user_allowed("ZhangSan"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let mut config = create_work_config();
        config.allowlist = vec!["ZhangSan".to_string(), "LiSi".to_string()];
        let channel = WeChatChannel::new(config);
        assert!(channel.is_user_allowed("ZhangSan"));
        assert!(channel.is_user_allowed("LiSi"));
        assert!(!channel.is_user_allowed("WangWu"));
    }

    #[test]
    fn test_work_mode() {
        let config = create_work_config();
        let channel = WeChatChannel::new(config);
        assert_eq!(channel.mode(), WeChatMode::Work);
    }

    #[test]
    fn test_oa_mode() {
        let config = create_oa_config();
        let channel = WeChatChannel::new(config);
        assert_eq!(channel.mode(), WeChatMode::OfficialAccount);
    }
}
