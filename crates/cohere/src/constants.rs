//! Constants for the Cohere API.

/// Default base URL for the Cohere API.
pub const DEFAULT_BASE_URL: &str = "https://api.cohere.com";

/// Default API version.
pub const DEFAULT_API_VERSION: &str = "v1";

/// API endpoints.
pub mod endpoints {
    /// Chat endpoint.
    pub const CHAT: &str = "/v1/chat";

    /// Generate endpoint (legacy).
    pub const GENERATE: &str = "/v1/generate";

    /// Embeddings endpoint.
    pub const EMBED: &str = "/v1/embed";

    /// Rerank endpoint.
    pub const RERANK: &str = "/v1/rerank";

    /// Classify endpoint.
    pub const CLASSIFY: &str = "/v1/classify";

    /// Summarize endpoint.
    pub const SUMMARIZE: &str = "/v1/summarize";

    /// Tokenize endpoint.
    pub const TOKENIZE: &str = "/v1/tokenize";

    /// Detokenize endpoint.
    pub const DETOKENIZE: &str = "/v1/detokenize";

    /// Check API key endpoint.
    pub const CHECK_API_KEY: &str = "/v1/check-api-key";
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

/// HTTP header names.
pub mod headers {
    /// Authorization header.
    pub const AUTHORIZATION: &str = "Authorization";

    /// Request ID header (for debugging).
    pub const REQUEST_ID: &str = "X-Request-ID";

    /// Client version header.
    pub const CLIENT_VERSION: &str = "X-Client-Version";

    /// Cohere version header.
    pub const COHERE_VERSION: &str = "Cohere-Version";
}

/// Model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Command R - Fast, efficient conversational model
    CommandR,
    /// Command R+ - Advanced conversational model
    CommandRPlus,
    /// Command - General purpose generation
    Command,
    /// Command Nightly - Latest experimental version
    CommandNightly,
    /// C4AI Aya 23 - Multilingual model
    C4aiAya23,
}

impl Model {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::CommandR => "command-r",
            Model::CommandRPlus => "command-r-plus",
            Model::Command => "command",
            Model::CommandNightly => "command-nightly",
            Model::C4aiAya23 => "c4ai-aya-23",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            Model::CommandR => 128_000,
            Model::CommandRPlus => 128_000,
            Model::Command => 4_096,
            Model::CommandNightly => 8_192,
            Model::C4aiAya23 => 8_192,
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            Model::CommandR => 4_096,
            Model::CommandRPlus => 4_096,
            Model::Command => 4_096,
            Model::CommandNightly => 4_096,
            Model::C4aiAya23 => 4_096,
        }
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
            "command-r" => Ok(Model::CommandR),
            "command-r-plus" => Ok(Model::CommandRPlus),
            "command" => Ok(Model::Command),
            "command-nightly" => Ok(Model::CommandNightly),
            "c4ai-aya-23" => Ok(Model::C4aiAya23),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

/// Embedding model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddingModel {
    /// English embedding model v3
    EmbedEnglishV3,
    /// Multilingual embedding model v3
    EmbedMultilingualV3,
    /// English light embedding model v3
    EmbedEnglishLightV3,
    /// Multilingual light embedding model v3
    EmbedMultilingualLightV3,
    /// English embedding model v2
    EmbedEnglishV2,
    /// English light embedding model v2
    EmbedEnglishLightV2,
    /// Multilingual embedding model v2
    EmbedMultilingualV2,
}

impl EmbeddingModel {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            EmbeddingModel::EmbedEnglishV3 => "embed-english-v3.0",
            EmbeddingModel::EmbedMultilingualV3 => "embed-multilingual-v3.0",
            EmbeddingModel::EmbedEnglishLightV3 => "embed-english-light-v3.0",
            EmbeddingModel::EmbedMultilingualLightV3 => "embed-multilingual-light-v3.0",
            EmbeddingModel::EmbedEnglishV2 => "embed-english-v2.0",
            EmbeddingModel::EmbedEnglishLightV2 => "embed-english-light-v2.0",
            EmbeddingModel::EmbedMultilingualV2 => "embed-multilingual-v2.0",
        }
    }

    /// Get the embedding dimension for this model.
    pub fn dimensions(&self) -> usize {
        match self {
            EmbeddingModel::EmbedEnglishV3 => 1024,
            EmbeddingModel::EmbedMultilingualV3 => 1024,
            EmbeddingModel::EmbedEnglishLightV3 => 384,
            EmbeddingModel::EmbedMultilingualLightV3 => 384,
            EmbeddingModel::EmbedEnglishV2 => 4096,
            EmbeddingModel::EmbedEnglishLightV2 => 1024,
            EmbeddingModel::EmbedMultilingualV2 => 768,
        }
    }
}

impl std::fmt::Display for EmbeddingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for EmbeddingModel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "embed-english-v3.0" => Ok(EmbeddingModel::EmbedEnglishV3),
            "embed-multilingual-v3.0" => Ok(EmbeddingModel::EmbedMultilingualV3),
            "embed-english-light-v3.0" => Ok(EmbeddingModel::EmbedEnglishLightV3),
            "embed-multilingual-light-v3.0" => Ok(EmbeddingModel::EmbedMultilingualLightV3),
            "embed-english-v2.0" => Ok(EmbeddingModel::EmbedEnglishV2),
            "embed-english-light-v2.0" => Ok(EmbeddingModel::EmbedEnglishLightV2),
            "embed-multilingual-v2.0" => Ok(EmbeddingModel::EmbedMultilingualV2),
            _ => Err(format!("Unknown embedding model: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(Model::CommandR.as_str(), "command-r");
        assert_eq!(Model::CommandRPlus.as_str(), "command-r-plus");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!("command-r".parse::<Model>().unwrap(), Model::CommandR);
        assert_eq!(
            "command-r-plus".parse::<Model>().unwrap(),
            Model::CommandRPlus
        );
    }

    #[test]
    fn test_model_display() {
        assert_eq!(Model::CommandR.to_string(), "command-r");
    }

    #[test]
    fn test_embedding_model_dimensions() {
        assert_eq!(EmbeddingModel::EmbedEnglishV3.dimensions(), 1024);
        assert_eq!(EmbeddingModel::EmbedEnglishLightV3.dimensions(), 384);
    }
}
