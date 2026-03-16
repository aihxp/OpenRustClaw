//! Axum WebSocket server.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use futures::StreamExt;
use openrustclaw_security::OriginValidator;
use crate::auth::extract_token;
use openrustclaw_core::error::{Error, Result as CoreResult, SecurityError};
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

/// Shared gateway state.
#[derive(Clone)]
pub struct GatewayState {
    pub session_manager: Arc<crate::sessions::SessionManager>,
    pub origin_validator: Arc<OriginValidator>,
    pub require_auth: bool,
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
            .with_state(state)
    }

    /// Get the bind address.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Response {
    if let Err(err) = validate_ws_request(&state, &headers) {
        warn!(error = %err, "Rejected websocket connection");
        return gateway_error_response(err);
    }

    ws.on_upgrade(handle_socket)
}

async fn health_handler() -> &'static str {
    "ok"
}

fn validate_ws_request(state: &GatewayState, headers: &HeaderMap) -> CoreResult<()> {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| Error::Security(SecurityError::InvalidOrigin {
            origin: "<missing>".to_string(),
        }))?;
    state.origin_validator.validate(origin)?;

    if state.require_auth {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        extract_token(auth_header)?;
    }

    Ok(())
}

fn gateway_error_response(err: Error) -> Response {
    let (status, message) = match err {
        Error::Security(SecurityError::AuthRequired) => {
            (StatusCode::UNAUTHORIZED, "authorization required".to_string())
        }
        Error::Security(SecurityError::TokenInvalid(message)) => {
            (StatusCode::UNAUTHORIZED, message)
        }
        Error::Security(SecurityError::InvalidOrigin { origin }) => {
            (StatusCode::FORBIDDEN, format!("invalid origin: {origin}"))
        }
        other => (StatusCode::BAD_REQUEST, other.to_string()),
    };

    (status, message).into_response()
}

async fn handle_socket(mut socket: WebSocket) {
    let hello = json!({
        "type": "connected",
        "message": "WebSocket connection established",
    });

    if socket
        .send(Message::Text(hello.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Close(_) => break,
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Text(text) => {
                let reply = json!({
                    "type": "ack",
                    "received": text.as_str(),
                });
                if socket
                    .send(Message::Text(reply.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Message::Binary(payload) => {
                if socket.send(Message::Binary(payload)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> GatewayState {
        GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: true,
        }
    }

    #[test]
    fn validate_ws_request_rejects_missing_origin() {
        let headers = HeaderMap::new();
        assert!(validate_ws_request(&test_state(), &headers).is_err());
    }

    #[test]
    fn validate_ws_request_rejects_missing_auth() {
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::ORIGIN, "http://localhost:3000".parse().unwrap());
        assert!(validate_ws_request(&test_state(), &headers).is_err());
    }

    #[test]
    fn validate_ws_request_accepts_valid_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::ORIGIN, "http://localhost:3000".parse().unwrap());
        headers.insert(axum::http::header::AUTHORIZATION, "Bearer test-token".parse().unwrap());
        assert!(validate_ws_request(&test_state(), &headers).is_ok());
    }
}
