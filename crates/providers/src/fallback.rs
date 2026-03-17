//! Provider fallback chain with cooldowns.
//!
//! The [`ProviderChain`] tries providers in order. When a provider returns a
//! rate limit or unavailable error, it is placed in cooldown and the next
//! provider is attempted. This enables seamless failover across multiple LLM
//! backends.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{info, warn};

use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::{CompletionRequest, CompletionResponse};

/// A fallback chain that tries LLM providers in order.
///
/// On rate limits or provider unavailability, the failing provider is placed
/// in a cooldown period and the next provider in the chain is tried.
pub struct ProviderChain {
    /// Ordered list of providers to try.
    providers: Vec<Arc<dyn LlmProvider>>,
    /// Map of provider name to cooldown expiry instant.
    cooldowns: DashMap<String, Instant>,
    /// How long to cooldown a provider after a rate limit or unavailable error.
    cooldown_duration: Duration,
}

impl ProviderChain {
    /// Create a new provider chain with the given providers.
    ///
    /// Providers are tried in the order given. The default cooldown duration
    /// is 60 seconds.
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Self {
        Self {
            providers,
            cooldowns: DashMap::new(),
            cooldown_duration: Duration::from_secs(60),
        }
    }

    /// Create a new provider chain with a custom cooldown duration.
    pub fn with_cooldown(providers: Vec<Arc<dyn LlmProvider>>, cooldown: Duration) -> Self {
        Self {
            providers,
            cooldowns: DashMap::new(),
            cooldown_duration: cooldown,
        }
    }

    /// Try providers in order until one succeeds.
    ///
    /// On rate limit or unavailable errors, the provider is placed in cooldown
    /// and the next provider is attempted. Returns the first successful
    /// response, or the last error if all providers fail.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut last_error = None;

        for provider in &self.providers {
            let name = provider.provider_name().to_string();

            // Skip if in cooldown.
            if let Some(cooldown_until) = self.cooldowns.get(&name)
                && Instant::now() < *cooldown_until
            {
                info!(provider = %name, "Skipping provider (in cooldown)");
                continue;
            }

            match provider.complete(request.clone()).await {
                Ok(response) => {
                    return Ok(response);
                }
                Err(Error::Provider(ProviderError::RateLimited { .. })) => {
                    warn!(provider = %name, "Rate limited, cooling down");
                    self.cooldowns
                        .insert(name.clone(), Instant::now() + self.cooldown_duration);
                    last_error = Some(Error::Provider(ProviderError::RateLimited {
                        provider: name,
                        retry_after_secs: Some(self.cooldown_duration.as_secs()),
                    }));
                }
                Err(Error::Provider(ProviderError::Unavailable { .. })) => {
                    warn!(provider = %name, "Provider unavailable, trying next");
                    self.cooldowns
                        .insert(name.clone(), Instant::now() + self.cooldown_duration);
                    last_error = Some(Error::Provider(ProviderError::Unavailable {
                        provider: name,
                        message: "Provider unavailable".to_string(),
                    }));
                }
                Err(e) => {
                    warn!(provider = %name, error = %e, "Provider error");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(Error::Provider(ProviderError::AllProvidersExhausted)))
    }

    /// Return the number of providers in the chain.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Return a list of provider names in the chain.
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.provider_name()).collect()
    }

    /// Check whether a specific provider is currently in cooldown.
    pub fn is_in_cooldown(&self, provider_name: &str) -> bool {
        self.cooldowns
            .get(provider_name)
            .is_some_and(|until| Instant::now() < *until)
    }

    /// Clear cooldown for a specific provider (e.g. after manual reset).
    pub fn clear_cooldown(&self, provider_name: &str) {
        self.cooldowns.remove(provider_name);
    }

    /// Clear all provider cooldowns.
    pub fn clear_all_cooldowns(&self) {
        self.cooldowns.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;

    use async_trait::async_trait;
    use futures::Stream;
    use openrustclaw_core::types::{FinishReason, Message, StreamChunk, TokenUsage, ToolFormat};

    /// A mock provider that always succeeds with a fixed response.
    struct MockSuccessProvider {
        name: &'static str,
    }

    #[async_trait]
    impl LlmProvider for MockSuccessProvider {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                id: format!("{}-response", self.name),
                message: Message::assistant(format!("Hello from {}", self.name)),
                model: "mock-model".to_string(),
                usage: TokenUsage::default(),
                provider: self.name.to_string(),
                finish_reason: FinishReason::Stop,
            })
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            Err(Error::Provider(ProviderError::StreamError {
                provider: self.name.to_string(),
                message: "Not supported".to_string(),
            }))
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }
        fn max_tokens(&self) -> usize {
            1000
        }
        fn provider_name(&self) -> &str {
            self.name
        }
        fn supports_strict_tools(&self) -> bool {
            false
        }
        fn supports_streaming_tool_deltas(&self) -> bool {
            false
        }
        fn native_tool_format(&self) -> ToolFormat {
            ToolFormat::OpenAi
        }
    }

    /// A mock provider that always returns a rate limit error.
    struct MockRateLimitedProvider {
        name: &'static str,
    }

    #[async_trait]
    impl LlmProvider for MockRateLimitedProvider {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            Err(Error::Provider(ProviderError::RateLimited {
                provider: self.name.to_string(),
                retry_after_secs: Some(30),
            }))
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            Err(Error::Provider(ProviderError::StreamError {
                provider: self.name.to_string(),
                message: "Not supported".to_string(),
            }))
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }
        fn max_tokens(&self) -> usize {
            1000
        }
        fn provider_name(&self) -> &str {
            self.name
        }
        fn supports_strict_tools(&self) -> bool {
            false
        }
        fn supports_streaming_tool_deltas(&self) -> bool {
            false
        }
        fn native_tool_format(&self) -> ToolFormat {
            ToolFormat::OpenAi
        }
    }

    fn make_request() -> CompletionRequest {
        CompletionRequest {
            messages: vec![Message::user("Hello")],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: None,
            system_prompt: None,
            stream: false,
        }
    }

    #[tokio::test]
    async fn first_provider_succeeds() {
        let chain = ProviderChain::new(vec![
            Arc::new(MockSuccessProvider { name: "primary" }),
            Arc::new(MockSuccessProvider { name: "fallback" }),
        ]);

        let response = chain.complete(make_request()).await.unwrap();
        assert_eq!(response.provider, "primary");
    }

    #[tokio::test]
    async fn falls_back_on_rate_limit() {
        let chain = ProviderChain::new(vec![
            Arc::new(MockRateLimitedProvider { name: "primary" }),
            Arc::new(MockSuccessProvider { name: "fallback" }),
        ]);

        let response = chain.complete(make_request()).await.unwrap();
        assert_eq!(response.provider, "fallback");
    }

    #[tokio::test]
    async fn all_providers_exhausted() {
        let chain = ProviderChain::new(vec![
            Arc::new(MockRateLimitedProvider { name: "provider_a" }),
            Arc::new(MockRateLimitedProvider { name: "provider_b" }),
        ]);

        let err = chain.complete(make_request()).await.unwrap_err();
        assert!(
            matches!(err, Error::Provider(ProviderError::RateLimited { .. })),
            "Expected rate limit error, got: {err}"
        );
    }

    #[tokio::test]
    async fn empty_chain_returns_exhausted() {
        let chain = ProviderChain::new(vec![]);
        let err = chain.complete(make_request()).await.unwrap_err();
        assert!(matches!(
            err,
            Error::Provider(ProviderError::AllProvidersExhausted)
        ));
    }

    #[test]
    fn provider_count_and_names() {
        let chain = ProviderChain::new(vec![
            Arc::new(MockSuccessProvider { name: "a" }),
            Arc::new(MockSuccessProvider { name: "b" }),
        ]);
        assert_eq!(chain.provider_count(), 2);
        assert_eq!(chain.provider_names(), vec!["a", "b"]);
    }

    #[test]
    fn cooldown_management() {
        let chain = ProviderChain::new(vec![]);
        assert!(!chain.is_in_cooldown("test"));

        chain
            .cooldowns
            .insert("test".to_string(), Instant::now() + Duration::from_secs(60));
        assert!(chain.is_in_cooldown("test"));

        chain.clear_cooldown("test");
        assert!(!chain.is_in_cooldown("test"));
    }
}
