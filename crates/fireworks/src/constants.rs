//! Constants for the Fireworks AI API.

/// Default base URL for the Fireworks AI API.
pub const DEFAULT_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";

/// Default app name.
pub const DEFAULT_APP_NAME: &str = "fireworks-ai-sdk";

/// API endpoints.
#[allow(dead_code)]
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/chat/completions";

    /// Completions endpoint.
    pub const COMPLETIONS: &str = "/completions";

    /// Embeddings endpoint.
    pub const EMBEDDINGS: &str = "/embeddings";

    /// Fine-tuning endpoint.
    pub const FINE_TUNING: &str = "/fine-tunes";

    /// Models endpoint.
    pub const MODELS: &str = "/models";

    /// Files endpoint (for fine-tuning uploads).
    pub const FILES: &str = "/files";

    /// Image generation endpoint.
    pub const IMAGE_GENERATION: &str = "/image-generation";
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

/// Popular Fireworks AI models.
#[allow(dead_code)]
pub mod models {
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

    /// Llama 3.2 11B Vision Instruct.
    pub const LLAMA_3_2_11B_VISION_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p2-11b-vision-instruct";

    /// Llama 3.2 90B Vision Instruct.
    pub const LLAMA_3_2_90B_VISION_INSTRUCT: &str = "accounts/fireworks/models/llama-v3p2-90b-vision-instruct";

    /// Qwen 2.5 72B Instruct.
    pub const QWEN_2_5_72B_INSTRUCT: &str = "accounts/fireworks/models/qwen2p5-72b-instruct";

    /// Qwen 2.5 Coder 32B Instruct.
    pub const QWEN_2_5_CODER_32B_INSTRUCT: &str = "accounts/fireworks/models/qwen2p5-coder-32b-instruct";

    /// DeepSeek V3.
    pub const DEEPSEEK_V3: &str = "accounts/fireworks/models/deepseek-v3";

    /// DeepSeek R1.
    pub const DEEPSEEK_R1: &str = "accounts/fireworks/models/deepseek-r1";

    /// Nous Hermes 2 Pro Llama 3 8B.
    pub const NOUS_HERMES_2_PRO_LLAMA_3_8B: &str = "accounts/fireworks/models/nous-hermes-2-pro-llama-3-8b";

    /// Nous Hermes 2 Mixtral 8x7B DPO.
    pub const NOUS_HERMES_2_MIXTRAL_8X7B_DPO: &str = "accounts/fireworks/models/nous-hermes-2-mixtral-8x7b-dpo";

    /// SDXL (image generation).
    pub const SDXL: &str = "accounts/fireworks/models/sdxl";

    /// Stable Diffusion 3 Medium (image generation).
    pub const STABLE_DIFFUSION_3_MEDIUM: &str = "accounts/fireworks/models/stable-diffusion-3-medium";

    /// Stable Diffusion 3.5 Large (image generation).
    pub const STABLE_DIFFUSION_3_5_LARGE: &str = "accounts/fireworks/models/stable-diffusion-3-5-large";

    /// Playground v2.5 (image generation).
    pub const PLAYGROUND_V2_5: &str = "accounts/fireworks/models/playground-v2-5-1024px-aesthetic";

    /// Flux.1 Schnell (image generation).
    pub const FLUX_1_SCHNELL: &str = "accounts/fireworks/models/flux-1-schnell";

    /// Flux.1 Dev (image generation).
    pub const FLUX_1_DEV: &str = "accounts/fireworks/models/flux-1-dev";

    /// Nomic Embed Text v1.5 (embedding).
    pub const NOMIC_EMBED_TEXT_V1_5: &str = "accounts/fireworks/models/nomic-embed-text-v1-5";

    /// Nomic Embed Text v1 (embedding).
    pub const NOMIC_EMBED_TEXT_V1: &str = "accounts/fireworks/models/nomic-embed-text-v1";

    /// WhereIsAI UAE Large v1 (embedding).
    pub const WHEREISAI_UAE_LARGE_V1: &str = "accounts/fireworks/models/whereisai-uae-large-v1";

    /// BGE Large En v1.5 (embedding).
    pub const BGE_LARGE_EN_V1_5: &str = "accounts/fireworks/models/bge-large-en-v1-5";

    /// BGE Base En v1.5 (embedding).
    pub const BGE_BASE_EN_V1_5: &str = "accounts/fireworks/models/bge-base-en-v1-5";
}
