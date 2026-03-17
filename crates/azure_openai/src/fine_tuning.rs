//! Fine-tuning API for Azure OpenAI.

use crate::client::AzureOpenAIClient;
use crate::error::Result;

/// Client for the fine-tuning API.
#[derive(Debug)]
pub struct FineTuning<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> FineTuning<'a> {
    /// Create a new fine-tuning client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Create a fine-tuning job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, FineTuningJobRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = FineTuningJobRequest::new("file-abc123")
    ///     .model("gpt-35-turbo-0613")
    ///     .validation_file("file-def456");
    ///
    /// let job = client.fine_tuning().create(request).await?;
    /// println!("Job ID: {}", job.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, request: FineTuningJobRequest) -> Result<FineTuningJob> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post("/openai/fine_tuning/jobs", body).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuningJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// List fine-tuning jobs.
    pub async fn list(&self, limit: Option<usize>) -> Result<FineTuningJobList> {
        let mut path = "/openai/fine_tuning/jobs".to_string();
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
                    let list: FineTuningJobList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Retrieve a fine-tuning job.
    pub async fn retrieve(&self, job_id: &str) -> Result<FineTuningJob> {
        let path = format!("{}/{}", "/openai/fine_tuning/jobs", job_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuningJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// Cancel a fine-tuning job.
    pub async fn cancel(&self, job_id: &str) -> Result<FineTuningJob> {
        let path = format!("{}/{}/cancel", "/openai/fine_tuning/jobs", job_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, serde_json::json!({})).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuningJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// List checkpoints for a fine-tuning job.
    pub async fn list_checkpoints(&self, job_id: &str) -> Result<FineTuningCheckpointList> {
        let path = format!("{}/{}/checkpoints", "/openai/fine_tuning/jobs", job_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: FineTuningCheckpointList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// List events for a fine-tuning job.
    pub async fn list_events(&self, job_id: &str) -> Result<FineTuningEventList> {
        let path = format!("{}/{}/events", "/openai/fine_tuning/jobs", job_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: FineTuningEventList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }
}

/// A fine-tuning job request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningJobRequest {
    /// The training file ID.
    pub training_file: String,
    /// The model to fine-tune.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The hyperparameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<Hyperparameters>,
    /// The suffix for the fine-tuned model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// The validation file ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    /// Integrations (e.g., Weights & Biases).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,
    /// The seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl FineTuningJobRequest {
    /// Create a new fine-tuning job request.
    pub fn new(training_file: impl Into<String>) -> Self {
        Self {
            training_file: training_file.into(),
            model: None,
            hyperparameters: None,
            suffix: None,
            validation_file: None,
            integrations: None,
            seed: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the hyperparameters.
    pub fn hyperparameters(mut self, hyperparameters: Hyperparameters) -> Self {
        self.hyperparameters = Some(hyperparameters);
        self
    }

    /// Set the suffix.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set the validation file.
    pub fn validation_file(mut self, file_id: impl Into<String>) -> Self {
        self.validation_file = Some(file_id.into());
        self
    }

    /// Set the seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Hyperparameters for fine-tuning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hyperparameters {
    /// Number of epochs ("auto" or a number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<EpochSetting>,
    /// Batch size ("auto" or a number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOrNumber>,
    /// Learning rate multiplier ("auto" or a number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOrNumber>,
}

impl Hyperparameters {
    /// Create new hyperparameters with auto settings.
    pub fn auto() -> Self {
        Self {
            n_epochs: Some(EpochSetting::Auto),
            batch_size: Some(AutoOrNumber::Auto),
            learning_rate_multiplier: Some(AutoOrNumber::Auto),
        }
    }

    /// Set the number of epochs.
    pub fn n_epochs(mut self, epochs: usize) -> Self {
        self.n_epochs = Some(EpochSetting::Number(epochs));
        self
    }

    /// Set the batch size.
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(AutoOrNumber::Number(size));
        self
    }

    /// Set the learning rate multiplier.
    pub fn learning_rate_multiplier(mut self, multiplier: f64) -> Self {
        self.learning_rate_multiplier = Some(AutoOrNumber::Float(multiplier));
        self
    }
}

/// Epoch setting (auto or number).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EpochSetting {
    /// Auto.
    #[serde(rename = "auto")]
    Auto,
    /// Number of epochs.
    Number(usize),
}

/// Auto or number setting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AutoOrNumber {
    /// Auto.
    #[serde(rename = "auto")]
    Auto,
    /// Integer number.
    Number(usize),
    /// Float number.
    Float(f64),
}

/// Integration configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Integration {
    /// The type of integration.
    #[serde(rename = "type")]
    pub integration_type: String,
    /// The Weights & Biases configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wandb: Option<WandbConfig>,
}

/// Weights & Biases configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WandbConfig {
    /// The project name.
    pub project: String,
    /// The run name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The entity name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// Tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// A fine-tuning job.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningJob {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The error (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<FineTuningError>,
    /// The fine-tuned model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuned_model: Option<String>,
    /// The Unix timestamp when the job finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    /// The hyperparameters.
    pub hyperparameters: Hyperparameters,
    /// The model.
    pub model: String,
    /// The organization ID.
    pub organization_id: String,
    /// The result files.
    pub result_files: Vec<String>,
    /// The status.
    pub status: FineTuningJobStatus,
    /// The number of trained tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trained_tokens: Option<usize>,
    /// The training file.
    pub training_file: String,
    /// The validation file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    /// The seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// The estimated finish.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_finish: Option<i64>,
}

/// Fine-tuning job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningJobStatus {
    /// Validating files.
    ValidatingFiles,
    /// Queued.
    Queued,
    /// Running.
    Running,
    /// Succeeded.
    Succeeded,
    /// Failed.
    Failed,
    /// Cancelled.
    Cancelled,
}

impl FineTuningJobStatus {
    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            FineTuningJobStatus::Succeeded
                | FineTuningJobStatus::Failed
                | FineTuningJobStatus::Cancelled
        )
    }
}

/// A fine-tuning error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
    /// The error parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

/// A fine-tuning job list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningJobList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<FineTuningJob>,
    /// Whether there are more items.
    pub has_more: bool,
}

/// A fine-tuning checkpoint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningCheckpoint {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The fine-tuned model checkpoint.
    pub fine_tuned_model_checkpoint: String,
    /// The fine-tuning job ID.
    pub fine_tuning_job_id: String,
    /// The metrics.
    pub metrics: CheckpointMetrics,
    /// The step number.
    pub step_number: usize,
}

/// Checkpoint metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointMetrics {
    /// The step.
    pub step: f64,
    /// The training loss.
    #[serde(rename = "train_loss")]
    pub train_loss: f64,
    /// The training accuracy.
    #[serde(rename = "train_mean_token_accuracy")]
    pub train_mean_token_accuracy: f64,
    /// The validation loss.
    #[serde(rename = "valid_loss", skip_serializing_if = "Option::is_none")]
    pub valid_loss: Option<f64>,
    /// The validation mean token accuracy.
    #[serde(
        rename = "valid_mean_token_accuracy",
        skip_serializing_if = "Option::is_none"
    )]
    pub valid_mean_token_accuracy: Option<f64>,
    /// The full validation loss.
    #[serde(rename = "full_valid_loss", skip_serializing_if = "Option::is_none")]
    pub full_valid_loss: Option<f64>,
    /// The full validation mean token accuracy.
    #[serde(
        rename = "full_valid_mean_token_accuracy",
        skip_serializing_if = "Option::is_none"
    )]
    pub full_valid_mean_token_accuracy: Option<f64>,
}

/// A fine-tuning checkpoint list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningCheckpointList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<FineTuningCheckpoint>,
    /// Whether there are more items.
    pub has_more: bool,
}

/// A fine-tuning event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningEvent {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The level.
    pub level: String,
    /// The message.
    pub message: String,
    /// The data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// The type.
    #[serde(rename = "type")]
    pub event_type: String,
}

/// A fine-tuning event list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuningEventList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<FineTuningEvent>,
    /// Whether there are more items.
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fine_tuning_job_request_builder() {
        let request = FineTuningJobRequest::new("file-123")
            .model("gpt-35-turbo")
            .suffix("my-model")
            .validation_file("file-456")
            .seed(42);

        assert_eq!(request.training_file, "file-123");
        assert_eq!(request.model, Some("gpt-35-turbo".to_string()));
        assert_eq!(request.suffix, Some("my-model".to_string()));
        assert_eq!(request.validation_file, Some("file-456".to_string()));
        assert_eq!(request.seed, Some(42));
    }

    #[test]
    fn test_hyperparameters() {
        let hp = Hyperparameters::auto()
            .n_epochs(3)
            .batch_size(4)
            .learning_rate_multiplier(0.1);

        assert!(matches!(hp.n_epochs, Some(EpochSetting::Number(3))));
        assert!(matches!(hp.batch_size, Some(AutoOrNumber::Number(4))));
        assert!(matches!(
            hp.learning_rate_multiplier,
            Some(AutoOrNumber::Float(0.1))
        ));
    }

    #[test]
    fn test_job_status() {
        assert!(FineTuningJobStatus::Succeeded.is_terminal());
        assert!(FineTuningJobStatus::Failed.is_terminal());
        assert!(!FineTuningJobStatus::Running.is_terminal());
    }

    #[test]
    fn test_checkpoint_metrics() {
        let metrics = CheckpointMetrics {
            step: 100.0,
            train_loss: 0.5,
            train_mean_token_accuracy: 0.9,
            valid_loss: Some(0.6),
            valid_mean_token_accuracy: Some(0.85),
            full_valid_loss: None,
            full_valid_mean_token_accuracy: None,
        };
        assert_eq!(metrics.step, 100.0);
        assert_eq!(metrics.train_loss, 0.5);
    }
}
