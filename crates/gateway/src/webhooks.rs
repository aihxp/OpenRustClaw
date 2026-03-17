//! Webhook System for External Integrations
//!
//! This module provides webhook handlers for external integrations like
//! GitHub, Gmail, Stripe, Slack, Discord, and Telegram.
//!
//! # Example
//! ```rust
//! use openrustclaw_gateway::webhooks::{WebhookManager, WebhookConfig, handlers};
//!
//! # async fn example() {
//! let config = WebhookConfig::default();
//! let mut manager = WebhookManager::new(config);
//!
//! // Register a GitHub webhook handler
//! manager.register(handlers::github(Some("my-secret".to_string())));
//!
//! // Create router
//! let router = manager.router();
//! # }
//! ```

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use chrono;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use openrustclaw_agent::routing::{AgentId, AgentRouter};
use openrustclaw_core::types::Message;

type HmacSha256 = Hmac<Sha256>;

/// Webhook manager
#[derive(Clone)]
pub struct WebhookManager {
    handlers: Arc<RwLock<HashMap<String, WebhookHandler>>>,
    config: WebhookConfig,
    agent_router: Option<Arc<AgentRouter>>,
}

/// Webhook configuration
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Base path for webhooks (e.g., "/webhooks")
    pub base_path: String,
    /// Header name for signature verification
    pub secret_header: String,
    /// Maximum body size in bytes
    pub max_body_size: usize,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            base_path: "/webhooks".to_string(),
            secret_header: "x-webhook-signature".to_string(),
            max_body_size: 1024 * 1024, // 1MB
            timeout_seconds: 30,
        }
    }
}

/// A webhook handler
#[derive(Clone, Debug)]
pub struct WebhookHandler {
    /// Path segment for this webhook (e.g., "github")
    pub path: String,
    /// Secret for signature verification
    pub secret: Option<String>,
    /// Supported webhook sources
    pub sources: Vec<WebhookSource>,
    /// Action to take when webhook fires
    pub action: WebhookAction,
    /// Rate limit configuration
    pub rate_limit: Option<RateLimitConfig>,
    /// Whether the webhook is enabled
    pub enabled: bool,
}

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit config
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }
}

/// Supported webhook sources
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookSource {
    /// Generic webhook (no specific source)
    Generic,
    /// GitHub webhook
    GitHub,
    /// GitLab webhook
    GitLab,
    /// Stripe webhook
    Stripe,
    /// Slack webhook
    Slack,
    /// Discord webhook
    Discord,
    /// Gmail Pub/Sub webhook
    Gmail,
    /// Telegram webhook
    Telegram,
    /// Custom webhook source
    Custom { name: String },
}

impl std::fmt::Display for WebhookSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookSource::Generic => write!(f, "generic"),
            WebhookSource::GitHub => write!(f, "github"),
            WebhookSource::GitLab => write!(f, "gitlab"),
            WebhookSource::Stripe => write!(f, "stripe"),
            WebhookSource::Slack => write!(f, "slack"),
            WebhookSource::Discord => write!(f, "discord"),
            WebhookSource::Gmail => write!(f, "gmail"),
            WebhookSource::Telegram => write!(f, "telegram"),
            WebhookSource::Custom { name } => write!(f, "custom:{}", name),
        }
    }
}

/// Target agent for webhook messages
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTarget {
    /// Main/default agent
    Main,
    /// Specific agent by ID
    Agent(String),
    /// Route based on content
    Router,
}

impl AgentTarget {
    /// Convert to AgentId
    pub fn to_agent_id(&self) -> AgentId {
        match self {
            AgentTarget::Main => AgentId::new("main"),
            AgentTarget::Agent(id) => AgentId::new(id.clone()),
            AgentTarget::Router => AgentId::new("router"),
        }
    }
}

/// Action to take when webhook fires
#[derive(Clone)]
pub enum WebhookAction {
    /// Forward to agent as a message
    AgentMessage {
        /// Target agent
        target: AgentTarget,
        /// Template for message (uses {{field.subfield}} syntax)
        template: String,
    },
    /// Trigger a skill
    TriggerSkill {
        /// Name of the skill to trigger
        skill_name: String,
        /// Template for skill input
        input_template: String,
    },
    /// Run custom handler
    Custom {
        /// Custom handler function
        #[allow(clippy::type_complexity)]
        handler: Arc<
            dyn Fn(WebhookPayload) -> Pin<Box<dyn Future<Output = Result<(), WebhookError>> + Send>>
                + Send
                + Sync,
        >,
    },
    /// Emit event to event bus
    EmitEvent {
        /// Event type string
        event_type: String,
    },
}

impl std::fmt::Debug for WebhookAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookAction::AgentMessage { target, template } => f
                .debug_struct("AgentMessage")
                .field("target", target)
                .field("template", template)
                .finish(),
            WebhookAction::TriggerSkill {
                skill_name,
                input_template,
            } => f
                .debug_struct("TriggerSkill")
                .field("skill_name", skill_name)
                .field("input_template", input_template)
                .finish(),
            WebhookAction::Custom { .. } => f.debug_struct("Custom").finish_non_exhaustive(),
            WebhookAction::EmitEvent { event_type } => f
                .debug_struct("EmitEvent")
                .field("event_type", event_type)
                .finish(),
        }
    }
}

/// Webhook payload
#[derive(Debug, Clone)]
pub struct WebhookPayload {
    /// Detected source of the webhook
    pub source: WebhookSource,
    /// Webhook path
    pub path: String,
    /// HTTP headers
    pub headers: HeaderMap,
    /// Request body
    pub body: Bytes,
    /// Timestamp when received
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Parsed JSON body (if valid)
    pub json_body: Option<serde_json::Value>,
}

impl WebhookPayload {
    /// Get a field from the JSON body using dot notation
    pub fn get(&self, path: &str) -> Option<&serde_json::Value> {
        let json = self.json_body.as_ref()?;
        path.split('.').try_fold(json, |acc, key| acc.get(key))
    }

    /// Get string value from path
    pub fn get_str(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(|v| v.as_str())
    }
}

/// Rate limit tracker
#[derive(Debug)]
pub struct RateLimitTracker {
    requests: Vec<std::time::Instant>,
    config: RateLimitConfig,
}

impl RateLimitTracker {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: Vec::new(),
            config,
        }
    }

    fn check(&mut self) -> bool {
        let now = std::time::Instant::now();
        let window = self.config.window;

        // Remove expired requests
        self.requests.retain(|t| now.duration_since(*t) < window);

        // Check if under limit
        if self.requests.len() >= self.config.max_requests as usize {
            false
        } else {
            self.requests.push(now);
            true
        }
    }
}

/// Shared rate limit state
type RateLimitState = Arc<RwLock<HashMap<String, RateLimitTracker>>>;

impl WebhookManager {
    /// Create a new webhook manager
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            config,
            agent_router: None,
        }
    }

    /// Create a new webhook manager with agent router
    pub fn with_agent_router(config: WebhookConfig, agent_router: Arc<AgentRouter>) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            config,
            agent_router: Some(agent_router),
        }
    }

    /// Register a webhook handler
    pub async fn register(&self, handler: WebhookHandler) {
        let mut handlers = self.handlers.write().await;
        info!(path = %handler.path, sources = ?handler.sources, "Registering webhook handler");
        handlers.insert(handler.path.clone(), handler);
    }

    /// Unregister a webhook handler
    pub async fn unregister(&self, path: &str) -> Option<WebhookHandler> {
        let mut handlers = self.handlers.write().await;
        info!(path = %path, "Unregistering webhook handler");
        handlers.remove(path)
    }

    /// Get a webhook handler
    pub async fn get_handler(&self, path: &str) -> Option<WebhookHandler> {
        let handlers = self.handlers.read().await;
        handlers.get(path).cloned()
    }

    /// List all registered handlers
    pub async fn list_handlers(&self) -> Vec<WebhookHandler> {
        let handlers = self.handlers.read().await;
        handlers.values().cloned().collect()
    }

    /// Create Axum router for webhooks
    pub fn router(&self) -> Router<WebhookState> {
        let rate_limits: RateLimitState = Arc::new(RwLock::new(HashMap::new()));

        Router::new()
            .route("/{*path}", post(handle_webhook))
            .with_state(WebhookState {
                manager: self.clone(),
                rate_limits,
            })
    }

    /// Handle incoming webhook
    pub async fn handle(
        &self,
        path: &str,
        headers: HeaderMap,
        body: Bytes,
        rate_limits: &RateLimitState,
    ) -> Result<impl IntoResponse, WebhookError> {
        // Check body size
        if body.len() > self.config.max_body_size {
            return Err(WebhookError::BodyTooLarge);
        }

        // Get handler
        let handler = {
            let handlers = self.handlers.read().await;
            handlers
                .get(path)
                .cloned()
                .ok_or_else(|| WebhookError::UnknownPath(path.to_string()))?
        };

        if !handler.enabled {
            return Err(WebhookError::Disabled);
        }

        // Check rate limit
        if let Some(rl_config) = &handler.rate_limit {
            let mut rate_limits = rate_limits.write().await;
            let tracker = rate_limits
                .entry(path.to_string())
                .or_insert_with(|| RateLimitTracker::new(rl_config.clone()));

            if !tracker.check() {
                warn!(path = %path, "Webhook rate limit exceeded");
                return Err(WebhookError::RateLimited);
            }
        }

        // Verify signature if secret is configured
        if let Some(secret) = &handler.secret {
            self.verify_signature(&headers, &body, secret, &handler.sources)?;
        }

        // Detect source and parse payload
        let source = self.detect_source(&headers);
        let json_body = serde_json::from_slice(&body).ok();

        let payload = WebhookPayload {
            source,
            path: path.to_string(),
            headers: headers.clone(),
            body: body.clone(),
            timestamp: chrono::Utc::now(),
            json_body,
        };

        debug!(path = %path, source = %payload.source, "Processing webhook");

        // Execute action
        match &handler.action {
            WebhookAction::AgentMessage { target, template } => {
                let message = self.render_template(template, &payload)?;
                self.send_to_agent(target, &message).await?;
            }
            WebhookAction::TriggerSkill {
                skill_name,
                input_template,
            } => {
                let input = self.render_template(input_template, &payload)?;
                self.trigger_skill(skill_name, &input).await?;
            }
            WebhookAction::Custom { handler } => {
                handler(payload).await?;
            }
            WebhookAction::EmitEvent { event_type } => {
                self.emit_event(event_type, &payload).await?;
            }
        }

        Ok((StatusCode::OK, "OK"))
    }

    /// Verify webhook signature
    fn verify_signature(
        &self,
        headers: &HeaderMap,
        body: &Bytes,
        secret: &str,
        sources: &[WebhookSource],
    ) -> Result<(), WebhookError> {
        for source in sources {
            match source {
                WebhookSource::GitHub => {
                    if let Some(sig) = headers.get("x-hub-signature-256") {
                        let sig = sig.to_str().map_err(|_| WebhookError::InvalidSignature)?;
                        let sig = sig
                            .strip_prefix("sha256=")
                            .ok_or(WebhookError::InvalidSignature)?;

                        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                            .map_err(|_| WebhookError::InvalidSecret)?;
                        mac.update(body);
                        let result = mac.finalize();
                        let expected = hex::encode(result.into_bytes());

                        if constant_time_eq::constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
                            return Ok(());
                        }
                    }
                }
                WebhookSource::Stripe => {
                    if let Some(sig_header) = headers.get("stripe-signature") {
                        let sig_str = sig_header
                            .to_str()
                            .map_err(|_| WebhookError::InvalidSignature)?;

                        // Parse Stripe signature header: t=timestamp,v1=signature
                        let mut timestamp_str = None;
                        let mut signature_hex = None;
                        for part in sig_str.split(',') {
                            if let Some(t) = part.strip_prefix("t=") {
                                timestamp_str = Some(t);
                            } else if let Some(v1) = part.strip_prefix("v1=") {
                                signature_hex = Some(v1);
                            }
                        }

                        let timestamp_str = timestamp_str.ok_or(WebhookError::InvalidSignature)?;
                        let signature_hex = signature_hex.ok_or(WebhookError::InvalidSignature)?;

                        // Replay protection: reject if timestamp is older than 300 seconds
                        let timestamp: i64 = timestamp_str
                            .parse()
                            .map_err(|_| WebhookError::InvalidSignature)?;
                        let now = chrono::Utc::now().timestamp();
                        if (now - timestamp).abs() > 300 {
                            debug!("Stripe webhook rejected: timestamp too old");
                            return Err(WebhookError::InvalidSignature);
                        }

                        // Reconstruct signed payload: "{timestamp}.{body}"
                        let signed_payload =
                            format!("{}.{}", timestamp_str, String::from_utf8_lossy(body));

                        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                            .map_err(|_| WebhookError::InvalidSecret)?;
                        mac.update(signed_payload.as_bytes());
                        let result = mac.finalize();
                        let expected = hex::encode(result.into_bytes());

                        if constant_time_eq::constant_time_eq(
                            signature_hex.as_bytes(),
                            expected.as_bytes(),
                        ) {
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }

        // Generic HMAC verification
        if let Some(sig) = headers.get(&self.config.secret_header) {
            let sig = sig.to_str().map_err(|_| WebhookError::InvalidSignature)?;

            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|_| WebhookError::InvalidSecret)?;
            mac.update(body);
            let result = mac.finalize();
            let expected = hex::encode(result.into_bytes());

            if constant_time_eq::constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
                return Ok(());
            }
        }

        Err(WebhookError::InvalidSignature)
    }

    /// Detect source from headers
    fn detect_source(&self, headers: &HeaderMap) -> WebhookSource {
        if headers.contains_key("x-github-event") {
            WebhookSource::GitHub
        } else if headers.contains_key("x-gitlab-event") {
            WebhookSource::GitLab
        } else if headers.contains_key("stripe-signature") {
            WebhookSource::Stripe
        } else if headers.contains_key("x-slack-signature") {
            WebhookSource::Slack
        } else if headers.contains_key("x-discord-signature") {
            WebhookSource::Discord
        } else {
            WebhookSource::Generic
        }
    }

    /// Render template with payload data
    fn render_template(
        &self,
        template: &str,
        payload: &WebhookPayload,
    ) -> Result<String, WebhookError> {
        // Simple template: {{field.subfield}}
        let body_json = payload
            .json_body
            .clone()
            .unwrap_or_else(|| serde_json::json!({"raw": String::from_utf8_lossy(&payload.body)}));

        let mut result = template.to_string();

        // Replace {{path.to.field}} with actual value
        let re = regex::Regex::new(r"\{\{(\w+(?:\.\w+)*)\}\}")
            .map_err(|e| WebhookError::TemplateError(e.to_string()))?;

        for cap in re.captures_iter(template) {
            let full = cap.get(0).unwrap().as_str();
            let path = cap.get(1).unwrap().as_str();

            let value = path.split('.').fold(&body_json, |acc, key| {
                acc.get(key).unwrap_or(&serde_json::Value::Null)
            });

            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            };

            result = result.replace(full, &replacement);
        }

        Ok(result)
    }

    async fn send_to_agent(&self, target: &AgentTarget, message: &str) -> Result<(), WebhookError> {
        if let Some(router) = &self.agent_router {
            let agent_id = target.to_agent_id();
            let msg = Message::user(message);

            // Find the agent and send message
            if let Some(agent) = router.get_agent(&agent_id) {
                agent
                    .sender
                    .send(msg)
                    .await
                    .map_err(|_| WebhookError::AgentError("Failed to send message".to_string()))?;
                debug!(agent = %agent_id, "Sent webhook message to agent");
            } else {
                return Err(WebhookError::AgentError(format!(
                    "Agent not found: {}",
                    agent_id
                )));
            }
        } else {
            warn!("No agent router configured, cannot send message");
        }
        Ok(())
    }

    async fn trigger_skill(&self, skill_name: &str, input: &str) -> Result<(), WebhookError> {
        // This would integrate with the skills system
        debug!(skill = %skill_name, input = %input, "Triggering skill");
        // Placeholder - actual implementation would call skill runner
        Ok(())
    }

    async fn emit_event(
        &self,
        event_type: &str,
        _payload: &WebhookPayload,
    ) -> Result<(), WebhookError> {
        // This would integrate with the event bus
        debug!(event_type = %event_type, "Emitting event");
        // Placeholder - actual implementation would emit to event bus
        Ok(())
    }
}

/// State for Axum webhook handler
#[derive(Clone)]
pub struct WebhookState {
    manager: WebhookManager,
    rate_limits: RateLimitState,
}

/// Axum handler
async fn handle_webhook(
    State(state): State<WebhookState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, String) {
    match state
        .manager
        .handle(&path, headers, body, &state.rate_limits)
        .await
    {
        Ok(_) => (StatusCode::OK, "OK".to_string()),
        Err(e) => {
            let status = match &e {
                WebhookError::UnknownPath(_) => StatusCode::NOT_FOUND,
                WebhookError::InvalidSignature => StatusCode::UNAUTHORIZED,
                WebhookError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
                WebhookError::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                WebhookError::Disabled => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            error!(error = %e, path = %path, "Webhook error");
            (status, e.to_string())
        }
    }
}

/// Pre-built handlers for common integrations
pub mod handlers {
    use super::*;

    /// GitHub webhook handler
    pub fn github(secret: Option<String>) -> WebhookHandler {
        WebhookHandler {
            path: "github".to_string(),
            secret,
            sources: vec![WebhookSource::GitHub],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "🔔 GitHub: {{action}} on {{repository.full_name}} by {{sender.login}}"
                    .to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(100, 60)),
            enabled: true,
        }
    }

    /// GitLab webhook handler
    pub fn gitlab(secret: Option<String>) -> WebhookHandler {
        WebhookHandler {
            path: "gitlab".to_string(),
            secret,
            sources: vec![WebhookSource::GitLab],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "🔔 GitLab: {{object_kind}} on {{project.name}} by {{user_name}}"
                    .to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(100, 60)),
            enabled: true,
        }
    }

    /// Gmail Pub/Sub handler
    pub fn gmail() -> WebhookHandler {
        WebhookHandler {
            path: "gmail".to_string(),
            secret: None,
            sources: vec![WebhookSource::Gmail],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "📧 New email notification received".to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(10, 60)),
            enabled: true,
        }
    }

    /// Stripe webhook handler
    pub fn stripe(secret: String) -> WebhookHandler {
        WebhookHandler {
            path: "stripe".to_string(),
            secret: Some(secret),
            sources: vec![WebhookSource::Stripe],
            action: WebhookAction::EmitEvent {
                event_type: "stripe.webhook".to_string(),
            },
            rate_limit: None,
            enabled: true,
        }
    }

    /// Slack webhook handler
    pub fn slack() -> WebhookHandler {
        WebhookHandler {
            path: "slack".to_string(),
            secret: None,
            sources: vec![WebhookSource::Slack],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "💬 Slack: {{text}}".to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(50, 60)),
            enabled: true,
        }
    }

    /// Discord webhook handler
    pub fn discord() -> WebhookHandler {
        WebhookHandler {
            path: "discord".to_string(),
            secret: None,
            sources: vec![WebhookSource::Discord],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "🎮 Discord message received".to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(50, 60)),
            enabled: true,
        }
    }

    /// Telegram webhook handler
    pub fn telegram(secret: Option<String>) -> WebhookHandler {
        WebhookHandler {
            path: "telegram".to_string(),
            secret,
            sources: vec![WebhookSource::Telegram],
            action: WebhookAction::AgentMessage {
                target: AgentTarget::Main,
                template: "📱 Telegram: {{message.text}}".to_string(),
            },
            rate_limit: Some(RateLimitConfig::new(50, 60)),
            enabled: true,
        }
    }

    /// Generic webhook handler
    pub fn generic(path: impl Into<String>) -> WebhookHandler {
        let path = path.into();
        WebhookHandler {
            path: path.clone(),
            secret: None,
            sources: vec![WebhookSource::Generic],
            action: WebhookAction::EmitEvent {
                event_type: format!("webhook.{}", path),
            },
            rate_limit: Some(RateLimitConfig::new(100, 60)),
            enabled: true,
        }
    }
}

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Unknown webhook path: {0}")]
    UnknownPath(String),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid secret")]
    InvalidSecret,
    #[error("Rate limited")]
    RateLimited,
    #[error("Webhook disabled")]
    Disabled,
    #[error("Template error: {0}")]
    TemplateError(String),
    #[error("Body too large")]
    BodyTooLarge,
    #[error("Agent error: {0}")]
    AgentError(String),
    #[error("Skill error: {0}")]
    SkillError(String),
    #[error("Event error: {0}")]
    EventError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_source_display() {
        assert_eq!(WebhookSource::GitHub.to_string(), "github");
        assert_eq!(WebhookSource::Generic.to_string(), "generic");
        assert_eq!(
            WebhookSource::Custom {
                name: "test".to_string()
            }
            .to_string(),
            "custom:test"
        );
    }

    #[test]
    fn test_agent_target_to_id() {
        assert_eq!(AgentTarget::Main.to_agent_id().0, "main");
        assert_eq!(AgentTarget::Agent("dev".to_string()).to_agent_id().0, "dev");
    }

    #[tokio::test]
    async fn test_webhook_manager_register_and_list() {
        let config = WebhookConfig::default();
        let manager = WebhookManager::new(config);

        let handler = handlers::github(None);
        manager.register(handler.clone()).await;

        let handlers = manager.list_handlers().await;
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].path, "github");
    }

    #[tokio::test]
    async fn test_webhook_manager_unregister() {
        let config = WebhookConfig::default();
        let manager = WebhookManager::new(config);

        let handler = handlers::github(None);
        manager.register(handler.clone()).await;

        let removed = manager.unregister("github").await;
        assert!(removed.is_some());

        let handlers = manager.list_handlers().await;
        assert!(handlers.is_empty());
    }

    #[test]
    fn test_webhook_payload_get() {
        let payload = WebhookPayload {
            source: WebhookSource::GitHub,
            path: "github".to_string(),
            headers: HeaderMap::new(),
            body: Bytes::new(),
            timestamp: chrono::Utc::now(),
            json_body: Some(serde_json::json!({
                "action": "opened",
                "repository": {
                    "full_name": "owner/repo"
                },
                "sender": {
                    "login": "testuser"
                }
            })),
        };

        assert_eq!(payload.get_str("action"), Some("opened"));
        assert_eq!(payload.get_str("repository.full_name"), Some("owner/repo"));
        assert_eq!(payload.get_str("sender.login"), Some("testuser"));
        assert_eq!(payload.get_str("nonexistent"), None);
    }

    #[tokio::test]
    async fn test_render_template() {
        let config = WebhookConfig::default();
        let manager = WebhookManager::new(config);

        let payload = WebhookPayload {
            source: WebhookSource::GitHub,
            path: "github".to_string(),
            headers: HeaderMap::new(),
            body: Bytes::new(),
            timestamp: chrono::Utc::now(),
            json_body: Some(serde_json::json!({
                "action": "opened",
                "repository": {
                    "full_name": "owner/repo"
                },
                "sender": {
                    "login": "testuser"
                }
            })),
        };

        let template = "Action: {{action}}, Repo: {{repository.full_name}}, User: {{sender.login}}";
        let result = manager.render_template(template, &payload).unwrap();

        assert_eq!(result, "Action: opened, Repo: owner/repo, User: testuser");
    }

    #[test]
    fn test_rate_limit_tracker() {
        let config = RateLimitConfig::new(2, 60);
        let mut tracker = RateLimitTracker::new(config);

        assert!(tracker.check());
        assert!(tracker.check());
        assert!(!tracker.check()); // Exceeded
    }

    #[test]
    fn test_detect_source() {
        let config = WebhookConfig::default();
        let manager = WebhookManager::new(config);

        let mut headers = HeaderMap::new();
        headers.insert("x-github-event", "push".parse().unwrap());
        assert_eq!(manager.detect_source(&headers), WebhookSource::GitHub);

        let mut headers = HeaderMap::new();
        headers.insert("x-gitlab-event", "push".parse().unwrap());
        assert_eq!(manager.detect_source(&headers), WebhookSource::GitLab);

        let mut headers = HeaderMap::new();
        headers.insert("stripe-signature", "test".parse().unwrap());
        assert_eq!(manager.detect_source(&headers), WebhookSource::Stripe);

        let headers = HeaderMap::new();
        assert_eq!(manager.detect_source(&headers), WebhookSource::Generic);
    }

    #[test]
    fn test_webhook_config_default() {
        let config = WebhookConfig::default();
        assert_eq!(config.base_path, "/webhooks");
        assert_eq!(config.secret_header, "x-webhook-signature");
        assert_eq!(config.max_body_size, 1024 * 1024);
        assert_eq!(config.timeout_seconds, 30);
    }
}
