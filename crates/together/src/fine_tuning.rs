//! Fine-tuning API for Together AI.

use std::path::Path;

use crate::client::TogetherClient;
use crate::constants::endpoints;
use crate::error::{Result, TogetherError};

/// Client for fine-tuning.
#[derive(Debug)]
pub struct FineTuning<'a> {
    client: &'a TogetherClient,
}

impl<'a> FineTuning<'a> {
    /// Create a new fine-tuning client.
    pub fn new(client: &'a TogetherClient) -> Self {
        Self { client }
    }

    /// Create a new fine-tuning job.
    pub async fn create_job(&self, request: FineTuneRequest) -> Result<FineTuneJob> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post(endpoints::FINE_TUNING, body).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuneJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// List fine-tuning jobs.
    pub async fn list_jobs(&self) -> Result<FineTuneJobList> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::FINE_TUNING).await?;
                    let body = client.handle_response(response).await?;
                    let list: FineTuneJobList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Get a fine-tuning job by ID.
    pub async fn get_job(&self, job_id: impl AsRef<str>) -> Result<FineTuneJob> {
        let path = format!("{}/{}", endpoints::FINE_TUNING, job_id.as_ref());
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuneJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// Cancel a fine-tuning job.
    pub async fn cancel_job(&self, job_id: impl AsRef<str>) -> Result<FineTuneJob> {
        let path = format!("{}/{}/cancel", endpoints::FINE_TUNING, job_id.as_ref());
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, serde_json::json!({})).await?;
                    let body = client.handle_response(response).await?;
                    let job: FineTuneJob = serde_json::from_value(body)?;
                    Ok(job)
                })
            })
            .await
    }

    /// List events for a fine-tuning job.
    pub async fn list_events(&self, job_id: impl AsRef<str>) -> Result<FineTuneEventList> {
        let path = format!("{}/{}/events", endpoints::FINE_TUNING, job_id.as_ref());
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: FineTuneEventList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Upload a file for fine-tuning.
    pub async fn upload_file(&self, file_path: impl AsRef<Path>, purpose: &str) -> Result<UploadedFile> {
        let file_path = file_path.as_ref().to_path_buf();
        let purpose = purpose.to_string();
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let file_path = file_path.clone();
                let purpose = purpose.clone();
                
                Box::pin(async move {
                    let file_name = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| TogetherError::Config {
                            message: "Invalid file path".to_string(),
                        })?;

                    let file_content = tokio::fs::read(&file_path).await?;
                    
                    let part = reqwest::multipart::Part::bytes(file_content)
                        .file_name(file_name.to_string())
                        .mime_str("application/jsonl")?;

                    let form = reqwest::multipart::Form::new()
                        .text("purpose", purpose)
                        .part("file", part);

                    let response = client.post_multipart(endpoints::FILES, form).await?;
                    let body = client.handle_response(response).await?;
                    let file: UploadedFile = serde_json::from_value(body)?;
                    Ok(file)
                })
            })
            .await
    }

    /// List uploaded files.
    pub async fn list_files(&self) -> Result<FileList> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::FILES).await?;
                    let body = client.handle_response(response).await?;
                    let list: FileList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Get an uploaded file.
    pub async fn get_file(&self, file_id: impl AsRef<str>) -> Result<UploadedFile> {
        let path = format!("{}/{}", endpoints::FILES, file_id.as_ref());
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let file: UploadedFile = serde_json::from_value(body)?;
                    Ok(file)
                })
            })
            .await
    }

    /// Delete an uploaded file.
    pub async fn delete_file(&self, file_id: impl AsRef<str>) -> Result<serde_json::Value> {
        let path = format!("{}/{}", endpoints::FILES, file_id.as_ref());
        
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.delete(&path).await?;
                    let body = client.handle_response(response).await?;
                    Ok(body)
                })
            })
            .await
    }
}

/// A fine-tuning request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuneRequest {
    /// The base model to fine-tune.
    pub model: String,
    /// The ID of the training file.
    pub training_file: String,
    /// The ID of the validation file (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    /// Number of epochs to train for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<u32>,
    /// Batch size for training.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
    /// Learning rate multiplier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate: Option<f64>,
    /// Suffix for the fine-tuned model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Whether to evaluate the model during training.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_steps: Option<u32>,
}

impl FineTuneRequest {
    /// Create a new fine-tuning request.
    pub fn new(model: impl Into<String>, training_file: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            training_file: training_file.into(),
            validation_file: None,
            n_epochs: None,
            batch_size: None,
            learning_rate: None,
            suffix: None,
            eval_steps: None,
        }
    }

    /// Set the validation file.
    pub fn validation_file(mut self, file_id: impl Into<String>) -> Self {
        self.validation_file = Some(file_id.into());
        self
    }

    /// Set the number of epochs.
    pub fn n_epochs(mut self, n: u32) -> Self {
        self.n_epochs = Some(n);
        self
    }

    /// Set the batch size.
    pub fn batch_size(mut self, size: u32) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Set the learning rate.
    pub fn learning_rate(mut self, rate: f64) -> Self {
        self.learning_rate = Some(rate);
        self
    }

    /// Set the suffix for the fine-tuned model name.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set the evaluation steps.
    pub fn eval_steps(mut self, steps: u32) -> Self {
        self.eval_steps = Some(steps);
        self
    }
}

/// A fine-tuning job.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuneJob {
    /// The job ID.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The model being fine-tuned.
    pub model: String,
    /// The fine-tuned model name (set after completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuned_model: Option<String>,
    /// The organization ID.
    pub organization_id: String,
    /// The training file ID.
    pub training_file: String,
    /// The validation file ID (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    /// The status of the job.
    pub status: String,
    /// When the job was created.
    pub created_at: i64,
    /// When the job was updated.
    pub updated_at: i64,
    /// The number of epochs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<u32>,
    /// The batch size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
    /// The learning rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate: Option<f64>,
}

/// A list of fine-tuning jobs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuneJobList {
    /// The list of jobs.
    pub data: Vec<FineTuneJob>,
}

/// A fine-tuning event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuneEvent {
    /// The event ID.
    pub id: String,
    /// The object type.
    pub object: String,
    /// When the event was created.
    pub created_at: i64,
    /// The event level (info, warning, error).
    pub level: String,
    /// The event message.
    pub message: String,
    /// The type of event.
    #[serde(rename = "type")]
    pub event_type: String,
}

/// A list of fine-tuning events.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FineTuneEventList {
    /// The list of events.
    pub data: Vec<FineTuneEvent>,
}

/// An uploaded file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadedFile {
    /// The file ID.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The number of bytes in the file.
    pub bytes: i64,
    /// When the file was created.
    pub created_at: i64,
    /// The filename.
    pub filename: String,
    /// The purpose of the file.
    pub purpose: String,
    /// The file format (e.g., "jsonl").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// A list of files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileList {
    /// The list of files.
    pub data: Vec<UploadedFile>,
}

/// Status of a fine-tuning job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Job is validating files.
    ValidatingFiles,
    /// Job is queued.
    Queued,
    /// Job is running.
    Running,
    /// Job succeeded.
    Succeeded,
    /// Job failed.
    Failed,
    /// Job was cancelled.
    Cancelled,
    /// Unknown status.
    Unknown,
}

impl JobStatus {
    /// Parse a status string.
    pub fn parse(status: &str) -> Self {
        match status {
            "validating_files" => JobStatus::ValidatingFiles,
            "queued" => JobStatus::Queued,
            "running" => JobStatus::Running,
            "succeeded" => JobStatus::Succeeded,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Unknown,
        }
    }

    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled)
    }

    /// Check if the job is running.
    pub fn is_running(&self) -> bool {
        matches!(self, JobStatus::Running | JobStatus::Queued | JobStatus::ValidatingFiles)
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JobStatus::ValidatingFiles => "validating_files",
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Succeeded => "succeeded",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
            JobStatus::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}
