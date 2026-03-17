//! Constants for the OpenRouter API.

/// Default base URL for the OpenRouter API.
pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";

/// HTTP Referer header value.
pub const DEFAULT_REFERER: &str = "https://openrustclaw.dev";

/// Default app name.
pub const DEFAULT_APP_NAME: &str = "openrouter-api";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/v1/chat/completions";

    /// Models endpoint.
    pub const MODELS: &str = "/v1/models";

    /// Key stats endpoint.
    pub const KEY_STATS: &str = "/v1/auth/key";

    /// Generation stats endpoint.
    pub const GENERATION: &str = "/v1/generation";
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

/// Model provider prefixes.
pub mod providers {
    /// Anthropic.
    pub const ANTHROPIC: &str = "anthropic";
    /// OpenAI.
    pub const OPENAI: &str = "openai";
    /// Google.
    pub const GOOGLE: &str = "google";
    /// Meta.
    pub const META: &str = "meta";
    /// Mistral.
    pub const MISTRAL: &str = "mistralai";
    /// Cohere.
    pub const COHERE: &str = "cohere";
    /// Azure.
    pub const AZURE: &str = "azure";
}
