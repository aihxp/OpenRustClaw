//! E2E tests for memory workflows.
//!
//! These tests cover:
//! - Store and retrieve memories
//! - Search across sessions
//! - Memory consolidation
//! - Export/import

use openrustclaw_core::traits::{MemoryStore, CoreMemoryStore};
use openrustclaw_core::types::{
    MemorySource, MemoryType, MemoryQuery,
};
use serial_test::serial;
use uuid::Uuid;

use crate::common::{
    init_test_tracing, CoreEntryBuilder, MemoryEntryBuilder, TestEnvironment,
};

/// Scenario 1: Store a semantic memory and retrieve it.
#[tokio::test]
#[serial]
async fn test_store_and_retrieve_semantic_memory() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let entry = MemoryEntryBuilder::new("Rust is a systems programming language focused on safety")
        .memory_type(MemoryType::Semantic)
        .user_id("user_123")
        .namespace("knowledge")
        .importance(0.8)
        .build();

    env.memory_store.store(entry).await.expect("Failed to store memory");

    // Search for the memory
    let query = MemoryQuery {
        text: "Rust programming language".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");

    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.entry.content.contains("Rust")));
}

/// Scenario 2: Store episodic memory with session context.
#[tokio::test]
#[serial]
async fn test_store_episodic_memory_with_session() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let session_id = Uuid::new_v4();
    let entry = MemoryEntryBuilder::new("User asked about database optimization")
        .memory_type(MemoryType::Episodic)
        .user_id("user_123")
        .session_id(session_id)
        .source(MemorySource::ConversationSummary)
        .namespace("conversations")
        .importance(0.6)
        .build();

    env.memory_store.store(entry).await.expect("Failed to store episodic memory");

    // Search for memories
    let query = MemoryQuery {
        text: "database optimization".to_string(),
        memory_types: vec![MemoryType::Episodic],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");

    assert!(!results.is_empty());
}

/// Scenario 3: Search memories across sessions for a user.
#[tokio::test]
#[serial]
async fn test_search_memories_across_sessions() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let user_id = "user_cross_session";
    let session1 = Uuid::new_v4();
    let session2 = Uuid::new_v4();

    // Store memories in different sessions
    let entry1 = MemoryEntryBuilder::new("User prefers dark mode interfaces")
        .user_id(user_id)
        .session_id(session1)
        .memory_type(MemoryType::Semantic)
        .namespace("preferences")
        .build();

    let entry2 = MemoryEntryBuilder::new("User likes keyboard shortcuts")
        .user_id(user_id)
        .session_id(session2)
        .memory_type(MemoryType::Semantic)
        .namespace("preferences")
        .build();

    env.memory_store.store(entry1).await.expect("Failed to store entry1");
    env.memory_store.store(entry2).await.expect("Failed to store entry2");

    // Search across all sessions
    let query = MemoryQuery {
        text: "user preferences interface".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: Some("preferences".to_string()),
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");

    assert!(results.len() >= 2);
}

/// Scenario 4: Memory deduplication by content hash.
#[tokio::test]
#[serial]
async fn test_memory_deduplication() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let content = "This is a unique fact about space";

    let entry1 = MemoryEntryBuilder::new(content)
        .user_id("user_123")
        .memory_type(MemoryType::Semantic)
        .build();

    let entry2 = MemoryEntryBuilder::new(content) // Same content
        .user_id("user_123")
        .memory_type(MemoryType::Semantic)
        .build();

    env.memory_store.store(entry1).await.expect("Failed to store first entry");
    env.memory_store.store(entry2).await.expect("Failed to store second entry");

    // Both entries should have same content hash
    // Search should return at least one result
    let query = MemoryQuery {
        text: "unique fact space".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");
    assert!(!results.is_empty());
}

/// Scenario 5: Memory with expiration is pruned.
#[tokio::test]
#[serial]
async fn test_memory_expiration() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Create a memory that expired yesterday
    let expired_entry = MemoryEntryBuilder::new("Temporary session data")
        .user_id("user_123")
        .memory_type(MemoryType::Episodic)
        .expires_at(chrono::Utc::now() - chrono::Duration::days(1))
        .build();

    env.memory_store.store(expired_entry).await.expect("Failed to store expired memory");

    // Run pruning
    let pruned = env.memory_store.expire_stale().await.expect("Failed to prune memories");

    // Should have pruned the expired memory
    assert!(pruned > 0);
}

/// Scenario 6: Core memory operations.
#[tokio::test]
#[serial]
async fn test_core_memory_operations() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let user_id = "user_core_test";

    // Store multiple core memories
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("name", "Alice Johnson").importance(0.95).build()).await.expect("Failed to store name");
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("role", "Software Engineer").importance(0.8).build()).await.expect("Failed to store role");
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("location", "San Francisco").importance(0.7).build()).await.expect("Failed to store location");

    // Retrieve all core memories
    let memories = env.core_memory_store.get_all(user_id).await.expect("Failed to get core memories");

    assert_eq!(memories.len(), 3);
    assert!(memories.iter().any(|m| m.key == "name" && m.value == "Alice Johnson"));
    assert!(memories.iter().any(|m| m.key == "role"));
    assert!(memories.iter().any(|m| m.key == "location"));
}

/// Scenario 7: Update core memory value.
#[tokio::test]
#[serial]
async fn test_update_core_memory() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let user_id = "user_update_test";

    // Store initial value
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("project", "Alpha").importance(0.8).build()).await.expect("Failed to store initial core memory");

    // Update the value
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("project", "Beta").importance(0.9).build()).await.expect("Failed to update core memory");

    // Verify update
    let memories = env.core_memory_store.get_all(user_id).await.expect("Failed to get core memories");

    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].key, "project");
    assert_eq!(memories[0].value, "Beta");
}

/// Scenario 8: Memory search with confidence threshold.
#[tokio::test]
#[serial]
async fn test_memory_search_confidence_threshold() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Store memories with different confidence levels
    let high_confidence = MemoryEntryBuilder::new("User definitely likes Python")
        .user_id("user_123")
        .confidence(0.95)
        .build();

    let low_confidence = MemoryEntryBuilder::new("User maybe likes JavaScript")
        .user_id("user_123")
        .confidence(0.3)
        .build();

    env.memory_store.store(high_confidence).await.expect("Failed to store high confidence memory");
    env.memory_store.store(low_confidence).await.expect("Failed to store low confidence memory");

    // Search with high confidence threshold
    let query = MemoryQuery {
        text: "programming languages".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.8,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");

    // Should only return high confidence results
    assert!(results.iter().all(|r| r.entry.confidence >= 0.8));
}

/// Scenario 9: Context manager builds context correctly.
#[tokio::test]
#[serial]
async fn test_context_manager_integration() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    let user_id = "user_ctx_test";

    // Store some memories
    let memory = MemoryEntryBuilder::new("User is working on a Rust project")
        .user_id(user_id)
        .memory_type(MemoryType::Episodic)
        .build();

    env.memory_store.store(memory).await.expect("Failed to store memory");

    // Store core memory
    env.core_memory_store.set(user_id, CoreEntryBuilder::new("language", "Rust").importance(0.9).build()).await.expect("Failed to store core memory");

    // Get core memory for context building
    let core_memories = env.core_memory_store.get_all(user_id).await.expect("Failed to get core memory");

    // Use context manager to build context
    let context = env.context_manager.build(
        "You are a helpful assistant",
        &core_memories,
        &[], // no tools
        &[openrustclaw_core::types::Message::user("Tell me about my project")],
    );

    // Context should include relevant information
    assert!(!context.system_prompt.is_empty());
    assert!(context.system_prompt.contains("Rust"));
}

/// Scenario 10: Memory serialization (export/import simulation).
#[tokio::test]
#[serial]
async fn test_memory_serialization() {
    init_test_tracing();
    let env = TestEnvironment::new().await;

    // Store some memories
    let memories = vec![
        MemoryEntryBuilder::new("Memory 1: User prefers CLI tools")
            .user_id("export_user")
            .build(),
        MemoryEntryBuilder::new("Memory 2: User works remotely")
            .user_id("export_user")
            .build(),
        MemoryEntryBuilder::new("Memory 3: User likes coffee")
            .user_id("export_user")
            .build(),
    ];

    for memory in memories {
        env.memory_store.store(memory).await.expect("Failed to store memory");
    }

    // Retrieve memories
    let query = MemoryQuery {
        text: "User".to_string(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    let results = env.memory_store.search(&query).await.expect("Failed to search memories");

    assert!(!results.is_empty());

    // Serialize memories to simulate export
    let entries: Vec<_> = results.into_iter().map(|r| r.entry).collect();
    let json = serde_json::to_string(&entries).expect("Failed to serialize memories");
    
    // Deserialize to simulate import
    let _: Vec<openrustclaw_core::types::MemoryEntry> =
        serde_json::from_str(&json).expect("Failed to deserialize memories");
}
