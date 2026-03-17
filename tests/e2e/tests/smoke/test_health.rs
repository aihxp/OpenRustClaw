//! Smoke Test: Health Checks
//!
//! Critical health checks that must pass for deployment.

use openrustclaw_e2e_tests::common::*;

/// CRITICAL: Gateway health endpoint
#[tokio::test]
async fn smoke_gateway_health() {
    let env = TestEnvironment::new().await;
    let (addr, _server) = env.start_gateway(false).await;

    let client = TestHttpClient::new(format!("http://{}", addr));
    let response = client.get("/health").await.expect("Health check failed");

    response.assert_success();

    let health: serde_json::Value = TestHttpClient::parse_json(response)
        .await
        .expect("Failed to parse health response");
    assert_eq!(health["status"], "healthy");
}

/// CRITICAL: Database connectivity
#[tokio::test]
async fn smoke_database_connectivity() {
    let env = TestEnvironment::new().await;

    // Simple query to verify DB is working
    let result: Result<i64, _> = sqlx::query_scalar("SELECT 1").fetch_one(&env.db_pool).await;

    assert_eq!(result.expect("DB query failed"), 1);
}

/// CRITICAL: Memory store functionality
#[tokio::test]
async fn smoke_memory_store() {
    let env = TestEnvironment::new().await;

    use openrustclaw_core::traits::MemoryStore;

    let entry = MemoryEntryBuilder::new("Smoke test memory")
        .user_id(TestUsers::alice().id)
        .build();

    // Store
    env.memory_store
        .store(entry.clone())
        .await
        .expect("Store failed");

    // Retrieve
    let query = create_memory_query("smoke test");
    let results = env
        .memory_store
        .search(&query)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Memory store not working");
}

/// CRITICAL: Session manager
#[tokio::test]
async fn smoke_session_manager() {
    let env = TestEnvironment::new().await;
    let user_id = TestUsers::alice().id;

    // Create session
    use openrustclaw_core::types::{Platform, SessionType};
    let session = env
        .session_manager
        .create_session(&user_id, SessionType::Dm, Platform::WebChat)
        .await
        .expect("Create failed");

    // Verify
    let retrieved = env
        .session_manager
        .get_session(&session.id.to_string())
        .await;
    assert!(retrieved.is_ok(), "Session manager not working");
}
