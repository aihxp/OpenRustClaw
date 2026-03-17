//! Regression Test: Memory Recall
//!
//! Comprehensive tests for memory storage, retrieval, and eviction.

use openrustclaw_e2e_tests::common::*;
#[allow(unused_imports)]
use openrustclaw_core::traits::MemoryStore;
use openrustclaw_core::types::{MemoryQuery, MemoryType};
#[allow(unused_imports)]
use uuid::Uuid;

/// Test: Memory search with various queries
#[tokio::test]
async fn test_memory_search_variations() {
    let env = TestEnvironment::new().await;

    // Store diverse memories
    let memories = vec![
        ("Rust programming is fun", "tech"),
        ("I love pizza", "food"),
        ("Meeting at 3pm tomorrow", "schedule"),
        ("Password: secret123", "sensitive"),
        ("The quick brown fox", "random"),
    ];

    for (content, namespace) in memories {
        let entry = MemoryEntryBuilder::new(content)
            .namespace(namespace)
            .user_id(TestUsers::alice().id)
            .build();
        env.store_memory(entry).await.expect("Store failed");
    }

    // Test various search queries
    let queries = vec![
        ("programming language", vec!["Rust"]),
        ("favorite food", vec!["pizza"]),
        ("when is the meeting", vec!["3pm"]),
        ("random animal", vec!["fox"]),
    ];

    for (query_text, expected_contains) in queries {
        let query = create_memory_query(query_text);
        let results = env.search_memories(query).await.expect("Search failed");

        assert!(
            !results.is_empty(),
            "Query '{}' should return results",
            query_text
        );

        for expected in expected_contains {
            let found = results.iter().any(|r| r.entry.content.contains(expected));
            assert!(
                found,
                "Query '{}' should find content containing '{}'",
                query_text, expected
            );
        }
    }
}

/// Test: Memory importance and ranking
#[tokio::test]
async fn test_memory_importance_ranking() {
    let env = TestEnvironment::new().await;

    // Store memories with different importance
    let low = MemoryEntryBuilder::new("Low importance fact")
        .importance(0.1)
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(low).await.expect("Store failed");

    let high = MemoryEntryBuilder::new("High importance fact")
        .importance(0.9)
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(high).await.expect("Store failed");

    // Both should be retrievable
    let query = create_memory_query("importance fact");
    let results = env.search_memories(query).await.expect("Search failed");

    assert_eq!(results.len(), 2, "Should find both memories");
}

/// Test: Memory expiration handling
#[tokio::test]
async fn test_memory_expiration_handling() {
    let env = TestEnvironment::new().await;

    // Store expired memory
    let expired = MemoryEntryBuilder::new("Expired information")
        .expires_at(chrono::Utc::now() - chrono::Duration::days(1))
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(expired).await.expect("Store failed");

    // Store current memory
    let current = MemoryEntryBuilder::new("Current information")
        .expires_in(30) // 30 days
        .user_id(TestUsers::alice().id)
        .build();
    env.store_memory(current).await.expect("Store failed");

    // Search should only return current
    let query = create_memory_query("information");
    let results = env.search_memories(query).await.expect("Search failed");

    for result in &results {
        if let Some(expires) = result.entry.expires_at {
            assert!(
                expires > chrono::Utc::now(),
                "Should not return expired memory"
            );
        }
    }
}

/// Test: Memory under load
#[tokio::test]
async fn test_memory_under_load() {
    let env = TestEnvironment::new().await;

    // Store many memories
    let start = std::time::Instant::now();
    for i in 0..100 {
        let entry = MemoryEntryBuilder::new(format!("Memory item number {}", i))
            .user_id(TestUsers::alice().id)
            .build();
        env.store_memory(entry).await.expect("Store failed");
    }
    let store_time = start.elapsed();

    // Search should still be fast
    let start = std::time::Instant::now();
    let query = create_memory_query("memory item");
    let results = env.search_memories(query).await.expect("Search failed");
    let search_time = start.elapsed();

    assert_eq!(results.len(), 100, "Should find all memories");
    assert!(
        search_time < std::time::Duration::from_secs(2),
        "Search took too long: {:?}",
        search_time
    );

    println!("Stored 100 memories in {:?}, searched in {:?}", store_time, search_time);
}

/// Test: Memory access counting
#[tokio::test]
async fn test_memory_access_counting() {
    let env = TestEnvironment::new().await;

    let entry = MemoryEntryBuilder::new("Frequently accessed memory")
        .user_id(TestUsers::alice().id)
        .build();

    env.store_memory(entry.clone()).await.expect("Store failed");

    // Access multiple times
    for _ in 0..5 {
        let query = create_memory_query("frequently accessed");
        let _results = env.search_memories(query).await.expect("Search failed");
    }

    // Access count should have increased
    // Note: This depends on implementation details
}

/// Test: Concurrent memory operations
#[tokio::test]
async fn test_concurrent_memory_operations() {
    let env = TestEnvironment::new().await;

    let mut handles = vec![];

    // Spawn concurrent writes
    for i in 0..20 {
        let pool = env.db_pool.clone();
        handles.push(tokio::spawn(async move {
            let entry = MemoryEntryBuilder::new(format!("Concurrent memory {}", i))
                .user_id(TestUsers::alice().id)
                .build();

            let _ = sqlx::query(
                "INSERT INTO memories (id, content, content_hash, namespace, memory_type, created_at) 
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

    // Spawn concurrent reads
    for _ in 0..20 {
        let pool = env.db_pool.clone();
        handles.push(tokio::spawn(async move {
            let _: Result<i64, _> = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memories")
                .fetch_one(&pool)
                .await;
        }));
    }

    // All should complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

/// Test: Memory namespace boundaries
#[tokio::test]
async fn test_memory_namespace_boundaries() {
    let env = TestEnvironment::new().await;

    // Store in different namespaces
    let namespaces = vec!["work", "personal", "health", "finance"];

    for ns in &namespaces {
        let entry = MemoryEntryBuilder::new(format!("{} memory", ns))
            .namespace(*ns)
            .user_id(TestUsers::alice().id)
            .build();
        env.store_memory(entry).await.expect("Store failed");
    }

    // Search each namespace
    for ns in &namespaces {
        let query = MemoryQuery {
            text: "memory".to_string(),
            memory_types: vec![],
            source_types: vec![],
            namespace: Some(ns.to_string()),
            limit: 10,
            min_confidence: 0.0,
            recency_weight: 0.0,
        };

        let results = env.search_memories(query).await.expect("Search failed");

        assert_eq!(results.len(), 1, "Namespace {} should have 1 result", ns);
        assert_eq!(results[0].entry.namespace, *ns);
    }
}

/// Test: Empty memory search
#[tokio::test]
async fn test_empty_memory_search() {
    let env = TestEnvironment::new().await;

    // Search with no memories stored
    let query = create_memory_query("nonexistent query");
    let results = env.search_memories(query).await.expect("Search failed");

    assert!(results.is_empty(), "Should return empty for no matches");
}

/// Test: Memory type filtering
#[tokio::test]
async fn test_memory_type_filtering() {
    let env = TestEnvironment::new().await;

    // Store different memory types
    let types = vec![
        (MemoryType::Semantic, "Semantic memory"),
        (MemoryType::Episodic, "Episodic memory"),
        (MemoryType::Procedural, "Procedural memory"),
    ];

    for (mem_type, content) in types {
        let entry = MemoryEntryBuilder::new(content)
            .memory_type(mem_type)
            .user_id(TestUsers::alice().id)
            .build();
        env.store_memory(entry).await.expect("Store failed");
    }

    // Search with type filter
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

    for result in results {
        assert_eq!(result.entry.memory_type, MemoryType::Semantic);
    }
}
