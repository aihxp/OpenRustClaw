//! Deployments API - Manage model deployments.

// Deployments module

use crate::client::ReplicateClient;
use crate::error::Result;
use crate::types::{Deployment, PaginatedResponse, Prediction};
use crate::predictions::PredictionRequest;

/// Deployments API client.
#[derive(Debug)]
pub struct Deployments<'a> {
    client: &'a ReplicateClient,
}

impl<'a> Deployments<'a> {
    /// Create a new deployments API client.
    pub fn new(client: &'a ReplicateClient) -> Self {
        Self { client }
    }

    /// List all deployments.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let deployments = client.deployments().list().await?;
    /// for deployment in deployments.results {
    ///     println!("{}/{} - {:?}", deployment.owner, deployment.name, deployment.current_release);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Result<PaginatedResponse<Deployment>> {
        let path = "/deployments";
        let response = self.client.get(path).await?;
        self.client.handle_response(response).await
    }

    /// Get a deployment by owner and name.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::ReplicateClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let deployment = client.deployments().get("myuser", "mydeployment").await?;
    /// println!("Deployment: {:?}", deployment);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, owner: &str, name: &str) -> Result<Deployment> {
        let path = format!("/deployments/{}/{}", owner, name);
        let response = self.client.get(&path).await?;
        self.client.handle_response(response).await
    }

    /// Create a new deployment.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, DeploymentConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let config = DeploymentConfig::new(
    ///     "stability-ai/sdxl",
    ///     "version_id_here",
    ///     "gpu-a100-large",
    ///     1,
    ///     5,
    /// );
    ///
    /// let deployment = client.deployments().create("myuser", "mydeployment", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        _owner: &str,
        name: &str,
        config: DeploymentConfig,
    ) -> Result<Deployment> {
        let path = "/deployments";
        let body = serde_json::json!({
            "name": name,
            "model": config.model,
            "version": config.version,
            "hardware": config.hardware,
            "min_instances": config.min_instances,
            "max_instances": config.max_instances,
        });
        let response = self.client.post(path, body).await?;
        self.client.handle_response(response).await
    }

    /// Update a deployment.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use replicate::{ReplicateClient, DeploymentConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ReplicateClient::new("your-api-token")?;
    ///
    /// let config = DeploymentConfig::new(
    ///     "stability-ai/sdxl",
    ///     "new_version_id",
    ///     "gpu-a100-large",
    ///     2,
    ///     10,
    /// );
    ///
    /// let deployment = client.deployments().update("myuser", "mydeployment", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        owner: &str,
        name: &str,
        config: DeploymentConfig,
    ) -> Result<Deployment> {
        let path = format!("/deployments/{}/{}", owner, name);
        let body = serde_json::json!({
            "model": config.model,
            "version": config.version,
            "hardware": config.hardware,
            "min_instances": config.min_instances,
            "max_instances": config.max_instances,
        });
        let response = self.client.patch(&path, body).await?;
        self.client.handle_response(response).await
    }

    /// Delete a deployment.
    ///
    /// Note: You can only delete deployments that have been offline and unused for at least 15 minutes.
    pub async fn delete(&self, owner: &str, name: &str) -> Result<()> {
        let path = format!("/deployments/{}/{}", owner, name);
        let response = self.client.delete(&path).await?;
        self.client.handle_empty_response(response).await
    }

    /// Create a prediction using a deployment.
    ///
    /// This is the preferred way to run deployed models.
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
    /// let prediction = client.deployments()
    ///     .create_prediction("myuser", "mydeployment", request)
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
        let path = format!("/deployments/{}/{}/predictions", owner, name);
        let mut body = serde_json::to_value(&request)?;
        
        // Remove model/version from body as they're determined by the deployment
        if let Some(obj) = body.as_object_mut() {
            obj.remove("model");
            obj.remove("version");
        }
        
        let mut headers = reqwest::header::HeaderMap::new();
        
        // Handle sync mode with Prefer header
        if let Some(wait_secs) = request.wait {
            headers.insert(
                "Prefer",
                reqwest::header::HeaderValue::from_str(&format!("wait={}", wait_secs))
                    .map_err(|e| crate::error::ReplicateError::InvalidHeader { message: e.to_string() })?,
            );
        }
        
        let response = if headers.is_empty() {
            self.client.post(&path, body).await?
        } else {
            self.client.post_with_headers(&path, body, headers).await?
        };
        
        self.client.handle_response(response).await
    }

    /// Run a deployment and wait for completion.
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
        
        self.wait_for_completion(owner, name, &prediction.id, poll_interval, timeout).await
    }

    /// Wait for a prediction on a deployment to complete.
    async fn wait_for_completion(
        &self,
        _owner: &str,
        _name: &str,
        id: &str,
        poll_interval: Option<std::time::Duration>,
        timeout: Option<std::time::Duration>,
    ) -> Result<Prediction> {
        use tracing::trace;
        
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
}

/// Configuration for creating or updating a deployment.
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    /// The full name of the model (e.g., "stability-ai/sdxl").
    pub model: String,
    /// The 64-character version ID.
    pub version: String,
    /// The hardware SKU.
    pub hardware: String,
    /// Minimum number of instances for scaling.
    pub min_instances: u32,
    /// Maximum number of instances for scaling.
    pub max_instances: u32,
}

impl DeploymentConfig {
    /// Create a new deployment configuration.
    pub fn new(
        model: impl Into<String>,
        version: impl Into<String>,
        hardware: impl Into<String>,
        min_instances: u32,
        max_instances: u32,
    ) -> Self {
        Self {
            model: model.into(),
            version: version.into(),
            hardware: hardware.into(),
            min_instances,
            max_instances,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the hardware.
    pub fn hardware(mut self, hardware: impl Into<String>) -> Self {
        self.hardware = hardware.into();
        self
    }

    /// Set the minimum instances.
    pub fn min_instances(mut self, min_instances: u32) -> Self {
        self.min_instances = min_instances;
        self
    }

    /// Set the maximum instances.
    pub fn max_instances(mut self, max_instances: u32) -> Self {
        self.max_instances = max_instances;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_config_builder() {
        let config = DeploymentConfig::new(
            "stability-ai/sdxl",
            "abc123...",
            "gpu-a100-large",
            1,
            5,
        );

        assert_eq!(config.model, "stability-ai/sdxl");
        assert_eq!(config.version, "abc123...");
        assert_eq!(config.hardware, "gpu-a100-large");
        assert_eq!(config.min_instances, 1);
        assert_eq!(config.max_instances, 5);
    }

    #[test]
    fn test_deployment_config_methods() {
        let config = DeploymentConfig::new("model", "version", "hardware", 1, 5)
            .min_instances(2)
            .max_instances(10);

        assert_eq!(config.min_instances, 2);
        assert_eq!(config.max_instances, 10);
    }
}
