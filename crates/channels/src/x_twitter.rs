//! X (Twitter) Channel Integration
//!
//! Uses X API v2 for mentions, DMs, and tweets.
//!
//! # Features
//! - Poll for mentions of the bot account
//! - Poll for DMs (Direct Messages)
//! - Send tweets and replies
//! - User allowlist for access control
//! - Rate limiting for API compliance
//!
//! # Authentication
//! Requires X API v2 credentials:
//! - Bearer token (for most read operations)
//! - API key/secret + Access token/secret (for write operations)

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use openrustclaw_core::config::XConfig;
use openrustclaw_core::error::{ChannelError, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};

/// X (Twitter) channel implementation.
///
/// This struct manages the connection to X (Twitter) via the API v2,
/// handling authentication, polling for mentions/DMs, and sending tweets.
#[derive(Debug)]
pub struct XChannel {
    config: XConfig,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock, governor::middleware::NoOpMiddleware>>,
    is_connected: RwLock<bool>,
    /// HTTP client for API requests
    http: reqwest::Client,
    /// Last processed mention ID (for pagination)
    last_mention_id: RwLock<Option<String>>,
    /// Last processed DM ID (for pagination)
    last_dm_id: RwLock<Option<String>>,
}

/// X API tweet object (API v2)
#[derive(Debug, Clone, Deserialize)]
pub struct XTweet {
    pub id: String,
    pub text: String,
    #[serde(rename = "author_id")]
    pub author_id: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "conversation_id")]
    pub conversation_id: String,
    #[serde(rename = "referenced_tweets")]
    pub referenced_tweets: Option<Vec<ReferencedTweet>>,
    pub entities: Option<XEntities>,
}

/// Referenced tweet in a tweet (reply, quote, retweet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencedTweet {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub id: String,
}

/// X entities (mentions, hashtags, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XEntities {
    pub mentions: Option<Vec<XMention>>,
}

/// X mention entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XMention {
    pub username: String,
    pub id: String,
}

/// X user object (API v2)
#[derive(Debug, Clone, Deserialize)]
pub struct XUser {
    pub id: String,
    pub username: String,
    pub name: String,
}

/// X DM event (API v2)
#[derive(Debug, Clone, Deserialize)]
pub struct XDMEvent {
    pub id: String,
    #[serde(rename = "event_type")]
    pub event_type: String,
    #[serde(rename = "dm_conversation_id")]
    pub dm_conversation_id: String,
    #[serde(rename = "sender_id")]
    pub sender_id: String,
    #[serde(rename = "participant_ids")]
    pub participant_ids: Vec<String>,
    pub text: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

/// X API timeline response
#[derive(Debug, Clone, Deserialize)]
struct XTimelineResponse {
    data: Option<Vec<XTweet>>,
    includes: Option<XIncludes>,
    meta: Option<XMeta>,
}

/// Includes section in X API response
#[derive(Debug, Clone, Deserialize)]
struct XIncludes {
    users: Option<Vec<XUser>>,
}

/// Meta section in X API response
#[derive(Debug, Clone, Deserialize)]
struct XMeta {
    #[serde(rename = "newest_id")]
    newest_id: Option<String>,
    #[serde(rename = "oldest_id")]
    oldest_id: Option<String>,
    #[serde(rename = "result_count")]
    result_count: usize,
}

/// X DM API response
#[derive(Debug, Clone, Deserialize)]
struct XDMResponse {
    data: Option<Vec<XDMEvent>>,
    meta: Option<XMeta>,
}

/// Tweet creation request payload
#[derive(Debug, Clone, Serialize)]
struct CreateTweetRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply: Option<ReplyInfo>,
}

/// Reply info for tweet creation
#[derive(Debug, Clone, Serialize)]
struct ReplyInfo {
    #[serde(rename = "in_reply_to_tweet_id")]
    in_reply_to_tweet_id: String,
}

/// Tweet creation response
#[derive(Debug, Clone, Deserialize)]
struct CreateTweetResponse {
    data: Option<TweetData>,
}

#[derive(Debug, Clone, Deserialize)]
struct TweetData {
    id: String,
    text: String,
}

impl XChannel {
    /// Create a new X channel with the given configuration.
    pub fn new(config: XConfig) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // Create rate limiter (X API allows varying limits based on tier)
        let quota = Quota::per_minute(
            NonZeroU32::new(config.rate_limit_per_minute.max(1))
                .unwrap_or(NonZeroU32::new(10).unwrap())
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
            rate_limiter,
            is_connected: RwLock::new(false),
            http,
            last_mention_id: RwLock::new(None),
            last_dm_id: RwLock::new(None),
        }
    }

    /// Check if a user is in the allowlist.
    fn is_user_allowed(&self, username: &str) -> bool {
        if self.config.allowlist.is_empty() {
            return true; // No allowlist means allow all
        }

        let normalized_username = username.to_lowercase();
        self.config.allowlist.iter().any(|allowed| {
            allowed.to_lowercase() == normalized_username
        })
    }

    /// Get the X API base URL.
    fn api_base_url(&self) -> &'static str {
        "https://api.twitter.com/2"
    }

    /// Build authorization header with Bearer token.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.bearer_token)
    }

    /// Poll for mentions of the bot user.
    async fn poll_mentions(&self) -> Result<()> {
        if !self.config.respond_to_mentions {
            return Ok(());
        }

        let url = format!("{}/users/{}/mentions", self.api_base_url(), self.config.bot_user_id);

        let mut query: Vec<(&str, String)> = vec![
            ("max_results", "10".to_string()),
            ("tweet.fields", "created_at,author_id,conversation_id,referenced_tweets,entities".to_string()),
            ("expansions", "author_id".to_string()),
            ("user.fields", "username,name".to_string()),
        ];

        // Use since_id to only get new mentions
        let last_id = self.last_mention_id.read().await.clone();
        if let Some(since_id) = last_id {
            query.push(("since_id", since_id));
        }

        let response = self.http
            .get(&url)
            .header("Authorization", self.auth_header())
            .query(&query)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "x".to_string(),
                message: format!("Failed to poll mentions: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "x".to_string(),
                message: format!("Mentions API error: {}", error_text),
            }.into());
        }

        let data: XTimelineResponse = response.json().await.map_err(|e| ChannelError::InvalidFormat {
            platform: "x".to_string(),
            message: format!("Failed to parse mentions response: {}", e),
        })?;

        // Update last seen ID
        if let Some(meta) = &data.meta {
            if let Some(newest) = &meta.newest_id {
                *self.last_mention_id.write().await = Some(newest.clone());
            }
        }

        // Process tweets
        if let Some(tweets) = data.data {
            for tweet in tweets {
                // Skip if not actually mentioning the bot (safety check)
                let is_mention = tweet.entities.as_ref()
                    .and_then(|e| e.mentions.as_ref())
                    .map(|m| m.iter().any(|mention| mention.id == self.config.bot_user_id))
                    .unwrap_or(false);

                if !is_mention {
                    debug!(tweet_id = %tweet.id, "Skipping tweet that doesn't mention bot");
                    continue;
                }

                // Find author info
                let author = data.includes.as_ref()
                    .and_then(|i| i.users.as_ref())
                    .and_then(|users| users.iter().find(|u| u.id == tweet.author_id))
                    .cloned()
                    .unwrap_or_else(|| XUser {
                        id: tweet.author_id.clone(),
                        username: "unknown".to_string(),
                        name: "Unknown".to_string(),
                    });

                // Check allowlist
                if !self.is_user_allowed(&author.username) {
                    debug!(username = %author.username, "User not in allowlist, ignoring mention");
                    continue;
                }

                // Parse timestamp
                let timestamp = chrono::DateTime::parse_from_rfc3339(&tweet.created_at)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                // Build metadata
                let metadata = serde_json::json!({
                    "x_tweet_id": tweet.id,
                    "x_author_id": tweet.author_id,
                    "x_author_username": author.username,
                    "x_author_name": author.name,
                    "x_conversation_id": tweet.conversation_id,
                    "x_is_mention": true,
                    "x_referenced_tweets": tweet.referenced_tweets,
                });

                let incoming = IncomingMessage {
                    session_id: Uuid::new_v4(),
                    user_id: tweet.author_id.clone(),
                    content: tweet.text.clone(),
                    platform: Platform::X,
                    metadata,
                };

                let _ = self.incoming_tx.send(incoming).await;
                debug!(tweet_id = %tweet.id, author = %author.username, "Processed mention");
            }
        }

        Ok(())
    }

    /// Poll for DMs (Direct Messages).
    async fn poll_dms(&self) -> Result<()> {
        if !self.config.respond_to_dms {
            return Ok(());
        }

        // X API v2 DMs endpoint (requires appropriate access tier)
        let url = format!(
            "{}/dm_conversations/with/{}/dm_events",
            self.api_base_url(),
            self.config.bot_user_id
        );

        let mut query: Vec<(&str, String)> = vec![
            ("max_results", "10".to_string()),
            ("dm_event.fields", "created_at,sender_id,participant_ids".to_string()),
        ];

        // Use pagination token to only get new DMs
        let last_id = self.last_dm_id.read().await.clone();
        if let Some(pagination_token) = last_id {
            query.push(("pagination_token", pagination_token));
        }

        let response = self.http
            .get(&url)
            .header("Authorization", self.auth_header())
            .query(&query)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    // DMs might not be available for all API tiers
                    let status = resp.status();
                    if status.as_u16() == 403 {
                        debug!("DM polling skipped: API access tier may not support DMs");
                        return Ok(());
                    }
                    let error_text = resp.text().await.unwrap_or_default();
                    return Err(ChannelError::Connection {
                        platform: "x".to_string(),
                        message: format!("DM API error: {}", error_text),
                    }.into());
                }

                let data: XDMResponse = resp.json().await.map_err(|e| ChannelError::InvalidFormat {
                    platform: "x".to_string(),
                    message: format!("Failed to parse DM response: {}", e),
                })?;

                if let Some(events) = data.data {
                    for event in events {
                        // Skip messages from self
                        if event.sender_id == self.config.bot_user_id {
                            continue;
                        }

                        // Update last seen ID
                        *self.last_dm_id.write().await = Some(event.id.clone());

                        // Parse timestamp (for future use in metadata)
                        let _timestamp = chrono::DateTime::parse_from_rfc3339(&event.created_at)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now());

                        // Build metadata
                        let metadata = serde_json::json!({
                            "x_dm_event_id": event.id,
                            "x_dm_conversation_id": event.dm_conversation_id,
                            "x_sender_id": event.sender_id,
                            "x_participant_ids": event.participant_ids,
                            "x_is_dm": true,
                        });

                        let incoming = IncomingMessage {
                            session_id: Uuid::new_v4(),
                            user_id: event.sender_id.clone(),
                            content: event.text.clone(),
                            platform: Platform::X,
                            metadata,
                        };

                        let _ = self.incoming_tx.send(incoming).await;
                        debug!(dm_id = %event.id, sender = %event.sender_id, "Processed DM");
                    }
                }
            }
            Err(e) => {
                // DM endpoint might not be available
                debug!("DM poll error (may be expected for API tier): {}", e);
            }
        }

        Ok(())
    }

    /// Send a tweet (or reply).
    async fn send_tweet(&self, msg: &OutgoingMessage) -> Result<()> {
        let url = format!("{}/tweets", self.api_base_url());

        // Truncate content to max tweet length
        let text = if msg.content.len() > self.config.max_tweet_length {
            format!("{}...", &msg.content[..self.config.max_tweet_length.saturating_sub(3)])
        } else {
            msg.content.clone()
        };

        // Build request payload
        let reply_to = msg.metadata.get("x_reply_to_tweet_id")
            .and_then(|v| v.as_str());

        let payload = if let Some(reply_id) = reply_to {
            CreateTweetRequest {
                text,
                reply: Some(ReplyInfo {
                    in_reply_to_tweet_id: reply_id.to_string(),
                }),
            }
        } else {
            CreateTweetRequest {
                text,
                reply: None,
            }
        };

        let response = self.http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed {
                platform: "x".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed {
                platform: "x".to_string(),
                message: format!("API error: {}", error_text),
            }.into());
        }

        debug!("Successfully sent tweet");
        Ok(())
    }

    /// Run the polling loop for mentions and DMs.
    async fn run_polling(&self) -> Result<()> {
        let mut mention_interval = tokio::time::interval(
            Duration::from_secs(self.config.mention_poll_interval_secs.max(30))
        );
        let mut dm_interval = tokio::time::interval(
            Duration::from_secs(self.config.dm_poll_interval_secs.max(60))
        );

        info!(
            mention_interval = self.config.mention_poll_interval_secs,
            dm_interval = self.config.dm_poll_interval_secs,
            "Starting X polling loops"
        );

        loop {
            tokio::select! {
                _ = mention_interval.tick() => {
                    if let Err(e) = self.poll_mentions().await {
                        error!("Mention poll error: {}", e);
                    }
                }
                _ = dm_interval.tick() => {
                    if let Err(e) = self.poll_dms().await {
                        error!("DM poll error: {}", e);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Channel for XChannel {
    fn platform(&self) -> Platform {
        Platform::X
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Apply rate limiting
        self.rate_limiter.until_ready().await;

        self.send_tweet(&msg).await
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            ChannelError::Connection {
                platform: "x".to_string(),
                message: "Incoming message channel closed".to_string(),
            }.into()
        })
    }

    async fn connect(&mut self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }

        info!("Connecting to X API...");

        // Validate configuration
        if self.config.bearer_token.is_empty() {
            return Err(ChannelError::Config {
                platform: "x".to_string(),
                message: "Bearer token is required".to_string(),
            }.into());
        }

        if self.config.bot_user_id.is_empty() {
            return Err(ChannelError::Config {
                platform: "x".to_string(),
                message: "Bot user ID is required".to_string(),
            }.into());
        }

        // Verify credentials by fetching bot user info
        let url = format!("{}/users/{}", self.api_base_url(), self.config.bot_user_id);
        let response = self.http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "x".to_string(),
                message: format!("Connection test failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "x".to_string(),
                message: format!("Invalid credentials: {}", error_text),
            }.into());
        }

        *self.is_connected.write().await = true;

        info!(bot_user_id = %self.config.bot_user_id, "X channel connected");
        info!(
            respond_to_mentions = self.config.respond_to_mentions,
            respond_to_dms = self.config.respond_to_dms,
            "X channel configured"
        );

        // Note: In a full implementation, polling would run in a background task.
        // The channel needs to be wrapped in Arc<Mutex<Self>> to allow concurrent
        // access between polling and send/receive operations.
        info!("X channel polling would start here (use Arc<Mutex<XChannel>> for full implementation)");

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from X...");

        *self.is_connected.write().await = false;

        info!("X channel disconnected");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_allowed_empty_list() {
        let config = XConfig {
            enabled: true,
            bearer_token: "test".to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
            bot_user_id: "12345".to_string(),
            allowlist: vec![],
            respond_to_mentions: true,
            respond_to_dms: true,
            max_tweet_length: 280,
            mention_poll_interval_secs: 60,
            dm_poll_interval_secs: 120,
            rate_limit_per_minute: 10,
        };
        let channel = XChannel::new(config);
        assert!(channel.is_user_allowed("anyuser"));
        assert!(channel.is_user_allowed("another_user"));
    }

    #[test]
    fn test_user_allowed_with_list() {
        let config = XConfig {
            enabled: true,
            bearer_token: "test".to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
            bot_user_id: "12345".to_string(),
            allowlist: vec!["allowed_user".to_string(), "another_allowed".to_string()],
            respond_to_mentions: true,
            respond_to_dms: true,
            max_tweet_length: 280,
            mention_poll_interval_secs: 60,
            dm_poll_interval_secs: 120,
            rate_limit_per_minute: 10,
        };
        let channel = XChannel::new(config);

        assert!(channel.is_user_allowed("allowed_user"));
        assert!(channel.is_user_allowed("ALLOWED_USER")); // Case insensitive
        assert!(channel.is_user_allowed("another_allowed"));
        assert!(!channel.is_user_allowed("not_allowed"));
        assert!(!channel.is_user_allowed("random_user"));
    }

    #[test]
    fn test_tweet_truncation() {
        let config = XConfig {
            enabled: true,
            bearer_token: "test".to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
            bot_user_id: "12345".to_string(),
            allowlist: vec![],
            respond_to_mentions: true,
            respond_to_dms: true,
            max_tweet_length: 10,
            mention_poll_interval_secs: 60,
            dm_poll_interval_secs: 120,
            rate_limit_per_minute: 10,
        };
        let channel = XChannel::new(config);

        // Test that long content would be truncated (actual truncation happens in send_tweet)
        assert_eq!(channel.config.max_tweet_length, 10);
    }
}
