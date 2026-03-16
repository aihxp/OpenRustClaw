//! Agent runtime integration tests.
//!
//! These tests verify:
//! - Basic message processing
//! - Tool calling workflow
//! - Multi-turn conversation
//! - Streaming responses

use std::sync::Arc;

use futures::StreamExt;
use openrustclaw_agent::runtime::{AgentResponse, AgentRuntime};
use openrustclaw_agent::tools::ToolRegistry;
use openrustclaw_core::error::Error;
use openrustclaw_core::traits::{LlmProvider, Tool, ToolContext};
use openrustclaw_core::types::{
    SkillCapability,
    CompletionRequest, CoreEntry, FinishReason, Message, Role, StreamChunk,
    TokenUsage, ToolCall, ToolOutput,
};
use serde_json::Value;

use crate::common::{
    create_test_tool_registry, init_test_tracing, CalculatorTool, EchoTool, FailingTool,
    MockConversationalProvider, MockSuccessProvider,
};

#[tokio::test]
async fn basic_message_processing() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "Hello! How can I help you today?",
    ));
    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("Hi there")];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.message.role, Role::Assistant);
    assert_eq!(response.message.content, "Hello! How can I help you today?");
    assert_eq!(response.tool_calls_made, 0);
}

#[tokio::test]
async fn message_with_core_memory() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "I remember your name is Alice!",
    ));
    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("What's my name?")];
    let core_memory = vec![CoreEntry {
        key: "user_name".to_string(),
        value: "Alice".to_string(),
        importance: 0.9,
        token_count: 3,
        updated_at: chrono::Utc::now(),
    }];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert!(response.message.content.contains("Alice"));
}

#[tokio::test]
async fn tool_calling_workflow() {
    init_test_tracing();

    let tool_call = ToolCall {
        id: "call_123".to_string(),
        name: "echo".to_string(),
        arguments: serde_json::json!({"message": "Hello from tool"}),
    };

    let provider: Arc<dyn LlmProvider> = Arc::new(
        MockSuccessProvider::new("test", "I'll echo that for you").with_tool_calls(vec![
            tool_call.clone(),
        ]),
    );

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(EchoTool));
    let tools = Arc::new(tools);

    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("Please echo 'Hello from tool'")];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    // The response should include the tool result
    assert_eq!(response.tool_calls_made, 1);
}

#[tokio::test]
async fn calculator_tool_execution() {
    init_test_tracing();

    let tool_call = ToolCall {
        id: "call_456".to_string(),
        name: "calculator".to_string(),
        arguments: serde_json::json!({
            "operation": "add",
            "a": 5.0,
            "b": 3.0
        }),
    };

    let provider: Arc<dyn LlmProvider> = Arc::new(
        MockSuccessProvider::new("test", "I'll calculate that").with_tool_calls(vec![tool_call]),
    );

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(CalculatorTool));
    let tools = Arc::new(tools);

    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("What is 5 plus 3?")];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.tool_calls_made, 1);
}

#[tokio::test]
async fn multiple_tool_calls() {
    init_test_tracing();

    let tool_calls = vec![
        ToolCall {
            id: "call_1".to_string(),
            name: "echo".to_string(),
            arguments: serde_json::json!({"message": "First call"}),
        },
        ToolCall {
            id: "call_2".to_string(),
            name: "echo".to_string(),
            arguments: serde_json::json!({"message": "Second call"}),
        },
    ];

    let provider: Arc<dyn LlmProvider> = Arc::new(
        MockSuccessProvider::new("test", "I'll echo both messages").with_tool_calls(tool_calls),
    );

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(EchoTool));
    let tools = Arc::new(tools);

    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("Echo two messages")];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.tool_calls_made, 2);
}

#[tokio::test]
async fn unknown_tool_returns_error() {
    init_test_tracing();

    let tool_call = ToolCall {
        id: "call_789".to_string(),
        name: "nonexistent_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let provider: Arc<dyn LlmProvider> = Arc::new(
        MockSuccessProvider::new("test", "I'll try that tool").with_tool_calls(vec![tool_call]),
    );

    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("Use nonexistent tool")];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    // Should still complete, but tool call would have errored
    assert_eq!(response.tool_calls_made, 1);
}

#[tokio::test]
async fn tool_error_handling() {
    init_test_tracing();

    let tool_call = ToolCall {
        id: "call_fail".to_string(),
        name: "failing_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let provider: Arc<dyn LlmProvider> = Arc::new(
        MockSuccessProvider::new("test", "I'll try that").with_tool_calls(vec![tool_call]),
    );

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(FailingTool {
        fail_message: "Intentional failure".to_string(),
    }));
    let tools = Arc::new(tools);

    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages = vec![Message::user("Use the failing tool")];
    let core_memory = vec![];

    // Should not panic, should handle gracefully
    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.tool_calls_made, 1);
}

#[tokio::test]
async fn multi_turn_conversation() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockConversationalProvider::new(
        "test",
        vec![
            "What's your name?".to_string(),
            "Nice to meet you!".to_string(),
            "How can I help?".to_string(),
        ],
    ));

    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let core_memory = vec![];

    // First turn
    let messages1 = vec![Message::user("Hello")];
    let response1 = runtime
        .process(&messages1, &core_memory, "session_123", "user_123")
        .await
        .unwrap();
    assert_eq!(response1.message.content, "What's your name?");

    // Second turn (with history)
    let messages2 = vec![
        Message::user("Hello"),
        response1.message.clone(),
        Message::user("I'm Alice"),
    ];
    let response2 = runtime
        .process(&messages2, &core_memory, "session_123", "user_123")
        .await
        .unwrap();
    assert_eq!(response2.message.content, "Nice to meet you!");

    // Third turn
    let messages3 = vec![
        Message::user("Hello"),
        response1.message,
        Message::user("I'm Alice"),
        response2.message,
        Message::user("I need help"),
    ];
    let response3 = runtime
        .process(&messages3, &core_memory, "session_123", "user_123")
        .await
        .unwrap();
    assert_eq!(response3.message.content, "How can I help?");
}

#[tokio::test]
async fn streaming_response() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "Streaming response content",
    ));

    let request = CompletionRequest {
        messages: vec![Message::user("Stream this")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: true,
    };

    let mut stream = provider.stream(request).await.unwrap();

    let mut content_parts = vec![];
    let mut got_done = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(StreamChunk::ContentDelta { delta }) => {
                content_parts.push(delta);
            }
            Ok(StreamChunk::Done { .. }) => {
                got_done = true;
            }
            Ok(StreamChunk::ToolCallDelta { .. }) => {
                // Not expected in this test
            }
            Err(e) => panic!("Stream error: {}", e),
        }
    }

    assert!(!content_parts.is_empty());
    assert!(got_done);
}

#[tokio::test]
async fn max_tokens_response() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "Truncated response",
    ));

    let request = CompletionRequest {
        messages: vec![Message::user("Long prompt")],
        model: None,
        max_tokens: Some(10),
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    // The mock provider doesn't actually enforce max_tokens, but we can test the request
    let response = provider.complete(request).await.unwrap();
    assert_eq!(response.finish_reason, FinishReason::Stop);
}

#[tokio::test]
async fn conversation_with_history() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "I see you mentioned that earlier",
    ));
    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    // Simulate conversation history
    let messages = vec![
        Message::user("My favorite color is blue"),
        Message::assistant("I'll remember that"),
        Message::user("What did I say earlier?"),
    ];
    let core_memory = vec![];

    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.message.content, "I see you mentioned that earlier");
}

#[tokio::test]
async fn empty_messages_handled() {
    init_test_tracing();

    let provider: Arc<dyn LlmProvider> = Arc::new(MockSuccessProvider::new(
        "test",
        "How can I help?",
    ));
    let tools = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider, tools, "TestAgent".to_string());

    let messages: Vec<Message> = vec![];
    let core_memory = vec![];

    // Should handle empty messages gracefully
    let response: AgentResponse = runtime
        .process(&messages, &core_memory, "session_123", "user_123")
        .await
        .unwrap();

    assert_eq!(response.message.content, "How can I help?");
}

#[tokio::test]
async fn tool_registry_definitions() {
    let registry = create_test_tool_registry();

    let definitions = registry.definitions();
    assert!(!definitions.is_empty());

    // Check that echo tool is defined
    let echo_def = definitions.iter().find(|d| d.name == "echo");
    assert!(echo_def.is_some());

    // Check that calculator tool is defined
    let calc_def = definitions.iter().find(|d| d.name == "calculator");
    assert!(calc_def.is_some());
}

#[tokio::test]
async fn tool_registry_empty() {
    let registry = ToolRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[tokio::test]
async fn tool_execution_context() {
    use async_trait::async_trait;

    struct ContextCheckingTool;

    #[async_trait]
    impl Tool for ContextCheckingTool {
        fn name(&self) -> &str {
            "context_checker"
        }

        fn description(&self) -> &str {
            "Checks tool context"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        fn capabilities_required(&self) -> Vec<SkillCapability> {
            vec![]
        }

        async fn execute(&self, _input: Value, ctx: &ToolContext) -> Result<ToolOutput, Error> {
            Ok(ToolOutput {
                tool_call_id: String::new(),
                content: format!("session={}, user={}", ctx.session_id, ctx.user_id),
                is_error: false,
            })
        }
    }

    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ContextCheckingTool));

    let ctx = ToolContext {
        session_id: "test_session".to_string(),
        user_id: "test_user".to_string(),
        workspace_path: None,
    };

    let tool_call = ToolCall {
        id: "call_1".to_string(),
        name: "context_checker".to_string(),
        arguments: serde_json::json!({}),
    };

    let result = registry.execute(&tool_call, &ctx).await.unwrap();
    assert!(result.content.contains("test_session"));
    assert!(result.content.contains("test_user"));
}
