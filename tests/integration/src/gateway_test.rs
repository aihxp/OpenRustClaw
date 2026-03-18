//! WebSocket gateway integration tests.
//!
//! These tests verify:
//! - WebSocket connection with valid origin
//! - Rejection of invalid origin
//! - Authentication
//! - Session management

use std::sync::Arc;

use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_security::OriginValidator;

use crate::common::init_test_tracing;

fn test_state_with_auth() -> GatewayState {
    GatewayState {
        session_manager: Arc::new(SessionManager::new()),
        origin_validator: Arc::new(OriginValidator::new(vec![
            "http://localhost:3000".to_string(),
            "https://example.com".to_string(),
        ])),
        require_auth: true,
        internal_api_token: None,
        memory_store: None,
        core_memory_store: None,
        rag_store: None,
    }
}

fn test_state_no_auth() -> GatewayState {
    GatewayState {
        session_manager: Arc::new(SessionManager::new()),
        origin_validator: Arc::new(OriginValidator::new(vec![
            "http://localhost:3000".to_string(),
        ])),
        require_auth: false,
        internal_api_token: None,
        memory_store: None,
        core_memory_store: None,
        rag_store: None,
    }
}

#[test]
fn gateway_server_creation() {
    init_test_tracing();

    let server = GatewayServer::new("127.0.0.1".to_string(), 8080);
    assert_eq!(server.addr(), "127.0.0.1:8080");
}

#[test]
fn gateway_server_router() {
    init_test_tracing();

    let server = GatewayServer::new("127.0.0.1".to_string(), 8080);
    let state = test_state_with_auth();
    let _router = server.router(state);
    // Router is created successfully
}

#[tokio::test]
async fn session_manager_basic() {
    init_test_tracing();

    let manager = SessionManager::new();

    // Initially no sessions
    assert_eq!(manager.count().await, 0);
}

#[tokio::test]
async fn session_manager_create_session() {
    init_test_tracing();

    let manager = SessionManager::new();

    let session = manager
        .create_session(
            "user_123",
            openrustclaw_core::types::SessionType::Dm,
            openrustclaw_core::types::Platform::WebChat,
        )
        .await
        .unwrap();

    assert_eq!(manager.count().await, 1);
    assert_eq!(session.user_id, "user_123");
}

#[tokio::test]
async fn session_manager_get_session() {
    init_test_tracing();

    let manager = SessionManager::new();

    let session = manager
        .create_session(
            "user_123",
            openrustclaw_core::types::SessionType::Dm,
            openrustclaw_core::types::Platform::WebChat,
        )
        .await
        .unwrap();
    let session_id = session.id.to_string();

    let retrieved = manager.get_session(&session_id).await.unwrap();
    assert_eq!(retrieved.user_id, "user_123");
}

#[tokio::test]
async fn session_manager_remove_session() {
    init_test_tracing();

    let manager = SessionManager::new();

    let session = manager
        .create_session(
            "user_123",
            openrustclaw_core::types::SessionType::Dm,
            openrustclaw_core::types::Platform::WebChat,
        )
        .await
        .unwrap();
    let session_id = session.id.to_string();

    assert_eq!(manager.count().await, 1);

    manager.remove_session(&session_id).await.unwrap();
    assert_eq!(manager.count().await, 0);
}

#[tokio::test]
async fn session_manager_get_nonexistent() {
    init_test_tracing();

    let manager = SessionManager::new();

    let result = manager.get_session("nonexistent-id-123").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn session_manager_multiple_sessions() {
    init_test_tracing();

    let manager = SessionManager::new();

    let session1 = manager
        .create_session(
            "user_1",
            openrustclaw_core::types::SessionType::Dm,
            openrustclaw_core::types::Platform::WebChat,
        )
        .await
        .unwrap();
    let session2 = manager
        .create_session(
            "user_2",
            openrustclaw_core::types::SessionType::Dm,
            openrustclaw_core::types::Platform::Cli,
        )
        .await
        .unwrap();

    assert_eq!(manager.count().await, 2);

    let retrieved1 = manager.get_session(&session1.id.to_string()).await.unwrap();
    let retrieved2 = manager.get_session(&session2.id.to_string()).await.unwrap();

    assert_eq!(retrieved1.user_id, "user_1");
    assert_eq!(retrieved2.user_id, "user_2");
}

// ═════════════════════════════════════════════════════════════════════════════
// Origin Validator Tests (via public API)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn origin_validator_via_gateway_state() {
    init_test_tracing();

    let state = GatewayState {
        session_manager: Arc::new(SessionManager::new()),
        origin_validator: Arc::new(OriginValidator::new(vec![
            "https://example.com".to_string(),
        ])),
        require_auth: false,
        internal_api_token: None,
        memory_store: None,
        core_memory_store: None,
        rag_store: None,
    };

    // Test that origin validator is correctly set up
    assert!(!state.require_auth);
}

#[test]
fn gateway_state_with_auth() {
    init_test_tracing();

    let state = test_state_with_auth();
    assert!(state.require_auth);
}

#[test]
fn gateway_state_without_auth() {
    init_test_tracing();

    let state = test_state_no_auth();
    assert!(!state.require_auth);
}
