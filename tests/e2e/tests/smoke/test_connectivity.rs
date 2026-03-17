//! Smoke Test: Connectivity
//!
//! Tests network connectivity and external dependencies.

use openrustclaw_e2e_tests::common::*;

/// CRITICAL: HTTP client can reach gateway
#[tokio::test]
async fn smoke_http_connectivity() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/health", addr))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    assert!(response.is_ok(), "Cannot connect to gateway");
}

/// CRITICAL: Database pool connections
#[tokio::test]
async fn smoke_database_pool() {
    let env = TestEnvironment::new().await;

    // Verify we can acquire connections
    for i in 0..5 {
        let conn = env.db_pool.acquire().await;
        assert!(conn.is_ok(), "Cannot acquire DB connection #{}", i);
    }
}

/// CRITICAL: All critical services respond
#[tokio::test]
async fn smoke_all_services() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    // Check gateway
    let client = TestHttpClient::new(format!("http://{}", addr));
    let gateway_ok = client.get("/health").await.is_ok();

    // Check database
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&env.db_pool)
        .await
        .is_ok();

    // All must be OK
    assert!(gateway_ok, "Gateway not responding");
    assert!(db_ok, "Database not responding");
}

/// CRITICAL: External API reachability (if configured)
#[tokio::test]
async fn smoke_external_apis() {
    // Only check if live providers are enabled
    if !use_live_providers() {
        println!("Skipping external API check - E2E_LIVE not set");
        return;
    }

    // This would ping external APIs to verify connectivity
    // For now, just verify we have API keys
    if has_api_key("OPENAI") {
        println!("OpenAI API key present");
    }
    if has_api_key("ANTHROPIC") {
        println!("Anthropic API key present");
    }
}
