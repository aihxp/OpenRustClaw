//! WebSocket gateway and session routing for OpenRustClaw.

pub mod auth;
pub mod health;
pub mod metrics_endpoint;
pub mod rate_limit;
pub mod router;
pub mod server;
pub mod sessions;
pub mod webhooks;

pub use health::{health_routes, HealthCheck, HealthCheckRegistry, HealthState};
pub use metrics_endpoint::{install_metrics, metrics_routes, MetricsState};
pub use server::GatewayServer;
pub use sessions::SessionManager;
pub use webhooks::{
    handlers, AgentTarget, RateLimitConfig, WebhookAction, WebhookConfig, WebhookError,
    WebhookHandler, WebhookManager, WebhookPayload, WebhookSource, WebhookState,
};
