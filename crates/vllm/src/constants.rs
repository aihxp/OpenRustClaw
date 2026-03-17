//! Constants for the vLLM API.

/// Default base URL for the vLLM API.
pub const DEFAULT_BASE_URL: &str = "http://localhost:8000";

/// Default app name.
pub const DEFAULT_APP_NAME: &str = "vllm-rust-sdk";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint (OpenAI-compatible).
    pub const CHAT_COMPLETIONS: &str = "/v1/chat/completions";

    /// Completions endpoint (OpenAI-compatible).
    pub const COMPLETIONS: &str = "/v1/completions";

    /// Embeddings endpoint (OpenAI-compatible).
    pub const EMBEDDINGS: &str = "/v1/embeddings";

    /// Models endpoint (OpenAI-compatible).
    pub const MODELS: &str = "/v1/models";

    /// Tokenize endpoint (vLLM-specific).
    pub const TOKENIZE: &str = "/tokenize";

    /// Detokenize endpoint (vLLM-specific).
    pub const DETOKENIZE: &str = "/detokenize";

    /// Health check endpoint.
    pub const HEALTH: &str = "/health";

    /// Metrics endpoint (Prometheus format).
    pub const METRICS: &str = "/metrics";
}

/// Default retry configuration.
pub mod retry {
    /// Maximum number of retry attempts.
    pub const MAX_RETRIES: u32 = 3;

    /// Initial retry delay in milliseconds.
    pub const INITIAL_DELAY_MS: u64 = 1000;

    /// Maximum retry delay in milliseconds.
    pub const MAX_DELAY_MS: u64 = 32000;
}

/// Default generation parameters.
pub mod defaults {
    /// Default temperature for sampling.
    pub const TEMPERATURE: f32 = 1.0;

    /// default_top_p for sampling.
    pub const TOP_P: f32 = 1.0;

    /// Default max tokens to generate.
    pub const MAX_TOKENS: usize = 256;
}

/// Popular models commonly used with vLLM.
#[allow(dead_code)]
pub mod models {
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

    /// DeepSeek R1 Distill Llama 70B.
    pub const DEEPSEEK_R1_LLAMA_70B: &str = "deepseek-ai/DeepSeek-R1-Distill-Llama-70B";

    /// DeepSeek R1 Distill Qwen 32B.
    pub const DEEPSEEK_R1_QWEN_32B: &str = "deepseek-ai/DeepSeek-R1-Distill-Qwen-32B";

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

/// PagedAttention configuration options.
pub mod paged_attention {
    /// Default block size for KV cache.
    pub const DEFAULT_BLOCK_SIZE: usize = 16;

    /// Default GPU memory utilization (0.0 to 1.0).
    pub const DEFAULT_GPU_MEMORY_UTILIZATION: f32 = 0.9;

    /// Default swap space per GPU (GiB).
    pub const DEFAULT_SWAP_SPACE: usize = 4;
}

/// Tensor/pipeline parallelism configuration.
pub mod parallelism {
    /// Environment variable for tensor parallel size.
    pub const TENSOR_PARALLEL_SIZE_ENV: &str = "VLLM_TENSOR_PARALLEL_SIZE";

    /// Environment variable for pipeline parallel size.
    pub const PIPELINE_PARALLEL_SIZE_ENV: &str = "VLLM_PIPELINE_PARALLEL_SIZE";

    /// Default tensor parallel size.
    pub const DEFAULT_TENSOR_PARALLEL_SIZE: usize = 1;

    /// Default pipeline parallel size.
    pub const DEFAULT_PIPELINE_PARALLEL_SIZE: usize = 1;
}
