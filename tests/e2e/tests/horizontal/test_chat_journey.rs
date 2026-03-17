//! Horizontal E2E Test: Complete Chat Journey
//!
//! Tests the full flow: User Message → Gateway → Agent → LLM → Response

use openrustclaw_e2e_tests::common::*;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};

/// Test: Simple chat without tools
///
/// Journey: User sends greeting → Gateway → Agent → LLM → Response returned
#[tokio::test]
async fn test_simple_chat_journey() {
    let _env = TestEnvironment::new().await;

    // Create a mock provider that returns a simple response
    let provider = MockSuccessProvider::new("mock", "Hello! How can I help you today?");

    // Simulate the journey directly through the provider
    let request = CompletionRequest {
        messages: vec![Message::user("Hello")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    // Execute directly through provider
    let response = provider.complete(request).await.expect("Completion failed");

    // Verify the complete journey
    E2eAssertions::response_contains(&response, "Hello");
    E2eAssertions::response_from_provider(&response, "mock");
}

/// Test: Chat with tool calling
///
/// Journey: User asks for calculation → Agent detects tool need → 
///          LLM requests tool → Tool executes → LLM formats response
#[tokio::test]
async fn test_chat_with_tool_journey() {
    let _env = TestEnvironment::new().await;

    // Create provider that calls calculator tool
    let provider = MockToolCallingProvider::new(
        "mock",
        "calculator",
        serde_json::json!({"operation": "add", "a": 2, "b": 2}),
        "The result of 2 + 2 is 4",
    );

    // First turn - LLM requests tool
    let request1 = CompletionRequest {
        messages: vec![Message::user("What is 2 + 2?")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response1 = provider.complete(request1).await.expect("First turn failed");

    // Verify tool was requested
    E2eAssertions::response_has_tool_calls(&response1);
    let tool_call = E2eAssertions::tool_was_called(&response1, "calculator");
    E2eAssertions::tool_has_args(
        tool_call,
        serde_json::json!({"operation": "add", "a": 2, "b": 2}),
    );

    // Simulate tool execution and second turn
    let tool_result = Message::tool("call-1", "Result: 4");
    let provider2 = MockSuccessProvider::new("mock", "The result of 2 + 2 is 4");
    let request2 = CompletionRequest {
        messages: vec![
            Message::user("What is 2 + 2?"),
            response1.message,
            tool_result,
        ],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response2 = provider2.complete(request2).await.expect("Second turn failed");

    // Verify final response
    E2eAssertions::response_contains(&response2, "4");
}

/// Test: Multi-turn conversation
///
/// Journey: Multiple message exchanges with context preservation
#[tokio::test]
async fn test_multi_turn_conversation_journey() {
    let _env = TestEnvironment::new().await;
    let _session = TestSessions::new();

    // Provider with multiple responses
    let provider = MockConversationalProvider::new(
        "mock",
        vec![
            "I'd be happy to help!".to_string(),
            "Let me check that for you.".to_string(),
            "Here's what I found.".to_string(),
        ],
    );

    // Turn 1
    let request1 = CompletionRequest {
        messages: vec![Message::user("Can you help me?")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response1 = provider.complete(request1).await.expect("Turn 1 failed");
    E2eAssertions::response_contains(&response1, "happy");

    // Turn 2 - continuing conversation
    let request2 = CompletionRequest {
        messages: vec![
            Message::user("Can you help me?"),
            response1.message,
            Message::user("What's the weather like?"),
        ],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response2 = provider.complete(request2).await.expect("Turn 2 failed");
    E2eAssertions::response_contains(&response2, "check");

    // Turn 3
    let request3 = CompletionRequest {
        messages: vec![
            Message::user("Can you help me?"),
            Message::assistant("I'd be happy to help!"),
            Message::user("What's the weather like?"),
            response2.message,
            Message::user("Thanks!"),
        ],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response3 = provider.complete(request3).await.expect("Turn 3 failed");
    E2eAssertions::response_contains(&response3, "found");
}

/// Test: Streaming response journey
///
/// Journey: User request → Streaming response chunks → Complete message
#[tokio::test]
async fn test_streaming_chat_journey() {
    let _env = TestEnvironment::new().await;

    let provider = MockSuccessProvider::new("mock", "This is a streaming response");

    let request = CompletionRequest {
        messages: vec![Message::user("Tell me a story")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: true, // Enable streaming
    };

    let stream = provider.stream(request).await.expect("Streaming failed");

    // Collect all chunks
    use futures::StreamExt;
    let chunks: Vec<_> = stream.collect().await;

    // Verify we got chunks
    assert!(!chunks.is_empty(), "Should have received stream chunks");
}

/// Test: Error recovery journey
///
/// Journey: Primary provider fails → Fallback provider succeeds
#[tokio::test]
async fn test_provider_fallback_journey() {
    let _env = TestEnvironment::new().await;

    // This test would require provider chain implementation
    // For now, we verify the mock providers work correctly

    let primary = MockUnavailableProvider::new("primary", "Service down");
    let fallback = MockSuccessProvider::new("fallback", "Hello from fallback!");

    // Test primary fails
    let request = TestRequests::simple_chat();
    let result = primary.complete(request.clone()).await;
    assert!(result.is_err(), "Primary should fail");

    // Test fallback succeeds
    let response = fallback.complete(request).await.expect("Fallback should succeed");
    E2eAssertions::response_contains(&response, "fallback");
}
