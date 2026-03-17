//! Model listing for Together AI.

use serde::{Deserialize, Serialize};

use crate::client::TogetherClient;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a TogetherClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a TogetherClient) -> Self {
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
    /// Get the provider prefix (e.g., "meta-llama", "mistralai").
    pub fn provider(&self) -> &str {
        self.id.split('/').next().unwrap_or("unknown")
    }

    /// Get the model name without provider prefix.
    pub fn model_name(&self) -> &str {
        self.id.split('/').nth(1).unwrap_or(&self.id)
    }

    /// Check if this is a chat model.
    pub fn is_chat_model(&self) -> bool {
        let id_lower = self.id.to_lowercase();
        id_lower.contains("chat") || id_lower.contains("instruct")
    }

    /// Check if this is an embedding model.
    pub fn is_embedding_model(&self) -> bool {
        let id_lower = self.id.to_lowercase();
        id_lower.contains("embed") || id_lower.contains("bge-") || id_lower.contains("gte-")
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
    /// Llama 3 70B Chat.
    pub const LLAMA_3_70B_CHAT: &str = "meta-llama/Llama-3-70b-chat-hf";

    /// Llama 3 8B Chat.
    pub const LLAMA_3_8B_CHAT: &str = "meta-llama/Llama-3-8b-chat-hf";

    /// Llama 3.1 405B Instruct.
    pub const LLAMA_3_1_405B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo";

    /// Llama 3.1 70B Instruct.
    pub const LLAMA_3_1_70B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo";

    /// Llama 3.1 8B Instruct.
    pub const LLAMA_3_1_8B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo";

    /// Mixtral 8x22B Instruct.
    pub const MIXTRAL_8X22B_INSTRUCT: &str = "mistralai/Mixtral-8x22B-Instruct-v0.1";

    /// Mixtral 8x7B Instruct.
    pub const MIXTRAL_8X7B_INSTRUCT: &str = "mistralai/Mixtral-8x7B-Instruct-v0.1";

    /// Mistral 7B Instruct.
    pub const MISTRAL_7B_INSTRUCT: &str = "mistralai/Mistral-7B-Instruct-v0.1";

    /// WizardLM 2 8x22B.
    pub const WIZARDLM_2_8X22B: &str = "microsoft/WizardLM-2-8x22B";

    /// Gemma 7B Instruct.
    pub const GEMMA_7B_IT: &str = "google/gemma-7b-it";

    /// Gemma 2 9B Instruct.
    pub const GEMMA_2_9B_IT: &str = "google/gemma-2-9b-it";

    /// Gemma 2 27B Instruct.
    pub const GEMMA_2_27B_IT: &str = "google/gemma-2-27b-it";

    /// Qwen 2.5 72B Instruct.
    pub const QWEN_2_5_72B_INSTRUCT: &str = "Qwen/Qwen2.5-72B-Instruct-Turbo";

    /// DeepSeek V3.
    pub const DEEPSEEK_V3: &str = "deepseek-ai/DeepSeek-V3";

    /// DeepSeek R1.
    pub const DEEPSEEK_R1: &str = "deepseek-ai/DeepSeek-R1";

    /// BGE Large En v1.5 (embedding).
    pub const BGE_LARGE_EN: &str = "BAAI/bge-large-en-v1.5";

    /// BGE Base En v1.5 (embedding).
    pub const BGE_BASE_EN: &str = "BAAI/bge-base-en-v1.5";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_parsing() {
        let model = ModelInfo {
            id: "meta-llama/Llama-3-8b-chat-hf".to_string(),
            object: "model".to_string(),
            organization: Some("Meta".to_string()),
            description: Some("Llama 3 8B Chat".to_string()),
            name: Some("Llama 3 8B Chat".to_string()),
            context_length: Some(8192),
            license: Some("llama3".to_string()),
            pricing: Some(ModelPricing {
                input: 0.0000002,
                output: 0.0000002,
            }),
        };

        assert_eq!(model.provider(), "meta-llama");
        assert_eq!(model.model_name(), "Llama-3-8b-chat-hf");
        assert!(model.is_chat_model());
        assert!(!model.is_embedding_model());
    }

    #[test]
    fn test_embedding_model_detection() {
        let model = ModelInfo {
            id: "BAAI/bge-large-en-v1.5".to_string(),
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
}
