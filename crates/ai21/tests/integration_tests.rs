//! Integration tests for the AI21 SDK.

use ai21::{
    Ai21Client, ChatRequest, CompletionRequest, ContextualAnswersRequest, Document, Message,
    Penalty, TokenizeRequest,
};

// Mock tests using wiremock
#[cfg(test)]
mod mock_tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::*};

    #[tokio::test]
    async fn test_chat_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/studio/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chat_123",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "jamba-1.5-large",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Paris is the capital of France."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 7,
                    "total_tokens": 17
                }
            })))
            .mount(&mock_server)
            .await;

        let client = Ai21Client::with_config(
            ai21::ClientConfig::new("test-key").base_url(&mock_server.uri()),
        )
        .unwrap();

        let request = ChatRequest::builder("jamba-1.5-large")
            .messages(vec![Message::user("What is the capital of France?")])
            .build();

        let response = client.chat().create(request).await.unwrap();

        assert_eq!(response.id, "chat_123");
        assert_eq!(response.model, "jamba-1.5-large");
        assert_eq!(response.content(), Some("Paris is the capital of France."));
        assert_eq!(response.usage.total_tokens, 17);
    }

    #[tokio::test]
    async fn test_completion_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/studio/v1/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "comp_123",
                "prompt": {
                    "text": "The capital of France is",
                    "tokens": []
                },
                "completions": [{
                    "data": {
                        "text": " Paris.",
                        "tokens": []
                    },
                    "tokens": []
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = Ai21Client::with_config(
            ai21::ClientConfig::new("test-key").base_url(&mock_server.uri()),
        )
        .unwrap();

        let request = CompletionRequest::builder("j2-ultra")
            .prompt("The capital of France is")
            .max_tokens(10)
            .build();

        let response = client.completions().create(request).await.unwrap();

        assert_eq!(response.id, "comp_123");
        assert_eq!(response.text(), Some(" Paris."));
    }

    #[tokio::test]
    async fn test_contextual_answers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/studio/v1/contextualAnswers"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "answer": "Paris",
                "confidence": 0.95,
                "id": "ans_123"
            })))
            .mount(&mock_server)
            .await;

        let client = Ai21Client::with_config(
            ai21::ClientConfig::new("test-key").base_url(&mock_server.uri()),
        )
        .unwrap();

        let documents = vec![Document::new("doc1", "The capital of France is Paris.")];

        let request = ContextualAnswersRequest::new("What is the capital of France?", documents);

        let response = client.rag().contextual_answers(request).await.unwrap();

        assert_eq!(response.answer, "Paris");
        assert_eq!(response.confidence, Some(0.95));
        assert_eq!(response.id, Some("ans_123".to_string()));
    }

    #[tokio::test]
    async fn test_tokenize() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/studio/v1/tokenize"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tokens": [
                    {"token": 100, "text": "Hello"},
                    {"token": 101, "text": " world"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = Ai21Client::with_config(
            ai21::ClientConfig::new("test-key").base_url(&mock_server.uri()),
        )
        .unwrap();

        let request = TokenizeRequest::new("j2-ultra", "Hello world");

        let response = client.tokenize().tokenize(request).await.unwrap();

        assert_eq!(response.tokens.len(), 2);
        assert_eq!(response.token_ids(), vec![100, 101]);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/studio/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "detail": "Invalid API key"
            })))
            .mount(&mock_server)
            .await;

        let client = Ai21Client::with_config(
            ai21::ClientConfig::new("invalid-key").base_url(&mock_server.uri()),
        )
        .unwrap();

        let request = ChatRequest::builder("jamba-1.5-large")
            .messages(vec![Message::user("Hello")])
            .build();

        let result = client.chat().create(request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Authentication") || err.to_string().contains("401"));
    }
}

// Unit tests (no network required)
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::builder("jamba-1.5-large")
            .system_message("You are helpful.")
            .user_message("Hello")
            .temperature(0.7)
            .max_tokens(100)
            .json_mode()
            .build();

        assert_eq!(request.model, "jamba-1.5-large");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert!(request.response_format.is_some());
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::builder("j2-ultra")
            .prompt("Test")
            .temperature(0.5)
            .presence_penalty(Penalty::new(0.3).with_numbers(true))
            .build();

        assert_eq!(request.model, "j2-ultra");
        assert_eq!(request.prompt, "Test");
        assert_eq!(request.temperature, Some(0.5));
        assert!(request.presence_penalty.is_some());
    }

    #[test]
    fn test_contextual_answers_builder() {
        let request = ContextualAnswersRequest::builder()
            .question("What is AI?")
            .add_document(Document::new("doc1", "AI is artificial intelligence."))
            .build();

        assert_eq!(request.question, "What is AI?");
        assert_eq!(request.context.len(), 1);
    }

    #[test]
    fn test_document_creation() {
        let doc =
            Document::new("id1", "content").with_metadata(serde_json::json!({"key": "value"}));

        assert_eq!(doc.id, Some("id1".to_string()));
        assert_eq!(doc.text, "content");
        assert!(doc.metadata.is_some());
    }

    #[test]
    fn test_penalty_builder() {
        let penalty = Penalty::new(1.5)
            .with_numbers(true)
            .with_punctuation(false)
            .with_emojis(true);

        assert_eq!(penalty.scale, Some(1.5));
        assert_eq!(penalty.apply_to_numbers, Some(true));
        assert_eq!(penalty.apply_to_punctuation, Some(false));
        assert_eq!(penalty.apply_to_emojis, Some(true));
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello").with_name("Alice");

        assert_eq!(msg.role, ai21::MessageRole::User);
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.name, Some("Alice".to_string()));
    }
}
