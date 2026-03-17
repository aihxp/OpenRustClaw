//! Batch API support for Anthropic (beta feature).
//!
//! The Batch API allows you to submit multiple requests for asynchronous processing
//! at 50% lower cost than standard API calls.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::client::AnthropicClient;
use crate::constants::endpoints;
use crate::error::{AnthropicError, Result};
use crate::types::{MessageRequest, MessageResponse};

/// Generate a unique ID for batch requests.
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("batch-{}", timestamp)
}

/// A client for the Batch API.
#[derive(Debug, Clone)]
pub struct BatchClient<'a> {
    client: &'a AnthropicClient,
}

impl<'a> BatchClient<'a> {
    /// Create a new batch client.
    pub fn new(client: &'a AnthropicClient) -> Self {
        Self { client }
    }

    /// Create a new batch request.
    pub async fn create(&self, requests: Vec<BatchRequest>) -> Result<Batch> {
        let custom_id = generate_id();
        self.create_with_id(&custom_id, requests).await
    }

    /// Create a new batch request with a custom ID.
    pub async fn create_with_id(&self, custom_id: &str, requests: Vec<BatchRequest>) -> Result<Batch> {
        let body = serde_json::json!({
            "requests": requests.iter().enumerate().map(|(i, req)| {
                serde_json::json!({
                    "custom_id": format!("{}-{}", custom_id, i),
                    "params": req,
                })
            }).collect::<Vec<_>>(),
        });

        let response = self
            .client
            .post(endpoints::BATCH, body)
            .await?;

        let body = self.client.handle_response(response).await?;
        let batch: Batch = serde_json::from_value(body)?;
        Ok(batch)
    }

    /// Get a batch by ID.
    pub async fn get(&self, batch_id: &str) -> Result<Batch> {
        let url = format!("{}/{}", endpoints::BATCH, batch_id);
        let response = self.client.post(&url, serde_json::json!({})).await?;
        let body = self.client.handle_response(response).await?;
        let batch: Batch = serde_json::from_value(body)?;
        Ok(batch)
    }

    /// List batches.
    pub async fn list(&self, limit: Option<usize>, before_id: Option<&str>, after_id: Option<&str>) -> Result<BatchList> {
        let mut params = HashMap::new();
        if let Some(limit) = limit {
            params.insert("limit", limit.to_string());
        }
        if let Some(before) = before_id {
            params.insert("before_id", before.to_string());
        }
        if let Some(after) = after_id {
            params.insert("after_id", after.to_string());
        }

        let query = params
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = if query.is_empty() {
            endpoints::BATCH.to_string()
        } else {
            format!("{}?{}", endpoints::BATCH, query)
        };

        let response = self.client.post(&url, serde_json::json!({})).await?;
        let body = self.client.handle_response(response).await?;
        let list: BatchList = serde_json::from_value(body)?;
        Ok(list)
    }

    /// Cancel a batch.
    pub async fn cancel(&self, batch_id: &str) -> Result<Batch> {
        let url = format!("{}/{}/cancel", endpoints::BATCH, batch_id);
        let response = self.client.post(&url, serde_json::json!({})).await?;
        let body = self.client.handle_response(response).await?;
        let batch: Batch = serde_json::from_value(body)?;
        Ok(batch)
    }

    /// Poll a batch until it completes (or fails).
    pub async fn poll_until_complete(
        &self,
        batch_id: &str,
        interval: std::time::Duration,
        timeout: std::time::Duration,
    ) -> Result<Batch> {
        let start = std::time::Instant::now();

        loop {
            let batch = self.get(batch_id).await?;

            match batch.processing_status {
                BatchStatus::Ended | BatchStatus::Cancelled | BatchStatus::Errored => {
                    return Ok(batch);
                }
                _ => {}
            }

            if start.elapsed() > timeout {
                return Err(AnthropicError::Timeout {
                    operation: format!("batch polling for {}", batch_id),
                });
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// Get the results of a completed batch.
    pub async fn results(&self, batch_id: &str) -> Result<Vec<BatchResult>> {
        let url = format!("{}/{}/results", endpoints::BATCH, batch_id);
        let response = self.client.post(&url, serde_json::json!({})).await?;
        
        if !response.status().is_success() {
            return Err(AnthropicError::from_response(response).await);
        }

        // Results are returned as newline-delimited JSON (NDJSON)
        let text = response.text().await.map_err(|e| AnthropicError::Http { source: e })?;
        let mut results = Vec::new();

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let result: BatchResult = serde_json::from_str(line)
                .map_err(|e| AnthropicError::Json { source: e })?;
            results.push(result);
        }

        Ok(results)
    }
}

/// A single request in a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    /// The model to use.
    pub model: String,
    /// The maximum number of tokens to generate.
    pub max_tokens: usize,
    /// The messages.
    pub messages: Vec<crate::types::Message>,
    /// System prompt (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<crate::types::SystemContent>,
    /// Tools (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::types::Tool>>,
    /// Temperature (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
}

impl BatchRequest {
    /// Create a batch request from a MessageRequest.
    pub fn from_request(request: MessageRequest) -> Self {
        Self {
            model: request.model,
            max_tokens: request.max_tokens,
            messages: request.messages,
            system: request.system,
            tools: request.tools,
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
        }
    }
}

/// A batch response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Unique identifier for the batch.
    pub id: String,
    /// The processing status.
    #[serde(rename = "processing_status")]
    pub processing_status: BatchStatus,
    /// Request counts.
    pub request_counts: RequestCounts,
    /// When the batch was created.
    pub created_at: String,
    /// When the batch was ended (if applicable).
    pub ended_at: Option<String>,
    /// When the batch expires (results will be deleted).
    pub expires_at: Option<String>,
    /// When results are available (if applicable).
    pub results_url: Option<String>,
    /// The original requests (truncated in API response).
    pub requests: Option<Vec<BatchRequestItem>>,
}

/// A request item in a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestItem {
    /// Custom ID for this request.
    pub custom_id: String,
    /// The request parameters.
    pub params: BatchRequest,
}

/// Status of a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Batch is being validated.
    Validating,
    /// Batch is in progress.
    InProgress,
    /// Batch is finalizing.
    Finalizing,
    /// Batch has ended.
    Ended,
    /// Batch was cancelled.
    Cancelled,
    /// Batch encountered an error.
    Errored,
}

/// Request counts for a batch.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RequestCounts {
    /// Number of requests being processed.
    pub processing: usize,
    /// Number of requests that succeeded.
    pub succeeded: usize,
    /// Number of requests that errored.
    pub errored: usize,
    /// Number of requests that were cancelled.
    pub cancelled: usize,
    /// Number of requests that expired.
    pub expired: usize,
    /// Total number of requests.
    pub total: usize,
}

/// A list of batches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchList {
    /// The batches.
    pub data: Vec<Batch>,
    /// Whether there are more batches.
    pub has_more: bool,
    /// First ID in the list (for pagination).
    pub first_id: Option<String>,
    /// Last ID in the list (for pagination).
    pub last_id: Option<String>,
}

/// A single result from a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// The custom ID of the request.
    pub custom_id: String,
    /// The result.
    pub result: BatchResultDetail,
}

/// Details of a batch result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BatchResultDetail {
    /// The request succeeded.
    Succeeded {
        /// The message response.
        message: MessageResponse,
    },
    /// The request errored.
    Errored {
        /// Whether this is a validation error.
        error: BatchError,
    },
    /// The request was cancelled.
    Cancelled,
    /// The request expired.
    Expired,
}

/// An error in a batch result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    /// The error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// The error message.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_counts() {
        let counts = RequestCounts {
            processing: 0,
            succeeded: 10,
            errored: 1,
            cancelled: 0,
            expired: 0,
            total: 11,
        };
        assert_eq!(counts.total, 11);
    }

    #[test]
    fn test_batch_status_serialization() {
        let status = BatchStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");
    }

    #[test]
    fn test_batch_result_detail() {
        let detail = BatchResultDetail::Cancelled;
        match detail {
            BatchResultDetail::Cancelled => {}
            _ => panic!("Expected Cancelled"),
        }
    }
}
