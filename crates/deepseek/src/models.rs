//! Models API for DeepSeek.

use crate::client::DeepSeekClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::{ListModelsResponse, Model};

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a DeepSeekClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a DeepSeekClient) -> Self {
        Self { client }
    }

    /// List all available models.
    pub async fn list(&self) -> Result<Vec<Model>> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::MODELS).await?;
                    let body = client.handle_response(response).await?;
                    let models_response: ListModelsResponse = serde_json::from_value(body)?;
                    Ok(models_response.data)
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    // These tests would require mocking the HTTP client
    // For now, we just verify the struct compiles correctly

    #[test]
    fn test_models_client_creation() {
        // This just verifies the code compiles
        // Actual testing requires mocking infrastructure
    }
}
