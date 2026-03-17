//! Chat completions API for Perplexity's Sonar models.
//!
//! Perplexity's chat completions API provides unique features like:
//! - Built-in web search with citations
//! - Search recency filters
//! - Related questions
//! - Image return capabilities

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::client::PerplexityClient;
use crate::constants::SearchRecencyFilter;
use crate::constants::endpoints;
use crate::error::{PerplexityError, Result};
use crate::types::{
    Citation, FinishReason, Message, RelatedQuestion, ResponseFormat, TokenUsage, Tool, ToolCall,
};

/// Client for the Chat Completions API.
#[derive(Debug)]
pub struct ChatEndpoint<'a> {
    pub(crate) client: &'a PerplexityClient,
}

impl<'a> ChatEndpoint<'a> {
    /// Send a chat completion request.
    pub async fn complete(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CHAT_COMPLETIONS, body).await?;
                    let body = client.handle_response(response).await?;
                    let chat_response: ChatResponse = serde_json::from_value(body)?;
                    Ok(chat_response)
                })
            })
            .await
    }

    /// Send a streaming chat completion request.
    #[cfg(feature = "streaming")]
    pub async fn complete_stream(
        &self,
        request: ChatRequest,
    ) -> Result<crate::streaming::ChatCompletionStream> {
        use crate::streaming::ChatCompletionStream;

        let mut request = request;
        request.stream = Some(true);

        let body = serde_json::to_value(&request)?;

        debug!("Initiating streaming chat request");

        let response = self.client.post(endpoints::CHAT_COMPLETIONS, body).await?;

        if !response.status().is_success() {
            return Err(PerplexityError::from_response(response).await);
        }

        Ok(ChatCompletionStream::new(response))
    }
}

/// A chat completion request.
///
/// Perplexity's API is OpenAI-compatible with additional Perplexity-specific features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use (e.g., "llama-3.1-sonar-large-128k-online").
    pub model: String,

    /// A list of messages comprising the conversation.
    pub messages: Vec<Message>,

    /// The maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Temperature for sampling (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Stop sequences to stop generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Response format configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool choice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,

    // Perplexity-specific parameters
    /// Filter search results by recency.
    /// Only applicable for online models.
    #[serde(
        rename = "search_recency_filter",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_search_recency_filter"
    )]
    pub search_recency_filter: Option<SearchRecencyFilter>,

    /// Return related questions.
    /// Only applicable for online models.
    #[serde(
        rename = "return_related_questions",
        skip_serializing_if = "Option::is_none"
    )]
    pub return_related_questions: Option<bool>,

    /// Return images in the response.
    /// Only applicable for online models.
    #[serde(rename = "return_images", skip_serializing_if = "Option::is_none")]
    pub return_images: Option<bool>,
}

fn serialize_search_recency_filter<S>(
    filter: &Option<SearchRecencyFilter>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match filter {
        Some(f) => serializer.serialize_str(f.as_str()),
        None => serializer.serialize_none(),
    }
}

impl ChatRequest {
    /// Create a new request builder for the given model.
    pub fn builder(model: impl Into<String>) -> ChatRequestBuilder {
        ChatRequestBuilder::new(model)
    }

    /// Create a simple request with a single user message.
    pub fn simple(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::builder(model).message(Message::user(message)).build()
    }
}

/// Builder for chat completion requests.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<usize>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    response_format: Option<ResponseFormat>,
    seed: Option<i64>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<String>,
    search_recency_filter: Option<SearchRecencyFilter>,
    return_related_questions: Option<bool>,
    return_images: Option<bool>,
}

impl ChatRequestBuilder {
    /// Create a new builder for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            response_format: None,
            seed: None,
            tools: None,
            tool_choice: None,
            search_recency_filter: None,
            return_related_questions: None,
            return_images: None,
        }
    }

    /// Add a message.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a system message.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// Add a user message.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add an assistant message.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Set all messages.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Set the maximum number of tokens.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set the top-k parameter.
    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        self
    }

    /// Add a stop sequence.
    pub fn stop(mut self, stop: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(stop.into());
        self
    }

    /// Set all stop sequences.
    pub fn stop_sequences(mut self, stops: Vec<String>) -> Self {
        self.stop = Some(stops);
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty.clamp(-2.0, 2.0));
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Enable JSON mode.
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Add a tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set all tools.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice.
    pub fn tool_choice(mut self, choice: impl Into<String>) -> Self {
        self.tool_choice = Some(choice.into());
        self
    }

    /// Set the search recency filter (Perplexity-specific).
    /// Only applicable for online models.
    pub fn search_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.search_recency_filter = Some(filter);
        self
    }

    /// Enable returning related questions (Perplexity-specific).
    /// Only applicable for online models.
    pub fn return_related_questions(mut self, enabled: bool) -> Self {
        self.return_related_questions = Some(enabled);
        self
    }

    /// Enable returning images (Perplexity-specific).
    /// Only applicable for online models.
    pub fn return_images(mut self, enabled: bool) -> Self {
        self.return_images = Some(enabled);
        self
    }

    /// Build the request.
    pub fn build(self) -> ChatRequest {
        ChatRequest {
            model: self.model,
            messages: self.messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            response_format: self.response_format,
            seed: self.seed,
            tools: self.tools,
            tool_choice: self.tool_choice,
            search_recency_filter: self.search_recency_filter,
            return_related_questions: self.return_related_questions,
            return_images: self.return_images,
        }
    }
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The unique identifier for the completion.
    pub id: String,

    /// The object type (always "chat.completion").
    pub object: String,

    /// The Unix timestamp when the completion was created.
    pub created: u64,

    /// The model used for the completion.
    pub model: String,

    /// The list of completion choices.
    pub choices: Vec<Choice>,

    /// Token usage information.
    pub usage: TokenUsage,

    /// Citations for the response (Perplexity-specific).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,

    /// Related questions (Perplexity-specific).
    #[serde(
        default,
        rename = "related_questions",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub related_questions: Vec<RelatedQuestion>,
}

impl ChatResponse {
    /// Get the content of the first choice.
    pub fn content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }

    /// Get the finish reason of the first choice.
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.choices.first().and_then(|c| c.finish_reason)
    }

    /// Check if the response has citations.
    pub fn has_citations(&self) -> bool {
        !self.citations.is_empty()
    }

    /// Get the citations.
    pub fn get_citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Check if the response has related questions.
    pub fn has_related_questions(&self) -> bool {
        !self.related_questions.is_empty()
    }

    /// Get the related questions.
    pub fn get_related_questions(&self) -> &[RelatedQuestion] {
        &self.related_questions
    }
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// The index of the choice.
    pub index: usize,

    /// The message generated by the model.
    pub message: ResponseMessage,

    /// The reason the completion finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// A message in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    /// The role of the message author.
    pub role: String,

    /// The content of the message.
    pub content: String,

    /// Tool calls in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
            .user("What is Rust?")
            .temperature(0.7)
            .max_tokens(1024)
            .build();

        assert_eq!(request.model, "llama-3.1-sonar-small-128k-online");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1024));
    }

    #[test]
    fn test_builder_perplexity_specific() {
        let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
            .user("Latest AI news")
            .search_recency_filter(SearchRecencyFilter::Week)
            .return_related_questions(true)
            .return_images(true)
            .build();

        assert!(request.search_recency_filter.is_some());
        assert_eq!(request.return_related_questions, Some(true));
        assert_eq!(request.return_images, Some(true));
    }

    #[test]
    fn test_builder_chain() {
        let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
            .system("You are a helpful assistant.")
            .user("What is the capital of France?")
            .assistant("The capital of France is Paris.")
            .user("What about Germany?")
            .temperature(0.5)
            .json_mode()
            .build();

        assert_eq!(request.messages.len(), 4);
        assert!(request.response_format.is_some());
    }

    #[test]
    fn test_response_helpers() {
        let response = ChatResponse {
            id: "resp_123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".to_string(),
                    content: "Paris is the capital of France.".to_string(),
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            citations: vec![Citation {
                index: 1,
                url: "https://example.com".to_string(),
                title: Some("Example".to_string()),
                published_date: None,
            }],
            related_questions: vec![RelatedQuestion {
                question: "What is the population of Paris?".to_string(),
            }],
        };

        assert_eq!(response.content(), Some("Paris is the capital of France."));
        assert_eq!(response.finish_reason(), Some(FinishReason::Stop));
        assert!(response.has_citations());
        assert_eq!(response.get_citations().len(), 1);
        assert!(response.has_related_questions());
        assert_eq!(response.get_related_questions().len(), 1);
    }
}
