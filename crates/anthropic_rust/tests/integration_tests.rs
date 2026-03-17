//! Integration tests for anthropic-rust.
//!
//! These tests use wiremock to mock the Anthropic API.

use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use anthropic_rust::{
    AnthropicClient, Message, MessageRequest, MessageResponse, MessageRole, Tool, ToolResult,
};

async fn setup_mock_server() -> (MockServer, AnthropicClient) {
    let mock_server = MockServer::start().await;
    let client = AnthropicClient::with_config(
        anthropic_rust::ClientConfig::new("test-key")
            .base_url(mock_server.uri())
            .max_retries(0), // Disable retries for tests
    )
    .unwrap();

    (mock_server, client)
}

#[tokio::test]
async fn test_simple_completion() {
    let (mock_server, client) = setup_mock_server().await;

    let mock_response = serde_json::json!({
        "id": "msg_01AbCdEfGhIjKlMnOpQrStUv",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello! How can I help you today?"}],
        "model": "claude-3-sonnet-20240229",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {"input_tokens": 10, "output_tokens": 20}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let request = MessageRequest::simple("claude-3-sonnet-20240229", "Hello!");
    let response = client.messages().create(request).await.unwrap();

    assert_eq!(response.id, "msg_01AbCdEfGhIjKlMnOpQrStUv");
    assert_eq!(response.text(), "Hello! How can I help you today?");
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 20);
}

#[tokio::test]
async fn test_completion_with_tools() {
    let (mock_server, client) = setup_mock_server().await;

    let mock_response = serde_json::json!({
        "id": "msg_02AbCdEfGhIjKlMnOpQrStUv",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "text", "text": "I'll check the weather for you."},
            {
                "type": "tool_use",
                "id": "toolu_01AbCdEfGhIjKlMnOpQrStUv",
                "name": "get_weather",
                "input": {"location": "San Francisco"}
            }
        ],
        "model": "claude-3-sonnet-20240229",
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 50, "output_tokens": 30}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let tool = Tool::builder("get_weather", "Get weather information")
        .string_property("location", "City name", true)
        .build();

    let request = MessageRequest::builder("claude-3-sonnet-20240229")
        .user("What's the weather?")
        .tool(tool)
        .max_tokens(1024)
        .build();

    let response = client.messages().create(request).await.unwrap();

    assert!(response.has_tool_use());
    assert!(response.stopped_for_tool());

    let tool_uses = response.tool_uses();
    assert_eq!(tool_uses.len(), 1);
    assert_eq!(tool_uses[0].name, "get_weather");
    assert_eq!(tool_uses[0].get_string("location"), Some("San Francisco"));
}

#[tokio::test]
async fn test_rate_limit_handling() {
    let (mock_server, client) = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "2")
                .set_body_json(serde_json::json!({
                    "error": {
                        "type": "rate_limit_error",
                        "message": "Rate limit exceeded"
                    }
                })),
        )
        .mount(&mock_server)
        .await;

    let request = MessageRequest::simple("claude-3-sonnet-20240229", "Hello");
    let result = client.messages().create(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Rate limit") || err.to_string().contains("rate limit"));
}

#[tokio::test]
async fn test_authentication_error() {
    let (mock_server, client) = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "type": "authentication_error",
                    "message": "Invalid API key"
                }
            })),
        )
        .mount(&mock_server)
        .await;

    let request = MessageRequest::simple("claude-3-sonnet-20240229", "Hello");
    let result = client.messages().create(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Authentication"));
}

#[tokio::test]
async fn test_conversation_thread() {
    let (mock_server, client) = setup_mock_server().await;

    // First response - asking for clarification
    let response1 = serde_json::json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Which city would you like the weather for?"}],
        "model": "claude-3-sonnet-20240229",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 15, "output_tokens": 10}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_json(serde_json::json!({
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": [{"type": "text", "text": "What's the weather?"}]}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response1))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response - actual weather
    let response2 = serde_json::json!({
        "id": "msg_02",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "The weather in San Francisco is sunny, 22°C."}],
        "model": "claude-3-sonnet-20240229",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 30, "output_tokens": 15}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response2))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // First turn
    let request1 = MessageRequest::builder("claude-3-sonnet-20240229")
        .user("What's the weather?")
        .max_tokens(1024)
        .build();

    let resp1 = client.messages().create(request1).await.unwrap();
    assert_eq!(resp1.text(), "Which city would you like the weather for?");

    // Second turn - include conversation history
    let request2 = MessageRequest::builder("claude-3-sonnet-20240229")
        .user("What's the weather?")
        .assistant(resp1.text())
        .user("San Francisco please")
        .max_tokens(1024)
        .build();

    let resp2 = client.messages().create(request2).await.unwrap();
    assert_eq!(resp2.text(), "The weather in San Francisco is sunny, 22°C.");
}

#[tokio::test]
async fn test_message_roles() {
    let msg_user = Message::user("Hello");
    assert_eq!(msg_user.role, MessageRole::User);

    let msg_assistant = Message::assistant("Hi there!");
    assert_eq!(msg_assistant.role, MessageRole::Assistant);
}

#[tokio::test]
async fn test_tool_result_creation() {
    let result = ToolResult::success("tool_123", "The result");
    assert_eq!(result.tool_use_id, "tool_123");
    assert_eq!(result.content, "The result");
    assert_eq!(result.is_error, Some(false));

    let error_result = ToolResult::error("tool_123", "Something went wrong");
    assert_eq!(error_result.is_error, Some(true));
}

#[tokio::test]
async fn test_request_builder() {
    let request = MessageRequest::builder("claude-3-opus-20240229")
        .system("You are a helpful assistant.")
        .user("Hello")
        .max_tokens(2048)
        .temperature(0.5)
        .top_p(0.9)
        .stream(true)
        .build();

    assert_eq!(request.model, "claude-3-opus-20240229");
    assert_eq!(request.max_tokens, 2048);
    assert_eq!(request.temperature, Some(0.5));
    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.stream, Some(true));
    assert!(request.system.is_some());
}
