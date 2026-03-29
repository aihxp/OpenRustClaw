# OpenRustClaw Memory

Memory management system for the OpenRustClaw assistant runtime.

## Overview

Provides three-tier memory architecture:

1. **Core Memory**: Essential user information (always available)
2. **Recall Memory**: Recent conversation history
3. **Archive Memory**: Long-term searchable memory

## Memory Types

### Core Memory

```rust
use openrustclaw_memory::CoreMemoryStore;

store.set(user_id, CoreEntry {
    key: "name".to_string(),
    value: "Alice".to_string(),
    importance: 1.0,
}).await?;
```

### Recall Memory

```rust
use openrustclaw_memory::RecallMemoryStore;

store.store(MemoryEntry {
    id: uuid::Uuid::new_v4().to_string(),
    content: "User likes pizza".to_string(),
    timestamp: chrono::Utc::now(),
    source: "conversation".to_string(),
}).await?;
```

### Archive Memory

```rust
use openrustclaw_memory::ArchiveMemoryStore;

// Search archived memories
let results = store.search("pizza preferences", 10).await?;
```

## Storage Backends

- **Redis**: High-performance caching
- **PostgreSQL**: Persistent storage
- **SQLite**: Embedded/development
- **Qdrant**: Vector search for semantic memory

## License

MIT OR Apache-2.0
