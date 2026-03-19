//! Matrix Protocol integration for OpenRustClaw.
//!
//! Features:
//! - Text messages in rooms
//! - Direct messages
//! - File attachments
//! - Auto-join rooms on invite
//! - User and room allowlists
//! - Message reactions
//! - Thread support
//! - Rate limiting

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::MatrixConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

pub struct MatrixChannel {
    config: MatrixConfig,
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
    access_token: RwLock<Option<String>>,
    sync_task: Mutex<Option<JoinHandle<()>>>,
    seen_events: Arc<RwLock<HashSet<String>>>,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    next_batch: String,
    #[serde(default)]
    rooms: SyncRooms,
}

#[derive(Debug, Default, Deserialize)]
struct SyncRooms {
    #[serde(default)]
    join: HashMap<String, SyncJoinedRoom>,
    #[serde(default)]
    invite: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct SyncJoinedRoom {
    #[serde(default)]
    timeline: SyncTimeline,
}

#[derive(Debug, Default, Deserialize)]
struct SyncTimeline {
    #[serde(default)]
    events: Vec<SyncEvent>,
}

#[derive(Debug, Deserialize)]
struct SyncEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: String,
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    state_key: Option<String>,
    #[serde(default)]
    content: serde_json::Value,
}

impl MatrixChannel {
    pub fn new(config: MatrixConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_per_second.max(1))
                .unwrap_or(NonZeroU32::new(10).expect("non-zero quota")),
        );

        Self {
            config,
            incoming_tx,
            incoming_rx: Mutex::new(incoming_rx),
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
            is_connected: RwLock::new(false),
            access_token: RwLock::new(None),
            sync_task: Mutex::new(None),
            seen_events: Arc::new(RwLock::new(HashSet::new())),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build matrix http client"),
        }
    }

    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.config.allowlist.is_empty() || self.config.allowlist.contains(&user_id.to_string())
    }

    #[allow(dead_code)]
    fn is_room_allowed(&self, room_id: &str) -> bool {
        self.config.room_allowlist.is_empty()
            || self.config.room_allowlist.contains(&room_id.to_string())
    }

    #[allow(dead_code)]
    fn extract_display_name(user_id: &str) -> String {
        user_id
            .split(':')
            .next()
            .unwrap_or(user_id)
            .trim_start_matches('@')
            .to_string()
    }

    fn html_to_text(html: &str) -> String {
        html.replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<p>", "")
            .replace("</p>", "\n")
            .replace("<b>", "**")
            .replace("</b>", "**")
            .replace("<i>", "*")
            .replace("</i>", "*")
            .replace("<code>", "`")
            .replace("</code>", "`")
    }

    #[allow(dead_code)]
    fn text_to_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\n', "<br>")
    }

    fn parse_formatted_body(metadata: &serde_json::Value) -> Option<String> {
        metadata
            .get("formatted_body")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    }

    fn parse_upload_file_reference(
        metadata: &serde_json::Value,
    ) -> Option<(String, Option<String>)> {
        let refs = metadata.get("file_references")?.as_array()?;
        for entry in refs {
            let local_path = entry
                .get("local_path")
                .or_else(|| entry.get("path"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())?;
            let name = entry
                .get("name")
                .or_else(|| entry.get("title"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            return Some((local_path.to_string(), name));
        }
        None
    }

    fn data_dir(&self) -> PathBuf {
        PathBuf::from(&self.config.data_dir)
    }

    fn downloads_dir(&self) -> PathBuf {
        self.data_dir().join("downloads")
    }

    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/_matrix/client/v3{}",
            self.config.homeserver.trim_end_matches('/'),
            path
        )
    }

    fn media_endpoint(&self, path: &str) -> String {
        format!(
            "{}/_matrix/media/v3{}",
            self.config.homeserver.trim_end_matches('/'),
            path
        )
    }

    fn parse_mxc_uri(uri: &str) -> Option<(&str, &str)> {
        let trimmed = uri.strip_prefix("mxc://")?;
        let mut parts = trimmed.splitn(2, '/');
        let server = parts.next()?;
        let media_id = parts.next()?;
        if server.is_empty() || media_id.is_empty() {
            return None;
        }
        Some((server, media_id))
    }

    fn safe_download_name(event_id: &str, fallback: &str) -> String {
        let event = event_id
            .trim_start_matches('$')
            .replace(['/', '\\', ':'], "_");
        let fallback = fallback.replace(['/', '\\'], "_");
        format!("{}-{}", event, fallback)
    }

    async fn upload_file_to_matrix(
        &self,
        token: &str,
        file_path: &str,
        filename: Option<&str>,
    ) -> Result<String> {
        let bytes = tokio::fs::read(file_path)
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Failed to read Matrix upload file: {}", e),
            })?;
        let filename = filename
            .map(ToString::to_string)
            .or_else(|| {
                PathBuf::from(file_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "file".to_string());

        let upload = self
            .http
            .post(self.media_endpoint("/upload"))
            .bearer_auth(token)
            .query(&[("filename", filename.as_str())])
            .body(bytes)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix upload failed: {}", e),
            })?;

        if !upload.status().is_success() {
            let body = upload.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix upload failed: {}", body),
            }
            .into());
        }

        let upload_body: serde_json::Value =
            upload.json().await.map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Failed to parse Matrix upload response: {}", e),
            })?;
        upload_body
            .get("content_uri")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .ok_or_else(|| {
                ChannelError::SendFailed {
                    platform: "matrix".to_string(),
                    message: "Matrix upload response missing content_uri".to_string(),
                }
                .into()
            })
    }

    async fn download_media_to_dir(
        http: &reqwest::Client,
        homeserver: &str,
        token: &str,
        downloads_dir: &PathBuf,
        content_uri: &str,
        event_id: &str,
        filename_hint: Option<&str>,
    ) -> Result<Option<String>> {
        let Some((server, media_id)) = Self::parse_mxc_uri(content_uri) else {
            return Ok(None);
        };
        if !downloads_dir.exists() {
            tokio::fs::create_dir_all(downloads_dir)
                .await
                .map_err(|e| ChannelError::SendFailed {
                    platform: "matrix".to_string(),
                    message: format!("Failed to create Matrix downloads directory: {}", e),
                })?;
        }

        let filename = Self::safe_download_name(event_id, filename_hint.unwrap_or("media"));
        let path = downloads_dir.join(filename);
        let response = http
            .get(format!(
                "{}/_matrix/media/v3/download/{}/{}",
                homeserver.trim_end_matches('/'),
                urlencoding::encode(server),
                urlencoding::encode(media_id)
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix media download failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix media download failed: {}", body),
            }
            .into());
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Failed to read Matrix media download: {}", e),
            })?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Failed to persist Matrix media download: {}", e),
            })?;
        Ok(Some(path.display().to_string()))
    }

    async fn bearer_token(&self) -> Result<String> {
        self.access_token.read().await.clone().ok_or_else(|| {
            ChannelError::AuthFailed {
                platform: "matrix".to_string(),
                message: "Matrix access token is not available".to_string(),
            }
            .into()
        })
    }

    async fn login_with_password(&self, password: &str) -> Result<String> {
        #[derive(Serialize)]
        struct LoginIdentifier<'a> {
            #[serde(rename = "type")]
            kind: &'a str,
            user: &'a str,
        }

        #[derive(Serialize)]
        struct LoginRequest<'a> {
            #[serde(rename = "type")]
            kind: &'a str,
            identifier: LoginIdentifier<'a>,
            password: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            device_id: Option<&'a str>,
        }

        #[derive(Deserialize)]
        struct LoginResponse {
            access_token: String,
        }

        let request = LoginRequest {
            kind: "m.login.password",
            identifier: LoginIdentifier {
                kind: "m.id.user",
                user: &self.config.user_id,
            },
            password,
            device_id: self.config.device_id.as_deref(),
        };

        let response = self
            .http
            .post(self.endpoint("/login"))
            .json(&request)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix login request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix login failed: {}", body),
            }
            .into());
        }

        let body: LoginResponse = response
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "matrix".to_string(),
                message: format!("Failed to parse Matrix login response: {}", e),
            })?;

        Ok(body.access_token)
    }

    async fn start_sync_loop(&self) -> Result<()> {
        let token = self.bearer_token().await?;
        let homeserver = self.config.homeserver.clone();
        let user_id = self.config.user_id.clone();
        let downloads_dir = self.downloads_dir();
        let room_allowlist = self.config.room_allowlist.clone();
        let user_allowlist = self.config.allowlist.clone();
        let auto_join_rooms = self.config.auto_join_rooms;
        let incoming_tx = self.incoming_tx.clone();
        let seen_events = self.seen_events.clone();
        let http = self.http.clone();

        let task = tokio::spawn(async move {
            let mut since: Option<String> = None;

            loop {
                let mut request = http
                    .get(format!(
                        "{}/_matrix/client/v3/sync",
                        homeserver.trim_end_matches('/')
                    ))
                    .bearer_auth(&token)
                    .query(&[("timeout", "30000")]);

                if let Some(ref since_token) = since {
                    request = request.query(&[("since", since_token.as_str())]);
                }

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        warn!(error = %error, "Matrix sync request failed");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    warn!(status = %status, body = %body, "Matrix sync returned non-success status");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let body: SyncResponse = match response.json().await {
                    Ok(body) => body,
                    Err(error) => {
                        warn!(error = %error, "Failed to parse Matrix sync response");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                since = Some(body.next_batch.clone());

                if auto_join_rooms {
                    for room_id in body.rooms.invite.keys() {
                        let _ = http
                            .post(format!(
                                "{}/_matrix/client/v3/join/{}",
                                homeserver.trim_end_matches('/'),
                                urlencoding::encode(room_id)
                            ))
                            .bearer_auth(&token)
                            .send()
                            .await;
                    }
                }

                for (room_id, room_state) in body.rooms.join {
                    if !room_allowlist.is_empty() && !room_allowlist.contains(&room_id) {
                        continue;
                    }

                    for event in room_state.timeline.events {
                        if event.event_type != "m.room.message"
                            && event.event_type != "m.reaction"
                            && event.event_type != "m.room.redaction"
                            && event.event_type != "m.room.member"
                        {
                            continue;
                        }
                        if event.sender == user_id {
                            continue;
                        }
                        if !user_allowlist.is_empty() && !user_allowlist.contains(&event.sender) {
                            continue;
                        }

                        let Some(event_id) = event.event_id.clone() else {
                            continue;
                        };

                        {
                            let mut seen = seen_events.write().await;
                            if !seen.insert(event_id.clone()) {
                                continue;
                            }
                        }

                        let relates_to = event
                            .content
                            .get("m.relates_to")
                            .cloned()
                            .unwrap_or_default();
                        if event.event_type == "m.room.redaction" {
                            let metadata = serde_json::json!({
                                "matrix_room_id": room_id,
                                "matrix_event_id": event_id,
                                "matrix_sender": event.sender,
                                "matrix_msgtype": "m.room.redaction",
                                "matrix_is_group": true,
                                "matrix_redacts": event.content.get("redacts").and_then(|value| value.as_str()),
                                "matrix_redaction_reason": event.content.get("reason").and_then(|value| value.as_str()),
                            });

                            let incoming = IncomingMessage {
                                session_id: Uuid::new_v4(),
                                user_id: event.sender.clone(),
                                content: "[matrix redaction]".to_string(),
                                platform: Platform::Matrix,
                                metadata,
                            };

                            if incoming_tx.send(incoming).await.is_err() {
                                debug!("matrix incoming channel closed");
                            }
                            continue;
                        }
                        if event.event_type == "m.reaction" {
                            let reaction_key =
                                relates_to.get("key").and_then(|value| value.as_str());
                            let reaction_target = relates_to
                                .get("event_id")
                                .and_then(|value| value.as_str());
                            let Some(reaction_key) = reaction_key else {
                                continue;
                            };

                            let metadata = serde_json::json!({
                                "matrix_room_id": room_id,
                                "matrix_event_id": event_id,
                                "matrix_sender": event.sender,
                                "matrix_msgtype": "m.reaction",
                                "matrix_is_group": true,
                                "matrix_reaction": reaction_key,
                                "matrix_reaction_target": reaction_target,
                                "matrix_reaction_rel_type": relates_to.get("rel_type").and_then(|value| value.as_str()),
                            });

                            let incoming = IncomingMessage {
                                session_id: Uuid::new_v4(),
                                user_id: event.sender.clone(),
                                content: "[matrix reaction]".to_string(),
                                platform: Platform::Matrix,
                                metadata,
                            };

                            if incoming_tx.send(incoming).await.is_err() {
                                debug!("matrix incoming channel closed");
                            }
                            continue;
                        }
                        if event.event_type == "m.room.member" {
                            let membership = event
                                .content
                                .get("membership")
                                .and_then(|value| value.as_str());
                            let display_name = event
                                .content
                                .get("displayname")
                                .and_then(|value| value.as_str());

                            let metadata = serde_json::json!({
                                "matrix_room_id": room_id,
                                "matrix_event_id": event_id,
                                "matrix_sender": event.sender,
                                "matrix_msgtype": "m.room.member",
                                "matrix_is_group": true,
                                "matrix_membership": membership,
                                "matrix_state_key": event.state_key,
                                "matrix_member_display_name": display_name,
                                "matrix_member_avatar_url": event.content.get("avatar_url").and_then(|value| value.as_str()),
                            });

                            let incoming = IncomingMessage {
                                session_id: Uuid::new_v4(),
                                user_id: event.sender.clone(),
                                content: "[matrix membership event]".to_string(),
                                platform: Platform::Matrix,
                                metadata,
                            };

                            if incoming_tx.send(incoming).await.is_err() {
                                debug!("matrix incoming channel closed");
                            }
                            continue;
                        }

                        let message_type = event
                            .content
                            .get("msgtype")
                            .and_then(|value| value.as_str())
                            .unwrap_or("m.text");
                        let body = event
                            .content
                            .get("body")
                            .and_then(|value| value.as_str())
                            .unwrap_or("");

                        if body.is_empty() {
                            continue;
                        }

                        let formatted = event
                            .content
                            .get("formatted_body")
                            .and_then(|value| value.as_str())
                            .map(Self::html_to_text);
                        let thread_root = relates_to
                            .get("event_id")
                            .and_then(|value| value.as_str())
                            .or_else(|| {
                                relates_to
                                    .get("m.in_reply_to")
                                    .and_then(|value| value.get("event_id"))
                                    .and_then(|value| value.as_str())
                            });
                        let reply_to = relates_to
                            .get("m.in_reply_to")
                            .and_then(|value| value.get("event_id"))
                            .and_then(|value| value.as_str());

                        let mut metadata = serde_json::json!({
                            "matrix_room_id": room_id,
                            "matrix_event_id": event_id,
                            "matrix_sender": event.sender,
                            "matrix_msgtype": message_type,
                            "matrix_thread_root": thread_root,
                            "matrix_has_thread": thread_root.is_some(),
                            "matrix_is_group": true,
                            "matrix_body_length": body.chars().count(),
                        });
                        if let Some(reply_to) = reply_to {
                            metadata["matrix_reply_to"] = serde_json::json!(reply_to);
                        }
                        metadata["matrix_has_reply"] = serde_json::json!(reply_to.is_some());
                        if let Some(format) =
                            event.content.get("format").and_then(|value| value.as_str())
                        {
                            metadata["matrix_format"] = serde_json::json!(format);
                        }
                        metadata["matrix_has_formatted_body"] =
                            serde_json::json!(event.content.get("formatted_body").is_some());
                        if let Some(formatted_body) = event
                            .content
                            .get("formatted_body")
                            .and_then(|value| value.as_str())
                        {
                            metadata["matrix_formatted_body"] = serde_json::json!(formatted_body);
                            metadata["matrix_formatted_body_length"] =
                                serde_json::json!(formatted_body.chars().count());
                        }
                        if let Some(content_uri) =
                            event.content.get("url").and_then(|value| value.as_str())
                        {
                            metadata["matrix_content_uri"] = serde_json::json!(content_uri);
                            metadata["matrix_has_media"] = serde_json::json!(true);
                            let filename_hint =
                                event.content.get("body").and_then(|value| value.as_str());
                            if let Some(filename_hint) = filename_hint {
                                metadata["matrix_media_filename"] =
                                    serde_json::json!(filename_hint);
                            }
                            let mime = event
                                .content
                                .pointer("/info/mimetype")
                                .and_then(|value| value.as_str());
                            let size = event
                                .content
                                .pointer("/info/size")
                                .and_then(|value| value.as_u64());
                            if let Some(mime) = mime {
                                metadata["matrix_media_mime_type"] = serde_json::json!(mime);
                            }
                            if let Some(size) = size {
                                metadata["matrix_media_size"] = serde_json::json!(size);
                            }
                            metadata["file_references"] = serde_json::json!([serde_json::json!({
                                "url": content_uri,
                                "name": filename_hint,
                                "mime": mime,
                                "size": size,
                            })]);
                            metadata["matrix_file_reference_count"] = serde_json::json!(1);
                            match Self::download_media_to_dir(
                                &http,
                                &homeserver,
                                &token,
                                &downloads_dir,
                                content_uri,
                                &event_id,
                                filename_hint,
                            )
                            .await
                            {
                                Ok(Some(path)) => {
                                    metadata["matrix_download_path"] = serde_json::json!(path);
                                    metadata["file_references"][0]["local_path"] =
                                        serde_json::json!(metadata["matrix_download_path"].as_str());
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    warn!(error = %error, event_id = %event_id, "Failed to download Matrix media");
                                }
                            }
                        } else {
                            metadata["matrix_has_media"] = serde_json::json!(false);
                            metadata["matrix_file_reference_count"] = serde_json::json!(0);
                        }

                        let incoming = IncomingMessage {
                            session_id: Uuid::new_v4(),
                            user_id: event.sender.clone(),
                            content: formatted.unwrap_or_else(|| body.to_string()),
                            platform: Platform::Matrix,
                            metadata,
                        };

                        let _ = incoming_tx.send(incoming).await;
                    }
                }
            }
        });

        *self.sync_task.lock().await = Some(task);
        Ok(())
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn platform(&self) -> Platform {
        Platform::Matrix
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "matrix".to_string(),
            }
            .into());
        }

        self.rate_limiter.until_ready().await;

        let room_id = msg
            .metadata
            .get("matrix_room_id")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ChannelError::InvalidFormat {
                platform: "matrix".to_string(),
                message: "Missing room_id in metadata".to_string(),
            })?;
        let token = self.bearer_token().await?;
        let reply_to_event_id = msg
            .metadata
            .get("matrix_reply_to")
            .or_else(|| msg.metadata.get("matrix_reply_to_event_id"))
            .and_then(|value| value.as_str());
        let thread_root = msg
            .metadata
            .get("matrix_thread_root")
            .and_then(|value| value.as_str());
        let reaction = msg
            .metadata
            .get("matrix_reaction")
            .and_then(|value| value.as_str());
        let txn_id = Uuid::new_v4().to_string();
        let uploaded_content_uri = if msg
            .metadata
            .get("matrix_content_uri")
            .and_then(|value| value.as_str())
            .is_none()
        {
            if let Some((file_path, filename)) = Self::parse_upload_file_reference(&msg.metadata) {
                Some(
                    self.upload_file_to_matrix(&token, &file_path, filename.as_deref())
                        .await?,
                )
            } else {
                None
            }
        } else {
            None
        };

        let (event_type, payload) = if let Some(emoji) = reaction {
            let target_event_id = reply_to_event_id.ok_or_else(|| ChannelError::InvalidFormat {
                platform: "matrix".to_string(),
                message: "Missing matrix_reply_to for matrix_reaction".to_string(),
            })?;
            (
                "m.reaction",
                serde_json::json!({
                    "m.relates_to": {
                        "rel_type": "m.annotation",
                        "event_id": target_event_id,
                        "key": emoji,
                    }
                }),
            )
        } else if let Some(content_uri) = uploaded_content_uri.as_deref().or_else(|| {
            msg.metadata
                .get("matrix_content_uri")
                .and_then(|value| value.as_str())
        }) {
            (
                "m.room.message",
                serde_json::json!({
                    "msgtype": "m.file",
                    "body": msg.content,
                    "url": content_uri,
                }),
            )
        } else {
            let mut body = serde_json::json!({
                "msgtype": "m.text",
                "body": msg.content,
            });

            if let Some(html) = Self::parse_formatted_body(&msg.metadata) {
                body["format"] = serde_json::json!("org.matrix.custom.html");
                body["formatted_body"] = serde_json::json!(html);
            }

            if let Some(thread_root) = thread_root {
                body["m.relates_to"] = serde_json::json!({
                    "rel_type": "m.thread",
                    "event_id": thread_root,
                    "is_falling_back": true,
                    "m.in_reply_to": {
                        "event_id": reply_to_event_id.unwrap_or(thread_root),
                    },
                });
            } else if let Some(reply_to) = reply_to_event_id {
                body["m.relates_to"] = serde_json::json!({
                    "m.in_reply_to": {
                        "event_id": reply_to,
                    }
                });
            }

            ("m.room.message", body)
        };

        let response = self
            .http
            .put(self.endpoint(&format!(
                "/rooms/{}/send/{}/{}",
                urlencoding::encode(room_id),
                event_type,
                txn_id
            )))
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix send request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix send failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Incoming message channel closed".to_string(),
            }
            .into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!(homeserver = %self.config.homeserver, "Connecting to Matrix homeserver...");

        if self.config.homeserver.is_empty() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Matrix homeserver URL is required".to_string(),
            }
            .into());
        }
        if self.config.user_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Matrix user_id is required".to_string(),
            }
            .into());
        }
        if self.config.access_token.is_none() && self.config.password.is_none() {
            return Err(ChannelError::Config {
                platform: "matrix".to_string(),
                message: "Either access_token or password must be provided".to_string(),
            }
            .into());
        }

        let data_dir = self.data_dir();
        if !data_dir.exists() {
            tokio::fs::create_dir_all(&data_dir)
                .await
                .map_err(|e| ChannelError::Config {
                    platform: "matrix".to_string(),
                    message: format!("Failed to create data directory: {}", e),
                })?;
        }

        let token = if let Some(token) = self.config.access_token.clone() {
            token
        } else if let Some(password) = self.config.password.as_deref() {
            self.login_with_password(password).await?
        } else {
            unreachable!("validated above")
        };

        let response = self
            .http
            .get(self.endpoint("/account/whoami"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "matrix".to_string(),
                message: format!("Matrix whoami request failed: {}", e),
            })?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(ChannelError::AuthFailed {
                platform: "matrix".to_string(),
                message: "Matrix access token is unauthorized".to_string(),
            }
            .into());
        }
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: format!("Matrix whoami failed: {}", body),
            }
            .into());
        }

        *self.access_token.write().await = Some(token);
        *self.is_connected.write().await = true;
        self.start_sync_loop().await?;

        info!(
            user_id = %self.config.user_id,
            encryption = self.config.enable_encryption,
            auto_join = self.config.auto_join_rooms,
            "Matrix channel connected"
        );

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Matrix...");
        *self.is_connected.write().await = false;
        *self.access_token.write().await = None;
        if let Some(task) = self.sync_task.lock().await.take() {
            task.abort();
        }
        info!("Matrix channel disconnected");
        Ok(())
    }
}

impl MatrixChannel {
    pub async fn join_room(&self, room_id_or_alias: &str) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        let token = self.bearer_token().await?;
        let response = self
            .http
            .post(self.endpoint(&format!("/join/{}", urlencoding::encode(room_id_or_alias))))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix join failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix join failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    pub async fn leave_room(&self, room_id: &str) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        let token = self.bearer_token().await?;
        let response = self
            .http
            .post(self.endpoint(&format!("/rooms/{}/leave", urlencoding::encode(room_id))))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix leave failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix leave failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    pub async fn send_formatted(&self, room_id: &str, text: &str, html: &str) -> Result<()> {
        self.send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: text.to_string(),
            metadata: serde_json::json!({
                "matrix_room_id": room_id,
                "formatted_body": html,
            }),
        })
        .await
    }

    pub async fn send_reaction(&self, room_id: &str, event_id: &str, emoji: &str) -> Result<()> {
        self.send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: String::new(),
            metadata: serde_json::json!({
                "matrix_room_id": room_id,
                "matrix_reply_to": event_id,
                "matrix_reaction": emoji,
            }),
        })
        .await
    }

    pub async fn send_file(
        &self,
        room_id: &str,
        file_path: &str,
        filename: Option<&str>,
    ) -> Result<()> {
        let token = self.bearer_token().await?;
        let resolved_filename = filename
            .map(ToString::to_string)
            .or_else(|| {
                PathBuf::from(file_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "file".to_string());
        let content_uri = self
            .upload_file_to_matrix(&token, file_path, Some(resolved_filename.as_str()))
            .await?;

        self.send(OutgoingMessage {
            session_id: Uuid::new_v4(),
            content: resolved_filename.clone(),
            metadata: serde_json::json!({
                "matrix_room_id": room_id,
                "matrix_content_uri": content_uri,
            }),
        })
        .await
    }

    pub async fn joined_rooms(&self) -> Result<Vec<String>> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: "Not connected".to_string(),
            }
            .into());
        }

        let token = self.bearer_token().await?;
        let response = self
            .http
            .get(self.endpoint("/joined_rooms"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "matrix".to_string(),
                message: format!("Matrix joined_rooms failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "matrix".to_string(),
                message: format!("Matrix joined_rooms failed: {}", body),
            }
            .into());
        }

        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ChannelError::Connection {
                    platform: "matrix".to_string(),
                    message: format!("Failed to parse Matrix joined_rooms response: {}", e),
                })?;

        Ok(body
            .get("joined_rooms")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|room| room.as_str().map(ToString::to_string))
            .collect())
    }

    pub async fn send_typing_indicator(&self, room_id: &str, typing: bool) -> Result<()> {
        let token = self.bearer_token().await?;
        let response = self
            .http
            .put(self.endpoint(&format!(
                "/rooms/{}/typing/{}",
                urlencoding::encode(room_id),
                urlencoding::encode(&self.config.user_id)
            )))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "typing": typing,
                "timeout": 30000,
            }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix typing request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix typing request failed: {}", body),
            }
            .into());
        }

        Ok(())
    }

    pub async fn redact_message(
        &self,
        room_id: &str,
        event_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        let token = self.bearer_token().await?;
        let response = self
            .http
            .put(self.endpoint(&format!(
                "/rooms/{}/redact/{}/{}",
                urlencoding::encode(room_id),
                urlencoding::encode(event_id),
                Uuid::new_v4()
            )))
            .bearer_auth(token)
            .json(&serde_json::json!({ "reason": reason }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix redact failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "matrix".to_string(),
                message: format!("Matrix redact failed: {}", body),
            }
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(homeserver: &str) -> MatrixConfig {
        MatrixConfig {
            enabled: true,
            homeserver: homeserver.to_string(),
            user_id: "@bot:matrix.org".to_string(),
            access_token: Some("token".to_string()),
            password: None,
            device_id: None,
            data_dir: "./target/test-matrix".to_string(),
            allowlist: vec![],
            room_allowlist: vec![],
            auto_join_rooms: true,
            enable_encryption: false,
            rate_limit_per_second: 10,
        }
    }

    #[test]
    fn test_user_allowed_empty_list() {
        let channel = MatrixChannel::new(test_config("https://matrix.org"));
        assert!(channel.is_user_allowed("@user:matrix.org"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let mut config = test_config("https://matrix.org");
        config.allowlist = vec![
            "@alice:matrix.org".to_string(),
            "@bob:matrix.org".to_string(),
        ];
        let channel = MatrixChannel::new(config);
        assert!(channel.is_user_allowed("@alice:matrix.org"));
        assert!(!channel.is_user_allowed("@charlie:matrix.org"));
    }

    #[test]
    fn test_room_allowed_with_list() {
        let mut config = test_config("https://matrix.org");
        config.room_allowlist = vec!["!room1:matrix.org".to_string()];
        let channel = MatrixChannel::new(config);
        assert!(channel.is_room_allowed("!room1:matrix.org"));
        assert!(!channel.is_room_allowed("!room2:matrix.org"));
    }

    #[test]
    fn test_extract_display_name() {
        assert_eq!(
            MatrixChannel::extract_display_name("@alice:matrix.org"),
            "alice"
        );
        assert_eq!(
            MatrixChannel::extract_display_name("just_text"),
            "just_text"
        );
    }

    #[test]
    fn test_text_to_html() {
        assert_eq!(
            MatrixChannel::text_to_html("Hello\nWorld"),
            "Hello<br>World"
        );
    }

    #[test]
    fn test_html_to_text() {
        assert_eq!(
            MatrixChannel::html_to_text("Hello<b>World</b>"),
            "Hello**World**"
        );
    }

    #[test]
    fn test_parse_formatted_body() {
        let metadata = serde_json::json!({ "formatted_body": "<b>Bold</b> message" });
        assert_eq!(
            MatrixChannel::parse_formatted_body(&metadata),
            Some("<b>Bold</b> message".to_string())
        );
    }

    #[test]
    fn test_parse_mxc_uri() {
        assert_eq!(
            MatrixChannel::parse_mxc_uri("mxc://matrix.org/abc123"),
            Some(("matrix.org", "abc123"))
        );
        assert_eq!(
            MatrixChannel::parse_mxc_uri("https://example.com/file"),
            None
        );
    }

    #[tokio::test]
    async fn test_download_media_to_dir() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/media/v3/download/matrix.org/abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"matrix-file".to_vec()))
            .mount(&server)
            .await;

        let downloads_dir =
            std::env::temp_dir().join(format!("orc-matrix-downloads-{}", Uuid::new_v4()));
        let client = reqwest::Client::new();
        let path = MatrixChannel::download_media_to_dir(
            &client,
            &server.uri(),
            "token",
            &downloads_dir,
            "mxc://matrix.org/abc123",
            "$evt1",
            Some("report.txt"),
        )
        .await
        .expect("download succeeds")
        .expect("download path");

        let bytes = tokio::fs::read(&path).await.expect("file persisted");
        assert_eq!(bytes, b"matrix-file");
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_dir_all(&downloads_dir).await;
    }

    #[tokio::test]
    async fn test_connect_and_send_message() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {}
            })))
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path_regex(
                r"^/_matrix/client/v3/rooms/%21room%3Amatrix\.org/send/m\.room\.message/.*$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "event_id": "$event"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "hello matrix".to_string(),
                metadata: serde_json::json!({
                    "matrix_room_id": "!room:matrix.org",
                }),
            })
            .await
            .expect("send succeeds");

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_send_uploads_local_file_reference() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {}
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/_matrix/media/v3/upload"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content_uri": "mxc://matrix.org/uploaded123"
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path_regex(
                r"^/_matrix/client/v3/rooms/%21room%3Amatrix\.org/send/m\.room\.message/.*$",
            ))
            .and(body_partial_json(serde_json::json!({
                "msgtype": "m.file",
                "url": "mxc://matrix.org/uploaded123"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "event_id": "$event"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let temp_root = std::env::temp_dir().join(format!("orc-matrix-upload-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_root).await.expect("temp dir");
        let file_path = temp_root.join("report.pdf");
        tokio::fs::write(&file_path, b"matrix-file")
            .await
            .expect("write temp file");

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "report.pdf".to_string(),
                metadata: serde_json::json!({
                    "matrix_room_id": "!room:matrix.org",
                    "file_references": [{
                        "local_path": file_path,
                        "name": "report.pdf"
                    }],
                }),
            })
            .await
            .expect("send succeeds");

        channel.disconnect().await.expect("disconnect succeeds");
        let _ = tokio::fs::remove_dir_all(temp_root).await;
    }

    #[tokio::test]
    async fn test_sync_receive_message() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.message",
                                    "sender": "@alice:matrix.org",
                                    "event_id": "$evt1",
                                    "content": {
                                        "msgtype": "m.text",
                                        "body": "hello from matrix"
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming message");

        assert_eq!(incoming.metadata["matrix_body_length"], 17);
        assert_eq!(incoming.metadata["matrix_has_reply"], false);
        assert_eq!(incoming.metadata["matrix_has_thread"], false);
        assert_eq!(incoming.metadata["matrix_has_formatted_body"], false);
        assert_eq!(incoming.metadata["matrix_has_media"], false);
        assert_eq!(incoming.metadata["matrix_file_reference_count"], 0);
        assert_eq!(incoming.user_id, "@alice:matrix.org");
        assert_eq!(incoming.content, "hello from matrix");
        assert_eq!(
            incoming
                .metadata
                .get("matrix_room_id")
                .and_then(|v| v.as_str()),
            Some("!room:matrix.org")
        );

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_sync_receive_message_preserves_reply_metadata() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.message",
                                    "sender": "@alice:matrix.org",
                                    "event_id": "$evt2",
                                    "content": {
                                        "msgtype": "m.text",
                                        "body": "reply from matrix",
                                        "format": "org.matrix.custom.html",
                                        "formatted_body": "<b>reply</b> from matrix",
                                        "m.relates_to": {
                                            "m.in_reply_to": {
                                                "event_id": "$root1"
                                            }
                                        }
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming message");

        assert_eq!(incoming.metadata["matrix_reply_to"], "$root1");
        assert_eq!(incoming.metadata["matrix_thread_root"], "$root1");
        assert_eq!(incoming.metadata["matrix_has_reply"], true);
        assert_eq!(incoming.metadata["matrix_has_thread"], true);
        assert_eq!(incoming.metadata["matrix_body_length"], 17);
        assert_eq!(incoming.metadata["matrix_has_formatted_body"], true);
        assert_eq!(incoming.metadata["matrix_formatted_body_length"], 24);
        assert_eq!(incoming.metadata["matrix_format"], "org.matrix.custom.html");
        assert_eq!(incoming.metadata["matrix_formatted_body"], "<b>reply</b> from matrix");

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_sync_receive_reaction_event() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.reaction",
                                    "sender": "@alice:matrix.org",
                                    "event_id": "$reaction1",
                                    "content": {
                                        "m.relates_to": {
                                            "rel_type": "m.annotation",
                                            "event_id": "$target1",
                                            "key": "👍"
                                        }
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming reaction");

        assert_eq!(incoming.content, "[matrix reaction]");
        assert_eq!(incoming.metadata["matrix_reaction"], "👍");
        assert_eq!(incoming.metadata["matrix_reaction_target"], "$target1");
        assert_eq!(incoming.metadata["matrix_reaction_rel_type"], "m.annotation");

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_sync_receive_redaction_event() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.redaction",
                                    "sender": "@alice:matrix.org",
                                    "event_id": "$redaction1",
                                    "content": {
                                        "redacts": "$target-redacted",
                                        "reason": "cleanup"
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming redaction");

        assert_eq!(incoming.content, "[matrix redaction]");
        assert_eq!(incoming.metadata["matrix_redacts"], "$target-redacted");
        assert_eq!(incoming.metadata["matrix_redaction_reason"], "cleanup");

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_sync_receive_membership_event() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.member",
                                    "sender": "@alice:matrix.org",
                                    "state_key": "@bob:matrix.org",
                                    "event_id": "$member1",
                                    "content": {
                                        "membership": "join",
                                        "displayname": "Bob",
                                        "avatar_url": "mxc://matrix.org/avatar1"
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming membership");

        assert_eq!(incoming.content, "[matrix membership event]");
        assert_eq!(incoming.metadata["matrix_membership"], "join");
        assert_eq!(incoming.metadata["matrix_state_key"], "@bob:matrix.org");
        assert_eq!(incoming.metadata["matrix_member_display_name"], "Bob");
        assert_eq!(incoming.metadata["matrix_member_avatar_url"], "mxc://matrix.org/avatar1");

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_sync_receive_media_message_preserves_structured_file_reference() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/media/v3/download/matrix.org/media-1"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"matrix-media".to_vec()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.message",
                                    "sender": "@alice:matrix.org",
                                    "event_id": "$evt-media",
                                    "content": {
                                        "msgtype": "m.file",
                                        "body": "report.pdf",
                                        "url": "mxc://matrix.org/media-1",
                                        "info": {
                                            "mimetype": "application/pdf",
                                            "size": 2048
                                        }
                                    }
                                }]
                            }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        let incoming = tokio::time::timeout(Duration::from_secs(2), channel.receive())
            .await
            .expect("receive timeout")
            .expect("incoming media");

        assert_eq!(incoming.metadata["matrix_has_media"], true);
        assert_eq!(incoming.metadata["matrix_media_filename"], "report.pdf");
        assert_eq!(incoming.metadata["matrix_content_uri"], "mxc://matrix.org/media-1");
        assert_eq!(incoming.metadata["matrix_media_mime_type"], "application/pdf");
        assert_eq!(incoming.metadata["matrix_media_size"], 2048);
        assert_eq!(incoming.metadata["matrix_file_reference_count"], 1);
        assert_eq!(incoming.metadata["file_references"][0]["name"], "report.pdf");
        assert_eq!(incoming.metadata["file_references"][0]["mime"], "application/pdf");
        assert_eq!(incoming.metadata["file_references"][0]["size"], 2048);
        assert!(incoming.metadata["file_references"][0]["local_path"]
            .as_str()
            .expect("local path")
            .contains("report.pdf"));

        if let Some(path) = incoming.metadata["matrix_download_path"].as_str() {
            let _ = tokio::fs::remove_file(path).await;
        }
        let _ = tokio::fs::remove_dir_all(PathBuf::from(test_config(&server.uri()).data_dir).join("downloads")).await;

        channel.disconnect().await.expect("disconnect succeeds");
    }

    #[tokio::test]
    async fn test_send_supports_reply_alias_and_thread_root() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@bot:matrix.org"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "30000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s1",
                "rooms": {}
            })))
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path_regex(
                r"^/_matrix/client/v3/rooms/%21room%3Amatrix\.org/send/m\.room\.message/.*$",
            ))
            .and(body_partial_json(serde_json::json!({
                "m.relates_to": {
                    "rel_type": "m.thread",
                    "event_id": "$thread-root",
                    "is_falling_back": true,
                    "m.in_reply_to": {
                        "event_id": "$reply-target"
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "event_id": "$event"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let mut channel = MatrixChannel::new(test_config(&server.uri()));
        channel.connect().await.expect("connect succeeds");

        channel
            .send(OutgoingMessage {
                session_id: Uuid::new_v4(),
                content: "threaded reply".to_string(),
                metadata: serde_json::json!({
                    "matrix_room_id": "!room:matrix.org",
                    "matrix_reply_to_event_id": "$reply-target",
                    "matrix_thread_root": "$thread-root",
                }),
            })
            .await
            .expect("send succeeds");

        channel.disconnect().await.expect("disconnect succeeds");
    }
}
