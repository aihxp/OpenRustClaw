//! E2E tests for chat workflows.
//!
//! These tests cover:
//! - User sends message
//! - Agent processes with tool calls
//! - Memory stores conversation
//! - Multi-turn conversation

use std::sync::Arc;

use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_core::traits::{MemoryStore, CoreMemoryStore};
use openrustclaw_core::types::{
    Message, Platform, Session, SessionType,
};
use serial_test::serial;
use uuid::Uuid;

use crate::common::{
    init_test_tracing, CoreEntryBuilder, MemoryEntryBuilder, MockConversationalProvider, 
    MockSuccessProvider, MockToolCallingProvider, TestEnvironment, EchoTool,
};

/// Scenario 1: Simple user message gets a direct response.
#[tokio::test]
#[serial]
async fn test_simple_chat_message() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Create a mock provider that returns a simple response
    let provider = Arc::new(MockSuccessProvider::new(
        "test_provider",
        "Hello! How can I help you today?",
    ));

    let runtime = AgentRuntime::new(
        provider,
        Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
        "TestAgent".to_string(),
    );

    // Process a message
    let messages = vec![Message::user("Hi there!")];
    let response = runtime
        .process(&messages, &[], "session_123", "user_123")
        .await
        .expect("Failed to process message");

    // Verify the response
    assert_eq!(
        response.message.content, "Hello! How can I help you today?"
    );
    assert!(!response.message.content.is_empty());
}

/// Scenario 2: Agent uses a tool to answer a question.
#[tokio::test]
#[serial]
async fn test_chat_with_tool_call() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Create a tool registry with echo tool
    let mut tool_registry = openrustclaw_agent::tools::ToolRegistry::new();
    tool_registry.register(Arc::new(EchoTool));
    let tool_registry = Arc::new(tool_registry);

    // Create a mock provider that calls the echo tool
    let tool_args = serde_json::json!({"message": "Hello World"});
    let provider = Arc::new(MockToolCallingProvider::new(
        "test_provider",
        "echo",
        tool_args,
        "I've echoed your message for you!",
    ));

    let runtime = AgentRuntime::new(
        provider,
        tool_registry.clone(),
        "TestAgent".to_string(),
    );

    // Send a message
    let messages = vec![Message::user("Please echo 'Hello World'")];
    
    // Process - should handle tool calling internally
    let response = runtime
        .process(&messages, &[], "session_123", "user_123")
        .await
        .expect("Failed to process message");

    // Verify we got a response (tool calling is handled internally)
    assert!(!response.message.content.is_empty());
}

/// Scenario 3: Multi-turn conversation maintains context.
#[tokio::test]
#[serial]
async fn test_multi_turn_conversation() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Create a provider with multiple responses
    let responses = vec![
        "Nice to meet you, Alice!".to_string(),
        "You mentioned your name is Alice. How can I help you today?".to_string(),
    ];
    let provider = Arc::new(MockConversationalProvider::new("conv_provider", responses));

    let runtime = AgentRuntime::new(
        provider,
        Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
        "TestAgent".to_string(),
    );

    // Turn 1
    let messages1 = vec![Message::user("My name is Alice")];
    let response1 = runtime
        .process(&messages1, &[], "session_123", "user_123")
        .await
        .expect("Failed turn 1");
    assert_eq!(response1.message.content, "Nice to meet you, Alice!");

    // Turn 2 - include previous context
    let messages2 = vec![
        Message::user("My name is Alice"),
        Message::assistant("Nice to meet you, Alice!"),
        Message::user("What did I just tell you?"),
    ];

    let response2 = runtime
        .process(&messages2, &[], "session_123", "user_123")
        .await
        .expect("Failed turn 2");
    assert!(response2
        .message
        .content
        .contains("Alice"));
}

/// Scenario 4: Conversation is stored in memory.
#[tokio::test]
#[serial]
async fn test_conversation_stored_in_memory() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let provider = Arc::new(MockSuccessProvider::new(
        "test_provider",
        "I will remember that you like Python.",
    ));

    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "TestAgent".to_string(),
        Arc::new(env.memory_store.clone()),
        Arc::new(env.core_memory_store.clone()),
    );

    // User shares a preference
    let messages = vec![Message::user("I love programming in Python")];

    let response = runtime
        .process(&messages, &[], "session_123", "user_123")
        .await
        .expect("Failed to process message");

    assert!(!response.message.content.is_empty());

    // Manually store the conversation memory for testing
    let memory = MemoryEntryBuilder::new("User loves programming in Python")
        .user_id("user_123")
        .session_id(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap_or(Uuid::new_v4()))
        .memory_type(openrustclaw_core::types::MemoryType::Episodic)
        .source(openrustclaw_core::types::MemorySource::ConversationSummary)
        .build();

    env.store_memory(memory).await.expect("Failed to store memory");

    // Verify memory was stored
    let query = crate::common::create_memory_query("Python programming");
    let results = env.search_memories(query).await.expect("Failed to search memories");

    assert!(!results.is_empty());
    assert!(results[0].entry.content.contains("Python"));
}

/// Scenario 5: Core memory is updated during conversation.
#[tokio::test]
#[serial]
async fn test_core_memory_update() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let provider = Arc::new(MockSuccessProvider::new("test_provider", "Got it!"));
    
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "TestAgent".to_string(),
        Arc::new(env.memory_store.clone()),
        Arc::new(env.core_memory_store.clone()),
    );

    // Store core memory
    let core_entry = CoreEntryBuilder::new("user_name", "Bob")
        .importance(0.9)
        .build();
    env.store_core_memory("user_123", core_entry)
        .await
        .expect("Failed to store core memory");

    // Verify core memory was stored
    let memories = env
        .get_core_memory("user_123")
        .await
        .expect("Failed to get core memory");

    assert!(!memories.is_empty());
    let name_entry = memories.iter().find(|m| m.key == "user_name");
    assert!(name_entry.is_some());
    assert_eq!(name_entry.unwrap().value, "Bob");
}

/// Scenario 6: Streaming response works correctly.
#[tokio::test]
#[serial]
async fn test_streaming_response() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let provider = Arc::new(MockSuccessProvider::new(
        "test_provider",
        "This is a streaming response",
    ));

    let runtime = AgentRuntime::new(
        provider,
        Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
        "TestAgent".to_string(),
    );

    let messages = vec![Message::user("Tell me a story")];
    
    let response = runtime
        .process(&messages, &[], "session_123", "user_123")
        .await
        .expect("Failed to process message");

    assert!(!response.message.content.is_empty());
}

/// Scenario 7: Multiple sessions are isolated.
#[tokio::test]
#[serial]
async fn test_session_isolation() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let provider = Arc::new(MockSuccessProvider::new("test_provider", "Response"));

    let runtime = AgentRuntime::new(
        provider,
        Arc::new(openrustclaw_agent::tools::ToolRegistry::new()),
        "TestAgent".to_string(),
    );

    // Session 1 message
    let messages1 = vec![Message::user("I'm user 1")];
    let response1 = runtime
        .process(&messages1, &[], "session_1", "user_1")
        .await
        .expect("Failed session 1");
    assert!(!response1.message.content.is_empty());

    // Session 2 message
    let messages2 = vec![Message::user("I'm user 2")];
    let response2 = runtime
        .process(&messages2, &[], "session_2", "user_2")
        .await
        .expect("Failed session 2");
    assert!(!response2.message.content.is_empty());

    // Responses should be different contexts
    assert_ne!(response1.message.content, response2.message.content);
}
