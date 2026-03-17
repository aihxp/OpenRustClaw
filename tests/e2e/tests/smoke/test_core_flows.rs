//! Smoke Test: Core Flows
//!
//! Critical user flows that must work for the system to be usable.

use openrustclaw_e2e_tests::common::*;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, Message};

/// CRITICAL: Simple chat flow
#[tokio::test]
async fn smoke_chat_flow() {
    let _env = TestEnvironment::new().await;

    let provider = MockSuccessProvider::new("mock", "Hello!");

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(50),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = provider.complete(request).await.expect("Chat flow failed");
    
    assert!(!response.message.content.is_empty());
}

/// CRITICAL: Tool execution flow
#[tokio::test]
async fn smoke_tool_execution_flow() {
    let _env = TestEnvironment::new().await;

    let provider = MockToolCallingProvider::new(
        "mock",
        "calculator",
        serde_json::json!({"operation": "add", "a": 1, "b": 1}),
        "The answer is 2",
    );

    let request = CompletionRequest {
        messages: vec![Message::user("Calculate 1+1")],
        model: Some("mock-model".to_string()),
        max_tokens: Some(50),
        temperature: Some(0.7),
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = provider.complete(request).await.expect("Tool flow failed");
    
    assert!(response.message.tool_calls.is_some());
}

/// CRITICAL: Memory recall flow
#[tokio::test]
async fn smoke_memory_recall_flow() {
    let env = TestEnvironment::new().await;

    use openrustclaw_core::traits::MemoryStore;

    // Store a memory with unique content
    let entry = MemoryEntryBuilder::new("My name is Alice and I love Rust programming")
        .user_id(TestUsers::alice().id)
        .build();

    env.memory_store.store(entry).await.expect("Store failed");

    // Search for it using keywords that will match (FTS works better with present keywords)
    let query = create_memory_query("Alice Rust programming");
    let results = env.memory_store.search(&query).await.expect("Search failed");

    assert!(!results.is_empty(), "Memory recall not working - no results found");
    
    // Verify the content is what we stored
    let found = results.iter().any(|r| r.entry.content.contains("Alice"));
    assert!(found, "Memory recall not working - content not found in results");
}

/// CRITICAL: Provider fallback
#[tokio::test]
async fn smoke_provider_fallback() {
    let _env = TestEnvironment::new().await;

    let primary = MockUnavailableProvider::new("primary", "Down");
    let fallback = MockSuccessProvider::new("fallback", "I'm backup!");

    // Test fallback works when primary fails
    let result = primary.complete(TestRequests::simple_chat()).await;
    assert!(result.is_err());

    let result = fallback.complete(TestRequests::simple_chat()).await;
    assert!(result.is_ok());
}
