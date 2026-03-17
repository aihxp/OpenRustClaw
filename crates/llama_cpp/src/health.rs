//! Health check API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::HealthResponse;

/// Client for the health API.
#[derive(Debug)]
pub struct Health<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Health<'a> {
    /// Create a new health client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self { client }
    }

    /// Check the server health.
    pub async fn check(&self) -> Result<HealthResponse> {
        let response = self.client.get(endpoints::HEALTH).await?;
        let body = self.client.handle_response(response).await?;
        let health_response: HealthResponse = serde_json::from_value(body)?;
        Ok(health_response)
    }

    /// Check if the server is healthy (convenience method).
    pub async fn is_healthy(&self) -> bool {
        match self.check().await {
            Ok(health) => health.is_healthy(),
            Err(_) => false,
        }
    }

    /// Wait for the server to become healthy.
    pub async fn wait_for_healthy(&self, timeout: std::time::Duration) -> Result<HealthResponse> {
        let start = std::time::Instant::now();
        let check_interval = std::time::Duration::from_millis(500);

        loop {
            match self.check().await {
                Ok(health) if health.is_healthy() => return Ok(health),
                _ => {
                    if start.elapsed() > timeout {
                        return Err(crate::error::LlamaCppError::Timeout {
                            operation: "wait for healthy".to_string(),
                        });
                    }
                    tokio::time::sleep(check_interval).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_healthy() {
        let health = HealthResponse {
            status: "ok".to_string(),
            slots_idle: Some(2),
            slots_processing: Some(1),
        };
        assert!(health.is_healthy());
    }

    #[test]
    fn test_health_response_unhealthy() {
        let health = HealthResponse {
            status: "error".to_string(),
            slots_idle: None,
            slots_processing: None,
        };
        assert!(!health.is_healthy());
    }
}
