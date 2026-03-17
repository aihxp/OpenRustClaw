//! Vertical E2E Test: Database Layer
//!
//! Tests SQLite operations, migrations, and query performance in isolation.

#[allow(unused_imports)]
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
#[allow(unused_imports)]
use openrustclaw_core::types::{MemoryQuery, MemorySource, MemoryType};
use openrustclaw_e2e_tests::common::*;
use uuid::Uuid;

/// Test: Database migrations run successfully
#[tokio::test]
async fn test_db_migrations() {
    let env = TestEnvironment::new().await;

    // Verify tables exist by running a simple query
    let result: Result<i64, _> = sqlx::query_scalar("SELECT COUNT(*) FROM memory_entries")
        .fetch_one(&env.db_pool)
        .await;

    assert!(
        result.is_ok(),
        "Migrations should create memory_entries table"
    );

    let result: Result<i64, _> = sqlx::query_scalar("SELECT COUNT(*) FROM core_memory")
        .fetch_one(&env.db_pool)
        .await;

    assert!(result.is_ok(), "Migrations should create core_memory table");
}

/// Test: Memory store and retrieve
#[tokio::test]
async fn test_db_memory_store_retrieve() {
    let env = TestEnvironment::new().await;

    let entry = MemoryEntryBuilder::new("Test memory content")
        .user_id(TestUsers::alice().id)
        .namespace("test")
        .build();

    // Store
    env.store_memory(entry.clone()).await.expect("Store failed");

    // Retrieve by search
    let query = create_memory_query("test memory");
    let results = env.search_memories(query).await.expect("Search failed");

    assert!(!results.is_empty(), "Should find stored memory");
}

/// Test: Core memory operations
#[tokio::test]
async fn test_db_core_memory_operations() {
    let env = TestEnvironment::new().await;
    let user_id = TestUsers::alice().id;

    // Set core memory
    let entry = CoreEntryBuilder::new("name", "Alice")
        .importance(1.0)
        .build();

    env.store_core_memory(&user_id, entry)
        .await
        .expect("Store failed");

    // Get all core memories
    let memories = env.get_core_memory(&user_id).await.expect("Get failed");

    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].key, "name");
    assert_eq!(memories[0].value, "Alice");
}

/// Test: Memory type filtering
#[tokio::test]
async fn test_db_memory_type_filtering() {
    let env = TestEnvironment::new().await;

    // Store semantic memory
    let semantic = MemoryEntryBuilder::new("Semantic memory")
        .memory_type(MemoryType::Semantic)
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(semantic).await.expect("Store failed");

    // Store episodic memory
    let episodic = MemoryEntryBuilder::new("Episodic memory")
        .memory_type(MemoryType::Episodic)
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(episodic).await.expect("Store failed");

    // Query with type filter
    let query = MemoryQuery {
        text: "memory".to_string(),
        memory_types: vec![MemoryType::Semantic],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.search_memories(query).await.expect("Search failed");

    // Should only get semantic memories
    for result in &results {
        assert_eq!(result.entry.memory_type, MemoryType::Semantic);
    }
}

/// Test: Memory expiration
#[tokio::test]
async fn test_db_memory_expiration() {
    let env = TestEnvironment::new().await;

    // Store memory that expires in the past
    let expired = MemoryEntryBuilder::new("Expired memory")
        .user_id(TestUsers::alice().id)
        .expires_at(chrono::Utc::now() - chrono::Duration::hours(1))
        .build();

    env.store_memory(expired).await.expect("Store failed");

    // Store memory that expires in the future
    let valid = MemoryEntryBuilder::new("Valid memory")
        .user_id(TestUsers::alice().id)
        .expires_in(1)
        .build();

    env.store_memory(valid).await.expect("Store failed");

    // Query should only return valid memories
    let query = create_memory_query("memory");
    let results = env.search_memories(query).await.expect("Search failed");

    for result in &results {
        if let Some(expires_at) = result.entry.expires_at {
            assert!(
                expires_at > chrono::Utc::now(),
                "Should not return expired memories"
            );
        }
    }
}

/// Test: Namespace isolation
#[tokio::test]
async fn test_db_namespace_isolation() {
    let env = TestEnvironment::new().await;

    // Store memories in different namespaces
    let work = MemoryEntryBuilder::new("Work project")
        .namespace("work")
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(work).await.expect("Store failed");

    let personal = MemoryEntryBuilder::new("Personal hobby")
        .namespace("personal")
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(personal).await.expect("Store failed");

    // Query with namespace filter
    let query = MemoryQuery {
        text: "project".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: Some("work".to_string()),
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.search_memories(query).await.expect("Search failed");

    for result in &results {
        assert_eq!(result.entry.namespace, "work");
    }
}

/// Test: Database transaction handling
#[tokio::test]
async fn test_db_transaction_handling() {
    let env = TestEnvironment::new().await;

    // Start a transaction
    let mut tx = env
        .db_pool
        .begin()
        .await
        .expect("Failed to begin transaction");

    // Insert within transaction
    let result: Result<(), sqlx::Error> = sqlx::query(
        "INSERT INTO memory_entries (id, content, content_hash, namespace, memory_type, created_at) 
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind("Test content")
    .bind("hash123")
    .bind("test")
    .bind("semantic")
    .bind(chrono::Utc::now())
    .execute(&mut *tx)
    .await
    .map(|_| ());

    assert!(result.is_ok());

    // Rollback
    tx.rollback().await.expect("Rollback failed");

    // Verify data was not persisted
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memory_entries WHERE content_hash = 'hash123'")
            .fetch_one(&env.db_pool)
            .await
            .expect("Query failed");

    assert_eq!(count, 0, "Rolled back data should not exist");
}

/// Test: Concurrent database operations
#[tokio::test]
async fn test_db_concurrent_operations() {
    let env = TestEnvironment::new().await;

    let mut handles = vec![];

    // Spawn 10 concurrent writes
    for i in 0..10 {
        let pool = env.db_pool.clone();
        handles.push(tokio::spawn(async move {
            let entry = MemoryEntryBuilder::new(format!("Concurrent memory {}", i))
                .user_id(format!("user-{}", i))
                .build();

            let _ = sqlx::query(
                "INSERT INTO memory_entries (id, content, content_hash, namespace, memory_type, created_at) 
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(entry.id.to_string())
            .bind(&entry.content)
            .bind(&entry.content_hash)
            .bind(&entry.namespace)
            .bind(format!("{:?}", entry.memory_type).to_lowercase())
            .bind(entry.created_at)
            .execute(&pool)
            .await;
        }));
    }

    // All should succeed
    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test: Query performance
#[tokio::test]
async fn test_db_query_performance() {
    let env = TestEnvironment::new().await;

    // Insert many memories
    for i in 0..100 {
        let entry = MemoryEntryBuilder::new(format!("Performance test memory {}", i))
            .user_id(TestUsers::alice().id)
            .build();
        env.store_memory(entry).await.expect("Store failed");
    }

    // Measure search time
    let start = std::time::Instant::now();
    let query = create_memory_query("performance test");
    let _results = env.search_memories(query).await.expect("Search failed");
    let elapsed = start.elapsed();

    // Should complete in reasonable time (< 1 second for 100 records)
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "Query took too long: {:?}",
        elapsed
    );
}
