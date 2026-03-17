//! Batch API for Azure OpenAI.
//!
//! The Batch API allows you to send multiple requests in a single file
//! for asynchronous processing at 50% lower cost.

use crate::client::AzureOpenAIClient;
use crate::error::Result;

/// Client for the batch API.
#[derive(Debug)]
pub struct Batch<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Batch<'a> {
    /// Create a new batch client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Create a batch.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, BatchRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = BatchRequest::new("file-abc123", "/v1/chat/completions");
    /// let batch = client.batch().create(request).await?;
    /// println!("Batch ID: {}", batch.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: BatchRequest) -> Result<BatchResponse> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post("/openai/batches", body).await?;
                    let body = client.handle_response(response).await?;
                    let batch: BatchResponse = serde_json::from_value(body)?;
                    Ok(batch)
                })
            })
            .await
    }

    /// Retrieve a batch.
    pub async fn retrieve(&self, batch_id: &str) -> Result<BatchResponse> {
        let path = format!("{}/{}", "/openai/batches", batch_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let batch: BatchResponse = serde_json::from_value(body)?;
                    Ok(batch)
                })
            })
            .await
    }

    /// Cancel a batch.
    pub async fn cancel(&self, batch_id: &str) -> Result<BatchResponse> {
        let path = format!("{}/{}/cancel", "/openai/batches", batch_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, serde_json::json!({})).await?;
                    let body = client.handle_response(response).await?;
                    let batch: BatchResponse = serde_json::from_value(body)?;
                    Ok(batch)
                })
            })
            .await
    }

    /// List batches.
    pub async fn list(&self, limit: Option<usize>) -> Result<BatchList> {
        let mut path = "/openai/batches".to_string();
        if let Some(limit) = limit {
            path.push_str(&format!("?limit={}", limit));
        }

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: BatchList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }
}

/// A batch request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchRequest {
    /// The ID of the input file.
    pub input_file_id: String,
    /// The endpoint to use.
    pub endpoint: String,
    /// The completion window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_window: Option<String>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl BatchRequest {
    /// Create a new batch request.
    pub fn new(input_file_id: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            input_file_id: input_file_id.into(),
            endpoint: endpoint.into(),
            completion_window: Some("24h".to_string()),
            metadata: None,
        }
    }

    /// Set the completion window.
    pub fn completion_window(mut self, window: impl Into<String>) -> Self {
        self.completion_window = Some(window.into());
        self
    }

    /// Set the metadata.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// A batch response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchResponse {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The endpoint.
    pub endpoint: String,
    /// The errors (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<BatchErrors>,
    /// The input file ID.
    pub input_file_id: String,
    /// The completion window.
    pub completion_window: String,
    /// The status.
    pub status: BatchStatus,
    /// The output file ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file_id: Option<String>,
    /// The error file ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_file_id: Option<String>,
    /// The creation timestamp.
    pub created_at: i64,
    /// The in-progress at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_progress_at: Option<i64>,
    /// The expires at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// The finalizing at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalizing_at: Option<i64>,
    /// The completed at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    /// The failed at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<i64>,
    /// The expired at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<i64>,
    /// The cancelling at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelling_at: Option<i64>,
    /// The cancelled at timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<i64>,
    /// The request counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_counts: Option<RequestCounts>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Batch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Validating.
    Validating,
    /// Failed.
    Failed,
    /// In progress.
    InProgress,
    /// Finalizing.
    Finalizing,
    /// Completed.
    Completed,
    /// Expired.
    Expired,
    /// Cancelling.
    Cancelling,
    /// Cancelled.
    Cancelled,
}

impl BatchStatus {
    /// Check if the batch is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BatchStatus::Completed
                | BatchStatus::Failed
                | BatchStatus::Expired
                | BatchStatus::Cancelled
        )
    }

    /// Check if the batch is processing.
    pub fn is_processing(&self) -> bool {
        matches!(
            self,
            BatchStatus::Validating | BatchStatus::InProgress | BatchStatus::Finalizing
        )
    }
}

/// Batch errors.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchErrors {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<BatchError>,
}

/// A batch error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
    /// The request ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// The line number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// Request counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestCounts {
    /// Total requests.
    pub total: usize,
    /// Completed requests.
    pub completed: usize,
    /// Failed requests.
    pub failed: usize,
}

/// A batch list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<BatchResponse>,
    /// The first ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    /// The last ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    /// Whether there are more items.
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_builder() {
        let request =
            BatchRequest::new("file-123", "/v1/chat/completions").completion_window("24h");

        assert_eq!(request.input_file_id, "file-123");
        assert_eq!(request.endpoint, "/v1/chat/completions");
        assert_eq!(request.completion_window, Some("24h".to_string()));
    }

    #[test]
    fn test_batch_status() {
        assert!(BatchStatus::Completed.is_terminal());
        assert!(BatchStatus::Failed.is_terminal());
        assert!(!BatchStatus::InProgress.is_terminal());
        assert!(BatchStatus::InProgress.is_processing());
    }

    #[test]
    fn test_request_counts() {
        let counts = RequestCounts {
            total: 100,
            completed: 95,
            failed: 5,
        };
        assert_eq!(counts.total, 100);
        assert_eq!(counts.completed, 95);
        assert_eq!(counts.failed, 5);
    }
}
