//! Integration tests for Azure OpenAI SDK.
//!
//! These tests require environment variables to be set:
//! - AZURE_OPENAI_API_KEY
//! - AZURE_OPENAI_RESOURCE
//! - AZURE_OPENAI_DEPLOYMENT

use azure_openai::{
    AzureOpenAIClient, AzureConfig, AzureRegion, ChatRequest,
    EmbeddingRequest, AzureOpenAIModel, AzureOpenAIError, ContentFilterResults,
};

fn create_test_client() -> Option<AzureOpenAIClient> {
    let api_key = std::env::var("AZURE_OPENAI_API_KEY").ok()?;
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE").ok()?;
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT").ok()?;

    AzureOpenAIClient::new(&resource_name, &deployment_name, api_key.clone()).ok()
}

#[test]
fn test_client_creation() {
    let client = AzureOpenAIClient::new("test-resource", "test-deployment", "test-key");
    assert!(client.is_ok());

    let client = client.unwrap();
    assert_eq!(client.resource_name(), "test-resource");
    assert_eq!(client.deployment_name(), "test-deployment");
    assert_eq!(client.base_url(), "https://test-resource.openai.azure.com".to_string());
    assert!(!client.is_azure_ad());
}

#[test]
fn test_azure_ad_client_creation() {
    let client = AzureOpenAIClient::with_azure_ad_token(
        "test-resource",
        "test-deployment",
        "test-token",
    );
    assert!(client.is_ok());

    let client = client.unwrap();
    assert!(client.is_azure_ad());
}

#[test]
fn test_config_builder() {
    let config = AzureConfig::api_key("test-key")
        .resource_name("my-resource")
        .deployment_name("my-deployment")
        .api_version("2024-06-01")
        .region(AzureRegion::EastUS)
        .max_retries(5);

    assert_eq!(config.resource_name, "my-resource");
    assert_eq!(config.deployment_name, "my-deployment");
    assert_eq!(config.api_version, "2024-06-01");
    assert_eq!(config.region, Some(AzureRegion::EastUS));
    assert_eq!(config.max_retries, 5);
}

#[test]
fn test_config_validation() {
    let config = AzureConfig::api_key("test-key");
    assert!(config.validate().is_err());

    let config = AzureConfig::api_key("test-key")
        .resource_name("my-resource")
        .deployment_name("my-deployment");
    assert!(config.validate().is_ok());
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::builder()
        .system("You are helpful")
        .user("Hello")
        .max_tokens(100)
        .temperature(0.7)
        .build();

    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.temperature, Some(0.7));
}

#[test]
fn test_embedding_request() {
    let request = EmbeddingRequest::single("Hello world");
    match &request.input {
        azure_openai::embeddings::EmbeddingInput::Single(s) => {
            assert_eq!(s, "Hello world");
        }
        _ => panic!("Expected single input"),
    }

    let request = EmbeddingRequest::new(vec!["a".to_string(), "b".to_string()]);
    match &request.input {
        azure_openai::embeddings::EmbeddingInput::Multiple(v) => {
            assert_eq!(v.len(), 2);
        }
        _ => panic!("Expected multiple inputs"),
    }
}

#[test]
fn test_azure_openai_model() {
    let model = AzureOpenAIModel::Gpt4O;
    assert_eq!(model.as_str(), "gpt-4o");
    assert_eq!(model.max_context_tokens(), 128_000);
    assert!(model.is_chat());
    assert!(!model.is_embedding());

    let embedding_model = AzureOpenAIModel::TextEmbedding3Small;
    assert!(embedding_model.is_embedding());
    assert!(!embedding_model.is_chat());
}

#[test]
fn test_azure_region() {
    let region = AzureRegion::EastUS;
    assert_eq!(region.as_str(), "eastus");
    assert_eq!(region.display_name(), "East US");

    // Test parsing
    let parsed: AzureRegion = "westeurope".parse().unwrap();
    assert_eq!(parsed, AzureRegion::WestEurope);

    let parsed: AzureRegion = "west-europe".parse().unwrap();
    assert_eq!(parsed, AzureRegion::WestEurope);
}

#[test]
fn test_error_types() {


    let auth_error = AzureOpenAIError::Authentication {
        message: "Invalid key".to_string(),
    };
    assert!(auth_error.is_auth_error());
    assert!(!auth_error.is_retryable());

    let rate_limit = AzureOpenAIError::RateLimit {
        retry_after: None,
        message: "Too many requests".to_string(),
    };
    assert!(rate_limit.is_rate_limit());
    assert!(rate_limit.is_retryable());

    let content_filtered = AzureOpenAIError::ContentFiltered {
        filter_results: ContentFilterResults::default(),
        message: "Content blocked".to_string(),
    };
    assert!(content_filtered.is_content_filtered());
}

#[test]
fn test_tool_choice() {
    use azure_openai::types::ToolChoice;

    let auto = ToolChoice::auto();
    assert!(matches!(auto, ToolChoice::Strategy(s) if s == "auto"));

    let none = ToolChoice::none();
    assert!(matches!(none, ToolChoice::Strategy(s) if s == "none"));

    let required = ToolChoice::required();
    assert!(matches!(required, ToolChoice::Strategy(s) if s == "required"));

    let function = ToolChoice::function("get_weather");
    match function {
        ToolChoice::Specific { tool_type, function } => {
            assert_eq!(tool_type, "function");
            assert_eq!(function.name, "get_weather");
        }
        _ => panic!("Expected Specific tool choice"),
    }
}

#[test]
fn test_build_url() {
    let client = AzureOpenAIClient::new("test-resource", "test-deployment", "test-key").unwrap();
    
    // build_url is private, verify through base_url instead
    let base_url = client.base_url();
    assert!(base_url.contains("https://test-resource.openai.azure.com"));
}

// Integration tests (require environment variables)

#[tokio::test]
#[ignore = "Requires Azure OpenAI credentials"]
async fn test_chat_completion() {
    let client = create_test_client().expect("Failed to create client");

    let request = ChatRequest::builder()
        .user("Say 'Hello, test!'")
        .max_tokens(50)
        .build();

    let response = client.chat().complete(request).await.expect("Request failed");
    
    assert!(!response.id.is_empty());
    assert!(!response.model.is_empty());
    assert_eq!(response.choices.len(), 1);
}

#[tokio::test]
#[ignore = "Requires Azure OpenAI credentials"]
async fn test_embeddings() {
    let client = create_test_client().expect("Failed to create client");

    let request = EmbeddingRequest::single("Hello world");
    let response = client.embeddings().create(request).await.expect("Request failed");
    
    assert!(!response.data.is_empty());
    assert!(!response.model.is_empty());
    
    let embedding = response.first().expect("No embedding returned");
    assert!(!embedding.embedding.is_empty());
}

#[tokio::test]
#[ignore = "Requires Azure OpenAI credentials"]
async fn test_streaming_chat() {
    use futures::StreamExt;

    let client = create_test_client().expect("Failed to create client");

    let request = ChatRequest::builder()
        .user("Hi")
        .max_tokens(20)
        .build();

    let chat = client.chat();
    let mut stream = chat.stream(request).await.expect("Failed to create stream");
    
    let mut received_content = false;
    while let Some(chunk) = stream.next().await {
        if let Ok(chunk) = chunk {
            if chunk.content().is_some() {
                received_content = true;
            }
            if chunk.is_done() {
                break;
            }
        }
    }
    
    assert!(received_content);
}
