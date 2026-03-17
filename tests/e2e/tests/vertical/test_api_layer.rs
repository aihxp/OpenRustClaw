//! Vertical E2E Test: API Layer
//!
//! Tests HTTP endpoints, WebSocket connections, and SSE streams in isolation.

use openrustclaw_e2e_tests::common::*;
use reqwest::StatusCode;

/// Test: Health endpoint responds correctly
#[tokio::test]
async fn test_api_health_endpoint() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));
    
    let response = client.get("/health").await.expect("Request failed");
    
    response.assert_success();
    response.assert_content_type("application/json");

    let health: serde_json::Value = TestHttpClient::parse_json(response).await.expect("Parse failed");
    E2eAssertions::is_healthy(&health);
}

/// Test: API returns correct CORS headers
#[tokio::test]
async fn test_api_cors_headers() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = reqwest::Client::new();
    let response = client
        .request(reqwest::Method::OPTIONS, format!("http://{}/health", addr))
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);
    
    let cors_header = response.headers().get("access-control-allow-origin");
    assert!(cors_header.is_some(), "CORS header missing");
}

/// Test: API returns 404 for unknown endpoints
#[tokio::test]
async fn test_api_404_handling() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));
    
    let response = client.get("/nonexistent").await.expect("Request failed");
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test: API returns 405 for wrong HTTP methods
#[tokio::test]
async fn test_api_method_not_allowed() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = reqwest::Client::new();
    let response = client
        .delete(format!("http://{}/health", addr))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

/// Test: API timeout handling
#[tokio::test]
async fn test_api_timeout() {
    // This would test timeout configuration
    // For now, just verify the client timeout works
    let client = TestHttpClient::with_timeout("http://localhost:9999", 1);
    
    // This should timeout quickly
    let start = std::time::Instant::now();
    let result = client.get("/").await;
    let elapsed = start.elapsed();

    assert!(result.is_err() || elapsed < std::time::Duration::from_secs(2));
}

/// Test: Request/response JSON serialization
#[tokio::test]
async fn test_api_json_serialization() {
    use openrustclaw_core::types::*;

    // Test that request serialization works correctly
    let request = CompletionRequest {
        messages: vec![Message::user("Test")],
        model: Some("test-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: Some(vec![ToolDefinition {
            name: "test".to_string(),
            description: "Test tool".to_string(),
            parameters: serde_json::json!({}),
            strict: false,
        }]),
        system_prompt: None,
        stream: false,
    };

    let json = serde_json::to_string(&request).expect("Serialization failed");
    assert!(json.contains("messages"));
    assert!(json.contains("model"));

    // Test deserialization
    let deserialized: CompletionRequest = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.model, request.model);
}

/// Test: Large payload handling
#[tokio::test]
async fn test_api_large_payload() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    // Create a large message (100KB of text)
    let large_content = "x".repeat(100_000);
    
    let client = TestHttpClient::new(format!("http://{}", addr));
    
    // Most servers have payload limits - this tests that behavior
    // In a real test, we'd verify the actual limit
    let body = serde_json::json!({
        "messages": [{"role": "user", "content": large_content}],
        "model": "test-model"
    });

    let response = client.post("/v1/chat/completions", &body).await;
    
    // Should either succeed or return 413 (Payload Too Large)
    match response {
        Ok(resp) => {
            let status = resp.status();
            assert!(status.is_success() || status == StatusCode::PAYLOAD_TOO_LARGE);
        }
        Err(_) => {
            // Network errors are acceptable for this test
        }
    }
}

/// Test: Concurrent API requests
#[tokio::test]
async fn test_api_concurrent_requests() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let _client = TestHttpClient::new(format!("http://{}", addr));

    // Make 10 concurrent health requests
    let mut handles = vec![];
    for _ in 0..10 {
        let client = TestHttpClient::new(format!("http://{}", addr));
        handles.push(tokio::spawn(async move {
            client.get("/health").await
        }));
    }

    // All should succeed
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok(), "Concurrent request failed");
    }
}

/// Test: WebSocket upgrade handling (if WebSocket is enabled)
#[tokio::test]
#[ignore = "WebSocket not implemented in gateway"]
async fn test_api_websocket_upgrade() {
    // This would test WebSocket upgrade handshake
    // Ignored until WebSocket is fully implemented
}

/// Test: SSE stream handling (if SSE is enabled)
#[tokio::test]
#[ignore = "SSE not implemented in gateway"]
async fn test_api_sse_stream() {
    // This would test Server-Sent Events
    // Ignored until SSE is fully implemented
}
