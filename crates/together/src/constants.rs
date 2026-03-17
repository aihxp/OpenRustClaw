//! Constants for the Together AI API.

/// Default base URL for the Together AI API.
pub const DEFAULT_BASE_URL: &str = "https://api.together.xyz";

/// Default app name.
pub const DEFAULT_APP_NAME: &str = "together-ai-sdk";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/v1/chat/completions";

    /// Completions endpoint.
    pub const COMPLETIONS: &str = "/v1/completions";

    /// Embeddings endpoint.
    pub const EMBEDDINGS: &str = "/v1/embeddings";

    /// Fine-tuning endpoint.
    pub const FINE_TUNING: &str = "/v1/fine-tunes";

    /// Models endpoint.
    pub const MODELS: &str = "/v1/models";

    /// Files endpoint (for fine-tuning uploads).
    pub const FILES: &str = "/v1/files";
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

/// Popular Together AI models.
#[allow(dead_code)]
pub mod models {
    /// Llama 3 70B Chat.
    pub const LLAMA_3_70B_CHAT: &str = "meta-llama/Llama-3-70b-chat-hf";

    /// Llama 3 8B Chat.
    pub const LLAMA_3_8B_CHAT: &str = "meta-llama/Llama-3-8b-chat-hf";

    /// Mixtral 8x22B Instruct.
    pub const MIXTRAL_8X22B_INSTRUCT: &str = "mistralai/Mixtral-8x22B-Instruct-v0.1";

    /// WizardLM 2 8x22B.
    pub const WIZARDLM_2_8X22B: &str = "microsoft/WizardLM-2-8x22B";

    /// Gemma 7B Instruct.
    pub const GEMMA_7B_IT: &str = "google/gemma-7b-it";

    /// Llama 3 70B (for embeddings).
    pub const LLAMA_3_70B: &str = "meta-llama/Llama-3-70b";

    /// BGE Large En v1.5 (embedding model).
    pub const BGE_LARGE_EN: &str = "BAAI/bge-large-en-v1.5";

    /// BGE Base En v1.5 (embedding model).
    pub const BGE_BASE_EN: &str = "BAAI/bge-base-en-v1.5";
}
