//! Constants for the Anthropic API.

/// Default base URL for the Anthropic API.
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default API version.
pub const DEFAULT_API_VERSION: &str = "2023-06-01";

/// Maximum context window for Claude 3 models (200K tokens).
pub const MAX_CONTEXT_TOKENS: usize = 200_000;

/// Maximum tokens for Claude 3.5 Sonnet output.
pub const MAX_OUTPUT_TOKENS: usize = 8192;

/// API endpoints.
pub mod endpoints {
    /// Messages endpoint.
    pub const MESSAGES: &str = "/v1/messages";

    /// Batch messages endpoint.
    pub const BATCH: &str = "/v1/messages/batches";

    /// Model listing endpoint.
    pub const MODELS: &str = "/v1/models";
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
    /// API key header.
    pub const X_API_KEY: &str = "x-api-key";

    /// API version header.
    pub const ANTHROPIC_VERSION: &str = "anthropic-version";

    /// Beta features header.
    pub const ANTHROPIC_BETA: &str = "anthropic-beta";

    /// Request ID header (for debugging).
    pub const REQUEST_ID: &str = "request-id";
}

/// Model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Claude 3.5 Sonnet (latest)
    Claude35Sonnet,
    /// Claude 3.5 Haiku
    Claude35Haiku,
    /// Claude 3 Opus
    Claude3Opus,
    /// Claude 3 Sonnet
    Claude3Sonnet,
    /// Claude 3 Haiku
    Claude3Haiku,
}

impl Model {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Claude35Sonnet => "claude-3-5-sonnet-20241022",
            Model::Claude35Haiku => "claude-3-5-haiku-20241022",
            Model::Claude3Opus => "claude-3-opus-20240229",
            Model::Claude3Sonnet => "claude-3-sonnet-20240229",
            Model::Claude3Haiku => "claude-3-haiku-20240307",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        match self {
            Model::Claude35Sonnet => 200_000,
            Model::Claude35Haiku => 200_000,
            Model::Claude3Opus => 200_000,
            Model::Claude3Sonnet => 200_000,
            Model::Claude3Haiku => 200_000,
        }
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            Model::Claude35Sonnet => 8192,
            Model::Claude35Haiku => 8192,
            Model::Claude3Opus => 4096,
            Model::Claude3Sonnet => 4096,
            Model::Claude3Haiku => 4096,
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
            "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet" | "claude-sonnet-4-20250514" => {
                Ok(Model::Claude35Sonnet)
            }
            "claude-3-5-haiku-20241022" | "claude-3-5-haiku" => Ok(Model::Claude35Haiku),
            "claude-3-opus-20240229" | "claude-3-opus" | "claude-opus-4-20250514" => {
                Ok(Model::Claude3Opus)
            }
            "claude-3-sonnet-20240229" | "claude-3-sonnet" => Ok(Model::Claude3Sonnet),
            "claude-3-haiku-20240307" | "claude-3-haiku" => Ok(Model::Claude3Haiku),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(Model::Claude35Sonnet.as_str(), "claude-3-5-sonnet-20241022");
        assert_eq!(Model::Claude3Haiku.as_str(), "claude-3-haiku-20240307");
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "claude-3-5-sonnet".parse::<Model>().unwrap(),
            Model::Claude35Sonnet
        );
        assert_eq!(
            "claude-3-opus-20240229".parse::<Model>().unwrap(),
            Model::Claude3Opus
        );
    }

    #[test]
    fn test_model_display() {
        assert_eq!(Model::Claude35Sonnet.to_string(), "claude-3-5-sonnet-20241022");
    }
}
