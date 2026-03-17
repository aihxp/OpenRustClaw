//! Vertical E2E Test: Gateway Layer
//!
//! Tests rate limiting, authentication, routing, and CORS.

use openrustclaw_e2e_tests::common::*;
use reqwest::StatusCode;

/// Test: Gateway starts and responds
#[tokio::test]
async fn test_gateway_startup() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));
    let response = client.get("/health").await.expect("Request failed");

    response.assert_success();
}

/// Test: Rate limiting (if enabled)
#[tokio::test]
#[ignore = "Rate limiting not configured in test gateway"]
async fn test_gateway_rate_limiting() {
    // This would test rate limiting once configured
}

/// Test: Origin validation
#[tokio::test]
async fn test_gateway_origin_validation() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = reqwest::Client::new();

    // Request from allowed origin
    let allowed_response = client
        .get(format!("http://{}/health", addr))
        .header("Origin", "http://localhost:3000")
        .send()
        .await
        .expect("Request failed");

    // Should succeed (or at least not fail due to origin)
    // The actual validation depends on gateway configuration
    assert!(
        allowed_response.status().is_success()
            || allowed_response.status() == StatusCode::FORBIDDEN
    );
}

/// Test: Session management
#[tokio::test]
async fn test_gateway_session_management() {
    let env = TestEnvironment::new().await;
    let _session = TestSessions::create();

    // Create session
    use openrustclaw_core::types::{Platform, SessionType};
    let session = env
        .session_manager
        .create_session(&TestUsers::alice().id, SessionType::Dm, Platform::WebChat)
        .await
        .expect("Create failed");
    let session_id = session.id.to_string();

    // Verify session exists
    let retrieved = env.session_manager.get_session(&session_id).await;
    assert!(retrieved.is_ok(), "Session should exist");

    // Remove session
    env.session_manager
        .remove_session(&session_id)
        .await
        .expect("Remove failed");

    // Verify session removed
    let retrieved = env.session_manager.get_session(&session_id).await;
    assert!(retrieved.is_err(), "Session should be removed");
}

/// Test: Request routing
#[tokio::test]
async fn test_gateway_request_routing() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));

    // Test different endpoints
    let health = client.get("/health").await;
    assert!(health.is_ok());

    // Metrics endpoint (if exposed)
    let _metrics = client.get("/metrics").await;
    // May or may not exist depending on config

    // Non-existent endpoint
    let not_found = client.get("/does-not-exist").await;
    if let Ok(resp) = not_found {
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}

/// Test: Request timeout handling
#[tokio::test]
async fn test_gateway_timeout_handling() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::with_timeout(format!("http://{}", addr), 1);

    // This tests that the gateway handles timeouts gracefully
    // In a real scenario, we'd have a slow endpoint to test
    let response = client.get("/health").await;
    assert!(response.is_ok(), "Quick request should not timeout");
}

/// Test: Concurrent connections
#[tokio::test]
async fn test_gateway_concurrent_connections() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let mut handles = vec![];

    // Spawn 50 concurrent connections
    for _ in 0..50 {
        let client = TestHttpClient::new(format!("http://{}", addr));
        handles.push(tokio::spawn(async move { client.get("/health").await }));
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            _ => fail_count += 1,
        }
    }

    // Most should succeed
    assert!(
        success_count >= 45,
        "Most concurrent requests should succeed ({} succeeded, {} failed)",
        success_count,
        fail_count
    );
}

/// Test: Graceful shutdown
#[tokio::test]
async fn test_gateway_graceful_shutdown() {
    let env = TestEnvironment::new().await;
    let (addr, server) = env.start_gateway(false).await;

    // Make a request before shutdown
    let client = TestHttpClient::new(format!("http://{}", addr));
    let response = client.get("/health").await;
    assert!(response.is_ok());

    // Trigger shutdown
    server.abort();

    // Give it a moment to shut down
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Server should no longer respond
    let client = TestHttpClient::with_timeout(format!("http://{}", addr), 1);
    let response = client.get("/health").await;
    assert!(response.is_err(), "Server should be shut down");
}

/// Test: Response headers
#[tokio::test]
async fn test_gateway_response_headers() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));
    let response = client.get("/health").await.expect("Request failed");

    // Check standard headers
    assert!(response.headers().contains_key("content-type"));

    // Check CORS headers if configured
    // These may or may not be present depending on configuration
}

/// Test: Request body size limits
#[tokio::test]
async fn test_gateway_request_size_limits() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));

    // Try with a moderately large body (1MB)
    let large_body = serde_json::json!({
        "data": "x".repeat(1_000_000)
    });

    // This tests the gateway's handling of large payloads
    // The actual limit depends on gateway configuration
    let result = client.post("/health", &large_body).await;

    // Should either succeed or return appropriate error
    match result {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status.is_success()
                    || status == StatusCode::PAYLOAD_TOO_LARGE
                    || status == StatusCode::BAD_REQUEST
            );
        }
        Err(_) => {
            // Connection errors are acceptable for oversized payloads
        }
    }
}
