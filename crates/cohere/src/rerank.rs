//! Rerank API for Cohere's reranking models.

use serde::{Deserialize, Serialize};

use crate::client::CohereClient;
use crate::constants::endpoints;
use crate::error::Result;
use crate::types::ApiMeta;

/// Client for the Rerank API.
#[derive(Debug)]
pub struct RerankEndpoint<'a> {
    pub(crate) client: &'a CohereClient,
}

impl<'a> RerankEndpoint<'a> {
    /// Send a rerank request.
    pub async fn create(&self, request: RerankRequest) -> Result<RerankResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::RERANK, body).await?;
                    let body = client.handle_response(response).await?;
                    let rerank_response: RerankResponse = serde_json::from_value(body)?;
                    Ok(rerank_response)
                })
            })
            .await
    }
}

/// A request to the Cohere Rerank API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    /// The model to use (e.g., "rerank-english-v3.0").
    pub model: String,

    /// The search query.
    pub query: String,

    /// The documents to rerank.
    pub documents: Vec<String>,

    /// The number of top results to return.
    #[serde(rename = "top_n", skip_serializing_if = "Option::is_none")]
    pub top_n: Option<usize>,

    /// Maximum number of chunks per document.
    #[serde(rename = "max_chunks_per_doc", skip_serializing_if = "Option::is_none")]
    pub max_chunks_per_doc: Option<usize>,

    /// Whether to return documents.
    #[serde(rename = "return_documents", skip_serializing_if = "Option::is_none")]
    pub return_documents: Option<bool>,
}

impl RerankRequest {
    /// Create a new request builder.
    pub fn builder() -> RerankRequestBuilder {
        RerankRequestBuilder::new()
    }

    /// Create a simple rerank request.
    pub fn simple(
        model: impl Into<String>,
        query: impl Into<String>,
        documents: Vec<String>,
    ) -> Self {
        Self::builder()
            .model(model)
            .query(query)
            .documents(documents)
            .build()
    }
}

/// Builder for rerank requests.
#[derive(Debug, Clone)]
pub struct RerankRequestBuilder {
    model: Option<String>,
    query: String,
    documents: Vec<String>,
    top_n: Option<usize>,
    max_chunks_per_doc: Option<usize>,
    return_documents: Option<bool>,
}

impl RerankRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            model: None,
            query: String::new(),
            documents: Vec::new(),
            top_n: None,
            max_chunks_per_doc: None,
            return_documents: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the query.
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Add a document.
    pub fn add_document(mut self, document: impl Into<String>) -> Self {
        self.documents.push(document.into());
        self
    }

    /// Set the documents.
    pub fn documents(mut self, documents: Vec<String>) -> Self {
        self.documents = documents;
        self
    }

    /// Set the number of top results to return.
    pub fn top_n(mut self, top_n: usize) -> Self {
        self.top_n = Some(top_n);
        self
    }

    /// Set the maximum chunks per document.
    pub fn max_chunks_per_doc(mut self, max: usize) -> Self {
        self.max_chunks_per_doc = Some(max);
        self
    }

    /// Set whether to return documents.
    pub fn return_documents(mut self, return_docs: bool) -> Self {
        self.return_documents = Some(return_docs);
        self
    }

    /// Build the request.
    pub fn build(self) -> RerankRequest {
        RerankRequest {
            model: self.model.unwrap_or_else(|| "rerank-english-v3.0".to_string()),
            query: self.query,
            documents: self.documents,
            top_n: self.top_n,
            max_chunks_per_doc: self.max_chunks_per_doc,
            return_documents: self.return_documents,
        }
    }
}

impl Default for RerankRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A response from the Cohere Rerank API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    /// The ID of the response.
    pub id: String,

    /// The reranked results.
    pub results: Vec<RerankResult>,

    /// API metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
}

impl RerankResponse {
    /// Get the top result.
    pub fn top(&self) -> Option<&RerankResult> {
        self.results.first()
    }

    /// Get results by index.
    pub fn get(&self, index: usize) -> Option<&RerankResult> {
        self.results.get(index)
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if there are no results.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

/// A single rerank result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// The index of the document in the original list.
    pub index: usize,

    /// The relevance score.
    #[serde(rename = "relevance_score")]
    pub relevance_score: f32,

    /// The document text (if return_documents was true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<RerankDocument>,
}

/// Document in a rerank result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankDocument {
    /// The document text.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = RerankRequest::builder()
            .model("rerank-english-v3.0")
            .query("What is machine learning?")
            .add_document("Machine learning is...")
            .add_document("Deep learning is...")
            .top_n(5)
            .build();

        assert_eq!(request.model, "rerank-english-v3.0");
        assert_eq!(request.query, "What is machine learning?");
        assert_eq!(request.documents.len(), 2);
        assert_eq!(request.top_n, Some(5));
    }

    #[test]
    fn test_builder_documents() {
        let docs = vec!["doc1".to_string(), "doc2".to_string(), "doc3".to_string()];
        let request = RerankRequest::builder()
            .model("rerank-multilingual-v3.0")
            .query("test")
            .documents(docs)
            .return_documents(true)
            .build();

        assert_eq!(request.documents.len(), 3);
        assert_eq!(request.return_documents, Some(true));
    }

    #[test]
    fn test_response_helpers() {
        let response = RerankResponse {
            id: "rerank_123".to_string(),
            results: vec![
                RerankResult {
                    index: 1,
                    relevance_score: 0.95,
                    document: Some(RerankDocument {
                        text: "document 1".to_string(),
                    }),
                },
                RerankResult {
                    index: 0,
                    relevance_score: 0.85,
                    document: Some(RerankDocument {
                        text: "document 0".to_string(),
                    }),
                },
            ],
            meta: None,
        };

        assert_eq!(response.len(), 2);
        let top = response.top().unwrap();
        assert_eq!(top.index, 1);
        assert_eq!(top.relevance_score, 0.95);
    }
}
