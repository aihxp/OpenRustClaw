//! RAG (Retrieval-Augmented Generation) API for AI21's Contextual Answers.

use serde::{Deserialize, Serialize};

use crate::client::Ai21Client;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::Document;

/// Client for the RAG API.
#[derive(Debug)]
pub struct RagEndpoint<'a> {
    pub(crate) client: &'a Ai21Client,
}

impl<'a> RagEndpoint<'a> {
    /// Send a contextual answers request.
    pub async fn contextual_answers(
        &self,
        request: ContextualAnswersRequest,
    ) -> Result<ContextualAnswersResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::CONTEXTUAL_ANSWERS, body).await?;
                    let body = client.handle_response(response).await?;
                    let answer_response: ContextualAnswersResponse = serde_json::from_value(body)?;
                    Ok(answer_response)
                })
            })
            .await
    }
}

/// A request to the Contextual Answers API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualAnswersRequest {
    /// The question to answer.
    pub question: String,

    /// The documents to use as context.
    pub context: Vec<Document>,
}

impl ContextualAnswersRequest {
    /// Create a new contextual answers request.
    pub fn new(question: impl Into<String>, documents: Vec<Document>) -> Self {
        Self {
            question: question.into(),
            context: documents,
        }
    }

    /// Create a new request builder.
    pub fn builder() -> ContextualAnswersRequestBuilder {
        ContextualAnswersRequestBuilder::new()
    }
}

/// Builder for contextual answers requests.
#[derive(Debug, Clone, Default)]
pub struct ContextualAnswersRequestBuilder {
    question: String,
    context: Vec<Document>,
}

impl ContextualAnswersRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the question.
    pub fn question(mut self, question: impl Into<String>) -> Self {
        self.question = question.into();
        self
    }

    /// Add a document to the context.
    pub fn add_document(mut self, document: Document) -> Self {
        self.context.push(document);
        self
    }

    /// Set the context documents.
    pub fn context(mut self, documents: Vec<Document>) -> Self {
        self.context = documents;
        self
    }

    /// Build the request.
    pub fn build(self) -> ContextualAnswersRequest {
        ContextualAnswersRequest {
            question: self.question,
            context: self.context,
        }
    }
}

/// A response from the Contextual Answers API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualAnswersResponse {
    /// The answer to the question.
    pub answer: String,

    /// The confidence score (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,

    /// The ID of the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl ContextualAnswersResponse {
    /// Get the answer text.
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// Check if the answer is empty or indicates no answer found.
    pub fn is_empty(&self) -> bool {
        self.answer.trim().is_empty()
            || self.answer.to_lowercase().contains("no answer")
            || self.answer.to_lowercase().contains("not found")
    }

    /// Get the confidence score if available.
    pub fn confidence(&self) -> Option<f32> {
        self.confidence
    }
}

/// Request for the Segments API (experimental RAG feature).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentsRequest {
    /// The text to segment.
    pub text: String,

    /// The maximum segment length in tokens.
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// The minimum segment length in tokens.
    #[serde(rename = "minTokens", skip_serializing_if = "Option::is_none")]
    pub min_tokens: Option<usize>,
}

impl SegmentsRequest {
    /// Create a new segments request.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            max_tokens: None,
            min_tokens: None,
        }
    }

    /// Set the maximum tokens per segment.
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the minimum tokens per segment.
    pub fn min_tokens(mut self, min_tokens: usize) -> Self {
        self.min_tokens = Some(min_tokens);
        self
    }
}

/// Response from the Segments API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentsResponse {
    /// The segments extracted from the text.
    pub segments: Vec<Segment>,
}

/// A segment of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// The segment text.
    pub text: String,

    /// The start index in the original text.
    pub start: usize,

    /// The end index in the original text.
    pub end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let docs = vec![
            Document::new("doc1", "Paris is the capital of France."),
            Document::new("doc2", "Berlin is the capital of Germany."),
        ];

        let request = ContextualAnswersRequest::new("What is the capital of France?", docs);

        assert_eq!(request.question, "What is the capital of France?");
        assert_eq!(request.context.len(), 2);
    }

    #[test]
    fn test_builder() {
        let request = ContextualAnswersRequest::builder()
            .question("What is AI?")
            .add_document(Document::new("doc1", "AI is artificial intelligence."))
            .add_document(Document::new("doc2", "AI can learn from data."))
            .build();

        assert_eq!(request.question, "What is AI?");
        assert_eq!(request.context.len(), 2);
    }

    #[test]
    fn test_response_helpers() {
        let response = ContextualAnswersResponse {
            answer: "Paris".to_string(),
            confidence: Some(0.95),
            id: Some("ans_123".to_string()),
        };

        assert_eq!(response.answer(), "Paris");
        assert_eq!(response.confidence(), Some(0.95));
        assert!(!response.is_empty());

        let empty_response = ContextualAnswersResponse {
            answer: "No answer found".to_string(),
            confidence: None,
            id: None,
        };
        assert!(empty_response.is_empty());
    }

    #[test]
    fn test_segments_request() {
        let request = SegmentsRequest::new("Long text to segment")
            .max_tokens(100)
            .min_tokens(10);

        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.min_tokens, Some(10));
    }
}
