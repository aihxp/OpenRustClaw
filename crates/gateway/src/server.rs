//! Axum WebSocket server.

use axum::{Router, routing::get, extract::State};
use tower_http::cors::CorsLayer;
use std::sync::Arc;

/// Shared gateway state.
#[derive(Clone)]
pub struct GatewayState {
    pub session_manager: Arc<crate::sessions::SessionManager>,
}

/// The gateway WebSocket server.
pub struct GatewayServer {
    host: String,
    port: u16,
}

impl GatewayServer {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    /// Build the Axum router.
    pub fn router(&self, state: GatewayState) -> Router {
        Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    /// Get the bind address.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

async fn ws_handler(State(_state): State<GatewayState>) -> &'static str {
    // TODO: Upgrade to WebSocket, validate origin, authenticate
    "WebSocket endpoint"
}

async fn health_handler() -> &'static str {
    "ok"
}
