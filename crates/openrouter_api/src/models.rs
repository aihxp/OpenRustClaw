//! Model listing for OpenRouter.

use serde::{Deserialize, Serialize};

/// Information about a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID.
    pub id: String,
    /// Model name.
    pub name: String,
    /// Model description.
    pub description: Option<String>,
    /// Pricing information.
    pub pricing: ModelPricing,
    /// Context length.
    pub context_length: Option<usize>,
    /// Whether the model supports tool use.
    pub supports_tool_use: Option<bool>,
    /// Whether the model supports vision.
    pub supports_vision: Option<bool>,
}

/// Pricing for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Price per input token.
    pub prompt: f64,
    /// Price per output token.
    pub completion: f64,
    /// Price per image (if applicable).
    pub image: Option<f64>,
    /// Price per request (if applicable).
    pub request: Option<f64>,
}

impl ModelInfo {
    /// Get the provider prefix (e.g., "anthropic", "openai").
    pub fn provider(&self) -> &str {
        self.id.split('/').next().unwrap_or("unknown")
    }

    /// Get the model name without provider prefix.
    pub fn model_name(&self) -> &str {
        self.id.split('/').nth(1).unwrap_or(&self.id)
    }

    /// Calculate the cost for a given number of tokens.
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = input_tokens as f64 * self.pricing.prompt;
        let output_cost = output_tokens as f64 * self.pricing.completion;
        input_cost + output_cost
    }
}

/// Response from the models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// List of models.
    pub data: Vec<ModelInfo>,
}

impl ModelsResponse {
    /// Find a model by ID.
    pub fn find(&self, id: &str) -> Option<&ModelInfo> {
        self.data.iter().find(|m| m.id == id)
    }

    /// Filter models by provider.
    pub fn by_provider(&self, provider: &str) -> Vec<&ModelInfo> {
        self.data
            .iter()
            .filter(|m| m.provider() == provider)
            .collect()
    }

    /// Get sorted models by price (cheapest first).
    pub fn by_price(&self) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.data.iter().collect();
        models.sort_by(|a, b| {
            let a_price = a.pricing.prompt + a.pricing.completion;
            let b_price = b.pricing.prompt + b.pricing.completion;
            a_price.partial_cmp(&b_price).unwrap_or(std::cmp::Ordering::Equal)
        });
        models
    }
}

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a crate::OpenRouterClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a crate::OpenRouterClient) -> Self {
        Self { client }
    }

    /// List available models.
    pub async fn list(&self) -> crate::Result<ModelsResponse> {
        use crate::constants::endpoints;
        
        let response = self.client.get(endpoints::MODELS).await?;
        let body = self.client.handle_response(response).await?;
        let models: ModelsResponse = serde_json::from_value(body)?;
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_parsing() {
        let model = ModelInfo {
            id: "anthropic/claude-3.5-sonnet".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            description: None,
            pricing: ModelPricing {
                prompt: 0.000003,
                completion: 0.000015,
                image: None,
                request: None,
            },
            context_length: Some(200000),
            supports_tool_use: Some(true),
            supports_vision: Some(true),
        };

        assert_eq!(model.provider(), "anthropic");
        assert_eq!(model.model_name(), "claude-3.5-sonnet");
    }

    #[test]
    fn test_calculate_cost() {
        let model = ModelInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            pricing: ModelPricing {
                prompt: 0.001,
                completion: 0.002,
                image: None,
                request: None,
            },
            context_length: None,
            supports_tool_use: None,
            supports_vision: None,
        };

        let cost = model.calculate_cost(1000, 500);
        assert_eq!(cost, 2.0); // 1000 * 0.001 + 500 * 0.002 = 2.0
    }
}
