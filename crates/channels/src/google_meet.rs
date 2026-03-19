//! Google Meet operator integration for spaces, conference records, artifacts, and events.

use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use governor::{Quota, RateLimiter};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use openrustclaw_core::config::GoogleMeetConfig;
use openrustclaw_core::error::{ChannelError, Result};

const DEFAULT_GOOGLE_MEET_API_BASE: &str = "https://meet.googleapis.com/v2";
const DEFAULT_GOOGLE_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_GOOGLE_MEET_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/meetings.space.created",
    "https://www.googleapis.com/auth/meetings.space.readonly",
    "https://www.googleapis.com/auth/meetings.space.settings",
];

#[derive(Debug, Clone)]
pub struct GoogleMeetClient {
    config: GoogleMeetConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedGoogleMeetEvent {
    pub event_type: String,
    pub conference_record: Option<String>,
    pub transcript: Option<String>,
    pub recording: Option<String>,
    pub participant: Option<String>,
    pub space: Option<String>,
    #[serde(default)]
    pub transcript_text: Option<String>,
    pub raw: Value,
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
struct TokenResponse {
    access_token: String,
}

pub struct GoogleMeetWebhookHandler {
    client: GoogleMeetClient,
}

impl GoogleMeetClient {
    pub fn new(config: GoogleMeetConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit_requests_per_second.max(1))
                .unwrap_or(NonZeroU32::new(5).expect("non-zero")),
        );

        Self {
            config,
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
            is_connected: Arc::new(RwLock::new(false)),
            access_token: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn webhook_handler(&self) -> GoogleMeetWebhookHandler {
        GoogleMeetWebhookHandler {
            client: self.clone(),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        if *self.is_connected.read().await {
            return Ok(());
        }
        if self.config.service_account_key_path.is_empty() {
            return Err(ChannelError::Config {
                platform: "google_meet".to_string(),
                message: "service_account_key_path is required".to_string(),
            }
            .into());
        }
        if self.config.delegated_user_email.is_empty() {
            return Err(ChannelError::Config {
                platform: "google_meet".to_string(),
                message: "delegated_user_email is required".to_string(),
            }
            .into());
        }

        let token = self.authenticate().await?;
        *self.access_token.write().await = Some(token);
        *self.is_connected.write().await = true;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.is_connected.write().await = false;
        *self.access_token.write().await = None;
        Ok(())
    }

    pub async fn create_space(&self) -> Result<Value> {
        self.post_json("spaces", &serde_json::json!({})).await
    }

    pub async fn get_space(&self, name: &str) -> Result<Value> {
        self.get_json(name).await
    }

    pub async fn end_active_conference(&self, space_name: &str) -> Result<Value> {
        self.post_json(
            &format!("{}:endActiveConference", space_name),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn list_conference_records(&self, page_size: usize) -> Result<Value> {
        self.get_json_with_query(
            "conferenceRecords",
            &[("pageSize", page_size.max(1).min(200).to_string())],
        )
        .await
    }

    pub async fn list_participants(
        &self,
        conference_record: &str,
        page_size: usize,
    ) -> Result<Value> {
        self.get_json_with_query(
            &format!("{}/participants", conference_record),
            &[("pageSize", page_size.max(1).min(200).to_string())],
        )
        .await
    }

    pub async fn list_recordings(
        &self,
        conference_record: &str,
        page_size: usize,
    ) -> Result<Value> {
        self.get_json_with_query(
            &format!("{}/recordings", conference_record),
            &[("pageSize", page_size.max(1).min(200).to_string())],
        )
        .await
    }

    pub async fn list_transcripts(
        &self,
        conference_record: &str,
        page_size: usize,
    ) -> Result<Value> {
        self.get_json_with_query(
            &format!("{}/transcripts", conference_record),
            &[("pageSize", page_size.max(1).min(200).to_string())],
        )
        .await
    }

    pub async fn list_transcript_entries(
        &self,
        transcript_name: &str,
        page_size: usize,
    ) -> Result<Value> {
        self.get_json_with_query(
            &format!("{}/entries", transcript_name),
            &[("pageSize", page_size.max(1).min(1000).to_string())],
        )
        .await
    }

    pub async fn hydrate_transcript_text(
        &self,
        transcript_name: &str,
        page_size: usize,
    ) -> Result<String> {
        let payload = self
            .list_transcript_entries(transcript_name, page_size)
            .await?;
        let text = payload
            .get("transcriptEntries")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(extract_transcript_entry_text)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Ok(text)
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
                    platform: "google_meet".to_string(),
                    message: format!(
                        "Google Meet access token env var '{}' is not set",
                        var_name.trim()
                    ),
                }
                .into()
            });
        }

        let key_data = tokio::fs::read_to_string(&self.config.service_account_key_path)
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: format!("Failed to read Google Meet service account key: {}", e),
            })?;

        if let Ok(value) = serde_json::from_str::<Value>(&key_data)
            && let Some(token) = value.get("access_token").and_then(|field| field.as_str())
        {
            return Ok(token.to_string());
        }

        let key: ServiceAccountKey =
            serde_json::from_str(&key_data).map_err(|e| ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: format!("Invalid Google Meet service account JSON: {}", e),
            })?;

        let issued_at = Utc::now().timestamp();
        let claims = ServiceAccountClaims {
            iss: key.client_email.clone(),
            scope: self.scopes().join(" "),
            aud: self.token_url(&key),
            exp: issued_at + 3600,
            iat: issued_at,
            sub: self.config.delegated_user_email.clone(),
        };

        let private_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes()).map_err(|e| {
            ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: format!("Invalid Google Meet private key: {}", e),
            }
        })?;

        let assertion =
            encode(&Header::new(Algorithm::RS256), &claims, &private_key).map_err(|e| {
                ChannelError::AuthFailed {
                    platform: "google_meet".to_string(),
                    message: format!("Failed to sign Google Meet service account JWT: {}", e),
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
                platform: "google_meet".to_string(),
                message: format!("Failed to exchange Google Meet service account JWT: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: format!("Google Meet OAuth exchange failed: {}", body),
            }
            .into());
        }

        let body: TokenResponse = response
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: format!("Failed to parse Google Meet OAuth response: {}", e),
            })?;

        Ok(body.access_token)
    }

    fn scopes(&self) -> Vec<String> {
        let mut scopes = BTreeSet::new();
        for scope in DEFAULT_GOOGLE_MEET_SCOPES {
            scopes.insert((*scope).to_string());
        }
        for scope in &self.config.additional_scopes {
            if !scope.trim().is_empty() {
                scopes.insert(scope.trim().to_string());
            }
        }
        scopes.into_iter().collect()
    }

    fn api_base(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_GOOGLE_MEET_API_BASE.to_string())
            .trim_end_matches('/')
            .to_string()
    }

    fn token_url(&self, key: &ServiceAccountKey) -> String {
        self.config
            .oauth_token_url
            .clone()
            .unwrap_or_else(|| key.token_uri.clone())
    }

    async fn require_access_token(&self) -> Result<String> {
        if !*self.is_connected.read().await {
            return Err(ChannelError::NotConnected {
                platform: "google_meet".to_string(),
            }
            .into());
        }
        self.access_token.read().await.clone().ok_or_else(|| {
            ChannelError::AuthFailed {
                platform: "google_meet".to_string(),
                message: "Missing access token for Google Meet API".to_string(),
            }
            .into()
        })
    }

    async fn get_json(&self, path: &str) -> Result<Value> {
        self.get_json_with_query(path, &[]).await
    }

    async fn get_json_with_query(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.rate_limiter.until_ready().await;
        let token = self.require_access_token().await?;
        let response = self
            .http_client
            .get(format!(
                "{}/{}",
                self.api_base(),
                path.trim_start_matches('/')
            ))
            .bearer_auth(token)
            .query(query)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Google Meet GET request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Google Meet GET request failed: {}", body),
            }
            .into());
        }

        response.json().await.map_err(|e| {
            ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Failed to parse Google Meet GET response: {}", e),
            }
            .into()
        })
    }

    async fn post_json(&self, path: &str, payload: &Value) -> Result<Value> {
        self.rate_limiter.until_ready().await;
        let token = self.require_access_token().await?;
        let response = self
            .http_client
            .post(format!(
                "{}/{}",
                self.api_base(),
                path.trim_start_matches('/')
            ))
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .map_err(|e| ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Google Meet POST request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Google Meet POST request failed: {}", body),
            }
            .into());
        }

        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::json!({}));
        }

        response.json().await.map_err(|e| {
            ChannelError::Connection {
                platform: "google_meet".to_string(),
                message: format!("Failed to parse Google Meet POST response: {}", e),
            }
            .into()
        })
    }
}

impl GoogleMeetWebhookHandler {
    async fn decode_value(&self, raw: Value) -> Result<DecodedGoogleMeetEvent> {
        let transcript = find_named_resource(&raw, "conferenceRecords/", "/transcripts/");
        let conference_record = find_string_prefix(&raw, "conferenceRecords/")
            .filter(|value| !value.contains("/transcripts/"));
        let recording = find_named_resource(&raw, "conferenceRecords/", "/recordings/");
        let participant = find_named_resource(&raw, "conferenceRecords/", "/participants/");
        let space = find_string_prefix(&raw, "spaces/");
        let event_type =
            find_event_type(&raw).unwrap_or_else(|| "google.workspace.meet.unknown".to_string());

        if let Some(ref allowed) = space
            && !self.client.config.allowed_spaces.is_empty()
            && !self
                .client
                .config
                .allowed_spaces
                .iter()
                .any(|value| value == allowed)
        {
            return Err(ChannelError::InvalidFormat {
                platform: "google_meet".to_string(),
                message: format!("Google Meet event for non-allowed space '{}'", allowed),
            }
            .into());
        }

        let transcript_text = if self.client.config.hydrate_transcript_events {
            if let Some(transcript_name) = transcript.as_deref() {
                let _ = self.client.connect().await;
                self.client
                    .hydrate_transcript_text(transcript_name, 200)
                    .await
                    .ok()
                    .filter(|text| !text.trim().is_empty())
            } else {
                None
            }
        } else {
            None
        };

        Ok(DecodedGoogleMeetEvent {
            event_type,
            conference_record,
            transcript,
            recording,
            participant,
            space,
            transcript_text,
            raw,
        })
    }

    pub async fn decode_push(&self, body: &[u8]) -> Result<DecodedGoogleMeetEvent> {
        let envelope: Value =
            serde_json::from_slice(body).map_err(|e| ChannelError::InvalidFormat {
                platform: "google_meet".to_string(),
                message: format!("Failed to parse Google Meet Pub/Sub envelope: {}", e),
            })?;

        if let Some(data) = envelope
            .get("message")
            .and_then(|message| message.get("data"))
            .and_then(Value::as_str)
        {
            let decoded = STANDARD
                .decode(data)
                .map_err(|e| ChannelError::InvalidFormat {
                    platform: "google_meet".to_string(),
                    message: format!("Failed to decode Google Meet Pub/Sub payload: {}", e),
                })?;

            let raw: Value =
                serde_json::from_slice(&decoded).map_err(|e| ChannelError::InvalidFormat {
                    platform: "google_meet".to_string(),
                    message: format!("Failed to parse Google Meet event payload: {}", e),
                })?;

            self.decode_value(raw).await
        } else {
            self.decode_value(envelope).await
        }
    }
}

fn find_event_type(value: &Value) -> Option<String> {
    for key in ["eventType", "event_type", "ce-type", "type"] {
        if let Some(found) = value.get(key).and_then(Value::as_str)
            && !found.trim().is_empty()
        {
            return Some(found.to_string());
        }
    }
    match value {
        Value::Object(map) => map.values().find_map(find_event_type),
        Value::Array(values) => values.iter().find_map(find_event_type),
        _ => None,
    }
}

fn find_string_prefix(value: &Value, prefix: &str) -> Option<String> {
    match value {
        Value::String(string) if string.starts_with(prefix) => Some(string.clone()),
        Value::Object(map) => map
            .values()
            .find_map(|item| find_string_prefix(item, prefix)),
        Value::Array(values) => values
            .iter()
            .find_map(|item| find_string_prefix(item, prefix)),
        _ => None,
    }
}

fn find_named_resource(value: &Value, prefix: &str, contains: &str) -> Option<String> {
    match value {
        Value::String(string) if string.starts_with(prefix) && string.contains(contains) => {
            Some(string.clone())
        }
        Value::Object(map) => map
            .values()
            .find_map(|item| find_named_resource(item, prefix, contains)),
        Value::Array(values) => values
            .iter()
            .find_map(|item| find_named_resource(item, prefix, contains)),
        _ => None,
    }
}

fn extract_transcript_entry_text(entry: &Value) -> Option<String> {
    for key in ["text", "transcriptText", "body"] {
        if let Some(value) = entry.get(key) {
            if let Some(text) = value.as_str() {
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn meet_config(server: &MockServer) -> GoogleMeetConfig {
        GoogleMeetConfig {
            enabled: true,
            service_account_key_path: "token:test-token".to_string(),
            delegated_user_email: "user@example.com".to_string(),
            webhook_path: "/webhooks/google-meet/events".to_string(),
            allowed_spaces: Vec::new(),
            rate_limit_requests_per_second: 5,
            api_base_url: Some(format!("{}/v2", server.uri())),
            oauth_token_url: Some(format!("{}/token", server.uri())),
            additional_scopes: Vec::new(),
            hydrate_transcript_events: true,
        }
    }

    #[tokio::test]
    async fn create_space_and_list_records() {
        let server = MockServer::start().await;
        let client = GoogleMeetClient::new(meet_config(&server));

        Mock::given(method("POST"))
            .and(path("/v2/spaces"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "name": "spaces/abc",
                "meetingUri": "https://meet.google.com/abc-defg-hij"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/conferenceRecords"))
            .and(query_param("pageSize", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "conferenceRecords": [{
                    "name": "conferenceRecords/123"
                }]
            })))
            .mount(&server)
            .await;

        client.connect().await.unwrap();
        let space = client.create_space().await.unwrap();
        assert_eq!(space["name"], "spaces/abc");
        let records = client.list_conference_records(20).await.unwrap();
        assert_eq!(
            records["conferenceRecords"][0]["name"],
            "conferenceRecords/123"
        );
    }

    #[tokio::test]
    async fn hydrate_transcript_from_push_event() {
        let server = MockServer::start().await;
        let client = GoogleMeetClient::new(meet_config(&server));
        client.connect().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/v2/conferenceRecords/123/transcripts/456/entries"))
            .and(query_param("pageSize", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transcriptEntries": [
                    { "text": "Hello team" },
                    { "body": { "text": "Action items next" } }
                ]
            })))
            .mount(&server)
            .await;

        let handler = client.webhook_handler();
        let payload = STANDARD.encode(
            serde_json::json!({
                "eventType": "google.workspace.meet.transcript.v2.fileGenerated",
                "transcript": {
                    "name": "conferenceRecords/123/transcripts/456"
                },
                "space": {
                    "name": "spaces/abc"
                }
            })
            .to_string(),
        );
        let event = handler
            .decode_push(
                serde_json::json!({
                    "message": { "data": payload }
                })
                .to_string()
                .as_bytes(),
            )
            .await
            .unwrap();

        assert_eq!(
            event.event_type,
            "google.workspace.meet.transcript.v2.fileGenerated"
        );
        assert_eq!(
            event.transcript.as_deref(),
            Some("conferenceRecords/123/transcripts/456")
        );
        assert_eq!(
            event.transcript_text.as_deref(),
            Some("Hello team\nAction items next")
        );
    }

    #[tokio::test]
    async fn decode_direct_event_payload() {
        let server = MockServer::start().await;
        let client = GoogleMeetClient::new(meet_config(&server));
        client.connect().await.unwrap();

        let handler = client.webhook_handler();
        let event = handler
            .decode_push(
                serde_json::json!({
                    "eventType": "google.workspace.meet.conference.v2.started",
                    "conferenceRecord": {
                        "name": "conferenceRecords/987"
                    },
                    "space": {
                        "name": "spaces/xyz"
                    }
                })
                .to_string()
                .as_bytes(),
            )
            .await
            .unwrap();

        assert_eq!(
            event.event_type,
            "google.workspace.meet.conference.v2.started"
        );
        assert_eq!(event.conference_record.as_deref(), Some("conferenceRecords/987"));
        assert_eq!(event.space.as_deref(), Some("spaces/xyz"));
    }

    #[tokio::test]
    async fn end_active_conference_posts_action() {
        let server = MockServer::start().await;
        let client = GoogleMeetClient::new(meet_config(&server));
        client.connect().await.unwrap();

        Mock::given(method("POST"))
            .and(path("/v2/spaces/abc:endActiveConference"))
            .and(body_string_contains("{}"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client.end_active_conference("spaces/abc").await.unwrap();
    }
}
