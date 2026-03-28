//! Gmail Pub/Sub integration for email-driven workflows.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::info;
use uuid::Uuid;

use openrustclaw_core::config::GmailPubSubConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

const DEFAULT_GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";
const DEFAULT_GOOGLE_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GMAIL_SCOPES: &str =
    "https://www.googleapis.com/auth/gmail.modify https://www.googleapis.com/auth/gmail.send";

#[derive(Clone)]
struct GmailRuntime {
    config: GmailPubSubConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    rate_limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
            governor::middleware::NoOpMiddleware,
        >,
    >,
    is_connected: Arc<RwLock<bool>>,
    access_token: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
}

pub struct GmailPubSub {
    runtime: GmailRuntime,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GmailNotification {
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "historyId")]
    pub history_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailProcessedMessageReport {
    pub message_id: String,
    pub thread_id: String,
    pub from: String,
    pub subject: String,
    pub received_at: String,
    pub attachment_count: usize,
    pub unread: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailNotificationReport {
    pub email_address: String,
    pub history_id: u64,
    pub processed_count: usize,
    pub skipped_count: usize,
    pub entries: Vec<GmailProcessedMessageReport>,
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub id: String,
    pub thread_id: String,
    pub history_id: u64,
    pub from: String,
    pub from_domain: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub message_id_header: Option<String>,
    pub references_header: Option<String>,
    pub in_reply_to_header: Option<String>,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,
    pub labels: Vec<String>,
    pub received_at: DateTime<Utc>,
    pub is_unread: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub mime_type: String,
    pub size: usize,
    pub attachment_id: String,
}

#[derive(Debug, Clone)]
struct OutgoingAttachment {
    filename: String,
    mime_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum EmailAction {
    Reply {
        body: String,
    },
    Label {
        add: Vec<String>,
        remove: Vec<String>,
    },
    Archive,
    Delete,
    Forward {
        to: String,
        body: String,
    },
    TriggerWorkflow {
        workflow_name: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_google_token_uri")]
    token_uri: String,
}

fn default_google_token_uri() -> String {
    DEFAULT_GOOGLE_OAUTH_TOKEN_URL.to_string()
}

#[derive(Debug, Serialize)]
struct ServiceAccountClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
    sub: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WatchResponse {
    #[serde(rename = "historyId")]
    history_id: String,
    expiration: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
struct MessageMetadata {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct MessagePart {
    #[serde(rename = "mimeType", default)]
    mime_type: String,
    #[serde(default)]
    body: MessagePartBody,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    parts: Vec<MessagePart>,
    #[serde(default)]
    headers: Vec<MessageHeader>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct MessagePartBody {
    data: Option<String>,
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
    #[serde(default)]
    size: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageHeader {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GmailMessage {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
    #[serde(rename = "labelIds", default)]
    label_ids: Vec<String>,
    #[serde(rename = "internalDate", default)]
    internal_date: String,
    payload: MessagePart,
    #[serde(rename = "historyId", default)]
    history_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponse {
    #[serde(default)]
    history: Vec<HistoryRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryRecord {
    #[serde(rename = "messagesAdded", default)]
    messages_added: Vec<MessageAdded>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageAdded {
    message: MessageMetadata,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Clone, Default)]
struct ParsedParts {
    text: String,
    html: Option<String>,
    attachments: Vec<Attachment>,
}

impl GmailPubSub {
    pub fn new(config: GmailPubSubConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).expect("non-zero")),
        );

        let runtime = GmailRuntime {
            config,
            incoming_tx,
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
            is_connected: Arc::new(RwLock::new(false)),
            access_token: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        };

        Self {
            runtime,
            incoming_rx: Mutex::new(incoming_rx),
        }
    }

    pub fn webhook_handler(&self) -> GmailWebhookHandler {
        GmailWebhookHandler {
            runtime: self.runtime.clone(),
        }
    }

    pub async fn process_notification(
        &self,
        notification: GmailNotification,
    ) -> Result<GmailNotificationReport> {
        self.runtime.process_notification(notification).await
    }

    pub async fn take_action(&self, email_id: &str, action: EmailAction) -> Result<()> {
        self.runtime.take_action(email_id, action).await
    }
}

impl GmailRuntime {
    fn sanitize_header_value(value: &str) -> String {
        value.replace('"', "'").replace(['\r', '\n'], " ")
    }

    async fn outgoing_attachments_from_metadata(
        metadata: &serde_json::Value,
    ) -> Result<Vec<OutgoingAttachment>> {
        let mut attachments = Vec::new();
        let Some(entries) = metadata
            .get("file_references")
            .and_then(|value| value.as_array())
        else {
            return Ok(attachments);
        };

        for entry in entries {
            let Some(local_path) = entry.get("local_path").and_then(|value| value.as_str()) else {
                continue;
            };

            let bytes =
                tokio::fs::read(local_path)
                    .await
                    .map_err(|e| ChannelError::SendFailed {
                        platform: "gmail".to_string(),
                        message: format!("Failed to read Gmail attachment '{}': {}", local_path, e),
                    })?;

            let filename = entry
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| {
                    Path::new(local_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.to_string())
                })
                .unwrap_or_else(|| "attachment".to_string());

            let mime_type = entry
                .get("mime")
                .or_else(|| entry.get("content_type"))
                .and_then(|value| value.as_str())
                .unwrap_or("application/octet-stream")
                .to_string();

            attachments.push(OutgoingAttachment {
                filename,
                mime_type,
                bytes,
            });
        }

        Ok(attachments)
    }

    fn build_reply_mime(
        reply_to: &str,
        subject: &str,
        message_id_header: Option<&str>,
        references: Option<&str>,
        body: &str,
        attachments: &[OutgoingAttachment],
    ) -> String {
        let mut mime = format!(
            "To: {}\r\nSubject: {}\r\n",
            reply_to,
            Self::sanitize_header_value(subject)
        );
        if let Some(message_id) = message_id_header {
            mime.push_str(&format!("In-Reply-To: {}\r\n", message_id));
            let references = references
                .map(|existing| format!("{} {}", existing, message_id))
                .unwrap_or_else(|| message_id.to_string());
            mime.push_str(&format!("References: {}\r\n", references));
        }

        if attachments.is_empty() {
            mime.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n\r\n");
            mime.push_str(body);
            return mime;
        }

        let boundary = "openrustclaw-gmail-boundary";
        mime.push_str("MIME-Version: 1.0\r\n");
        mime.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
            boundary
        ));
        mime.push_str(&format!(
            "--{}\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\n\r\n{}\r\n",
            boundary, body
        ));

        for attachment in attachments {
            mime.push_str(&format!(
                "--{}\r\nContent-Type: {}; name=\"{}\"\r\nContent-Disposition: attachment; filename=\"{}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
                boundary,
                attachment.mime_type,
                Self::sanitize_header_value(&attachment.filename),
                Self::sanitize_header_value(&attachment.filename),
                STANDARD.encode(&attachment.bytes)
            ));
        }

        mime.push_str(&format!("--{}--", boundary));
        mime
    }

    fn build_new_message_mime(
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body: &str,
        attachments: &[OutgoingAttachment],
    ) -> String {
        let mut mime = format!(
            "To: {}\r\nSubject: {}\r\n",
            to.join(", "),
            Self::sanitize_header_value(subject)
        );
        if !cc.is_empty() {
            mime.push_str(&format!("Cc: {}\r\n", cc.join(", ")));
        }
        if !bcc.is_empty() {
            mime.push_str(&format!("Bcc: {}\r\n", bcc.join(", ")));
        }

        if attachments.is_empty() {
            mime.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n\r\n");
            mime.push_str(body);
            return mime;
        }

        let boundary = "openrustclaw-gmail-boundary";
        mime.push_str("MIME-Version: 1.0\r\n");
        mime.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
            boundary
        ));
        mime.push_str(&format!(
            "--{}\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\n\r\n{}\r\n",
            boundary, body
        ));

        for attachment in attachments {
            mime.push_str(&format!(
                "--{}\r\nContent-Type: {}; name=\"{}\"\r\nContent-Disposition: attachment; filename=\"{}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
                boundary,
                attachment.mime_type,
                Self::sanitize_header_value(&attachment.filename),
                Self::sanitize_header_value(&attachment.filename),
                STANDARD.encode(&attachment.bytes)
            ));
        }

        mime.push_str(&format!("--{}--", boundary));
        mime
    }

    fn metadata_address_list(metadata: &serde_json::Value, key: &str) -> Vec<String> {
        if let Some(values) = metadata.get(key).and_then(|value| value.as_array()) {
            return values
                .iter()
                .filter_map(|value| value.as_str())
                .map(|value| value.to_string())
                .collect();
        }

        metadata
            .get(key)
            .and_then(|value| value.as_str())
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim())
                    .filter(|item| !item.is_empty())
                    .map(|item| item.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn metadata_string(metadata: &serde_json::Value, key: &str) -> Option<String> {
        metadata
            .get(key)
            .and_then(|value| value.as_str())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    fn attachment_file_references(email: &EmailMessage) -> Vec<serde_json::Value> {
        email
            .attachments
            .iter()
            .map(|attachment| {
                serde_json::json!({
                    "url": format!(
                        "gmail-attachment://{}/{}",
                        email.id,
                        attachment.attachment_id
                    ),
                    "name": attachment.filename,
                    "mime": attachment.mime_type,
                    "size": attachment.size,
                    "attachment_id": attachment.attachment_id,
                    "message_id": email.id,
                    "thread_id": email.thread_id,
                })
            })
            .collect()
    }

    fn gmail_api_base(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_GMAIL_API_BASE.to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn token_url(&self, key: &ServiceAccountKey) -> String {
        self.config
            .oauth_token_url
            .clone()
            .unwrap_or_else(|| key.token_uri.clone())
    }

    fn topic_name(&self) -> String {
        if let Some(topic) = self.config.topic_name.clone() {
            return topic;
        }
        if self.config.subscription_name.contains("/topics/") {
            return self.config.subscription_name.clone();
        }
        if self.config.subscription_name.contains("/subscriptions/") {
            return self
                .config
                .subscription_name
                .replace("/subscriptions/", "/topics/");
        }
        format!(
            "projects/{}/topics/{}",
            self.config.project_id, self.config.subscription_name
        )
    }

    fn user_path(&self) -> String {
        urlencoding::encode(&self.config.user_email).to_string()
    }

    fn parse_addresses(s: &str) -> Vec<String> {
        s.split(',')
            .map(|address| address.trim().to_string())
            .filter(|address| !address.is_empty())
            .collect()
    }

    fn address_domains(addresses: &[String]) -> Vec<String> {
        addresses
            .iter()
            .filter_map(|address| address.split('@').nth(1))
            .map(|domain| domain.trim_end_matches('>').trim().to_string())
            .filter(|domain| !domain.is_empty())
            .collect()
    }

    fn should_process(&self, email: &EmailMessage) -> bool {
        if !self.config.label_filters.is_empty() {
            let has_matching = self
                .config
                .label_filters
                .iter()
                .any(|filter| email.labels.iter().any(|label| label.contains(filter)));
            if !has_matching {
                return false;
            }
        }
        if let Some(query) = &self.config.query_filter {
            let search_text = format!("{} {} {}", email.from, email.subject, email.body_text);
            if !search_text.to_lowercase().contains(&query.to_lowercase()) {
                return false;
            }
        }
        true
    }

    fn decode_base64(data: &str) -> Option<Vec<u8>> {
        let normalized = data.replace('-', "+").replace('_', "/");
        STANDARD.decode(normalized).ok()
    }

    fn parse_headers(headers: &[MessageHeader]) -> HashMap<String, String> {
        headers
            .iter()
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect()
    }

    fn get_parts(payload: &MessagePart) -> ParsedParts {
        let mut result = ParsedParts::default();

        match payload.mime_type.as_str() {
            "text/plain" => {
                if let Some(data) = &payload.body.data {
                    result.text = Self::decode_base64(data)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .unwrap_or_default();
                }
            }
            "text/html" => {
                if let Some(data) = &payload.body.data {
                    result.html =
                        Self::decode_base64(data).and_then(|bytes| String::from_utf8(bytes).ok());
                }
            }
            mime if mime.starts_with("multipart/") => {
                for part in &payload.parts {
                    let sub = Self::get_parts(part);
                    if result.text.is_empty() && !sub.text.is_empty() {
                        result.text = sub.text;
                    }
                    if result.html.is_none() && sub.html.is_some() {
                        result.html = sub.html;
                    }
                    result.attachments.extend(sub.attachments);
                }
            }
            _ => {}
        }

        if let Some(attachment_id) = &payload.body.attachment_id
            && !payload.filename.is_empty()
        {
            result.attachments.push(Attachment {
                filename: payload.filename.clone(),
                mime_type: payload.mime_type.clone(),
                size: payload.body.size.max(0) as usize,
                attachment_id: attachment_id.clone(),
            });
        }

        result
    }

    fn parse_message(&self, msg: GmailMessage) -> EmailMessage {
        let headers = Self::parse_headers(&msg.payload.headers);
        let parts = Self::get_parts(&msg.payload);
        let internal_date = msg
            .internal_date
            .parse::<i64>()
            .unwrap_or_else(|_| Utc::now().timestamp_millis());

        EmailMessage {
            id: msg.id,
            thread_id: msg.thread_id,
            history_id: msg.history_id.parse().unwrap_or(0),
            from: headers.get("From").cloned().unwrap_or_default(),
            from_domain: headers
                .get("From")
                .and_then(|value| value.split('@').nth(1))
                .map(|value| value.trim_end_matches('>').trim().to_string()),
            to: headers
                .get("To")
                .map(|value| Self::parse_addresses(value))
                .unwrap_or_default(),
            cc: headers
                .get("Cc")
                .map(|value| Self::parse_addresses(value))
                .unwrap_or_default(),
            subject: headers.get("Subject").cloned().unwrap_or_default(),
            message_id_header: headers.get("Message-ID").cloned(),
            references_header: headers.get("References").cloned(),
            in_reply_to_header: headers.get("In-Reply-To").cloned(),
            body_text: parts.text,
            body_html: parts.html,
            attachments: parts.attachments,
            labels: msg.label_ids.clone(),
            received_at: DateTime::from_timestamp_millis(internal_date).unwrap_or_else(Utc::now),
            is_unread: msg.label_ids.iter().any(|label| label == "UNREAD"),
        }
    }

    async fn authenticate(&self) -> Result<String> {
        if let Some(token) = self.config.service_account_key_path.strip_prefix("token:") {
            let token = token.trim();
            if !token.is_empty() {
                return Ok(token.to_string());
            }
        }

        if let Some(var_name) = self.config.service_account_key_path.strip_prefix("env:") {
            return std::env::var(var_name.trim()).map_err(|_| {
                ChannelError::AuthFailed {
                    platform: "gmail".to_string(),
                    message: format!(
                        "Gmail access token env var '{}' is not set",
                        var_name.trim()
                    ),
                }
                .into()
            });
        }

        let key_data = tokio::fs::read_to_string(&self.config.service_account_key_path)
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Failed to read Gmail service account key: {}", e),
            })?;

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&key_data)
            && let Some(token) = value.get("access_token").and_then(|field| field.as_str())
        {
            return Ok(token.to_string());
        }

        let key: ServiceAccountKey =
            serde_json::from_str(&key_data).map_err(|e| ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Invalid Gmail service account JSON: {}", e),
            })?;

        let issued_at = Utc::now().timestamp();
        let claims = ServiceAccountClaims {
            iss: key.client_email.clone(),
            scope: GMAIL_SCOPES.to_string(),
            aud: self.token_url(&key),
            exp: issued_at + 3600,
            iat: issued_at,
            sub: self.config.user_email.clone(),
        };

        let private_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes()).map_err(|e| {
            ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Invalid Gmail service account private key: {}", e),
            }
        })?;

        let assertion =
            encode(&Header::new(Algorithm::RS256), &claims, &private_key).map_err(|e| {
                ChannelError::AuthFailed {
                    platform: "gmail".to_string(),
                    message: format!("Failed to sign Gmail service account JWT: {}", e),
                }
            })?;

        let response = self
            .http_client
            .post(self.token_url(&key))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Failed to exchange Gmail service account JWT: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail OAuth exchange failed: {}", body),
            }
            .into());
        }

        let body: TokenResponse = response
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Gmail OAuth response: {}", e),
            })?;

        Ok(body.access_token)
    }

    async fn require_access_token(&self) -> Result<String> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "gmail".to_string(),
            }
            .into());
        }
        self.access_token.read().await.clone().ok_or_else(|| {
            ChannelError::AuthFailed {
                platform: "gmail".to_string(),
                message: "Missing access token for Gmail API".to_string(),
            }
            .into()
        })
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        token: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let response = self
            .http_client
            .get(format!(
                "{}/{}",
                self.gmail_api_base(),
                path.trim_start_matches('/')
            ))
            .bearer_auth(token)
            .query(query)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Gmail GET request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Gmail GET request failed: {}", body),
            }
            .into());
        }

        response.json().await.map_err(|e| {
            ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Gmail GET response: {}", e),
            }
            .into()
        })
    }

    async fn setup_watch(&self, token: &str) -> Result<WatchResponse> {
        let response = self
            .http_client
            .post(format!(
                "{}/users/{}/watch",
                self.gmail_api_base(),
                self.user_path()
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "topicName": self.topic_name(),
                "labelIds": self.config.label_filters,
                "labelFilterBehavior": "INCLUDE",
            }))
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to set up Gmail watch: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to set up Gmail watch: {}", body),
            }
            .into());
        }

        response.json().await.map_err(|e| {
            ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Gmail watch response: {}", e),
            }
            .into()
        })
    }

    async fn stop_watch(&self, token: &str) -> Result<()> {
        let response = self
            .http_client
            .post(format!(
                "{}/users/{}/stop",
                self.gmail_api_base(),
                self.user_path()
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to stop Gmail watch: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "gmail".to_string(),
                message: format!("Failed to stop Gmail watch: {}", body),
            }
            .into());
        }

        Ok(())
    }

    async fn get_new_messages(&self, history_id: u64, token: &str) -> Result<Vec<MessageMetadata>> {
        let response: HistoryResponse = self
            .get_json(
                &format!("users/{}/history", self.user_path()),
                token,
                &[
                    ("startHistoryId", history_id.to_string()),
                    ("historyTypes", "messageAdded".to_string()),
                    ("maxResults", self.config.max_history_fetch.to_string()),
                ],
            )
            .await?;

        let mut messages = Vec::new();
        let mut seen = HashSet::new();
        for record in response.history {
            for added in record.messages_added {
                if seen.insert(added.message.id.clone()) {
                    messages.push(added.message);
                }
            }
        }
        Ok(messages)
    }

    async fn get_gmail_message(&self, id: &str, token: &str) -> Result<GmailMessage> {
        self.get_json(
            &format!(
                "users/{}/messages/{}",
                self.user_path(),
                urlencoding::encode(id)
            ),
            token,
            &[("format", "full".to_string())],
        )
        .await
    }

    async fn get_message(&self, id: &str, token: &str) -> Result<Option<EmailMessage>> {
        Ok(Some(
            self.parse_message(self.get_gmail_message(id, token).await?),
        ))
    }

    async fn process_notification(
        &self,
        notification: GmailNotification,
    ) -> Result<GmailNotificationReport> {
        info!(
            email = %notification.email_address,
            history_id = notification.history_id,
            "Processing Gmail notification"
        );

        let mut report = GmailNotificationReport {
            email_address: notification.email_address.clone(),
            history_id: notification.history_id,
            processed_count: 0,
            skipped_count: 0,
            entries: Vec::new(),
        };
        let token = self.require_access_token().await?;
        let new_messages = self
            .get_new_messages(notification.history_id, &token)
            .await?;

        for metadata in new_messages {
            if let Some(email) = self.get_message(&metadata.id, &token).await? {
                if !self.should_process(&email) {
                    report.skipped_count += 1;
                    continue;
                }
                report.entries.push(GmailProcessedMessageReport {
                    message_id: email.id.clone(),
                    thread_id: email.thread_id.clone(),
                    from: email.from.clone(),
                    subject: email.subject.clone(),
                    received_at: email.received_at.to_rfc3339(),
                    attachment_count: email.attachments.len(),
                    unread: email.is_unread,
                });
                let incoming = IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: email.from.clone(),
                    content: format!("Subject: {}\n\n{}", email.subject, email.body_text),
                    platform: Platform::Gmail,
                    metadata: {
                        let file_references = Self::attachment_file_references(&email);
                        let to_domains = Self::address_domains(&email.to);
                        let cc_domains = Self::address_domains(&email.cc);
                        let attachment_names: Vec<_> = email
                            .attachments
                            .iter()
                            .map(|attachment| attachment.filename.clone())
                            .collect();
                        let attachment_mime_types: Vec<_> = email
                            .attachments
                            .iter()
                            .map(|attachment| attachment.mime_type.clone())
                            .collect();
                        let attachment_total_size: usize = email
                            .attachments
                            .iter()
                            .map(|attachment| attachment.size)
                            .sum();

                        let mut metadata = serde_json::Map::new();
                        metadata.insert("gmail_message_id".into(), serde_json::json!(email.id));
                        metadata.insert(
                            "gmail_has_message_id".into(),
                            serde_json::json!(!email.id.is_empty()),
                        );
                        metadata
                            .insert("gmail_thread_id".into(), serde_json::json!(email.thread_id));
                        metadata.insert(
                            "gmail_has_thread_id".into(),
                            serde_json::json!(!email.thread_id.is_empty()),
                        );
                        metadata.insert(
                            "gmail_history_id".into(),
                            serde_json::json!(email.history_id),
                        );
                        metadata.insert(
                            "gmail_has_history_id".into(),
                            serde_json::json!(email.history_id > 0),
                        );
                        metadata.insert("gmail_subject".into(), serde_json::json!(email.subject));
                        metadata.insert(
                            "gmail_has_subject".into(),
                            serde_json::json!(!email.subject.is_empty()),
                        );
                        metadata.insert("gmail_from".into(), serde_json::json!(email.from));
                        metadata.insert(
                            "gmail_from_length".into(),
                            serde_json::json!(email.from.chars().count()),
                        );
                        metadata.insert(
                            "gmail_from_domain".into(),
                            serde_json::json!(email.from_domain),
                        );
                        metadata.insert(
                            "gmail_from_domain_length".into(),
                            serde_json::json!(
                                email
                                    .from_domain
                                    .as_ref()
                                    .map(|value| value.chars().count())
                            ),
                        );
                        metadata.insert(
                            "gmail_has_from_domain".into(),
                            serde_json::json!(email.from_domain.is_some()),
                        );
                        metadata.insert("gmail_to".into(), serde_json::json!(email.to));
                        metadata.insert(
                            "gmail_has_to".into(),
                            serde_json::json!(!email.to.is_empty()),
                        );
                        metadata.insert("gmail_to_count".into(), serde_json::json!(email.to.len()));
                        metadata.insert("gmail_to_domains".into(), serde_json::json!(to_domains));
                        metadata.insert(
                            "gmail_to_domain_count".into(),
                            serde_json::json!(
                                metadata["gmail_to_domains"]
                                    .as_array()
                                    .map(|items| items.len())
                                    .unwrap_or(0)
                            ),
                        );
                        metadata.insert(
                            "gmail_has_to_domains".into(),
                            serde_json::json!(
                                metadata["gmail_to_domains"]
                                    .as_array()
                                    .map(|items| !items.is_empty())
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert("gmail_cc".into(), serde_json::json!(email.cc));
                        metadata.insert("gmail_cc_count".into(), serde_json::json!(email.cc.len()));
                        metadata.insert(
                            "gmail_has_cc".into(),
                            serde_json::json!(!email.cc.is_empty()),
                        );
                        metadata.insert("gmail_cc_domains".into(), serde_json::json!(cc_domains));
                        metadata.insert(
                            "gmail_cc_domain_count".into(),
                            serde_json::json!(
                                metadata["gmail_cc_domains"]
                                    .as_array()
                                    .map(|items| items.len())
                                    .unwrap_or(0)
                            ),
                        );
                        metadata.insert(
                            "gmail_has_cc_domains".into(),
                            serde_json::json!(
                                metadata["gmail_cc_domains"]
                                    .as_array()
                                    .map(|items| !items.is_empty())
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert("gmail_labels".into(), serde_json::json!(email.labels));
                        metadata.insert(
                            "gmail_has_labels".into(),
                            serde_json::json!(!email.labels.is_empty()),
                        );
                        metadata.insert(
                            "gmail_label_count".into(),
                            serde_json::json!(email.labels.len()),
                        );
                        metadata.insert(
                            "gmail_has_attachments".into(),
                            serde_json::json!(!email.attachments.is_empty()),
                        );
                        metadata.insert(
                            "gmail_attachment_count".into(),
                            serde_json::json!(email.attachments.len()),
                        );
                        metadata.insert(
                            "gmail_attachment_names".into(),
                            serde_json::json!(attachment_names),
                        );
                        metadata.insert(
                            "gmail_attachment_name_count".into(),
                            serde_json::json!(
                                metadata["gmail_attachment_names"]
                                    .as_array()
                                    .map(|items| items.len())
                                    .unwrap_or(0)
                            ),
                        );
                        metadata.insert(
                            "gmail_has_attachment_names".into(),
                            serde_json::json!(
                                metadata["gmail_attachment_names"]
                                    .as_array()
                                    .map(|items| items.iter().any(|item| item
                                        .as_str()
                                        .map(|s| !s.is_empty())
                                        .unwrap_or(false)))
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert(
                            "gmail_attachment_mime_types".into(),
                            serde_json::json!(attachment_mime_types),
                        );
                        metadata.insert(
                            "gmail_attachment_mime_type_count".into(),
                            serde_json::json!(
                                metadata["gmail_attachment_mime_types"]
                                    .as_array()
                                    .map(|items| items.len())
                                    .unwrap_or(0)
                            ),
                        );
                        metadata.insert(
                            "gmail_has_attachment_mime_types".into(),
                            serde_json::json!(
                                metadata["gmail_attachment_mime_types"]
                                    .as_array()
                                    .map(|items| items.iter().any(|item| item
                                        .as_str()
                                        .map(|s| !s.is_empty())
                                        .unwrap_or(false)))
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert(
                            "gmail_attachment_total_size".into(),
                            serde_json::json!(attachment_total_size),
                        );
                        metadata.insert(
                            "gmail_has_attachment_total_size".into(),
                            serde_json::json!(attachment_total_size > 0),
                        );
                        metadata.insert(
                            "gmail_attachments".into(),
                            serde_json::json!(email.attachments),
                        );
                        metadata
                            .insert("file_references".into(), serde_json::json!(file_references));
                        metadata.insert(
                            "gmail_file_reference_count".into(),
                            serde_json::json!(
                                metadata["file_references"]
                                    .as_array()
                                    .map(|items| items.len())
                                    .unwrap_or(0)
                            ),
                        );
                        metadata.insert(
                            "gmail_has_file_references".into(),
                            serde_json::json!(
                                metadata["gmail_file_reference_count"]
                                    .as_u64()
                                    .map(|count| count > 0)
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert(
                            "gmail_subject_length".into(),
                            serde_json::json!(email.subject.chars().count()),
                        );
                        metadata.insert(
                            "gmail_has_subject_length".into(),
                            serde_json::json!(!email.subject.is_empty()),
                        );
                        metadata.insert(
                            "gmail_has_html_body".into(),
                            serde_json::json!(email.body_html.is_some()),
                        );
                        metadata.insert(
                            "gmail_has_body_text".into(),
                            serde_json::json!(!email.body_text.is_empty()),
                        );
                        metadata.insert(
                            "gmail_body_text_length".into(),
                            serde_json::json!(email.body_text.chars().count()),
                        );
                        metadata.insert(
                            "gmail_body_html_length".into(),
                            serde_json::json!(
                                email.body_html.as_ref().map(|html| html.chars().count())
                            ),
                        );
                        metadata.insert(
                            "gmail_has_body_html_length".into(),
                            serde_json::json!(
                                email
                                    .body_html
                                    .as_ref()
                                    .map(|html| !html.is_empty())
                                    .unwrap_or(false)
                            ),
                        );
                        metadata.insert(
                            "gmail_received_at".into(),
                            serde_json::json!(email.received_at.to_rfc3339()),
                        );
                        metadata.insert("gmail_has_received_at".into(), serde_json::json!(true));
                        metadata
                            .insert("gmail_is_unread".into(), serde_json::json!(email.is_unread));
                        metadata.insert(
                            "gmail_message_id_header".into(),
                            serde_json::json!(email.message_id_header),
                        );
                        metadata.insert(
                            "gmail_has_message_id_header".into(),
                            serde_json::json!(email.message_id_header.is_some()),
                        );
                        metadata.insert(
                            "gmail_references".into(),
                            serde_json::json!(email.references_header),
                        );
                        metadata.insert(
                            "gmail_has_references".into(),
                            serde_json::json!(email.references_header.is_some()),
                        );
                        metadata.insert(
                            "gmail_in_reply_to".into(),
                            serde_json::json!(email.in_reply_to_header),
                        );
                        metadata.insert(
                            "gmail_has_in_reply_to".into(),
                            serde_json::json!(email.in_reply_to_header.is_some()),
                        );
                        serde_json::Value::Object(metadata)
                    },
                };

                self.incoming_tx
                    .send(incoming)
                    .await
                    .map_err(|e| ChannelError::Connection {
                        platform: "gmail".to_string(),
                        message: format!("Failed to enqueue Gmail event: {}", e),
                    })?;
            }
        }

        report.processed_count = report.entries.len();
        Ok(report)
    }

    async fn take_action(&self, email_id: &str, action: EmailAction) -> Result<()> {
        let token = self.require_access_token().await?;
        match action {
            EmailAction::Reply { body } => {
                self.reply_to_message(email_id, &body, &[], &token).await
            }
            EmailAction::Label { add, remove } => {
                self.modify_labels(email_id, add, remove, &token).await
            }
            EmailAction::Archive => {
                self.modify_labels(email_id, Vec::new(), vec!["INBOX".to_string()], &token)
                    .await
            }
            EmailAction::Delete => self.delete_message(email_id, &token).await,
            EmailAction::Forward { to, body } => {
                self.forward_message(email_id, &to, &body, &[], &token)
                    .await
            }
            EmailAction::TriggerWorkflow { workflow_name } => {
                info!(workflow = %workflow_name, email_id = %email_id, "Gmail workflow trigger requested");
                Ok(())
            }
        }
    }

    async fn modify_labels(
        &self,
        email_id: &str,
        add: Vec<String>,
        remove: Vec<String>,
        token: &str,
    ) -> Result<()> {
        let response = self
            .http_client
            .post(format!(
                "{}/users/{}/messages/{}/modify",
                self.gmail_api_base(),
                self.user_path(),
                urlencoding::encode(email_id)
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "addLabelIds": add,
                "removeLabelIds": remove,
            }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail modify request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail modify request failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    async fn delete_message(&self, email_id: &str, token: &str) -> Result<()> {
        let response = self
            .http_client
            .delete(format!(
                "{}/users/{}/messages/{}",
                self.gmail_api_base(),
                self.user_path(),
                urlencoding::encode(email_id)
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail delete request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail delete request failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    async fn reply_to_message(
        &self,
        email_id: &str,
        body: &str,
        attachments: &[OutgoingAttachment],
        token: &str,
    ) -> Result<()> {
        let message = self.get_gmail_message(email_id, token).await?;
        let headers = Self::parse_headers(&message.payload.headers);
        let reply_to = headers
            .get("Reply-To")
            .or_else(|| headers.get("From"))
            .cloned()
            .unwrap_or_default();
        let subject = headers
            .get("Subject")
            .map(|value| {
                if value.to_lowercase().starts_with("re:") {
                    value.clone()
                } else {
                    format!("Re: {}", value)
                }
            })
            .unwrap_or_else(|| "Re:".to_string());
        let message_id_header = headers.get("Message-ID").cloned();
        let references = headers.get("References").cloned();

        let mime = Self::build_reply_mime(
            &reply_to,
            &subject,
            message_id_header.as_deref(),
            references.as_deref(),
            body,
            attachments,
        );

        let response = self
            .http_client
            .post(format!(
                "{}/users/{}/messages/send",
                self.gmail_api_base(),
                self.user_path()
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "raw": URL_SAFE_NO_PAD.encode(mime.as_bytes()),
                "threadId": message.thread_id,
            }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail reply request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail reply request failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    async fn forward_message(
        &self,
        email_id: &str,
        to: &str,
        body: &str,
        attachments: &[OutgoingAttachment],
        token: &str,
    ) -> Result<()> {
        let message = self.parse_message(self.get_gmail_message(email_id, token).await?);
        let subject = if message.subject.to_lowercase().starts_with("fwd:") {
            message.subject.clone()
        } else {
            format!("Fwd: {}", message.subject)
        };
        let forward_body = format!(
            "{}\n\n--- Forwarded message ---\nFrom: {}\nSubject: {}\n\n{}",
            body, message.from, message.subject, message.body_text
        );

        self.send_new_message(
            &[to.to_string()],
            &[],
            &[],
            &subject,
            &forward_body,
            attachments,
            None,
            token,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_new_message(
        &self,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body: &str,
        attachments: &[OutgoingAttachment],
        thread_id: Option<&str>,
        token: &str,
    ) -> Result<()> {
        let mime = Self::build_new_message_mime(to, cc, bcc, subject, body, attachments);

        let mut payload = serde_json::json!({
            "raw": URL_SAFE_NO_PAD.encode(mime.as_bytes()),
        });
        if let Some(thread_id) = thread_id.filter(|value| !value.is_empty()) {
            payload["threadId"] = serde_json::json!(thread_id);
        }

        let response = self
            .http_client
            .post(format!(
                "{}/users/{}/messages/send",
                self.gmail_api_base(),
                self.user_path()
            ))
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail send request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "gmail".to_string(),
                message: format!("Gmail send request failed: {}", body),
            }
            .into());
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for GmailPubSub {
    fn platform(&self) -> Platform {
        Platform::Gmail
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.runtime.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "gmail".to_string(),
            }
            .into());
        }

        self.runtime.rate_limiter.until_ready().await;

        let token = self.runtime.require_access_token().await?;
        let attachments = GmailRuntime::outgoing_attachments_from_metadata(&msg.metadata).await?;
        if let Some(action) = msg
            .metadata
            .get("gmail_action")
            .and_then(|value| value.as_str())
            .map(|value| value.to_ascii_lowercase())
            && action != "send"
        {
            let message_id = GmailRuntime::metadata_string(&msg.metadata, "gmail_message_id");
            return match action.as_str() {
                "reply" => {
                    let Some(message_id) = message_id else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_message_id for gmail_action=reply".to_string(),
                        }
                        .into());
                    };
                    self.runtime
                        .reply_to_message(&message_id, &msg.content, &attachments, &token)
                        .await
                }
                "label" => {
                    let Some(message_id) = message_id else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_message_id for gmail_action=label".to_string(),
                        }
                        .into());
                    };
                    let add =
                        GmailRuntime::metadata_address_list(&msg.metadata, "gmail_add_labels");
                    let remove =
                        GmailRuntime::metadata_address_list(&msg.metadata, "gmail_remove_labels");
                    self.runtime
                        .modify_labels(&message_id, add, remove, &token)
                        .await
                }
                "archive" => {
                    let Some(message_id) = message_id else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_message_id for gmail_action=archive"
                                .to_string(),
                        }
                        .into());
                    };
                    self.runtime
                        .modify_labels(&message_id, Vec::new(), vec!["INBOX".to_string()], &token)
                        .await
                }
                "delete" => {
                    let Some(message_id) = message_id else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_message_id for gmail_action=delete".to_string(),
                        }
                        .into());
                    };
                    self.runtime.delete_message(&message_id, &token).await
                }
                "forward" => {
                    let Some(message_id) = message_id else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_message_id for gmail_action=forward"
                                .to_string(),
                        }
                        .into());
                    };
                    let Some(forward_to) =
                        GmailRuntime::metadata_string(&msg.metadata, "gmail_forward_to")
                    else {
                        return Err(ChannelError::InvalidFormat {
                            platform: "gmail".to_string(),
                            message: "Missing gmail_forward_to for gmail_action=forward"
                                .to_string(),
                        }
                        .into());
                    };
                    self.runtime
                        .forward_message(
                            &message_id,
                            &forward_to,
                            &msg.content,
                            &attachments,
                            &token,
                        )
                        .await
                }
                other => {
                    return Err(ChannelError::InvalidFormat {
                        platform: "gmail".to_string(),
                        message: format!("Unsupported gmail_action '{}'", other),
                    }
                    .into());
                }
            };
        }
        if let Some(message_id) = msg
            .metadata
            .get("gmail_message_id")
            .and_then(|value| value.as_str())
        {
            return self
                .runtime
                .reply_to_message(message_id, &msg.content, &attachments, &token)
                .await;
        }

        let to = GmailRuntime::metadata_address_list(&msg.metadata, "gmail_to");
        if to.is_empty() {
            return Err(ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: "Missing gmail_message_id for reply or gmail_to for direct Gmail send"
                    .to_string(),
            }
            .into());
        }

        let cc = GmailRuntime::metadata_address_list(&msg.metadata, "gmail_cc");
        let bcc = GmailRuntime::metadata_address_list(&msg.metadata, "gmail_bcc");
        let subject = msg
            .metadata
            .get("gmail_subject")
            .and_then(|value| value.as_str())
            .unwrap_or("OpenRustClaw");
        let thread_id = GmailRuntime::metadata_string(&msg.metadata, "gmail_thread_id");

        self.runtime
            .send_new_message(
                &to,
                &cc,
                &bcc,
                subject,
                &msg.content,
                &attachments,
                thread_id.as_deref(),
                &token,
            )
            .await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "gmail".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.runtime.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to Gmail Pub/Sub...");
        if self.runtime.config.project_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail Pub/Sub project ID is required".to_string(),
            }
            .into());
        }
        if self.runtime.config.subscription_name.is_empty()
            && self.runtime.config.topic_name.is_none()
        {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail Pub/Sub subscription name or topic_name is required".to_string(),
            }
            .into());
        }
        if self.runtime.config.user_email.is_empty() {
            return Err(ChannelError::Config {
                platform: "gmail".to_string(),
                message: "Gmail user email is required".to_string(),
            }
            .into());
        }

        let token = self.runtime.authenticate().await?;
        *self.runtime.access_token.write().await = Some(token.clone());

        let watch = self.runtime.setup_watch(&token).await?;
        info!(
            history_id = %watch.history_id,
            expiration = %watch.expiration,
            "Gmail watch established"
        );

        *self.runtime.is_connected.write().await = true;
        info!("Gmail Pub/Sub channel connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(token) = self.runtime.access_token.read().await.clone()
            && let Err(error) = self.runtime.stop_watch(&token).await
        {
            info!(error = %error, "Failed to stop Gmail watch during disconnect");
        }
        *self.runtime.is_connected.write().await = false;
        *self.runtime.access_token.write().await = None;
        info!("Gmail Pub/Sub channel disconnected");
        Ok(())
    }
}

pub struct GmailWebhookHandler {
    runtime: GmailRuntime,
}

impl GmailWebhookHandler {
    pub async fn handle_push(&self, body: &[u8]) -> Result<GmailNotificationReport> {
        if let Ok(notification) = serde_json::from_slice::<GmailNotification>(body) {
            return self.runtime.process_notification(notification).await;
        }

        let envelope: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Pub/Sub envelope: {}", e),
            })?;

        let data = envelope
            .get("message")
            .and_then(|message| message.get("data"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: "Missing message.data in Pub/Sub envelope".to_string(),
            })?;

        let decoded = STANDARD
            .decode(data)
            .map_err(|e| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: format!("Failed to decode Pub/Sub message data: {}", e),
            })?;

        let notification: GmailNotification =
            serde_json::from_slice(&decoded).map_err(|e| ChannelError::InvalidFormat {
                platform: "gmail".to_string(),
                message: format!("Failed to parse Gmail notification: {}", e),
            })?;

        self.runtime.process_notification(notification).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn gmail_config(server: &MockServer, key_path: String) -> GmailPubSubConfig {
        GmailPubSubConfig {
            enabled: true,
            project_id: "test-project".to_string(),
            subscription_name: "gmail-notifications".to_string(),
            topic_name: Some("projects/test-project/topics/gmail-notifications".to_string()),
            service_account_key_path: key_path,
            user_email: "user@example.com".to_string(),
            label_filters: vec!["INBOX".to_string()],
            query_filter: None,
            auto_reply: false,
            max_history_fetch: 100,
            rate_limit_requests_per_second: 10,
            api_base_url: Some(format!("{}/gmail/v1", server.uri())),
            oauth_token_url: Some(format!("{}/token", server.uri())),
        }
    }

    #[test]
    fn test_parse_addresses() {
        let addresses = GmailRuntime::parse_addresses("user1@example.com, user2@example.com");
        assert_eq!(addresses, vec!["user1@example.com", "user2@example.com"]);
    }

    #[tokio::test]
    async fn test_connect_watch_and_process_push() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/history"))
            .and(query_param("startHistoryId", "100"))
            .and(query_param("historyTypes", "messageAdded"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "history": [{
                    "messagesAdded": [{
                        "message": {
                            "id": "msg-1",
                            "threadId": "thread-1"
                        }
                    }]
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-1"))
            .and(query_param("format", "full"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1",
                "threadId": "thread-1",
                "labelIds": ["INBOX", "UNREAD"],
                "internalDate": "1710000000000",
                "historyId": "100",
                "payload": {
                    "mimeType": "multipart/alternative",
                    "headers": [
                        {"name": "From", "value": "sender@example.com"},
                        {"name": "To", "value": "user@example.com"},
                        {"name": "Cc", "value": "cc@example.com"},
                        {"name": "Subject", "value": "Test subject"},
                        {"name": "Message-ID", "value": "<msg-1@example.com>"},
                        {"name": "References", "value": "<root@example.com>"},
                        {"name": "In-Reply-To", "value": "<root@example.com>"}
                    ],
                    "parts": [{
                        "mimeType": "text/plain",
                        "body": {
                            "data": "SGVsbG8gZnJvbSBHbWFpbA=="
                        }
                    }, {
                        "mimeType": "text/html",
                        "body": {
                            "data": "PHA+SGVsbG8gZnJvbSBHbWFpbDwvcD4="
                        }
                    }, {
                        "mimeType": "application/pdf",
                        "filename": "report.pdf",
                        "body": {
                            "attachmentId": "att-1",
                            "size": 2048
                        }
                    }]
                }
            })))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();

        let handler = gmail.webhook_handler();
        let notification = STANDARD.encode(
            serde_json::json!({
                "emailAddress": "user@example.com",
                "historyId": 100
            })
            .to_string(),
        );
        let report = handler
            .handle_push(
                serde_json::json!({
                    "message": {
                        "data": notification
                    }
                })
                .to_string()
                .as_bytes(),
            )
            .await
            .unwrap();
        assert_eq!(report.email_address, "user@example.com");
        assert_eq!(report.history_id, 100);
        assert_eq!(report.processed_count, 1);
        assert_eq!(report.skipped_count, 0);
        assert_eq!(report.entries[0].message_id, "msg-1");
        assert_eq!(report.entries[0].subject, "Test subject");

        let incoming = gmail.receive().await.unwrap();
        assert_eq!(incoming.user_id, "sender@example.com");
        assert_eq!(incoming.metadata["gmail_message_id"], "msg-1");
        assert_eq!(incoming.metadata["gmail_has_message_id"], true);
        assert_eq!(incoming.metadata["gmail_thread_id"], "thread-1");
        assert_eq!(incoming.metadata["gmail_has_thread_id"], true);
        assert_eq!(incoming.metadata["gmail_history_id"], 100);
        assert_eq!(incoming.metadata["gmail_has_history_id"], true);
        assert_eq!(incoming.metadata["gmail_has_subject"], true);
        assert_eq!(incoming.metadata["gmail_has_subject_length"], true);
        assert_eq!(incoming.metadata["gmail_from_length"], 18);
        assert_eq!(incoming.metadata["gmail_to"][0], "user@example.com");
        assert_eq!(incoming.metadata["gmail_has_to"], true);
        assert_eq!(incoming.metadata["gmail_to_count"], 1);
        assert_eq!(incoming.metadata["gmail_to_domains"][0], "example.com");
        assert_eq!(incoming.metadata["gmail_to_domain_count"], 1);
        assert_eq!(incoming.metadata["gmail_has_to_domains"], true);
        assert_eq!(incoming.metadata["gmail_cc"][0], "cc@example.com");
        assert_eq!(incoming.metadata["gmail_cc_count"], 1);
        assert_eq!(incoming.metadata["gmail_has_cc"], true);
        assert_eq!(incoming.metadata["gmail_cc_domains"][0], "example.com");
        assert_eq!(incoming.metadata["gmail_cc_domain_count"], 1);
        assert_eq!(incoming.metadata["gmail_has_cc_domains"], true);
        assert_eq!(incoming.metadata["gmail_from_domain"], "example.com");
        assert_eq!(incoming.metadata["gmail_from_domain_length"], 11);
        assert_eq!(incoming.metadata["gmail_has_from_domain"], true);
        assert_eq!(incoming.metadata["gmail_has_labels"], true);
        assert_eq!(incoming.metadata["gmail_label_count"], 2);
        assert_eq!(incoming.metadata["gmail_is_unread"], true);
        assert_eq!(incoming.metadata["gmail_subject_length"], 12);
        assert_eq!(incoming.metadata["gmail_has_body_text"], true);
        assert_eq!(incoming.metadata["gmail_body_text_length"], 16);
        assert_eq!(incoming.metadata["gmail_body_html_length"], 23);
        assert_eq!(
            incoming.metadata["gmail_received_at"],
            "2024-03-09T16:00:00+00:00"
        );
        assert_eq!(incoming.metadata["gmail_has_received_at"], true);
        assert_eq!(incoming.metadata["gmail_has_attachments"], true);
        assert_eq!(incoming.metadata["gmail_attachment_names"][0], "report.pdf");
        assert_eq!(incoming.metadata["gmail_attachment_name_count"], 1);
        assert_eq!(incoming.metadata["gmail_has_attachment_names"], true);
        assert_eq!(incoming.metadata["gmail_attachment_total_size"], 2048);
        assert_eq!(incoming.metadata["gmail_has_attachment_total_size"], true);
        assert_eq!(
            incoming.metadata["gmail_attachment_mime_types"][0],
            "application/pdf"
        );
        assert_eq!(incoming.metadata["gmail_attachment_mime_type_count"], 1);
        assert_eq!(incoming.metadata["gmail_has_attachment_mime_types"], true);
        assert_eq!(
            incoming.metadata["gmail_message_id_header"],
            "<msg-1@example.com>"
        );
        assert_eq!(incoming.metadata["gmail_has_message_id_header"], true);
        assert_eq!(incoming.metadata["gmail_references"], "<root@example.com>");
        assert_eq!(incoming.metadata["gmail_has_references"], true);
        assert_eq!(incoming.metadata["gmail_in_reply_to"], "<root@example.com>");
        assert_eq!(incoming.metadata["gmail_has_in_reply_to"], true);
        assert_eq!(
            incoming.metadata["gmail_attachment_count"],
            serde_json::json!(1)
        );
        assert_eq!(
            incoming.metadata["gmail_file_reference_count"],
            serde_json::json!(1)
        );
        assert_eq!(incoming.metadata["gmail_has_file_references"], true);
        assert_eq!(incoming.metadata["gmail_has_body_html_length"], true);
        assert_eq!(
            incoming.metadata["file_references"][0]["url"],
            "gmail-attachment://msg-1/att-1"
        );
        assert_eq!(
            incoming.metadata["file_references"][0]["name"],
            "report.pdf"
        );
    }

    #[tokio::test]
    async fn test_reply_send() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-1"))
            .and(query_param("format", "full"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1",
                "threadId": "thread-1",
                "labelIds": ["INBOX"],
                "internalDate": "1710000000000",
                "historyId": "100",
                "payload": {
                    "mimeType": "text/plain",
                    "headers": [
                        {"name": "From", "value": "sender@example.com"},
                        {"name": "Subject", "value": "Original subject"},
                        {"name": "Message-ID", "value": "<msg-1@example.com>"}
                    ],
                    "body": {
                        "data": "SGVsbG8="
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/messages/send"))
            .and(body_string_contains("threadId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sent-1"
            })))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "Reply body".to_string(),
                metadata: serde_json::json!({
                    "gmail_message_id": "msg-1"
                }),
            })
            .await
            .unwrap();
    }

    #[test]
    fn test_build_reply_mime_with_attachment() {
        let mime = GmailRuntime::build_reply_mime(
            "sender@example.com",
            "Re: Original subject",
            Some("<msg-1@example.com>"),
            None,
            "Reply body",
            &[OutgoingAttachment {
                filename: "notes.txt".to_string(),
                mime_type: "text/plain".to_string(),
                bytes: b"attachment body".to_vec(),
            }],
        );

        assert!(mime.contains("Content-Type: multipart/mixed"));
        assert!(mime.contains("Content-Disposition: attachment; filename=\"notes.txt\""));
        assert!(mime.contains(&STANDARD.encode("attachment body")));
    }

    #[test]
    fn test_build_new_message_mime_with_attachment() {
        let mime = GmailRuntime::build_new_message_mime(
            &[String::from("to@example.com")],
            &[String::from("cc@example.com")],
            &[],
            "Status update",
            "Message body",
            &[OutgoingAttachment {
                filename: "report.txt".to_string(),
                mime_type: "text/plain".to_string(),
                bytes: b"report bytes".to_vec(),
            }],
        );

        assert!(mime.contains("To: to@example.com"));
        assert!(mime.contains("Cc: cc@example.com"));
        assert!(mime.contains("Subject: Status update"));
        assert!(mime.contains("Content-Type: multipart/mixed"));
        assert!(mime.contains("Content-Disposition: attachment; filename=\"report.txt\""));
        assert!(mime.contains(&STANDARD.encode("report bytes")));
    }

    #[tokio::test]
    async fn test_reply_send_with_local_attachment() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-1"))
            .and(query_param("format", "full"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1",
                "threadId": "thread-1",
                "labelIds": ["INBOX"],
                "internalDate": "1710000000000",
                "historyId": "100",
                "payload": {
                    "mimeType": "text/plain",
                    "headers": [
                        {"name": "From", "value": "sender@example.com"},
                        {"name": "Subject", "value": "Original subject"},
                        {"name": "Message-ID", "value": "<msg-1@example.com>"}
                    ],
                    "body": {
                        "data": "SGVsbG8="
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/messages/send"))
            .and(body_string_contains("threadId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sent-1"
            })))
            .mount(&server)
            .await;

        let attachment_path =
            std::env::temp_dir().join(format!("orc-gmail-attachment-{}.txt", Uuid::new_v4()));
        tokio::fs::write(&attachment_path, b"attachment body")
            .await
            .expect("write attachment");

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "Reply body".to_string(),
                metadata: serde_json::json!({
                    "gmail_message_id": "msg-1",
                    "file_references": [{
                        "local_path": attachment_path,
                        "name": "notes.txt",
                        "mime": "text/plain"
                    }]
                }),
            })
            .await
            .unwrap();

        let _ = tokio::fs::remove_file(attachment_path).await;
    }

    #[tokio::test]
    async fn test_direct_send_with_local_attachment() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sent-2"
            })))
            .mount(&server)
            .await;

        let attachment_path = std::env::temp_dir().join(format!(
            "orc-gmail-direct-attachment-{}.txt",
            Uuid::new_v4()
        ));
        tokio::fs::write(&attachment_path, b"direct attachment body")
            .await
            .expect("write attachment");

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "Direct body".to_string(),
                metadata: serde_json::json!({
                    "gmail_to": ["person@example.com"],
                    "gmail_cc": "copy@example.com",
                    "gmail_subject": "Status update",
                    "file_references": [{
                        "local_path": attachment_path,
                        "name": "report.txt",
                        "mime": "text/plain"
                    }]
                }),
            })
            .await
            .unwrap();

        let _ = tokio::fs::remove_file(attachment_path).await;
    }

    #[tokio::test]
    async fn test_handle_direct_notification_payload() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/history"))
            .and(query_param("startHistoryId", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "history": [{
                    "messagesAdded": [{
                        "message": {
                            "id": "msg-1",
                            "threadId": "thread-1"
                        }
                    }]
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-1"))
            .and(query_param("format", "full"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1",
                "threadId": "thread-1",
                "labelIds": ["INBOX"],
                "internalDate": "1710000000000",
                "historyId": "100",
                "payload": {
                    "mimeType": "text/plain",
                    "headers": [
                        {"name": "From", "value": "sender@example.com"},
                        {"name": "To", "value": "user@example.com"},
                        {"name": "Subject", "value": "Direct notification"}
                    ],
                    "body": {
                        "data": "SGVsbG8="
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();

        let report = gmail
            .webhook_handler()
            .handle_push(br#"{"emailAddress":"user@example.com","historyId":100}"#)
            .await
            .unwrap();
        assert_eq!(report.processed_count, 1);
        assert_eq!(report.entries[0].message_id, "msg-1");
        assert_eq!(report.entries[0].subject, "Direct notification");

        let incoming = gmail.receive().await.unwrap();
        assert_eq!(incoming.metadata["gmail_message_id"], "msg-1");
        assert_eq!(incoming.content, "Subject: Direct notification\n\nHello");
    }

    #[tokio::test]
    async fn test_disconnect_stops_watch() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/stop"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_send_label_action_via_metadata() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user%40example.com/messages/msg-1/modify",
            ))
            .and(body_string_contains("STARRED"))
            .and(body_string_contains("UNREAD"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1"
            })))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: String::new(),
                metadata: serde_json::json!({
                    "gmail_action": "label",
                    "gmail_message_id": "msg-1",
                    "gmail_add_labels": ["STARRED"],
                    "gmail_remove_labels": ["UNREAD"]
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_archive_and_delete_actions_via_metadata() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user%40example.com/messages/msg-1/modify",
            ))
            .and(body_string_contains("INBOX"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1"
            })))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-2"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: String::new(),
                metadata: serde_json::json!({
                    "gmail_action": "archive",
                    "gmail_message_id": "msg-1"
                }),
            })
            .await
            .unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: String::new(),
                metadata: serde_json::json!({
                    "gmail_action": "delete",
                    "gmail_message_id": "msg-2"
                }),
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_forward_action_with_local_attachment() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user%40example.com/messages/msg-1"))
            .and(query_param("format", "full"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg-1",
                "threadId": "thread-1",
                "labelIds": ["INBOX"],
                "internalDate": "1710000000000",
                "historyId": "100",
                "payload": {
                    "mimeType": "text/plain",
                    "headers": [
                        {"name": "From", "value": "sender@example.com"},
                        {"name": "Subject", "value": "Original subject"}
                    ],
                    "body": {
                        "data": "SGVsbG8="
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sent-forward"
            })))
            .mount(&server)
            .await;

        let attachment_path = std::env::temp_dir().join(format!(
            "orc-gmail-forward-attachment-{}.txt",
            Uuid::new_v4()
        ));
        tokio::fs::write(&attachment_path, b"forward attachment")
            .await
            .expect("write attachment");

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "Please see below".to_string(),
                metadata: serde_json::json!({
                    "gmail_action": "forward",
                    "gmail_message_id": "msg-1",
                    "gmail_forward_to": "forward@example.com",
                    "file_references": [{
                        "local_path": attachment_path,
                        "name": "notes.txt",
                        "mime": "text/plain"
                    }]
                }),
            })
            .await
            .unwrap();

        let _ = tokio::fs::remove_file(attachment_path).await;
    }

    #[tokio::test]
    async fn test_direct_send_supports_gmail_thread_id() {
        let server = MockServer::start().await;
        let config = gmail_config(&server, "token:test-token".to_string());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/watch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "historyId": "100",
                "expiration": "999999"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user%40example.com/messages/send"))
            .and(body_string_contains("thread-99"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sent-threaded"
            })))
            .mount(&server)
            .await;

        let mut gmail = GmailPubSub::new(config);
        gmail.connect().await.unwrap();
        gmail
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "Threaded send".to_string(),
                metadata: serde_json::json!({
                    "gmail_to": ["person@example.com"],
                    "gmail_subject": "Thread update",
                    "gmail_thread_id": "thread-99"
                }),
            })
            .await
            .unwrap();
    }
}
