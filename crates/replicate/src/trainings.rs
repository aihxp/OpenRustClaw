//! Trainings API - Fine-tune models.

use serde::Serialize;

use crate::client::ReplicateClient;
use crate::error::Result;
use crate::types::{PaginatedResponse, Training, WebhookEvents};

/// Trainings API client.
#[derive(Debug)]
pub struct Trainings<'a> {
    client: &'a ReplicateClient,
}

impl<'a> Trainings<'a> {
    /// Create a new trainings API client.
    pub fn new(client: &'a ReplicateClient) -> Self {
        Self { client }
    }

    /// Create a new training.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, TrainingRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let request = TrainingRequest::new(
    ///     "myuser/my-fine-tuned-model",
    /// );
    ///
    /// let training = client.trainings()
    ///     .create("base-owner", "base-model", "version-id", request)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        model_owner: &str,
        model_name: &str,
        version_id: &str,
        request: TrainingRequest,
    ) -> Result<Training> {
        let path = format!("/models/{}/{}/versions/{}/trainings", model_owner, model_name, version_id);
        let body = serde_json::to_value(&request)?;
        let response = self.client.post(&path, body).await?;
        self.client.handle_response(response).await
    }

    /// Get a training by ID.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let training = client.trainings().get("training_abc123").await?;
    /// println!("Status: {:?}", training.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Result<Training> {
        let path = format!("/trainings/{}", id);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// List all trainings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let trainings = client.trainings().list(None).await?;
    /// for training in trainings.results {
    ///     println!("{} - {:?}", training.id, training.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        cursor: Option<&str>,
    ) -> Result<PaginatedResponse<Training>> {
        let mut path = "/trainings".to_string();
        if let Some(cursor) = cursor {
            path.push_str(&format!("?cursor={}", cursor));
        }
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Cancel a running training.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// client.trainings().cancel("training_abc123").await?;
    /// println!("Training cancelled");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, id: &str) -> Result<Training> {
        let path = format!("/trainings/{}/cancel", id);
        let response = self.client.post(&path, serde_json::json!({})).await?;
        self.client.handle_response(response).await
    }

    /// Wait for a training to complete.
    ///
    /// Polls the training status until it reaches a terminal state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let training = client.trainings()
    ///     .wait_for_completion("training_abc123")
    ///     .await?;
    ///
    /// if training.status.is_success() {
    ///     println!("Output: {:?}", training.output);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(&self, id: &str) -> Result<Training> {
        self.wait_for_completion_with_options(id, None, None).await
    }

    /// Wait for a training to complete with custom options.
    ///
    /// # Arguments
    ///
    /// * `id` - The training ID
    /// * `poll_interval` - How often to poll (default: 10 seconds)
    /// * `timeout` - Maximum time to wait (default: 1 hour)
    pub async fn wait_for_completion_with_options(
        &self,
        id: &str,
        poll_interval: Option<std::time::Duration>,
        timeout: Option<std::time::Duration>,
    ) -> Result<Training> {
        let poll_interval = poll_interval.unwrap_or(std::time::Duration::from_secs(10));
        let timeout = timeout.unwrap_or(std::time::Duration::from_secs(3600));
        let start = std::time::Instant::now();

        loop {
            let training = self.get(id).await?;

            if training.status.is_terminal() {
                return Ok(training);
            }

            if start.elapsed() > timeout {
                return Err(crate::error::ReplicateError::Timeout {
                    operation: format!("waiting for training {}", id),
                });
            }

            tracing::trace!(
                training_id = %id,
                status = %training.status,
                "Waiting for training"
            );

            tokio::time::sleep(poll_interval).await;
        }
    }
}

/// A request to create a training.
#[derive(Debug, Clone, Serialize)]
pub struct TrainingRequest {
    /// The destination model (format: "owner/name").
    destination: String,
    /// The input parameters for training.
    input: serde_json::Value,
    /// Webhook URL for updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook: Option<String>,
    /// Webhook events to subscribe to.
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_events_filter: Option<Vec<WebhookEvents>>,
}

impl TrainingRequest {
    /// Create a new training request.
    ///
    /// # Arguments
    ///
    /// * `destination` - The destination model in format "owner/name"
    pub fn new(destination: impl Into<String>) -> Self {
        Self {
            destination: destination.into(),
            input: serde_json::Value::Object(serde_json::Map::new()),
            webhook: None,
            webhook_events_filter: None,
        }
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
}

/// Builder for training requests.
pub struct TrainingRequestBuilder {
    request: TrainingRequest,
}

impl TrainingRequestBuilder {
    /// Create a new builder.
    pub fn new(destination: impl Into<String>) -> Self {
        Self {
            request: TrainingRequest::new(destination),
        }
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

    /// Build the request.
    pub fn build(self) -> TrainingRequest {
        self.request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_request_builder() {
        let request = TrainingRequest::new("myuser/my-model")
            .input("train_data", "https://example.com/data.zip")
            .input("num_train_epochs", 3);

        assert_eq!(request.destination, "myuser/my-model");
        assert_eq!(request.input["train_data"], "https://example.com/data.zip");
        assert_eq!(request.input["num_train_epochs"], 3);
    }

    #[test]
    fn test_training_request_webhook() {
        let request = TrainingRequest::new("myuser/my-model")
            .webhook("https://example.com/webhook")
            .webhook_events(vec![WebhookEvents::Completed, WebhookEvents::Logs]);

        assert_eq!(request.webhook, Some("https://example.com/webhook".to_string()));
        assert_eq!(
            request.webhook_events_filter,
            Some(vec![WebhookEvents::Completed, WebhookEvents::Logs])
        );
    }
}
