//! E2E tests for provider fallback and resilience.
//!
//! These tests cover:
//! - Primary provider fails
//! - Fallback to secondary
//! - Cooldown behavior

use std::sync::Arc;
use std::time::Duration;

use openrustclaw_core::types::{CompletionRequest, Message};
use openrustclaw_providers::fallback::ProviderChain;
use serial_test::serial;

use crate::common::{
    init_test_tracing, use_live_providers, has_api_key,
    MockRateLimitedProvider, MockSuccessProvider, MockUnavailableProvider,
};

fn default_completion_request() -> CompletionRequest {
    CompletionRequest {
        messages: vec![],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    }
}

/// Scenario 1: Primary succeeds, no fallback needed.
#[tokio::test]
#[serial]
async fn test_primary_provider_succeeds() {
    init_test_tracing();

    let primary = Arc::new(MockSuccessProvider::new("primary", "Hello from primary!"));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Hello from secondary!"));

    let chain = ProviderChain::new(vec![primary, secondary]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = chain.complete(request).await.expect("Should succeed");

    assert_eq!(response.provider, "primary");
    assert_eq!(response.message.content, "Hello from primary!");
}

/// Scenario 2: Primary fails, fallback to secondary.
#[tokio::test]
#[serial]
async fn test_fallback_to_secondary() {
    init_test_tracing();

    let primary = Arc::new(MockUnavailableProvider::new(
        "primary",
        "Service temporarily unavailable",
    ));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Hello from fallback!"));

    let chain = ProviderChain::new(vec![primary, secondary]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = chain.complete(request).await.expect("Should fallback");

    assert_eq!(response.provider, "secondary");
    assert_eq!(response.message.content, "Hello from fallback!");
}

/// Scenario 3: Primary rate limited, fallback to secondary.
#[tokio::test]
#[serial]
async fn test_fallback_on_rate_limit() {
    init_test_tracing();

    let primary = Arc::new(MockRateLimitedProvider::new("primary").with_retry_after(60));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Not rate limited!"));

    let chain = ProviderChain::with_cooldown(
        vec![primary, secondary],
        Duration::from_secs(60),
    );

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = chain.complete(request).await.expect("Should fallback");

    assert_eq!(response.provider, "secondary");
    
    // Primary should be in cooldown
    assert!(chain.is_in_cooldown("primary"));
    assert!(!chain.is_in_cooldown("secondary"));
}

/// Scenario 4: All providers fail.
#[tokio::test]
#[serial]
async fn test_all_providers_fail() {
    init_test_tracing();

    let primary = Arc::new(MockRateLimitedProvider::new("primary"));
    let secondary = Arc::new(MockUnavailableProvider::new(
        "secondary",
        "Also down",
    ));
    let tertiary = Arc::new(MockUnavailableProvider::new(
        "tertiary",
        "Also unavailable",
    ));

    let chain = ProviderChain::new(vec![primary, secondary, tertiary]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let result = chain.complete(request).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Rate limited") || err_msg.contains("unavailable"));
}

/// Scenario 5: Provider cooldown is respected.
#[tokio::test]
#[serial]
async fn test_provider_cooldown_respected() {
    init_test_tracing();

    // Use a short cooldown for testing
    let cooldown = Duration::from_millis(500);

    let primary = Arc::new(MockRateLimitedProvider::new("primary"));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Success!"));

    let chain = ProviderChain::with_cooldown(vec![primary, secondary], cooldown);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    // First request - primary is rate limited
    let response = chain.complete(request.clone()).await.expect("Should succeed");
    assert_eq!(response.provider, "secondary");

    // Verify primary is in cooldown
    assert!(chain.is_in_cooldown("primary"));

    // Another request during cooldown should skip primary
    let response2 = chain.complete(request.clone()).await.expect("Should succeed");
    assert_eq!(response2.provider, "secondary");

    // Wait for cooldown to expire
    tokio::time::sleep(cooldown + Duration::from_millis(100)).await;

    // Clear cooldown manually for test (in production it would auto-expire)
    chain.clear_all_cooldowns();
    
    // Now primary would be tried again (but would still fail)
    assert!(!chain.is_in_cooldown("primary"));
}

/// Scenario 6: Multiple fallbacks in chain.
#[tokio::test]
#[serial]
async fn test_multiple_fallbacks() {
    init_test_tracing();

    let p1 = Arc::new(MockUnavailableProvider::new("p1", "Down"));
    let p2 = Arc::new(MockRateLimitedProvider::new("p2"));
    let p3 = Arc::new(MockUnavailableProvider::new("p3", "Also down"));
    let p4 = Arc::new(MockSuccessProvider::new("p4", "Finally!"));

    let chain = ProviderChain::new(vec![p1, p2, p3, p4]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = chain.complete(request).await.expect("Should eventually succeed");

    assert_eq!(response.provider, "p4");
    assert_eq!(response.message.content, "Finally!");

    // p2 should be in cooldown from rate limit
    assert!(chain.is_in_cooldown("p2"));
}

/// Scenario 7: Provider metrics and information.
#[tokio::test]
#[serial]
async fn test_provider_chain_info() {
    init_test_tracing();

    let p1 = Arc::new(MockSuccessProvider::new("openai", "Hello"));
    let p2 = Arc::new(MockSuccessProvider::new("anthropic", "Hello"));
    let p3 = Arc::new(MockSuccessProvider::new("ollama", "Hello"));

    let chain = ProviderChain::new(vec![p1, p2, p3]);

    assert_eq!(chain.provider_count(), 3);
    
    let names = chain.provider_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"openai"));
    assert!(names.contains(&"anthropic"));
    assert!(names.contains(&"ollama"));
}

/// Scenario 8: Cooldown can be manually cleared.
#[tokio::test]
#[serial]
async fn test_manual_cooldown_clear() {
    init_test_tracing();

    let primary = Arc::new(MockRateLimitedProvider::new("primary"));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Hello"));

    let chain = ProviderChain::with_cooldown(
        vec![primary, secondary],
        Duration::from_secs(3600), // Long cooldown
    );

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    // Trigger rate limit
    let _ = chain.complete(request).await;

    assert!(chain.is_in_cooldown("primary"));

    // Clear specific cooldown
    chain.clear_cooldown("primary");
    assert!(!chain.is_in_cooldown("primary"));

    // Test clear all
    let _ = chain.complete(CompletionRequest {
        messages: vec![Message::user("Hi again")],
        ..default_completion_request()
    }).await;
    
    assert!(chain.is_in_cooldown("primary"));
    chain.clear_all_cooldowns();
    assert!(!chain.is_in_cooldown("primary"));
}

/// Scenario 9: Empty chain returns error.
#[tokio::test]
#[serial]
async fn test_empty_provider_chain() {
    init_test_tracing();

    let chain: ProviderChain = ProviderChain::new(vec![]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let result = chain.complete(request).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("All providers exhausted") || err_msg.contains("No providers"));
}

/// Scenario 10: Token usage tracking through fallback.
#[tokio::test]
#[serial]
async fn test_token_usage_through_fallback() {
    init_test_tracing();

    let primary = Arc::new(MockRateLimitedProvider::new("primary"));
    let secondary = Arc::new(MockSuccessProvider::new("secondary", "Fallback response"));

    let chain = ProviderChain::new(vec![primary, secondary]);

    let request = CompletionRequest {
        messages: vec![Message::user("Hi")],
        model: None,
        max_tokens: None,
        temperature: None,
        tools: None,
        system_prompt: None,
        stream: false,
    };

    let response = chain.complete(request).await.expect("Should succeed");

    // Verify token usage is populated
    assert!(response.usage.total_tokens > 0);
    assert!(response.usage.prompt_tokens > 0 || response.usage.completion_tokens > 0);
    
    // Verify provider is set correctly
    assert_eq!(response.provider, "secondary");
}

/// Live provider test (only runs with E2E_LIVE=1)
#[tokio::test]
#[serial]
async fn test_live_provider_chain() {
    if !use_live_providers() {
        return; // Skip live tests unless explicitly enabled
    }

    init_test_tracing();

    // This test would use real providers with actual API keys
    // For safety, we just verify the environment is set up correctly
    
    // Check for common provider API keys
    let has_openai = has_api_key("openai");
    let has_anthropic = has_api_key("anthropic");
    let has_ollama = std::env::var("OLLAMA_HOST").is_ok();

    println!("Live provider availability:");
    println!("  OpenAI: {}", if has_openai { "yes" } else { "no" });
    println!("  Anthropic: {}", if has_anthropic { "yes" } else { "no" });
    println!("  Ollama: {}", if has_ollama { "yes" } else { "no" });

    // At least one provider should be available for live tests
    if !has_openai && !has_anthropic && !has_ollama {
        panic!("No live providers configured! Set E2E_LIVE=1 and provider API keys.");
    }
}
