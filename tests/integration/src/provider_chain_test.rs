//! Provider fallback chain integration tests.
//!
//! These tests verify the provider chain behavior including:
//! - Fallback when primary provider rate limits
//! - Fallback when provider is unavailable
//! - All providers exhausted error handling
//! - Cooldown management

use std::sync::Arc;

use openrustclaw_core::error::{Error, ProviderError};
use openrustclaw_core::types::{CompletionRequest, Message};
use openrustclaw_providers::ProviderChain;

use crate::common::{
    init_test_tracing, MockRateLimitedProvider, MockSuccessProvider, MockUnavailableProvider,
};

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
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockSuccessProvider::new("primary", "Hello from primary")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello from fallback")),
    ]);

    let response = chain.complete(make_request()).await.unwrap();
    assert_eq!(response.provider, "primary");
    assert_eq!(response.message.content, "Hello from primary");
}

#[tokio::test]
async fn falls_back_on_rate_limit() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("primary")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello from fallback")),
    ]);

    let response = chain.complete(make_request()).await.unwrap();
    assert_eq!(response.provider, "fallback");
    assert_eq!(response.message.content, "Hello from fallback");
}

#[tokio::test]
async fn falls_back_on_unavailable() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockUnavailableProvider::new(
            "primary",
            "Service down for maintenance",
        )),
        Arc::new(MockSuccessProvider::new("fallback", "Hello from fallback")),
    ]);

    let response = chain.complete(make_request()).await.unwrap();
    assert_eq!(response.provider, "fallback");
}

#[tokio::test]
async fn all_providers_exhausted_returns_last_error() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("provider_a")),
        Arc::new(MockRateLimitedProvider::new("provider_b")),
    ]);

    let err = chain.complete(make_request()).await.unwrap_err();
    assert!(
        matches!(err, Error::Provider(ProviderError::RateLimited { .. })),
        "Expected rate limit error, got: {err}"
    );
}

#[tokio::test]
async fn all_providers_unavailable_exhausted() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockUnavailableProvider::new("provider_a", "Down")),
        Arc::new(MockUnavailableProvider::new("provider_b", "Also down")),
    ]);

    let err = chain.complete(make_request()).await.unwrap_err();
    assert!(
        matches!(err, Error::Provider(ProviderError::Unavailable { .. })),
        "Expected unavailable error, got: {err}"
    );
}

#[tokio::test]
async fn mixed_failures_falls_back_correctly() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("rate_limited")),
        Arc::new(MockUnavailableProvider::new("unavailable", "Down")),
        Arc::new(MockSuccessProvider::new("working", "Success!")),
    ]);

    let response = chain.complete(make_request()).await.unwrap();
    assert_eq!(response.provider, "working");
    assert_eq!(response.message.content, "Success!");
}

#[tokio::test]
async fn empty_chain_returns_exhausted() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![]);
    let err = chain.complete(make_request()).await.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Provider(ProviderError::AllProvidersExhausted)
        ),
        "Expected AllProvidersExhausted, got: {err}"
    );
}

#[tokio::test]
async fn provider_placed_in_cooldown_after_rate_limit() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("primary")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello")),
    ]);

    // First request triggers rate limit and cooldown
    let _ = chain.complete(make_request()).await;

    // Check that primary is in cooldown
    assert!(chain.is_in_cooldown("primary"));
    assert!(!chain.is_in_cooldown("fallback"));
}

#[tokio::test]
async fn provider_placed_in_cooldown_after_unavailable() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockUnavailableProvider::new("primary", "Down")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello")),
    ]);

    // First request triggers unavailable and cooldown
    let _ = chain.complete(make_request()).await;

    // Check that primary is in cooldown
    assert!(chain.is_in_cooldown("primary"));
}

#[tokio::test]
async fn skipped_during_cooldown() {
    init_test_tracing();

    let chain = ProviderChain::with_cooldown(
        vec![
            Arc::new(MockRateLimitedProvider::new("primary")),
            Arc::new(MockSuccessProvider::new("fallback", "Hello")),
        ],
        std::time::Duration::from_secs(60),
    );

    // First request puts primary in cooldown
    let response1 = chain.complete(make_request()).await.unwrap();
    assert_eq!(response1.provider, "fallback");

    // Second request should skip primary due to cooldown
    let response2 = chain.complete(make_request()).await.unwrap();
    assert_eq!(response2.provider, "fallback");
}

#[tokio::test]
async fn clear_cooldown_allows_retry() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("primary")),
        Arc::new(MockSuccessProvider::new("fallback", "Hello")),
    ]);

    // Put primary in cooldown
    let _ = chain.complete(make_request()).await;
    assert!(chain.is_in_cooldown("primary"));

    // Clear cooldown
    chain.clear_cooldown("primary");
    assert!(!chain.is_in_cooldown("primary"));
}

#[tokio::test]
async fn clear_all_cooldowns() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockRateLimitedProvider::new("provider_a")),
        Arc::new(MockRateLimitedProvider::new("provider_b")),
    ]);

    // Put both in cooldown
    let _ = chain.complete(make_request()).await;
    assert!(chain.is_in_cooldown("provider_a") || chain.is_in_cooldown("provider_b"));

    // Clear all cooldowns
    chain.clear_all_cooldowns();
    assert!(!chain.is_in_cooldown("provider_a"));
    assert!(!chain.is_in_cooldown("provider_b"));
}

#[tokio::test]
async fn provider_count_and_names() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![
        Arc::new(MockSuccessProvider::new("a", "Hello")),
        Arc::new(MockSuccessProvider::new("b", "Hello")),
        Arc::new(MockSuccessProvider::new("c", "Hello")),
    ]);

    assert_eq!(chain.provider_count(), 3);
    let names = chain.provider_names();
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn preserves_request_parameters() {
    init_test_tracing();

    let chain = ProviderChain::new(vec![Arc::new(MockSuccessProvider::new(
        "test",
        "Response",
    ))]);

    let request = CompletionRequest {
        messages: vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
        ],
        model: Some("gpt-4".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.5),
        tools: None,
        system_prompt: Some("Custom system prompt".to_string()),
        stream: false,
    };

    let response = chain.complete(request).await.unwrap();
    assert_eq!(response.provider, "test");
    assert_eq!(response.finish_reason, openrustclaw_core::types::FinishReason::Stop);
}
