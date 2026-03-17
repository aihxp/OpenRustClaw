//! Routing strategies for OpenRouter.

use serde::{Deserialize, Serialize};

/// Routing strategy for model selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// Optimize for lowest price.
    Price,
    /// Optimize for highest throughput.
    Throughput,
    /// Default quality routing.
    #[default]
    Quality,
    /// Include web search results.
    Online,
}

impl RouteStrategy {
    /// Get the model suffix for this strategy.
    pub fn suffix(&self) -> &str {
        match self {
            RouteStrategy::Price => ":floor",
            RouteStrategy::Throughput => ":nitro",
            RouteStrategy::Quality => "",
            RouteStrategy::Online => ":online",
        }
    }

    /// Apply this strategy to a model ID.
    pub fn apply(&self, model: &str) -> String {
        let suffix = self.suffix();
        if suffix.is_empty() {
            model.to_string()
        } else {
            format!("{}{}", model, suffix)
        }
    }
}

impl std::fmt::Display for RouteStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteStrategy::Price => write!(f, "price"),
            RouteStrategy::Throughput => write!(f, "throughput"),
            RouteStrategy::Quality => write!(f, "quality"),
            RouteStrategy::Online => write!(f, "online"),
        }
    }
}

/// Fallback configuration.
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Models to try in order.
    pub models: Vec<String>,
    /// Whether to include the original request in fallback attempts.
    pub include_original: bool,
}

impl FallbackConfig {
    /// Create a new fallback configuration.
    pub fn new(models: Vec<String>) -> Self {
        Self {
            models,
            include_original: true,
        }
    }

    /// Set whether to include the original request.
    pub fn include_original(mut self, include: bool) -> Self {
        self.include_original = include;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_strategy_suffix() {
        assert_eq!(RouteStrategy::Price.suffix(), ":floor");
        assert_eq!(RouteStrategy::Throughput.suffix(), ":nitro");
        assert_eq!(RouteStrategy::Quality.suffix(), "");
        assert_eq!(RouteStrategy::Online.suffix(), ":online");
    }

    #[test]
    fn test_apply_strategy() {
        let model = "anthropic/claude-3.5-sonnet";
        assert_eq!(
            RouteStrategy::Price.apply(model),
            "anthropic/claude-3.5-sonnet:floor"
        );
        assert_eq!(RouteStrategy::Quality.apply(model), model);
    }

    #[test]
    fn test_fallback_config() {
        let config = FallbackConfig::new(vec!["model-a".to_string(), "model-b".to_string()]);
        assert_eq!(config.models.len(), 2);
        assert!(config.include_original);
    }
}
