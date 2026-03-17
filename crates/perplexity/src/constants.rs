//! Constants for the Perplexity API.

use serde::{Deserialize, Serialize};

/// Default base URL for the Perplexity API.
pub const DEFAULT_BASE_URL: &str = "https://api.perplexity.ai";

/// Default API version.
pub const DEFAULT_API_VERSION: &str = "v1";

/// API endpoints.
pub mod endpoints {
    /// Chat completions endpoint.
    pub const CHAT_COMPLETIONS: &str = "/chat/completions";
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

    /// Content-Type header.
    pub const CONTENT_TYPE: &str = "Content-Type";

    /// Accept header.
    pub const ACCEPT: &str = "Accept";

    /// Request ID header (for debugging).
    pub const REQUEST_ID: &str = "X-Request-ID";

    /// Client version header.
    pub const CLIENT_VERSION: &str = "X-Client-Version";
}

/// Perplexity model identifiers.
///
/// Perplexity offers Sonar models with built-in search capabilities.
/// Online models have real-time web search enabled.
/// Chat models are for conversational use without search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Llama 3.1 Sonar Small 128k Online - Fast, efficient model with search
    Llama31SonarSmall128kOnline,
    /// Llama 3.1 Sonar Large 128k Online - Powerful model with search
    Llama31SonarLarge128kOnline,
    /// Llama 3.1 Sonar Huge 128k Online - Most capable model with search
    Llama31SonarHuge128kOnline,
    /// Llama 3.1 Sonar Small 128k Chat - Small model without search
    Llama31SonarSmall128kChat,
    /// Llama 3.1 Sonar Large 128k Chat - Large model without search
    Llama31SonarLarge128kChat,
}

impl Model {
    /// Get the model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Llama31SonarSmall128kOnline => "llama-3.1-sonar-small-128k-online",
            Model::Llama31SonarLarge128kOnline => "llama-3.1-sonar-large-128k-online",
            Model::Llama31SonarHuge128kOnline => "llama-3.1-sonar-huge-128k-online",
            Model::Llama31SonarSmall128kChat => "llama-3.1-sonar-small-128k-chat",
            Model::Llama31SonarLarge128kChat => "llama-3.1-sonar-large-128k-chat",
        }
    }

    /// Get the maximum context window for this model.
    pub fn max_context_tokens(&self) -> usize {
        // All models have 128k context
        128_000
    }

    /// Get the maximum output tokens for this model.
    pub fn max_output_tokens(&self) -> usize {
        match self {
            Model::Llama31SonarSmall128kOnline => 4_096,
            Model::Llama31SonarLarge128kOnline => 4_096,
            Model::Llama31SonarHuge128kOnline => 4_096,
            Model::Llama31SonarSmall128kChat => 4_096,
            Model::Llama31SonarLarge128kChat => 4_096,
        }
    }

    /// Check if this model has online search capabilities.
    pub fn has_search(&self) -> bool {
        matches!(
            self,
            Model::Llama31SonarSmall128kOnline
                | Model::Llama31SonarLarge128kOnline
                | Model::Llama31SonarHuge128kOnline
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
            "llama-3.1-sonar-small-128k-online" => Ok(Model::Llama31SonarSmall128kOnline),
            "llama-3.1-sonar-large-128k-online" => Ok(Model::Llama31SonarLarge128kOnline),
            "llama-3.1-sonar-huge-128k-online" => Ok(Model::Llama31SonarHuge128kOnline),
            "llama-3.1-sonar-small-128k-chat" => Ok(Model::Llama31SonarSmall128kChat),
            "llama-3.1-sonar-large-128k-chat" => Ok(Model::Llama31SonarLarge128kChat),
            _ => Err(format!("Unknown model: {s}")),
        }
    }
}

/// Search recency filter options.
///
/// Controls how recent the search results should be when using online models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRecencyFilter {
    /// Last hour
    Hour,
    /// Last day
    Day,
    /// Last week
    Week,
    /// Last month
    Month,
    /// Last year
    Year,
}

impl SearchRecencyFilter {
    /// Get the filter string.
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchRecencyFilter::Hour => "hour",
            SearchRecencyFilter::Day => "day",
            SearchRecencyFilter::Week => "week",
            SearchRecencyFilter::Month => "month",
            SearchRecencyFilter::Year => "year",
        }
    }
}

impl std::fmt::Display for SearchRecencyFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_as_str() {
        assert_eq!(
            Model::Llama31SonarSmall128kOnline.as_str(),
            "llama-3.1-sonar-small-128k-online"
        );
        assert_eq!(
            Model::Llama31SonarLarge128kOnline.as_str(),
            "llama-3.1-sonar-large-128k-online"
        );
    }

    #[test]
    fn test_model_has_search() {
        assert!(Model::Llama31SonarSmall128kOnline.has_search());
        assert!(Model::Llama31SonarLarge128kOnline.has_search());
        assert!(Model::Llama31SonarHuge128kOnline.has_search());
        assert!(!Model::Llama31SonarSmall128kChat.has_search());
        assert!(!Model::Llama31SonarLarge128kChat.has_search());
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "llama-3.1-sonar-small-128k-online"
                .parse::<Model>()
                .unwrap(),
            Model::Llama31SonarSmall128kOnline
        );
        assert_eq!(
            "llama-3.1-sonar-huge-128k-online".parse::<Model>().unwrap(),
            Model::Llama31SonarHuge128kOnline
        );
    }

    #[test]
    fn test_search_recency_filter() {
        assert_eq!(SearchRecencyFilter::Hour.as_str(), "hour");
        assert_eq!(SearchRecencyFilter::Day.as_str(), "day");
        assert_eq!(SearchRecencyFilter::Month.as_str(), "month");
    }
}
