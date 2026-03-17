//! Models API - Manage and run models.

use tracing::trace;

use crate::client::ReplicateClient;
use crate::error::Result;
use crate::predictions::PredictionRequest;
use crate::types::{Model, ModelVersion, PaginatedResponse, Prediction};

/// Models API client.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a ReplicateClient,
}

impl<'a> Models<'a> {
    /// Create a new models API client.
    pub fn new(client: &'a ReplicateClient) -> Self {
        Self { client }
    }

    /// List public models.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let models = client.models().list(None, None).await?;
    /// for model in models.results {
    ///     println!("{} - {:?}", model.name, model.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        cursor: Option<&str>,
        per_page: Option<u32>,
    ) -> Result<PaginatedResponse<Model>> {
        let mut path = "/models".to_string();
        let mut params = vec![];

        if let Some(cursor) = cursor {
            params.push(format!("cursor={}", cursor));
        }
        if let Some(per_page) = per_page {
            params.push(format!("per_page={}", per_page));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Get a model by owner and name.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let model = client.models().get("black-forest-labs", "flux-schnell").await?;
    /// println!("Model: {} - {:?}", model.name, model.description);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, owner: &str, name: &str) -> Result<Model> {
        let path = format!("/models/{}/{}", owner, name);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Create a new model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, ModelInfo};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let model = client.models()
    ///     .create("my-username", "my-model", "public", "cpu")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        owner: &str,
        name: &str,
        visibility: &str,
        hardware: &str,
    ) -> Result<Model> {
        let path = "/models";
        let body = serde_json::json!({
            "owner": owner,
            "name": name,
            "visibility": visibility,
            "hardware": hardware,
        });
        let response = self.client.post(path, body).await?;
        self.client.handle_response(response).await
    }

    /// Delete a model.
    ///
    /// Note: You can only delete models you own, that are private,
    /// and have no versions associated with them.
    pub async fn delete(&self, owner: &str, name: &str) -> Result<()> {
        let path = format!("/models/{}/{}", owner, name);
        let response = self.client.delete(&path).await?;
        self.client.handle_empty_response(response).await
    }

    /// Update model metadata.
    pub async fn update(
        &self,
        owner: &str,
        name: &str,
        description: Option<&str>,
        github_url: Option<&str>,
        paper_url: Option<&str>,
        license_url: Option<&str>,
    ) -> Result<Model> {
        let path = format!("/models/{}/{}", owner, name);
        let mut body = serde_json::Map::new();

        if let Some(desc) = description {
            body.insert("description".to_string(), serde_json::json!(desc));
        }
        if let Some(url) = github_url {
            body.insert("github_url".to_string(), serde_json::json!(url));
        }
        if let Some(url) = paper_url {
            body.insert("paper_url".to_string(), serde_json::json!(url));
        }
        if let Some(url) = license_url {
            body.insert("license_url".to_string(), serde_json::json!(url));
        }

        let response = self
            .client
            .patch(&path, serde_json::Value::Object(body))
            .await?;
        self.client.handle_response(response).await
    }

    /// List versions of a model.
    pub async fn list_versions(&self, owner: &str, name: &str) -> Result<Vec<ModelVersion>> {
        let path = format!("/models/{}/{}/versions", owner, name);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Get a specific version of a model.
    pub async fn get_version(
        &self,
        owner: &str,
        name: &str,
        version_id: &str,
    ) -> Result<ModelVersion> {
        let path = format!("/models/{}/{}/versions/{}", owner, name, version_id);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Delete a model version.
    ///
    /// Note: This has restrictions - see API documentation.
    pub async fn delete_version(&self, owner: &str, name: &str, version_id: &str) -> Result<()> {
        let path = format!("/models/{}/{}/versions/{}", owner, name, version_id);
        let response = self.client.delete(&path).await?;
        self.client.handle_empty_response(response).await
    }

    /// Create a prediction using an official model.
    ///
    /// This is the preferred way to run official models.
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
    ///     .input("prompt", "A cat");
    ///
    /// let prediction = client.models()
    ///     .create_prediction("black-forest-labs", "flux-schnell", request)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_prediction(
        &self,
        owner: &str,
        name: &str,
        request: PredictionRequest,
    ) -> Result<Prediction> {
        let path = format!("/models/{}/{}/predictions", owner, name);
        let mut body = serde_json::to_value(&request)?;

        // Remove model/version from body as they're in the URL
        if let Some(obj) = body.as_object_mut() {
            obj.remove("model");
            obj.remove("version");
        }

        let mut headers = reqwest::header::HeaderMap::new();

        // Handle sync mode with Prefer header
        if let Some(wait_secs) = request.wait {
            headers.insert(
                "Prefer",
                reqwest::header::HeaderValue::from_str(&format!("wait={}", wait_secs)).map_err(
                    |e| crate::error::ReplicateError::InvalidHeader {
                        message: e.to_string(),
                    },
                )?,
            );
        }

        let response = if headers.is_empty() {
            self.client.post(&path, body).await?
        } else {
            self.client.post_with_headers(&path, body, headers).await?
        };

        self.client.handle_response(response).await
    }

    /// Run an official model and wait for completion.
    ///
    /// Convenience method that creates a prediction and polls until completion.
    pub async fn run(
        &self,
        owner: &str,
        name: &str,
        request: PredictionRequest,
    ) -> Result<Prediction> {
        let prediction = self.create_prediction(owner, name, request.clone()).await?;

        // Poll for completion
        let poll_interval = request.poll_interval;
        let timeout = request.timeout;

        self.wait_for_completion(owner, name, &prediction.id, poll_interval, timeout)
            .await
    }

    /// Wait for a prediction on an official model to complete.
    async fn wait_for_completion(
        &self,
        _owner: &str,
        _name: &str,
        id: &str,
        poll_interval: Option<std::time::Duration>,
        timeout: Option<std::time::Duration>,
    ) -> Result<Prediction> {
        let poll_interval = poll_interval.unwrap_or(std::time::Duration::from_secs(1));
        let timeout = timeout.unwrap_or(std::time::Duration::from_secs(3600));
        let start = std::time::Instant::now();

        loop {
            let prediction = self.client.predictions().get(id).await?;

            if prediction.status.is_terminal() {
                return Ok(prediction);
            }

            if start.elapsed() > timeout {
                return Err(crate::error::ReplicateError::Timeout {
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

    /// List examples for a model.
    pub async fn list_examples(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<PaginatedResponse<crate::types::Prediction>> {
        let path = format!("/models/{}/{}/examples", owner, name);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Get a model's README.
    pub async fn get_readme(&self, owner: &str, name: &str) -> Result<String> {
        let path = format!("/models/{}/{}/readme", owner, name);
        let response = self.client.get(&path).await?;
        let text = response
            .text()
            .await
            .map_err(crate::error::ReplicateError::from)?;
        Ok(text)
    }

    /// Search for models.
    pub async fn search(
        &self,
        query: &str,
        per_page: Option<u32>,
    ) -> Result<PaginatedResponse<Model>> {
        let mut path = format!("/models?query={}", urlencoding::encode(query));

        if let Some(per_page) = per_page {
            path.push_str(&format!("&per_page={}", per_page));
        }

        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }
}

/// Information for creating a model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// The owner of the model.
    pub owner: String,
    /// The name of the model.
    pub name: String,
    /// A description of the model.
    pub description: Option<String>,
    /// Whether the model is public or private.
    pub visibility: String,
    /// The hardware SKU.
    pub hardware: String,
    /// URL for the cover image.
    pub cover_image_url: Option<String>,
    /// URL for the GitHub repository.
    pub github_url: Option<String>,
    /// URL for the paper.
    pub paper_url: Option<String>,
    /// URL for the license.
    pub license_url: Option<String>,
}

impl ModelInfo {
    /// Create a new model info with required fields.
    pub fn new(
        owner: impl Into<String>,
        name: impl Into<String>,
        visibility: &str,
        hardware: &str,
    ) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
            description: None,
            visibility: visibility.to_string(),
            hardware: hardware.to_string(),
            cover_image_url: None,
            github_url: None,
            paper_url: None,
            license_url: None,
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the cover image URL.
    pub fn cover_image_url(mut self, url: impl Into<String>) -> Self {
        self.cover_image_url = Some(url.into());
        self
    }

    /// Set the GitHub URL.
    pub fn github_url(mut self, url: impl Into<String>) -> Self {
        self.github_url = Some(url.into());
        self
    }

    /// Set the paper URL.
    pub fn paper_url(mut self, url: impl Into<String>) -> Self {
        self.paper_url = Some(url.into());
        self
    }

    /// Set the license URL.
    pub fn license_url(mut self, url: impl Into<String>) -> Self {
        self.license_url = Some(url.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_builder() {
        let info = ModelInfo::new("myuser", "mymodel", "public", "gpu-a100")
            .description("My awesome model")
            .github_url("https://github.com/myuser/mymodel");

        assert_eq!(info.owner, "myuser");
        assert_eq!(info.name, "mymodel");
        assert_eq!(info.visibility, "public");
        assert_eq!(info.hardware, "gpu-a100");
        assert_eq!(info.description, Some("My awesome model".to_string()));
        assert_eq!(
            info.github_url,
            Some("https://github.com/myuser/mymodel".to_string())
        );
    }
}
