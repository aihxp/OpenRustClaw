# ADR-003: Three-Tier Memory Architecture

## Status
Accepted

## Context
AI agents need to manage context within LLM token limits while maintaining long-term knowledge. We needed a memory system that:
- Respects token budgets
- Provides fast access to recent context
- Enables long-term knowledge retrieval
- Supports different storage backends

## Decision
We implemented a three-tier memory hierarchy:

```
┌─────────────────────────────────────────────────────┐
│                    CORE MEMORY                       │
│         (Active conversation context)               │
│              Fastest access, smallest               │
│                    ~4K-8K tokens                    │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                   RECALL MEMORY                      │
│         (Recent history with TTL)                   │
│         Minutes to hours retention                  │
│              Redis / In-Memory                      │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                   ARCHIVE MEMORY                     │
│         (Long-term storage + vectors)               │
│         Days to years retention                     │
│    Vector DB / SQLite / Postgres                    │
└─────────────────────────────────────────────────────┘
```

### Core Memory
- Stores current conversation messages
- Manages token budget with summarization
- In-process, zero-copy access

### Recall Memory
- Recent conversations (configurable TTL)
- Fast key-value lookups
- Backends: Redis, in-memory

### Archive Memory
- Long-term storage with semantic search
- Vector embeddings for similarity
- Backends: Pinecone, Qdrant, pgvector

## Consequences

### Positive
- **Token efficiency**: Only relevant context sent to LLM
- **Performance**: Hot data in memory, cold data persisted
- **Flexibility**: Pluggable backends for different needs
- **Cost**: Tiered storage reduces infrastructure costs

### Negative
- **Complexity**: Three systems instead of one
- **Consistency**: Data moves between tiers
- **Debugging**: Harder to trace data flow

## Implementation

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn save(&self, entry: MemoryEntry) -> Result<()>;
    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
    async fn archive(&self, entry: MemoryEntry) -> Result<()>;
    async fn search_archive(&self, embedding: Vec<f32>) -> Result<Vec<MemoryEntry>>;
}
```

## References
- [Memory Crate](../../crates/memory/)
- [Memory Types](../../crates/core/src/types/memory.rs)
