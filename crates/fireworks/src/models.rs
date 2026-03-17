//! Model listing for Fireworks AI.

use serde::{Deserialize, Serialize};

use crate::client::FireworksClient;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a FireworksClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a FireworksClient) -> Self {
        Self { client }
    }

    /// List available models.
    pub async fn list(&self) -> Result<ModelsResponse> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::MODELS).await?;
                    let body = client.handle_response(response).await?;
                    let models: ModelsResponse = serde_json::from_value(body)?;
                    Ok(models)
                })
            })
            .await
    }

    /// Get a specific model by ID.
    pub async fn get(&self, model_id: impl AsRef<str>) -> Result<ModelInfo> {
        let path = format!("{}/{}", endpoints::MODELS, model_id.as_ref());

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let model: ModelInfo = serde_json::from_value(body)?;
                    Ok(model)
                })
            })
            .await
    }
}

/// Information about a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID.
    pub id: String,
    /// Model object type.
    pub object: String,
    /// Organization that created the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    /// Model description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<usize>,
    /// License.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Pricing information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,
}

impl ModelInfo {
    /// Get the provider/account prefix.
    pub fn provider(&self) -> &str {
        self.id.split('/').next().unwrap_or("unknown")
    }

    /// Get the model name without provider prefix.
    pub fn model_name(&self) -> &str {
        self.id.split('/').next_back().unwrap_or(&self.id)
    }

    /// Check if this is a chat model.
    pub fn is_chat_model(&self) -> bool {
        let id_lower = self.id.to_lowercase();
        id_lower.contains("chat") || id_lower.contains("instruct") || id_lower.contains("function")
    }

    /// Check if this is an embedding model.
    pub fn is_embedding_model(&self) -> bool {
        let id_lower = self.id.to_lowercase();
        id_lower.contains("embed")
            || id_lower.contains("bge-")
            || id_lower.contains("gte-")
            || id_lower.contains("nomic-embed")
    }

    /// Check if this is an image generation model.
    pub fn is_image_model(&self) -> bool {
        let id_lower = self.id.to_lowercase();
        id_lower.contains("sdxl")
            || id_lower.contains("stable-diffusion")
            || id_lower.contains("flux")
            || id_lower.contains("playground")
    }

    /// Get pricing per million tokens.
    pub fn pricing_per_million(&self) -> Option<(f64, f64)> {
        self.pricing
            .as_ref()
            .map(|p| (p.input * 1_000_000.0, p.output * 1_000_000.0))
    }
}

/// Pricing for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Price per input token.
    pub input: f64,
    /// Price per output token.
    pub output: f64,
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

    /// Filter models by type (chat, embedding, etc.).
    pub fn chat_models(&self) -> Vec<&ModelInfo> {
        self.data.iter().filter(|m| m.is_chat_model()).collect()
    }

    /// Get embedding models.
    pub fn embedding_models(&self) -> Vec<&ModelInfo> {
        self.data
            .iter()
            .filter(|m| m.is_embedding_model())
            .collect()
    }

    /// Get image generation models.
    pub fn image_models(&self) -> Vec<&ModelInfo> {
        self.data.iter().filter(|m| m.is_image_model()).collect()
    }

    /// Get sorted models by price (cheapest first).
    pub fn by_price(&self) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.data.iter().collect();
        models.sort_by(|a, b| {
            let a_price = a
                .pricing
                .as_ref()
                .map(|p| p.input + p.output)
                .unwrap_or(f64::MAX);
            let b_price = b
                .pricing
                .as_ref()
                .map(|p| p.input + p.output)
                .unwrap_or(f64::MAX);
            a_price
                .partial_cmp(&b_price)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        models
    }

    /// Get models sorted by context length (largest first).
    pub fn by_context_length(&self) -> Vec<&ModelInfo> {
        let mut models: Vec<_> = self.data.iter().collect();
        models.sort_by(|a, b| {
            let a_ctx = a.context_length.unwrap_or(0);
            let b_ctx = b.context_length.unwrap_or(0);
            b_ctx.cmp(&a_ctx) // Descending order
        });
        models
    }
}

/// Popular model IDs as constants.
pub mod popular {
    /// Llama 3.1 405B Instruct.
    pub const LLAMA_3_1_405B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p1-405b-instruct";

    /// Llama 3.1 70B Instruct.
    pub const LLAMA_3_1_70B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p1-70b-instruct";

    /// Llama 3.1 8B Instruct.
    pub const LLAMA_3_1_8B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p1-8b-instruct";

    /// Mixtral 8x22B Instruct.
    pub const MIXTRAL_8X22B_INSTRUCT: &str = "accounts/fireworks/models/mixtral-8x22b-instruct";

    /// Mixtral 8x7B Instruct.
    pub const MIXTRAL_8X7B_INSTRUCT: &str = "accounts/fireworks/models/mixtral-8x7b-instruct";

    /// Firefunction V2 (function calling).
    pub const FIREFUNCTION_V2: &str = "accounts/fireworks/models/firefunction-v2";

    /// Llama 3 70B Instruct.
    pub const LLAMA_3_70B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3-70b-instruct";

    /// Llama 3 8B Instruct.
    pub const LLAMA_3_8B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3-8b-instruct";

    /// Llama 3.2 1B Instruct.
    pub const LLAMA_3_2_1B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p2-1b-instruct";

    /// Llama 3.2 3B Instruct.
    pub const LLAMA_3_2_3B_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p2-3b-instruct";

    /// Qwen 2.5 72B Instruct.
    pub const QWEN_2_5_72B_INSTRUCT: &str = "accounts/fireworks/models/qwen2p5-72b-instruct";

    /// DeepSeek V3.
    pub const DEEPSEEK_V3: &str = "accounts/fireworks/models/deepseek-v3";

    /// DeepSeek R1.
    pub const DEEPSEEK_R1: &str = "accounts/fireworks/models/deepseek-r1";

    /// SDXL (image generation).
    pub const SDXL: &str = "accounts/fireworks/models/sdxl";

    /// Flux.1 Dev (image generation).
    pub const FLUX_1_DEV: &str = "accounts/fireworks/models/flux-1-dev";

    /// Nomic Embed Text v1.5 (embedding).
    pub const NOMIC_EMBED_TEXT_V1_5: &str = "accounts/fireworks/models/nomic-embed-text-v1-5";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_parsing() {
        let model = ModelInfo {
            id: "accounts/fireworks/models/llama-v3p1-8b-instruct".to_string(),
            object: "model".to_string(),
            organization: Some("Fireworks".to_string()),
            description: Some("Llama 3.1 8B Instruct".to_string()),
            name: Some("Llama 3.1 8B Instruct".to_string()),
            context_length: Some(131072),
            license: Some("llama3.1".to_string()),
            pricing: Some(ModelPricing {
                input: 0.0000002,
                output: 0.0000002,
            }),
        };

        assert_eq!(model.provider(), "accounts");
        assert_eq!(model.model_name(), "llama-v3p1-8b-instruct");
        assert!(model.is_chat_model());
        assert!(!model.is_embedding_model());
        assert!(!model.is_image_model());
    }

    #[test]
    fn test_embedding_model_detection() {
        let model = ModelInfo {
            id: "accounts/fireworks/models/nomic-embed-text-v1-5".to_string(),
            object: "model".to_string(),
            organization: None,
            description: None,
            name: None,
            context_length: None,
            license: None,
            pricing: None,
        };

        assert!(model.is_embedding_model());
        assert!(!model.is_chat_model());
    }

    #[test]
    fn test_image_model_detection() {
        let model = ModelInfo {
            id: "accounts/fireworks/models/flux-1-dev".to_string(),
            object: "model".to_string(),
            organization: None,
            description: None,
            name: None,
            context_length: None,
            license: None,
            pricing: None,
        };

        assert!(model.is_image_model());
        assert!(!model.is_chat_model());
    }
}
