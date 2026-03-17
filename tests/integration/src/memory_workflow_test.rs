//! Memory system integration tests.
//!
//! These tests verify:
//! - Storing and retrieving memories
//! - Hybrid search (BM25 + vector)
//! - Deduplication
//! - TTL expiration
//! - Core memory budget enforcement

use chrono::{Duration, Utc};

use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
use openrustclaw_core::types::{CoreEntry, MemoryQuery, MemorySource, MemoryType};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore};
use openrustclaw_memory::{CoreMemoryManager, MemoryPolicies, RecallMemory};

use crate::common::{CoreEntryBuilder, MemoryEntryBuilder, create_test_db, init_test_tracing};

// ═════════════════════════════════════════════════════════════════════════════
// Recall Memory Tests
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn store_and_retrieve_memory() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let entry = MemoryEntryBuilder::new("Test memory content")
        .memory_type(MemoryType::Semantic)
        .user_id("user_123")
        .build();

    let entry_id = entry.id;
    store.store(entry).await.unwrap();

    let retrieved = store.get(&entry_id.to_string()).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.content, "Test memory content");
    assert_eq!(retrieved.memory_type, MemoryType::Semantic);
    assert_eq!(retrieved.user_id, Some("user_123".to_string()));
}

#[tokio::test]
async fn store_memory_with_ttl() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let expires_at = Utc::now() + Duration::days(30);
    let entry = MemoryEntryBuilder::new("Temporary memory")
        .memory_type(MemoryType::Episodic)
        .expires_at(expires_at)
        .build();

    store.store(entry.clone()).await.unwrap();

    let retrieved = store.get(&entry.id.to_string()).await.unwrap().unwrap();
    assert!(retrieved.expires_at.is_some());
}

#[tokio::test]
async fn deduplication_check_finds_existing() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let entry = MemoryEntryBuilder::new("Unique content for dedupe test").build();
    let content_hash = entry.content_hash.clone();
    let entry_id = entry.id;

    store.store(entry).await.unwrap();

    let existing = store.dedupe_check(&content_hash).await.unwrap();
    assert_eq!(existing, Some(entry_id.to_string()));
}

#[tokio::test]
async fn deduplication_check_returns_none_for_new() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let existing = store.dedupe_check("nonexistent_hash_12345").await.unwrap();
    assert!(existing.is_none());
}

#[tokio::test]
async fn expire_stale_memories() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    // Create an expired memory
    let expired_entry = MemoryEntryBuilder::new("Expired memory")
        .expires_at(Utc::now() - Duration::hours(1))
        .build();

    // Create a valid memory
    let valid_entry = MemoryEntryBuilder::new("Valid memory")
        .expires_at(Utc::now() + Duration::days(1))
        .build();

    store.store(expired_entry.clone()).await.unwrap();
    store.store(valid_entry.clone()).await.unwrap();

    // Expire stale memories
    let expired_count = store.expire_stale().await.unwrap();
    assert_eq!(expired_count, 1);

    // Verify expired memory is gone
    let retrieved = store.get(&expired_entry.id.to_string()).await.unwrap();
    assert!(retrieved.is_none());

    // Verify valid memory remains
    let retrieved = store.get(&valid_entry.id.to_string()).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn delete_memory() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let entry = MemoryEntryBuilder::new("To be deleted").build();
    let entry_id = entry.id;

    store.store(entry).await.unwrap();

    // Verify it exists
    assert!(store.get(&entry_id.to_string()).await.unwrap().is_some());

    // Delete it
    store.delete(&entry_id.to_string()).await.unwrap();

    // Verify it's gone
    assert!(store.get(&entry_id.to_string()).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_nonexistent_memory_errors() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let result = store.delete("nonexistent-id-123").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn search_memories_by_text() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool.clone());

    // Insert test memories
    let entry1 = MemoryEntryBuilder::new("Rust programming language").build();
    let entry2 = MemoryEntryBuilder::new("Python programming language").build();
    let entry3 = MemoryEntryBuilder::new("Cooking recipes").build();

    store.store(entry1).await.unwrap();
    store.store(entry2).await.unwrap();
    store.store(entry3).await.unwrap();

    // Search for "programming"
    let query = MemoryQuery {
        text: "programming".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = store.search(&query).await.unwrap();

    // Should find at least 2 results containing "programming"
    assert!(results.len() >= 2);

    // Results should contain "programming"
    for result in &results {
        assert!(result.entry.content.contains("programming"));
    }
}

#[tokio::test]
async fn search_with_memory_type_filter() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let semantic = MemoryEntryBuilder::new("Semantic knowledge")
        .memory_type(MemoryType::Semantic)
        .build();
    let episodic = MemoryEntryBuilder::new("Episodic memory")
        .memory_type(MemoryType::Episodic)
        .build();

    store.store(semantic).await.unwrap();
    store.store(episodic).await.unwrap();

    let query = MemoryQuery {
        text: "memory".to_string(),
        memory_types: vec![MemoryType::Episodic],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = store.search(&query).await.unwrap();

    for result in &results {
        assert_eq!(result.entry.memory_type, MemoryType::Episodic);
    }
}

#[tokio::test]
async fn search_with_namespace_filter() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    let global = MemoryEntryBuilder::new("Global namespace content")
        .namespace("global")
        .build();
    let private = MemoryEntryBuilder::new("Private namespace content")
        .namespace("private")
        .build();

    store.store(global).await.unwrap();
    store.store(private).await.unwrap();

    let query = MemoryQuery {
        text: "content".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: Some("private".to_string()),
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = store.search(&query).await.unwrap();

    for result in &results {
        assert_eq!(result.entry.namespace, "private");
    }
}

#[tokio::test]
async fn search_limits_results() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteMemoryStore::new(pool);

    // Insert multiple memories
    for i in 0..10 {
        let entry = MemoryEntryBuilder::new(format!("Test memory {}", i)).build();
        store.store(entry).await.unwrap();
    }

    let query = MemoryQuery {
        text: "Test memory".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 3,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = store.search(&query).await.unwrap();
    assert!(results.len() <= 3);
}

// ═════════════════════════════════════════════════════════════════════════════
// Core Memory Tests
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn core_memory_store_and_retrieve() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteCoreMemoryStore::new(pool);

    let entry = CoreEntryBuilder::new("user_name", "Alice")
        .importance(0.9)
        .build();

    store.set("user_123", entry).await.unwrap();

    let entries = store.get_all("user_123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "user_name");
    assert_eq!(entries[0].value, "Alice");
}

#[tokio::test]
async fn core_memory_update_existing() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteCoreMemoryStore::new(pool);

    let entry1 = CoreEntryBuilder::new("preference", "value1").build();
    let entry2 = CoreEntryBuilder::new("preference", "value2").build();

    store.set("user_123", entry1).await.unwrap();
    store.set("user_123", entry2).await.unwrap();

    let entries = store.get_all("user_123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value, "value2");
}

#[tokio::test]
async fn core_memory_remove() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteCoreMemoryStore::new(pool);

    let entry = CoreEntryBuilder::new("to_remove", "value").build();
    store.set("user_123", entry).await.unwrap();

    store.remove("user_123", "to_remove").await.unwrap();

    let entries = store.get_all("user_123").await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn core_memory_render() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteCoreMemoryStore::new(pool);

    let entry1 = CoreEntryBuilder::new("name", "Alice").build();
    let entry2 = CoreEntryBuilder::new("project", "OpenRustClaw").build();

    store.set("user_123", entry1).await.unwrap();
    store.set("user_123", entry2).await.unwrap();

    let rendered = store.render("user_123").await.unwrap();
    assert!(rendered.contains("**name**: Alice"));
    assert!(rendered.contains("**project**: OpenRustClaw"));
    assert!(rendered.contains("## Core Memory"));
}

#[tokio::test]
async fn core_memory_total_tokens() {
    init_test_tracing();

    let pool = create_test_db().await;
    let store = SqliteCoreMemoryStore::new(pool);

    let entry1 = CoreEntryBuilder::new("key1", "short").build();
    let entry2 = CoreEntryBuilder::new("key2", "also short").build();

    store.set("user_123", entry1.clone()).await.unwrap();
    store.set("user_123", entry2.clone()).await.unwrap();

    let total = store.total_tokens("user_123").await.unwrap();
    assert_eq!(total, entry1.token_count + entry2.token_count);
}

// ═════════════════════════════════════════════════════════════════════════════
// Core Memory Manager Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn core_memory_manager_budget_check() {
    let manager = CoreMemoryManager::new(100, 10);

    let entry1 = CoreEntryBuilder::new("key1", "value1").build();
    let entry2 = CoreEntryBuilder::new("key2", "value2").build();

    let entries = vec![entry1.clone()];
    assert!(!manager.would_exceed_budget(&entries, &entry2));
}

#[test]
fn core_memory_manager_budget_exceeded() {
    let manager = CoreMemoryManager::new(10, 10);

    let large_entry = CoreEntryBuilder::new("key", "a very long value that exceeds budget").build();
    let entries: Vec<CoreEntry> = vec![];

    assert!(manager.would_exceed_budget(&entries, &large_entry));
}

#[test]
fn core_memory_manager_entry_limit() {
    let manager = CoreMemoryManager::new(1000, 2);

    let entry1 = CoreEntryBuilder::new("key1", "value1").build();
    let entry2 = CoreEntryBuilder::new("key2", "value2").build();
    let entry3 = CoreEntryBuilder::new("key3", "value3").build();

    let entries = vec![entry1, entry2];
    assert!(manager.would_exceed_budget(&entries, &entry3));
}

#[test]
fn core_memory_manager_eviction_candidate() {
    let entries = vec![
        CoreEntryBuilder::new("important", "value")
            .importance(0.9)
            .build(),
        CoreEntryBuilder::new("less_important", "value")
            .importance(0.3)
            .build(),
        CoreEntryBuilder::new("medium", "value")
            .importance(0.6)
            .build(),
    ];

    let candidate = CoreMemoryManager::eviction_candidate(&entries);
    assert_eq!(candidate, Some(1)); // least_important has index 1
}

#[test]
fn core_memory_manager_render_empty() {
    let rendered = CoreMemoryManager::render(&[]);
    assert!(rendered.is_empty());
}

// ═════════════════════════════════════════════════════════════════════════════
// Memory Policies Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn content_hash_deterministic() {
    let hash1 = MemoryPolicies::content_hash("test content");
    let hash2 = MemoryPolicies::content_hash("test content");
    assert_eq!(hash1, hash2);
}

#[test]
fn content_hash_unique() {
    let hash1 = MemoryPolicies::content_hash("content a");
    let hash2 = MemoryPolicies::content_hash("content b");
    assert_ne!(hash1, hash2);
}

#[test]
fn score_importance_explicit_statement() {
    let score = MemoryPolicies::score_importance(&MemorySource::ExplicitUserStatement);
    assert_eq!(score, 1.0);
}

#[test]
fn score_importance_user_correction() {
    let score = MemoryPolicies::score_importance(&MemorySource::UserCorrection);
    assert_eq!(score, 0.95);
}

#[test]
fn score_importance_background() {
    let score = MemoryPolicies::score_importance(&MemorySource::BackgroundIngestion);
    assert_eq!(score, 0.5);
}

#[test]
fn cosine_similarity_identical() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let sim = MemoryPolicies::cosine_similarity(&a, &b);
    assert!((sim - 1.0).abs() < 0.001);
}

#[test]
fn cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let sim = MemoryPolicies::cosine_similarity(&a, &b);
    assert!(sim.abs() < 0.001);
}

#[test]
fn cosine_similarity_opposite() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![-1.0, 0.0, 0.0];
    let sim = MemoryPolicies::cosine_similarity(&a, &b);
    assert!((sim - (-1.0)).abs() < 0.001);
}

#[test]
fn decay_score_zero_days() {
    let policies = MemoryPolicies::default();
    let score = policies.decay_score(1.0, 0.0);
    assert!((score - 1.0).abs() < 0.001);
}

#[test]
fn decay_score_half_life() {
    let policies = MemoryPolicies::default();
    let score = policies.decay_score(1.0, policies.decay_half_life_days);
    assert!((score - 0.5).abs() < 0.01);
}

// ═════════════════════════════════════════════════════════════════════════════
// Recall Memory Manager Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn recall_memory_prepares_entry() {
    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    let entry = recall.prepare_entry(
        "Test content",
        MemoryType::Semantic,
        MemorySource::ExplicitUserStatement,
        Some("user_123"),
        None,
        Some("test_ns"),
    );

    assert_eq!(entry.content, "Test content");
    assert_eq!(entry.memory_type, MemoryType::Semantic);
    assert_eq!(entry.user_id, Some("user_123".to_string()));
    assert_eq!(entry.namespace, "test_ns");
    assert_eq!(entry.importance, 1.0); // From ExplicitUserStatement
}

#[test]
fn recall_memory_episodic_has_ttl() {
    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    let entry = recall.prepare_entry(
        "Episodic content",
        MemoryType::Episodic,
        MemorySource::AgentInference,
        None,
        None,
        None,
    );

    assert!(entry.expires_at.is_some());
}

#[test]
fn recall_memory_semantic_no_ttl() {
    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    let entry = recall.prepare_entry(
        "Semantic content",
        MemoryType::Semantic,
        MemorySource::AgentInference,
        None,
        None,
        None,
    );

    assert!(entry.expires_at.is_none());
}

#[test]
fn recall_memory_apply_decay() {
    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    let mut results = vec![openrustclaw_core::types::ScoredMemory {
        entry: MemoryEntryBuilder::new("recent").build(),
        score: 1.0,
    }];

    recall.apply_decay(&mut results);

    // Score should be unchanged (no last_accessed)
    assert!((results[0].score - 1.0).abs() < 0.001);
}
