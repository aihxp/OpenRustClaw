//! Model listing for vLLM.

use serde::{Deserialize, Serialize};

use crate::client::VllmClient;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for the models API.
#[derive(Debug)]
pub struct Models<'a> {
    client: &'a VllmClient,
}

impl<'a> Models<'a> {
    /// Create a new models client.
    pub fn new(client: &'a VllmClient) -> Self {
        Self { client }
    }

    /// List available models.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let models = client.models().list().await?;
    /// for model in &models.data {
    ///     println!("Model: {}", model.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let model = client.models().get("meta-llama/Llama-3-8b-chat-hf").await?;
    /// println!("Model: {:?}", model.id);
    /// # Ok(())
    /// # }
    /// ```
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
    /// Unix timestamp when the model was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Organization that owns the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
    /// Model permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Vec<ModelPermission>>,
    /// Model capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ModelCapabilities>,
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
}

/// Model permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPermission {
    /// Permission ID.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Unix timestamp when the permission was created.
    pub created: i64,
    /// Whether the model allows fine-tuning.
    #[serde(default)]
    pub allow_fine_tuning: bool,
    /// Whether the model allows creating samples.
    #[serde(default)]
    pub allow_create_engine: bool,
    /// Whether the model allows sampling.
    #[serde(default)]
    pub allow_sampling: bool,
    /// Whether the model allows logprobs.
    #[serde(default)]
    pub allow_logprobs: bool,
    /// Whether the model allows search indices.
    #[serde(default)]
    pub allow_search_indices: bool,
    /// Whether the model allows view.
    #[serde(default)]
    pub allow_view: bool,
    /// Whether the model is owned by the user.
    #[serde(default)]
    pub is_blocking: bool,
}

/// Model capabilities for vLLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Maximum context length.
    #[serde(rename = "max_model_len", skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<usize>,
    /// Whether the model supports chat completions.
    #[serde(default)]
    pub chat_completion: bool,
    /// Whether the model supports completions.
    #[serde(default)]
    pub completion: bool,
    /// Whether the model supports embeddings.
    #[serde(default)]
    pub embedding: bool,
}

/// Response from the models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// The object type.
    pub object: String,
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
        self.data.iter().filter(|m| m.is_embedding_model()).collect()
    }

    /// Get all model IDs.
    pub fn model_ids(&self) -> Vec<&str> {
        self.data.iter().map(|m| m.id.as_str()).collect()
    }
}

/// Model configuration for vLLM server startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name or path.
    pub model: String,
    /// Tensor parallel size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_parallel_size: Option<usize>,
    /// GPU memory utilization (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_utilization: Option<f32>,
    /// Maximum number of batched tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_batched_tokens: Option<usize>,
    /// Maximum number of sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_seqs: Option<usize>,
    /// Maximum model length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<usize>,
    /// Quantization method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    /// Data type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtype: Option<String>,
    /// Device type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Enable prefix caching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_prefix_caching: Option<bool>,
    /// Block size for PagedAttention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<usize>,
    /// Swap space in GiB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_space: Option<usize>,
    /// Enforce eager execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_eager: Option<bool>,
}

impl ModelConfig {
    /// Create a new model configuration.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            tensor_parallel_size: None,
            gpu_memory_utilization: None,
            max_num_batched_tokens: None,
            max_num_seqs: None,
            max_model_len: None,
            quantization: None,
            dtype: None,
            device: None,
            enable_prefix_caching: None,
            block_size: None,
            swap_space: None,
            enforce_eager: None,
        }
    }

    /// Set tensor parallel size.
    pub fn tensor_parallel_size(mut self, size: usize) -> Self {
        self.tensor_parallel_size = Some(size);
        self
    }

    /// Set GPU memory utilization.
    pub fn gpu_memory_utilization(mut self, utilization: f32) -> Self {
        self.gpu_memory_utilization = Some(utilization.clamp(0.0, 1.0));
        self
    }

    /// Set max batched tokens.
    pub fn max_num_batched_tokens(mut self, tokens: usize) -> Self {
        self.max_num_batched_tokens = Some(tokens);
        self
    }

    /// Set max sequences.
    pub fn max_num_seqs(mut self, seqs: usize) -> Self {
        self.max_num_seqs = Some(seqs);
        self
    }

    /// Set max model length.
    pub fn max_model_len(mut self, len: usize) -> Self {
        self.max_model_len = Some(len);
        self
    }

    /// Set quantization.
    pub fn quantization(mut self, method: impl Into<String>) -> Self {
        self.quantization = Some(method.into());
        self
    }

    /// Set data type.
    pub fn dtype(mut self, dtype: impl Into<String>) -> Self {
        self.dtype = Some(dtype.into());
        self
    }

    /// Enable prefix caching.
    pub fn enable_prefix_caching(mut self, enabled: bool) -> Self {
        self.enable_prefix_caching = Some(enabled);
        self
    }
}

/// Popular model IDs as constants.
pub mod popular {
    // Meta Llama models
    /// Llama 3.1 8B Instruct.
    pub const LLAMA_3_1_8B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-8B-Instruct";

    /// Llama 3.1 70B Instruct.
    pub const LLAMA_3_1_70B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-70B-Instruct";

    /// Llama 3.1 405B Instruct.
    pub const LLAMA_3_1_405B_INSTRUCT: &str = "meta-llama/Meta-Llama-3.1-405B-Instruct";

    /// Llama 3 8B Instruct.
    pub const LLAMA_3_8B_INSTRUCT: &str = "meta-llama/Meta-Llama-3-8B-Instruct";

    /// Llama 3 70B Instruct.
    pub const LLAMA_3_70B_INSTRUCT: &str = "meta-llama/Meta-Llama-3-70B-Instruct";

    // Mistral models
    /// Mistral 7B Instruct v0.3.
    pub const MISTRAL_7B_INSTRUCT: &str = "mistralai/Mistral-7B-Instruct-v0.3";

    /// Mixtral 8x7B Instruct.
    pub const MIXTRAL_8X7B_INSTRUCT: &str = "mistralai/Mixtral-8x7B-Instruct-v0.1";

    /// Mixtral 8x22B Instruct.
    pub const MIXTRAL_8X22B_INSTRUCT: &str = "mistralai/Mixtral-8x22B-Instruct-v0.1";

    /// Mistral Large.
    pub const MISTRAL_LARGE: &str = "mistralai/Mistral-Large-Instruct-2407";

    // Qwen models
    /// Qwen 2.5 7B Instruct.
    pub const QWEN_2_5_7B_INSTRUCT: &str = "Qwen/Qwen2.5-7B-Instruct";

    /// Qwen 2.5 72B Instruct.
    pub const QWEN_2_5_72B_INSTRUCT: &str = "Qwen/Qwen2.5-72B-Instruct";

    // DeepSeek models
    /// DeepSeek V3.
    pub const DEEPSEEK_V3: &str = "deepseek-ai/DeepSeek-V3";

    /// DeepSeek R1.
    pub const DEEPSEEK_R1: &str = "deepseek-ai/DeepSeek-R1";

    // Google models
    /// Gemma 2 9B Instruct.
    pub const GEMMA_2_9B_INSTRUCT: &str = "google/gemma-2-9b-it";

    /// Gemma 2 27B Instruct.
    pub const GEMMA_2_27B_INSTRUCT: &str = "google/gemma-2-27b-it";

    // Embedding models
    /// BGE Large EN v1.5.
    pub const BGE_LARGE_EN: &str = "BAAI/bge-large-en-v1.5";

    /// BGE Base EN v1.5.
    pub const BGE_BASE_EN: &str = "BAAI/bge-base-en-v1.5";

    /// E5 Mistral 7B Instruct.
    pub const E5_MISTRAL_7B: &str = "intfloat/e5-mistral-7b-instruct";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_provider() {
        let model = ModelInfo {
            id: "meta-llama/Llama-3-8b-chat-hf".to_string(),
            object: "model".to_string(),
            created: None,
            owned_by: Some("Meta".to_string()),
            permission: None,
            capabilities: None,
        };

        assert_eq!(model.provider(), "meta-llama");
        assert_eq!(model.model_name(), "Llama-3-8b-chat-hf");
        assert!(model.is_chat_model());
    }

    #[test]
    fn test_embedding_model_detection() {
        let model = ModelInfo {
            id: "BAAI/bge-large-en-v1.5".to_string(),
            object: "model".to_string(),
            created: None,
            owned_by: None,
            permission: None,
            capabilities: None,
        };

        assert!(model.is_embedding_model());
        assert!(!model.is_chat_model());
    }

    #[test]
    fn test_models_response_filtering() {
        let response = ModelsResponse {
            object: "list".to_string(),
            data: vec![
                ModelInfo {
                    id: "meta-llama/Llama-3-8b-chat-hf".to_string(),
                    object: "model".to_string(),
                    created: None,
                    owned_by: None,
                    permission: None,
                    capabilities: None,
                },
                ModelInfo {
                    id: "BAAI/bge-large-en-v1.5".to_string(),
                    object: "model".to_string(),
                    created: None,
                    owned_by: None,
                    permission: None,
                    capabilities: None,
                },
            ],
        };

        assert_eq!(response.chat_models().len(), 1);
        assert_eq!(response.embedding_models().len(), 1);
        assert_eq!(response.by_provider("meta-llama").len(), 1);
    }
}
