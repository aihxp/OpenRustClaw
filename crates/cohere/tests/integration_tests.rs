//! Integration tests for the Cohere SDK.

#[cfg(feature = "classify")]
use cohere::Example;
#[cfg(feature = "rerank")]
use cohere::RerankRequest;
use cohere::{
    ChatRequest, ClientConfig, CohereClient, Document, EmbedRequest, EmbeddingModel, InputType,
    Message, MessageRole, Model,
};
use std::time::Duration;

#[test]
fn test_client_creation() {
    let client = CohereClient::new("test-key").unwrap();
    assert_eq!(client.base_url(), "https://api.cohere.com");
    assert_eq!(client.api_version(), "v1");
}

#[test]
fn test_client_config() {
    let config = ClientConfig::new("test-key")
        .base_url("https://custom.api.com")
        .api_version("v2")
        .max_retries(5)
        .timeout(Duration::from_secs(60));

    // Verify config was created correctly
    assert_eq!(config.get_base_url(), "https://custom.api.com");
    assert_eq!(config.get_api_version(), "v2");
    assert_eq!(config.get_max_retries(), 5);
    assert_eq!(config.get_timeout(), Duration::from_secs(60));
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::builder("command-r")
        .message("Hello")
        .temperature(0.7)
        .max_tokens(1024)
        .preamble("You are helpful.")
        .add_message(MessageRole::User, "Previous message")
        .build();

    assert_eq!(request.model, "command-r");
    assert_eq!(request.message, "Hello");
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1024));
    assert!(request.preamble.is_some());
    assert!(request.chat_history.is_some());
}

#[test]
fn test_chat_request_with_documents() {
    let docs = vec![
        Document::new("doc1", "Content 1").with_title("Title 1"),
        Document::new("doc2", "Content 2").with_title("Title 2"),
    ];

    let request = ChatRequest::builder("command-r")
        .message("What is this?")
        .documents(docs)
        .build();

    assert!(request.documents.is_some());
    assert_eq!(request.documents.as_ref().unwrap().len(), 2);
}

#[test]
fn test_embed_request_builder() {
    let request = EmbedRequest::builder()
        .model("embed-english-v3.0")
        .add_text("Hello world")
        .add_text("Second text")
        .input_type(InputType::SearchDocument)
        .build();

    assert_eq!(request.model, "embed-english-v3.0");
    assert_eq!(request.texts.len(), 2);
    assert_eq!(request.input_type, Some(InputType::SearchDocument));
}

#[cfg(feature = "rerank")]
#[test]
fn test_rerank_request_builder() {
    let docs = vec!["doc1".to_string(), "doc2".to_string(), "doc3".to_string()];

    let request = RerankRequest::builder()
        .model("rerank-english-v3.0")
        .query("Test query")
        .documents(docs)
        .top_n(2)
        .return_documents(true)
        .build();

    assert_eq!(request.model, "rerank-english-v3.0");
    assert_eq!(request.query, "Test query");
    assert_eq!(request.documents.len(), 3);
    assert_eq!(request.top_n, Some(2));
    assert_eq!(request.return_documents, Some(true));
}

#[test]
fn test_message_creation() {
    let user_msg = Message::user("Hello");
    assert_eq!(user_msg.role, MessageRole::User);
    assert_eq!(user_msg.content, "Hello");

    let assistant_msg = Message::assistant("Hi there!");
    assert_eq!(assistant_msg.role, MessageRole::Assistant);

    let system_msg = Message::system("You are helpful.");
    assert_eq!(system_msg.role, MessageRole::System);
}

#[test]
fn test_document_creation() {
    let doc = Document::new("doc1", "Content")
        .with_title("Title")
        .with_metadata("author", serde_json::json!("John"));

    assert_eq!(doc.id, "doc1");
    assert_eq!(doc.text, "Content");
    assert_eq!(doc.title, Some("Title".to_string()));
    assert!(doc.extra.is_some());
}

#[cfg(feature = "classify")]
#[test]
fn test_example_creation() {
    let example = Example::new("This is great!", "positive");
    assert_eq!(example.text, "This is great!");
    assert_eq!(example.label, "positive");
}

#[test]
fn test_model_constants() {
    assert_eq!(Model::CommandR.as_str(), "command-r");
    assert_eq!(Model::CommandRPlus.as_str(), "command-r-plus");
    assert_eq!(Model::Command.as_str(), "command");

    // Test model parsing
    let model: Model = "command-r".parse().unwrap();
    assert_eq!(model, Model::CommandR);

    assert_eq!(
        EmbeddingModel::EmbedEnglishV3.as_str(),
        "embed-english-v3.0"
    );
    assert_eq!(
        EmbeddingModel::EmbedMultilingualV3.as_str(),
        "embed-multilingual-v3.0"
    );
}

#[test]
fn test_error_types() {
    use cohere::CohereError;

    let auth_error = CohereError::Authentication {
        message: "Invalid key".to_string(),
    };
    assert!(auth_error.is_auth_error());
    assert!(!auth_error.is_retryable());

    let rate_limit = CohereError::RateLimit {
        retry_after: None,
        message: "Too many requests".to_string(),
    };
    assert!(rate_limit.is_rate_limit());
    assert!(rate_limit.is_retryable());
}

#[test]
fn test_finish_reason() {
    use cohere::FinishReason;

    let complete = FinishReason::Complete;
    let max_tokens = FinishReason::MaxTokens;

    // Test that these are distinct variants
    assert_ne!(
        std::mem::discriminant(&complete),
        std::mem::discriminant(&max_tokens)
    );
}

#[test]
fn test_constants() {
    use cohere::{DEFAULT_API_VERSION, DEFAULT_BASE_URL};

    assert_eq!(DEFAULT_BASE_URL, "https://api.cohere.com");
    assert_eq!(DEFAULT_API_VERSION, "v1");
}

#[test]
fn test_connector() {
    use cohere::types::Connector;

    let connector = Connector::new("web-search")
        .with_access_token("token123")
        .continue_on_failure(true);

    assert_eq!(connector.id, "web-search");
    assert_eq!(connector.user_access_token, Some("token123".to_string()));
    assert_eq!(connector.continue_on_failure, Some(true));
}
