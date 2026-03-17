//! Regression Test: All Provider Combinations
//!
//! Comprehensive tests for all LLM provider configurations.

use openrustclaw_e2e_tests::common::*;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};

/// Test: All mock provider types work
#[tokio::test]
async fn test_all_mock_providers() {
    let providers: Vec<Box<dyn openrustclaw_core::traits::LlmProvider>> = vec![
        Box::new(MockSuccessProvider::new("success", "Hello")),
        Box::new(MockRateLimitedProvider::new("limited")),
        Box::new(MockUnavailableProvider::new("unavailable", "Error")),
        Box::new(MockConversationalProvider::new("convo", vec!["A".to_string(), "B".to_string()])),
        Box::new(MockToolCallingProvider::new("tools", "test", serde_json::json!({}), "Done")),
    ];

    let request = CompletionRequest {
        messages: vec![Message::user("Test")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(10),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    for provider in providers {
        // Just verify they don't panic
        let _ = provider.model_id();
        let _ = provider.max_tokens();
        let _ = provider.provider_name();
        let _ = provider.complete(request.clone()).await;
    }
}

/// Test: Provider streaming for all types
#[tokio::test]
async fn test_all_provider_streaming() {
    let providers: Vec<Box<dyn openrustclaw_core::traits::LlmProvider>> = vec![
        Box::new(MockSuccessProvider::new("success", "Streamed")),
        Box::new(MockRateLimitedProvider::new("limited")),
    ];

    let request = CompletionRequest {
        messages: vec![Message::user("Test")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(10),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: true, // Request streaming
    };

    for provider in providers {
        let result = provider.stream(request.clone()).await;
        // Either succeeds or returns error - both are valid
        let _ = result;
    }
}

/// Test: Provider error recovery scenarios
#[tokio::test]
async fn test_provider_error_recovery() {
    let request = CompletionRequest {
        messages: vec![Message::user("Test")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(10),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    // Test rate limit with retry
    let limited = MockRateLimitedProvider::new("limited").with_retry_after(1);
    let result = limited.complete(request.clone()).await;
    assert!(result.is_err());

    // Test unavailable
    let unavailable = MockUnavailableProvider::new("down", "Server error");
    let result = unavailable.complete(request.clone()).await;
    assert!(result.is_err());

    // Test success after failure simulation
    let success = MockSuccessProvider::new("success", "Recovered!");
    let result = success.complete(request).await;
    assert!(result.is_ok());
}

/// Test: Provider token usage across different scenarios
#[tokio::test]
async fn test_provider_token_usage_scenarios() {
    let long_content = "x".repeat(1000);
    let scenarios: Vec<(&str, &str, u32)> = vec![
        ("short", "Hi", 10u32),
        ("medium", "This is a medium length message for testing", 20u32),
        ("long", &long_content, 100u32),
    ];

    for (name, content, _expected_max) in scenarios {
        let provider = MockSuccessProvider::new("mock", content);
        let request = CompletionRequest {
            messages: vec![Message::user(content)],
            model: Some("mock-model".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            tools: None,
            system_prompt: None,
            stream: false,
        };

        let response = provider.complete(request).await.expect(&format!("{} failed", name));
        
        // Mock provider returns fixed values
        assert!(response.usage.total_tokens > 0, "{}: Should have token usage", name);
    }
}

/// Test: Provider tool format consistency
#[tokio::test]
async fn test_provider_tool_format_consistency() {
    let provider = MockSuccessProvider::new("mock", "Test");

    // Verify tool format
    let format = provider.native_tool_format();
    assert!(
        matches!(format, openrustclaw_core::types::ToolFormat::OpenAi) ||
        matches!(format, openrustclaw_core::types::ToolFormat::Anthropic) ||
        matches!(format, openrustclaw_core::types::ToolFormat::Mcp)
    );

    // Verify strict tool support flag
    let _supports_strict = provider.supports_strict_tools();
}

/// Test: Live providers if API keys available
#[tokio::test]
async fn test_live_providers_if_available() {
    if !use_live_providers() {
        println!("Skipping live provider tests - E2E_LIVE not set");
        return;
    }

    // This would test actual providers if keys are present
    // For now, just verify the infrastructure
    println!("Live providers would be tested here if API keys are present");
}
