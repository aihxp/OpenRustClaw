# Memory Traits API Reference

This reference documents the traits for memory storage and retrieval.

---

## `MemoryStore`

Trait for the recall memory storage backend.

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a new memory entry (after policy enforcement)
    async fn store(&self, entry: MemoryEntry) -> Result<()>;

    /// Search memories using hybrid BM25 + vector + MMR + temporal decay
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>>;

    /// Get a memory entry by ID
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>>;

    /// Delete a memory entry by ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// Check if content with this hash already exists
    async fn dedupe_check(&self, content_hash: &str) -> Result<Option<String>>;

    /// Expire entries past their TTL
    async fn expire_stale(&self) -> Result<u64>;
}
```

### Example Implementation

```rust
use async_trait::async_trait;
use openrustclaw_core::traits::MemoryStore;
use openrustclaw_core::types::*;
use sqlx::SqlitePool;

pub struct SqliteMemoryStore {
    pool: SqlitePool,
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn store(&self, entry: MemoryEntry) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_entries 
             (id, memory_type, content, content_hash, namespace, importance, confidence, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(entry.id.to_string())
        .bind(entry.memory_type.to_string())
        .bind(&entry.content)
        .bind(&entry.content_hash)
        .bind(&entry.namespace)
        .bind(entry.importance)
        .bind(entry.confidence)
        .bind(entry.created_at)
        .bind(entry.expires_at)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<ScoredMemory>> {
        // Hybrid search implementation
        todo!()
    }
    
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let row = sqlx::query_as::<_, MemoryRow>(
            "SELECT * FROM memory_entries WHERE id = ?1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| r.into()))
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM memory_entries WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    async fn dedupe_check(&self, content_hash: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM memory_entries WHERE content_hash = ?1"
        )
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| r.0))
    }
    
    async fn expire_stale(&self) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM memory_entries WHERE expires_at < ?1"
        )
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
}
```

---

## `CoreMemoryStore`

Trait for the core memory tier.

```rust
#[async_trait]
pub trait CoreMemoryStore: Send + Sync {
    /// Get all core memory entries for a user
    async fn get_all(&self, user_id: &str) -> Result<Vec<CoreEntry>>;

    /// Set a core memory entry
    async fn set(&self, user_id: &str, entry: CoreEntry) -> Result<()>;

    /// Remove a core memory entry by key
    async fn remove(&self, user_id: &str, key: &str) -> Result<()>;

    /// Render all core memory entries as formatted string
    async fn render(&self, user_id: &str) -> Result<String>;

    /// Get total token count of all core memory entries
    async fn total_tokens(&self, user_id: &str) -> Result<usize>;
}
```

### Usage Example

```rust
use openrustclaw_core::traits::CoreMemoryStore;
use openrustclaw_core::types::CoreEntry;

// Store core memory
store.set("user_123", CoreEntry {
    key: "name".into(),
    value: "Alice".into(),
    importance: 0.9,
    token_count: 2,
    updated_at: Utc::now(),
}).await?;

// Retrieve and render
let rendered = store.render("user_123").await?;
println!("{}", rendered);
// Output:
// name: Alice
```

---

## `EmbeddingProvider`

For generating vector embeddings of text.

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed one or more texts into vectors
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// The dimensionality of the embedding vectors
    fn dimensions(&self) -> usize;

    /// The embedding model identifier
    fn model_id(&self) -> &str;
}
```

---

## `ContextManager`

Manages conversation context and memory retrieval.

```rust
pub struct ContextManager {
    core_memory: Arc<dyn CoreMemoryStore>,
    recall_memory: Arc<dyn MemoryStore>,
    max_context_tokens: usize,
    tokenizer: Arc<dyn Tokenizer>,
}

impl ContextManager {
    /// Create new context manager
    pub fn new(
        core_memory: Arc<dyn CoreMemoryStore>,
        recall_memory: Arc<dyn MemoryStore>,
        max_context_tokens: usize,
    ) -> Self;
    
    /// Build context for a conversation turn
    pub async fn build_context(
        &self,
        user_id: &str,
        current_message: &Message,
        conversation_history: &[Message],
    ) -> Result<Context>;
    
    /// Generate search query from conversation
    pub async fn generate_search_query(
        &self,
        message: &Message,
        history: &[Message],
    ) -> Result<String>;
}

pub struct Context {
    pub core_memory: String,
    pub relevant_memories: Vec<ScoredMemory>,
    pub conversation_history: Vec<Message>,
}
```

---

## Memory Error Types

```rust
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Core memory budget exceeded: {used} tokens, max {max}")]
    CoreMemoryBudgetExceeded { used: usize, max: usize },

    #[error("Deduplication: entry already exists with id {existing_id}")]
    Duplicate { existing_id: String },

    #[error("Memory store error: {0}")]
    Store(String),
}
```

---

## Memory Policies

```rust
pub struct MemoryPolicies {
    deduplication: DeduplicationPolicy,
    importance: ImportancePolicy,
    confidence: ConfidencePolicy,
    ttl: TtlPolicy,
}

impl MemoryPolicies {
    /// Apply all policies to a proposed memory entry
    pub async fn apply(&self, entry: &MemoryEntry) -> Result<PolicyDecision>;
}

pub enum PolicyDecision {
    Accept { importance: f32, confidence: f32, expires_at: Option<DateTime<Utc>> },
    Duplicate { existing_id: String },
    Reject { reason: String },
}
```
