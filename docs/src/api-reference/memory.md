# Memory System API Reference

This reference documents the memory, RAG, and context management for OpenRustClaw.

**Crate**: `openrustclaw-memory`

---

## Overview

OpenRustClaw uses a three-tier database-first memory architecture:

1. **Core Memory** (~500 tokens): Tiny, always in prompt, key-value pairs
2. **Recall Memory**: Searchable via `memory_search` tool, never auto-injected
3. **Archive**: Consolidated long-term summaries

---

## MemoryStore Trait

The trait for recall memory storage backend.

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

**Implementations**:
- SQLite with libSQL for vector search
- In-memory store for testing

---

## CoreMemoryStore Trait

The trait for the core memory tier.

```rust
#[async_trait]
pub trait CoreMemoryStore: Send + Sync {
    /// Get all core memory entries for a user
    async fn get_all(&self, user_id: &str) -> Result<Vec<CoreEntry>>;
    
    /// Set a core memory entry (overwrites if key exists)
    async fn set(&self, user_id: &str, entry: CoreEntry) -> Result<()>;
    
    /// Remove a core memory entry by key
    async fn remove(&self, user_id: &str, key: &str) -> Result<()>;
    
    /// Render all entries as formatted string for system prompt
    async fn render(&self, user_id: &str) -> Result<String>;
    
    /// Get total token count of all entries for a user
    async fn total_tokens(&self, user_id: &str) -> Result<usize>;
}
```

---

## ContextManager

Manages the system prompt and context to stay within token budgets.

```rust
pub struct ContextManager {
    max_context_tokens: usize,
    compression_threshold: f32,  // 0.85 = compress at 85% capacity
}

pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `ContextManager::new(max_tokens)` | Create with maximum context size |
| `build_system_prompt(base, core_memory, tools)` | Build system prompt with core memory |
| `select_messages(messages, system_tokens)` | Select messages fitting in budget |
| `needs_compression(current_tokens)` | Check if compression is needed |
| `estimate_tokens(text)` | Rough token estimation (~4 chars/token) |
| `build(base, core, tools, messages)` | Build complete context |

**Example**:
```rust
use openrustclaw_memory::ContextManager;
use openrustclaw_core::types::{CoreEntry, Message, ToolDefinition};

let manager = ContextManager::new(8192);

let core_memory = vec![
    CoreEntry {
        key: "user_name".to_string(),
        value: "Alice".to_string(),
        importance: 0.9,
        token_count: 4,
        updated_at: Utc::now(),
    },
];

let tools = vec![];  // Tool definitions
let messages = vec![Message::user("Hello!")];

let context = manager.build(
    "You are a helpful assistant.",
    &core_memory,
    &tools,
    &messages,
);

println!("System prompt:\n{}", context.system_prompt);
println!("Estimated tokens: {}", context.estimated_tokens);
```

---

## CoreMemoryManager

Manages core memory entries for system prompt injection.

```rust
pub struct CoreMemoryManager {
    max_tokens: usize,
    max_entries: usize,
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `CoreMemoryManager::new(max_tokens, max_entries)` | Create with budget limits |
| `CoreMemoryManager::render(entries)` | Render entries as formatted string |
| `would_exceed_budget(current, new)` | Check if adding would exceed budget |
| `eviction_candidate(entries)` | Find lowest-importance entry to evict |
| `new_entry(key, value, importance)` | Create a new core entry |

**Example**:
```rust
use openrustclaw_memory::CoreMemoryManager;

let manager = CoreMemoryManager::new(500, 20);

// Create new entry
let entry = CoreMemoryManager::new_entry(
    "favorite_color",
    "blue",
    0.8,
);

// Check budget
let current = vec![];
if !manager.would_exceed_budget(&current, &entry) {
    // Add entry
}

// Render for prompt
let prompt_section = CoreMemoryManager::render(&[entry]);
```

---

## RecallMemory

Recall memory manager with policy enforcement.

```rust
pub struct RecallMemory {
    policies: MemoryPolicies,
}

impl RecallMemory {
    pub fn new(policies: MemoryPolicies) -> Self;
    
    /// Prepare entry with policy enforcement
    pub fn prepare_entry(
        &self,
        content: &str,
        memory_type: MemoryType,
        source: MemorySource,
        user_id: Option<&str>,
        session_id: Option<Uuid>,
        namespace: Option<&str>,
    ) -> MemoryEntry;
    
    /// Apply temporal decay to scored results
    pub fn apply_decay(&self, results: &mut Vec<ScoredMemory>);
}
```

**Example**:
```rust
use openrustclaw_memory::{RecallMemory, MemoryPolicies};
use openrustclaw_core::types::{MemoryType, MemorySource};

let policies = MemoryPolicies::default();
let recall = RecallMemory::new(policies);

// Prepare a memory entry
let entry = recall.prepare_entry(
    "User likes Python over JavaScript",
    MemoryType::Semantic,
    MemorySource::ExplicitUserStatement,
    Some("user_42"),
    None,
    Some("preferences"),
);
```

---

## MemoryPolicies

Configuration for memory storage policies.

```rust
pub struct MemoryPolicies {
    pub ttl_episodic_days: i32,      // TTL for episodic memories (0 = no expiry)
    pub decay_half_life_days: f64,   // Score decay half-life
    pub min_importance: f32,         // Minimum importance threshold
    pub dedupe_threshold: f64,       // Similarity threshold for dedupe
}

impl Default for MemoryPolicies {
    fn default() -> Self {
        Self {
            ttl_episodic_days: 90,
            decay_half_life_days: 30.0,
            min_importance: 0.1,
            dedupe_threshold: 0.95,
        }
    }
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `content_hash(content)` | Compute SHA-256 hash for deduplication |
| `score_importance(source)` | Calculate importance based on source |
| `decay_score(score, days)` | Apply temporal decay to relevance score |

---

## Hybrid Search

### Reciprocal Rank Fusion

Fuses BM25 and vector search results using RRF.

```rust
use openrustclaw_memory::search::reciprocal_rank_fusion;
use openrustclaw_core::types::ScoredMemory;

let bm25_results: Vec<ScoredMemory> = // ... from BM25 search
let vector_results: Vec<ScoredMemory> = // ... from vector search

// k is RRF constant, typically 60.0
let fused = reciprocal_rank_fusion(bm25_results, vector_results, 60.0);
```

### MMR Reranking

Maximal Marginal Relevance for diversity in results.

```rust
use openrustclaw_memory::search::mmr_rerank;

// lambda: 0.0 = max diversity, 1.0 = max relevance, 0.5 = balanced
let diverse_results = mmr_rerank(results, limit: 10, lambda: 0.5);
```

---

## Memory Tools

The agent runtime includes built-in memory tools:

### `MemorySearchTool`

Searches recall memory.

```rust
use openrustclaw_agent::MemorySearchTool;

let tool = MemorySearchTool::new(memory_store);

// Tool is automatically registered with:
// name: "memory_search"
// description: "Search for relevant memories"
```

### `MemoryStoreTool`

Stores new memories.

```rust
use openrustclaw_agent::MemoryStoreTool;

let tool = MemoryStoreTool::new(memory_store);

// Tool is automatically registered with:
// name: "memory_store"
// description: "Store a new memory"
```

### `CoreMemoryUpdateTool`

Updates core memory.

```rust
use openrustclaw_agent::CoreMemoryUpdateTool;

let tool = CoreMemoryUpdateTool::new(core_memory_store);

// Tool is automatically registered with:
// name: "core_memory_update"
// description: "Update core memory"
```

---

## Configuration

### Memory Configuration

```toml
[memory]
# Core memory limits
core_max_tokens = 500
core_max_entries = 20

# Recall memory policies
recall_ttl_episodic_days = 90
decay_half_life_days = 30.0
min_importance = 0.1

# Search configuration
search_default_limit = 10
search_min_confidence = 0.0
search_recency_weight = 0.0

# Hybrid search
rrf_k = 60.0
mmr_lambda = 0.5
```

---

## Usage Examples

### Complete Memory Setup

```rust
use openrustclaw_memory::{ContextManager, CoreMemoryManager, RecallMemory, MemoryPolicies};
use openrustclaw_agent::{AgentRuntime, ToolRegistry};
use openrustclaw_core::traits::{MemoryStore, CoreMemoryStore};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize stores (implementations omitted)
    let memory_store: Arc<dyn MemoryStore> = // ... initialize
    let core_memory_store: Arc<dyn CoreMemoryStore> = // ... initialize
    
    // Create context manager
    let context_manager = ContextManager::new(8192);
    
    // Create agent runtime with memory
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "MyAgent".to_string(),
        memory_store,
        core_memory_store,
    );
    
    // Runtime now has memory_search, memory_store, core_memory_update tools
    Ok(())
}
```

### Manual Memory Operations

```rust
use openrustclaw_core::types::{MemoryEntry, MemoryQuery, MemoryType, CoreEntry};

// Store a memory
let entry = MemoryEntry {
    id: Uuid::new_v4(),
    memory_type: MemoryType::Semantic,
    content: "User prefers dark mode".to_string(),
    content_hash: "abc123".to_string(),
    source: Some("conversation".to_string()),
    source_type: Some(SourceType::Conversation),
    session_id: Some(session_id),
    user_id: Some("user_42".to_string()),
    namespace: "preferences".to_string(),
    importance: 0.8,
    confidence: 1.0,
    access_count: 0,
    last_accessed: None,
    created_at: Utc::now(),
    expires_at: None,
    metadata: json!({}),
};

memory_store.store(entry).await?;

// Search memories
let query = MemoryQuery {
    text: "user preferences".to_string(),
    memory_types: vec![MemoryType::Semantic],
    source_types: vec![],
    namespace: Some("user_42".to_string()),
    limit: 10,
    min_confidence: 0.5,
    recency_weight: 0.3,
};

let results = memory_store.search(&query).await?;
for scored in results {
    println!("{}: {}", scored.score, scored.entry.content);
}

// Update core memory
let core_entry = CoreEntry {
    key: "name".to_string(),
    value: "Alice".to_string(),
    importance: 0.9,
    token_count: 3,
    updated_at: Utc::now(),
};

core_memory_store.set("user_42", core_entry).await?;
```

---

## Error Handling

```rust
use openrustclaw_core::error::{Error, MemoryError};

match result {
    Err(Error::Memory(MemoryError::CoreMemoryBudgetExceeded { used, max })) => {
        // Evict low-importance entries
        evict_entries(used - max).await?;
    }
    Err(Error::Memory(MemoryError::Duplicate { existing_id })) => {
        // Skip or update existing
        println!("Memory already exists: {}", existing_id);
    }
    Err(Error::Memory(MemoryError::Embedding(e))) => {
        // Handle embedding service error
        eprintln!("Embedding failed: {}", e);
    }
    _ => {}
}
```
