//! Constants for the OpenAI API.

/// Default base URL for the OpenAI API.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// API version prefix.
pub const API_VERSION: &str = "/v1";

/// Maximum context window for GPT-4o (128K tokens).
pub const GPT4O_MAX_TOKENS: usize = 128_000;

/// Maximum context window for GPT-4 Turbo (128K tokens).
pub const GPT4_TURBO_MAX_TOKENS: usize = 128_000;

/// Maximum context window for GPT-3.5 Turbo (16K tokens).
pub const GPT35_TURBO_MAX_TOKENS: usize = 16_384;

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/v1/chat/completions";

    /// Embeddings endpoint.
    pub const EMBEDDINGS: &str = "/v1/embeddings";

    /// Models endpoint.
    pub const MODELS: &str = "/v1/models";

    /// Assistants endpoint.
    pub const ASSISTANTS: &str = "/v1/assistants";

    /// Threads endpoint.
    pub const THREADS: &str = "/v1/threads";

    /// Batch endpoint.
    pub const BATCH: &str = "/v1/batches";

    /// Files endpoint.
    pub const FILES: &str = "/v1/files";

    /// Fine-tuning endpoint.
    pub const FINE_TUNING: &str = "/v1/fine_tuning/jobs";
}

/// Default retry configuration.
pub mod retry {
    /// Maximum number of retry attempts.
    pub const MAX_RETRIES: u32 = 3;

    /// Initial retry delay in milliseconds.
    pub const INITIAL_DELAY_MS: u64 = 1000;

    /// Maximum retry delay in milliseconds.
    pub const MAX_DELAY_MS: u64 = 32000;

    /// Exponential backoff multiplier.
    pub const BACKOFF_MULTIPLIER: f64 = 2.0;
}

/// Model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// GPT-4o (latest)
    Gpt4O,
    /// GPT-4o mini
    Gpt4OMini,
    /// GPT-4 Turbo
    Gpt4Turbo,
    /// GPT-4
    Gpt4,
    /// GPT-3.5 Turbo
    Gpt35Turbo,
    /// GPT-3.5 Turbo 16K
    Gpt35Turbo16K,
    /// Text Embedding 3 Small
    TextEmbedding3Small,
    /// Text Embedding 3 Large
    TextEmbedding3Large,
    /// Text Embedding Ada 002
    TextEmbeddingAda002,
}

impl Model {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Gpt4O => "gpt-4o",
            Model::Gpt4OMini => "gpt-4o-mini",
            Model::Gpt4Turbo => "gpt-4-turbo-preview",
            Model::Gpt4 => "gpt-4",
            Model::Gpt35Turbo => "gpt-3.5-turbo",
            Model::Gpt35Turbo16K => "gpt-3.5-turbo-16k",
            Model::TextEmbedding3Small => "text-embedding-3-small",
            Model::TextEmbedding3Large => "text-embedding-3-large",
            Model::TextEmbeddingAda002 => "text-embedding-ada-002",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            Model::Gpt4O => 128_000,
            Model::Gpt4OMini => 128_000,
            Model::Gpt4Turbo => 128_000,
            Model::Gpt4 => 8_192,
            Model::Gpt35Turbo => 4_096,
            Model::Gpt35Turbo16K => 16_384,
            Model::TextEmbedding3Small => 8_191,
            Model::TextEmbedding3Large => 8_191,
            Model::TextEmbeddingAda002 => 8_191,
        }
    }

    /// Check if this is an embedding model.
    pub fn is_embedding(&self) -> bool {
        matches!(
            self,
            Model::TextEmbedding3Small | Model::TextEmbedding3Large | Model::TextEmbeddingAda002
        )
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "gpt-4o" => Ok(Model::Gpt4O),
            "gpt-4o-mini" => Ok(Model::Gpt4OMini),
            "gpt-4-turbo" | "gpt-4-turbo-preview" => Ok(Model::Gpt4Turbo),
            "gpt-4" => Ok(Model::Gpt4),
            "gpt-3.5-turbo" => Ok(Model::Gpt35Turbo),
            "gpt-3.5-turbo-16k" => Ok(Model::Gpt35Turbo16K),
            "text-embedding-3-small" => Ok(Model::TextEmbedding3Small),
            "text-embedding-3-large" => Ok(Model::TextEmbedding3Large),
            "text-embedding-ada-002" => Ok(Model::TextEmbeddingAda002),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(Model::Gpt4O.as_str(), "gpt-4o");
        assert_eq!(Model::Gpt35Turbo.as_str(), "gpt-3.5-turbo");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!("gpt-4o".parse::<Model>().unwrap(), Model::Gpt4O);
        assert_eq!("gpt-4-turbo".parse::<Model>().unwrap(), Model::Gpt4Turbo);
    }

    #[test]
    fn test_model_is_embedding() {
        assert!(Model::TextEmbedding3Small.is_embedding());
        assert!(!Model::Gpt4O.is_embedding());
    }
}
