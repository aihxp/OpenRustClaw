//! Vertical E2E Test: Provider Layer
//!
//! Tests LLM provider implementations, fallback logic, and retry mechanisms.

use openrustclaw_e2e_tests::common::*;
use openrustclaw_core::traits::LlmProvider;

/// Test: Provider creation
#[tokio::test]
async fn test_provider_creation() {
    // Test that we can create mock providers
    let provider = MockSuccessProvider::new("test", "Hello");
    
    assert_eq!(provider.provider_name(), "test");
    assert_eq!(provider.model_id(), "mock-model");
}

/// Test: Provider completion
#[tokio::test]
async fn test_provider_completion() {
    let provider = MockSuccessProvider::new("mock", "Test response");
    
    let request = TestRequests::simple_chat();
    let response = provider.complete(request).await.expect("Completion failed");
    
    E2eAssertions::response_contains(&response, "Test response");
    assert_eq!(response.provider, "mock");
}

/// Test: Provider streaming
#[tokio::test]
async fn test_provider_streaming() {
    let provider = MockSuccessProvider::new("mock", "Streamed content");
    
    let request = TestRequests::streaming();
    let stream = provider.stream(request).await.expect("Streaming failed");

    use futures::StreamExt;
    let chunks: Vec<_> = stream.collect().await;
    
    assert!(!chunks.is_empty(), "Should receive stream chunks");
}

/// Test: Provider rate limit handling
#[tokio::test]
async fn test_provider_rate_limit() {
    let provider = MockRateLimitedProvider::new("limited").with_retry_after(30);
    
    let request = TestRequests::simple_chat();
    let result = provider.complete(request).await;
    
    assert!(result.is_err(), "Should return rate limit error");
    
    let error = result.unwrap_err();
    let error_string = format!("{}", error);
    assert!(error_string.contains("rate limit") || error_string.contains("RateLimit"));
}

/// Test: Provider unavailable handling
#[tokio::test]
async fn test_provider_unavailable() {
    let provider = MockUnavailableProvider::new("down", "Service unavailable");
    
    let request = TestRequests::simple_chat();
    let result = provider.complete(request).await;
    
    assert!(result.is_err(), "Should return unavailable error");
}

/// Test: Provider token counting
#[tokio::test]
async fn test_provider_token_usage() {
    let provider = MockSuccessProvider::new("mock", "Response");
    
    let request = TestRequests::simple_chat();
    let response = provider.complete(request).await.expect("Completion failed");
    
    // Mock provider returns fixed token counts
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 10);
    assert_eq!(response.usage.total_tokens, 20);
}

/// Test: Provider tool support detection
#[tokio::test]
async fn test_provider_tool_support() {
    let provider = MockSuccessProvider::new("mock", "Hello");
    
    assert!(provider.supports_strict_tools());
    assert_eq!(provider.native_tool_format(), openrustclaw_core::types::ToolFormat::OpenAi);
}

/// Test: Multi-provider fallback simulation
#[tokio::test]
async fn test_multi_provider_fallback_simulation() {
    let primary = MockUnavailableProvider::new("primary", "Primary down");
    let secondary = MockRateLimitedProvider::new("secondary");
    let tertiary = MockSuccessProvider::new("tertiary", "Success from tertiary");

    // Simulate fallback chain
    let request = TestRequests::simple_chat();
    
    // Try primary
    let result = primary.complete(request.clone()).await;
    assert!(result.is_err());

    // Try secondary
    let result = secondary.complete(request.clone()).await;
    assert!(result.is_err());

    // Try tertiary - should succeed
    let response = tertiary.complete(request).await.expect("Should succeed");
    E2eAssertions::response_contains(&response, "tertiary");
}

/// Test: Provider response format consistency
#[tokio::test]
async fn test_provider_response_format() {
    let provider = MockSuccessProvider::new("mock", "Test response");
    
    let request = TestRequests::simple_chat();
    let response = provider.complete(request).await.expect("Completion failed");
    
    // Verify response structure
    assert!(!response.id.is_empty(), "Response should have ID");
    assert!(!response.message.content.is_empty(), "Response should have content");
    assert!(!response.model.is_empty(), "Response should specify model");
}

/// Test: Provider with tool calls
#[tokio::test]
async fn test_provider_tool_calls() {
    let tool_call = openrustclaw_core::types::ToolCall {
        id: "call-1".to_string(),
        name: "calculator".to_string(),
        arguments: serde_json::json!({"a": 1, "b": 2}),
    };

    let provider = MockSuccessProvider::new("mock", "Result is 3")
        .with_tool_calls(vec![tool_call]);
    
    let request = TestRequests::simple_chat();
    let response = provider.complete(request).await.expect("Completion failed");
    
    assert_eq!(response.finish_reason, openrustclaw_core::types::FinishReason::ToolUse);
    assert!(response.message.tool_calls.is_some());
}

/// Test: Live provider check (skipped if no API key)
#[tokio::test]
async fn test_live_provider_check() {
    if !use_live_providers() {
        println!("Skipping live provider test - E2E_LIVE not set");
        return;
    }

    // This would test actual provider if API keys are available
    // For now just verify the check works
    assert!(has_api_key("OPENAI") || has_api_key("ANTHROPIC") || !use_live_providers());
}
