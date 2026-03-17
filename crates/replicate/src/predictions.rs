//! Predictions API - Create and manage predictions.

use serde::Serialize;
use std::time::Duration;
use tracing::trace;

use crate::client::ReplicateClient;
use crate::error::{ReplicateError, Result};
use crate::types::{PaginatedResponse, Prediction, WebhookEvents};

/// Predictions API client.
#[derive(Debug)]
pub struct Predictions<'a> {
    client: &'a ReplicateClient,
}

impl<'a> Predictions<'a> {
    /// Create a new predictions API client.
    pub fn new(client: &'a ReplicateClient) -> Self {
        Self { client }
    }

    /// Create a new prediction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, PredictionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let request = PredictionRequest::new()
    ///     .model("black-forest-labs/flux-schnell")
    ///     .input("prompt", "Astronaut riding a horse");
    ///
    /// let prediction = client.predictions().create(request).await?;
    /// println!("Created prediction: {}", prediction.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: PredictionRequest) -> Result<Prediction> {
        let body = serde_json::to_value(&request)?;
        let mut headers = reqwest::header::HeaderMap::new();

        // Handle sync mode with Prefer header
        if let Some(wait_secs) = request.wait {
            headers.insert(
                "Prefer",
                reqwest::header::HeaderValue::from_str(&format!("wait={}", wait_secs)).map_err(
                    |e| ReplicateError::InvalidHeader {
                        message: e.to_string(),
                    },
                )?,
            );
        }

        let response = if headers.is_empty() {
            self.client.post("/predictions", body).await?
        } else {
            self.client
                .post_with_headers("/predictions", body, headers)
                .await?
        };

        self.client.handle_response(response).await
    }

    /// Get a prediction by ID.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let prediction = client.predictions().get("pred_abc123").await?;
    /// println!("Status: {}", prediction.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Result<Prediction> {
        let path = format!("/predictions/{}", id);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// List all predictions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let predictions = client.predictions().list(None).await?;
    /// for prediction in predictions.results {
    ///     println!("{} - {:?}", prediction.id, prediction.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, cursor: Option<&str>) -> Result<PaginatedResponse<Prediction>> {
        let mut path = "/predictions".to_string();
        if let Some(cursor) = cursor {
            path.push_str(&format!("?cursor={}", cursor));
        }
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Cancel a running prediction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// client.predictions().cancel("pred_abc123").await?;
    /// println!("Prediction cancelled");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, id: &str) -> Result<Prediction> {
        let path = format!("/predictions/{}/cancel", id);
        let response = self.client.post(&path, serde_json::json!({})).await?;
        self.client.handle_response(response).await
    }

    /// Wait for a prediction to complete.
    ///
    /// Polls the prediction status until it reaches a terminal state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let prediction = client.predictions()
    ///     .wait_for_completion("pred_abc123")
    ///     .await?;
    ///
    /// if prediction.status.is_success() {
    ///     println!("Output: {:?}", prediction.output);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(&self, id: &str) -> Result<Prediction> {
        self.wait_for_completion_with_options(id, None, None).await
    }

    /// Wait for a prediction to complete with custom options.
    ///
    /// # Arguments
    ///
    /// * `id` - The prediction ID
    /// * `poll_interval` - How often to poll (default: 1 second)
    /// * `timeout` - Maximum time to wait (default: 60 minutes)
    pub async fn wait_for_completion_with_options(
        &self,
        id: &str,
        poll_interval: Option<Duration>,
        timeout: Option<Duration>,
    ) -> Result<Prediction> {
        let poll_interval = poll_interval.unwrap_or(Duration::from_secs(1));
        let timeout = timeout.unwrap_or(Duration::from_secs(3600));
        let start = std::time::Instant::now();

        loop {
            let prediction = self.get(id).await?;

            if prediction.status.is_terminal() {
                return Ok(prediction);
            }

            if start.elapsed() > timeout {
                return Err(ReplicateError::Timeout {
                    operation: format!("waiting for prediction {}", id),
                });
            }

            trace!(
                prediction_id = %id,
                status = %prediction.status,
                "Waiting for prediction"
            );

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Run a prediction and wait for completion in one call.
    ///
    /// This is a convenience method that creates a prediction and polls until completion.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, PredictionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let request = PredictionRequest::new()
    ///     .model("black-forest-labs/flux-schnell")
    ///     .input("prompt", "A cat");
    ///
    /// let prediction = client.predictions().run(request).await?;
    /// println!("Output: {:?}", prediction.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&self, request: PredictionRequest) -> Result<Prediction> {
        let poll_interval = request.poll_interval;
        let timeout = request.timeout;
        let prediction = self.create(request).await?;
        self.wait_for_completion_with_options(&prediction.id, poll_interval, timeout)
            .await
    }

    /// Create a prediction using sync mode (wait for completion).
    ///
    /// The API will hold the connection open for up to 60 seconds.
    /// If the prediction takes longer, it will return with a starting status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, PredictionRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let request = PredictionRequest::new()
    ///     .model("black-forest-labs/flux-schnell")
    ///     .input("prompt", "A cat");
    ///
    /// let prediction = client.predictions().run_sync(request, Some(60)).await?;
    /// println!("Output: {:?}", prediction.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_sync(
        &self,
        request: PredictionRequest,
        wait_secs: Option<u8>,
    ) -> Result<Prediction> {
        let wait_secs = wait_secs.unwrap_or(60).clamp(1, 60);
        let request = request.wait(wait_secs);
        self.create(request).await
    }
}

/// A request to create a prediction.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PredictionRequest {
    /// The model identifier (for official models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    /// The version ID (for community models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
    /// The input parameters.
    pub(crate) input: serde_json::Value,
    /// Webhook URL for updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) webhook: Option<String>,
    /// Webhook events to subscribe to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) webhook_events_filter: Option<Vec<WebhookEvents>>,
    /// Wait for completion (sync mode).
    #[serde(skip)]
    pub(crate) wait: Option<u8>,
    /// Poll interval for wait_for_completion.
    #[serde(skip)]
    pub(crate) poll_interval: Option<Duration>,
    /// Timeout for wait_for_completion.
    #[serde(skip)]
    pub(crate) timeout: Option<Duration>,
}

impl PredictionRequest {
    /// Create a new prediction request.
    pub fn new() -> Self {
        Self {
            model: None,
            version: None,
            input: serde_json::Value::Object(serde_json::Map::new()),
            webhook: None,
            webhook_events_filter: None,
            wait: None,
            poll_interval: None,
            timeout: None,
        }
    }

    /// Set the model (for official models).
    ///
    /// Use this for official models like `black-forest-labs/flux-schnell`
    /// or `meta/meta-llama-3-70b-instruct`.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the version ID (for community models).
    ///
    /// Use this for community models that require a specific version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Add an input parameter.
    pub fn input(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        if let Some(obj) = self.input.as_object_mut() {
            obj.insert(key.into(), value.into());
        }
        self
    }

    /// Set all input parameters at once.
    pub fn inputs(mut self, inputs: impl Into<serde_json::Value>) -> Self {
        self.input = inputs.into();
        self
    }

    /// Set the webhook URL.
    pub fn webhook(mut self, url: impl Into<String>) -> Self {
        self.webhook = Some(url.into());
        self
    }

    /// Set the webhook events to subscribe to.
    pub fn webhook_events(mut self, events: Vec<WebhookEvents>) -> Self {
        self.webhook_events_filter = Some(events);
        self
    }

    /// Enable sync mode and wait for the specified number of seconds.
    ///
    /// Must be between 1 and 60 seconds.
    pub fn wait(mut self, seconds: u8) -> Self {
        self.wait = Some(seconds.clamp(1, 60));
        self
    }

    /// Set the poll interval for wait_for_completion.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = Some(interval);
        self
    }

    /// Set the timeout for wait_for_completion.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Builder for prediction requests.
pub struct PredictionRequestBuilder {
    request: PredictionRequest,
}

impl PredictionRequestBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            request: PredictionRequest::new(),
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.request.model = Some(model.into());
        self
    }

    /// Set the version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.request.version = Some(version.into());
        self
    }

    /// Add an input parameter.
    pub fn input(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        if let Some(obj) = self.request.input.as_object_mut() {
            obj.insert(key.into(), value.into());
        }
        self
    }

    /// Set all inputs.
    pub fn inputs(mut self, inputs: impl Into<serde_json::Value>) -> Self {
        self.request.input = inputs.into();
        self
    }

    /// Set the webhook URL.
    pub fn webhook(mut self, url: impl Into<String>) -> Self {
        self.request.webhook = Some(url.into());
        self
    }

    /// Set webhook events.
    pub fn webhook_events(mut self, events: Vec<WebhookEvents>) -> Self {
        self.request.webhook_events_filter = Some(events);
        self
    }

    /// Set sync wait time.
    pub fn wait(mut self, seconds: u8) -> Self {
        self.request.wait = Some(seconds.clamp(1, 60));
        self
    }

    /// Set poll interval.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.request.poll_interval = Some(interval);
        self
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.request.timeout = Some(timeout);
        self
    }

    /// Build the request.
    pub fn build(self) -> PredictionRequest {
        self.request
    }
}

impl Default for PredictionRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_request_builder() {
        let request = PredictionRequest::new()
            .model("black-forest-labs/flux-schnell")
            .input("prompt", "A cat")
            .input("width", 1024);

        assert_eq!(
            request.model,
            Some("black-forest-labs/flux-schnell".to_string())
        );
        assert_eq!(request.input["prompt"], "A cat");
        assert_eq!(request.input["width"], 1024);
    }

    #[test]
    fn test_prediction_request_version() {
        let request = PredictionRequest::new()
            .version("abc123...")
            .input("text", "Hello");

        assert_eq!(request.version, Some("abc123...".to_string()));
    }

    #[test]
    fn test_prediction_request_webhook() {
        let request = PredictionRequest::new()
            .model("meta/meta-llama-3-70b-instruct")
            .webhook("https://example.com/webhook")
            .webhook_events(vec![WebhookEvents::Completed]);

        assert_eq!(
            request.webhook,
            Some("https://example.com/webhook".to_string())
        );
        assert_eq!(
            request.webhook_events_filter,
            Some(vec![WebhookEvents::Completed])
        );
    }

    #[test]
    fn test_prediction_request_wait() {
        let request = PredictionRequest::new().model("test").wait(30);

        assert_eq!(request.wait, Some(30));
    }

    #[test]
    fn test_prediction_request_wait_clamped() {
        let request = PredictionRequest::new().model("test").wait(100);

        assert_eq!(request.wait, Some(60));
    }
}
