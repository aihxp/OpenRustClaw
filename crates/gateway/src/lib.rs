//! WebSocket gateway and session routing for OpenRustClaw.

pub mod auth;
pub mod health;
pub mod metrics_endpoint;
pub mod rate_limit;
pub mod router;
pub mod server;
pub mod sessions;
pub mod webhooks;

pub use health::{HealthCheck, HealthCheckRegistry, HealthState, health_routes};
pub use metrics_endpoint::{MetricsState, install_metrics, metrics_routes};
pub use server::GatewayServer;
pub use sessions::SessionManager;
pub use webhooks::{
    AgentTarget, RateLimitConfig, WebhookAction, WebhookConfig, WebhookError, WebhookHandler,
    WebhookManager, WebhookPayload, WebhookSource, WebhookState, handlers,
};
