//! Files API for Azure OpenAI.
//!
//! The Files API is used to upload files for fine-tuning, assistants, and batch processing.

use secrecy::ExposeSecret;

use crate::client::AzureOpenAIClient;
use crate::error::{AzureOpenAIError, Result};

/// Client for the files API.
#[derive(Debug)]
pub struct Files<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Files<'a> {
    /// Create a new files client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Upload a file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_openai::{AzureOpenAIClient, FileUploadRequest, FilePurpose};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = AzureOpenAIClient::new(
    ///     "my-resource",
    ///     "my-deployment",
    ///     "my-api-key",
    /// )?;
    ///
    /// let request = FileUploadRequest::new("data.jsonl", FilePurpose::FineTune);
    /// let file = client.files().upload(request).await?;
    /// println!("File ID: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(&self, request: FileUploadRequest) -> Result<FileObject> {
        // Read the file
        let file_bytes = tokio::fs::read(&request.file_path)
            .await
            .map_err(|e| AzureOpenAIError::Config {
                message: format!("Failed to read file: {e}"),
            })?;

        let file_name = std::path::Path::new(&request.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| AzureOpenAIError::Config {
                message: format!("Failed to set MIME type: {e}"),
            })?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("purpose", request.purpose.as_str());

        let response = self.client.post_multipart("/openai/files", form).await?;
        let body = self.client.handle_response(response).await?;
        let file: FileObject = serde_json::from_value(body)?;
        Ok(file)
    }

    /// List files.
    pub async fn list(&self, purpose: Option<FilePurpose>) -> Result<FileList> {
        let mut path = "/openai/files".to_string();
        if let Some(purpose) = purpose {
            path.push_str(&format!("?purpose={}", purpose.as_str()));
        }

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: FileList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Retrieve a file.
    pub async fn retrieve(&self, file_id: &str) -> Result<FileObject> {
        let path = format!("/openai/files/{}", file_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let file: FileObject = serde_json::from_value(body)?;
                    Ok(file)
                })
            })
            .await
    }

    /// Delete a file.
    pub async fn delete(&self, file_id: &str) -> Result<FileDeletionStatus> {
        let path = format!("/openai/files/{}", file_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.delete(&path).await?;
                    let body = client.handle_response(response).await?;
                    let status: FileDeletionStatus = serde_json::from_value(body)?;
                    Ok(status)
                })
            })
            .await
    }

    /// Retrieve file content.
    pub async fn retrieve_content(&self, file_id: &str) -> Result<bytes::Bytes> {
        let path = format!("/openai/files/{}/content", file_id);

        let url = self.client.build_url(&path);
        let mut request = self.client.http().get(&url);

        // Add authentication headers
        if self.client.is_azure_ad() {
            let auth_header = self.client.config().authorization_header().await?;
            request = request.header("Authorization", auth_header);
        } else {
            if let crate::AzureCredential::ApiKey(key) = &self.client.config().credential {
                request = request.header("api-key", key.expose_secret());
            }
        }

        let response = request
            .send()
            .await
            .map_err(AzureOpenAIError::from)?;

        if !response.status().is_success() {
            return Err(AzureOpenAIError::from_response(response).await);
        }

        response.bytes().await.map_err(AzureOpenAIError::from)
    }
}

/// A file upload request.
#[derive(Debug, Clone)]
pub struct FileUploadRequest {
    /// The path to the file.
    pub file_path: String,
    /// The purpose of the file.
    pub purpose: FilePurpose,
}

impl FileUploadRequest {
    /// Create a new file upload request.
    pub fn new(file_path: impl Into<String>, purpose: FilePurpose) -> Self {
        Self {
            file_path: file_path.into(),
            purpose,
        }
    }
}

/// File purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilePurpose {
    /// For fine-tuning.
    FineTune,
    /// For fine-tuning results.
    FineTuneResults,
    /// For assistants.
    Assistants,
    /// For assistant results.
    AssistantsOutput,
    /// For batch API.
    Batch,
    /// For batch results.
    BatchOutput,
    /// For vision (images).
    Vision,
}

impl FilePurpose {
    /// Get the purpose string.
    pub fn as_str(&self) -> &'static str {
        match self {
            FilePurpose::FineTune => "fine-tune",
            FilePurpose::FineTuneResults => "fine-tune-results",
            FilePurpose::Assistants => "assistants",
            FilePurpose::AssistantsOutput => "assistants_output",
            FilePurpose::Batch => "batch",
            FilePurpose::BatchOutput => "batch_output",
            FilePurpose::Vision => "vision",
        }
    }
}

/// A file object.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileObject {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The number of bytes.
    pub bytes: usize,
    /// The creation timestamp.
    pub created_at: i64,
    /// The filename.
    pub filename: String,
    /// The purpose.
    pub purpose: String,
    /// The status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The status details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<String>,
}

/// A file list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<FileObject>,
}

/// A file deletion status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileDeletionStatus {
    /// The ID.
    pub id: String,
    /// The object type.
    pub object: String,
    /// Whether the deletion was successful.
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_upload_request() {
        let request = FileUploadRequest::new("data.jsonl", FilePurpose::FineTune);
        assert_eq!(request.file_path, "data.jsonl");
        assert_eq!(request.purpose, FilePurpose::FineTune);
    }

    #[test]
    fn test_file_purpose() {
        assert_eq!(FilePurpose::FineTune.as_str(), "fine-tune");
        assert_eq!(FilePurpose::Assistants.as_str(), "assistants");
        assert_eq!(FilePurpose::Batch.as_str(), "batch");
    }

    #[test]
    fn test_file_object() {
        let file = FileObject {
            id: "file-123".to_string(),
            object: "file".to_string(),
            bytes: 1024,
            created_at: 1234567890,
            filename: "data.jsonl".to_string(),
            purpose: "fine-tune".to_string(),
            status: Some("processed".to_string()),
            status_details: None,
        };
        assert_eq!(file.id, "file-123");
        assert_eq!(file.bytes, 1024);
    }
}
