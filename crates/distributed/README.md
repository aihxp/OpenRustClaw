# OpenRustClaw Distributed

Distributed systems support for OpenRustClaw.

## Overview

Provides distributed capabilities for horizontal scaling:

- **Consensus**: Raft-based consensus for cluster coordination
- **Memory**: Distributed memory with eventual consistency
- **Load Balancing**: Request distribution across nodes
- **Clustering**: Node discovery and membership

## Components

### Raft Consensus

```rust
use openrustclaw_distributed::consensus::{RaftNode, RaftConfig};

let config = RaftConfig {
    node_id: "node-1".to_string(),
    peers: vec!["node-2".to_string(), "node-3".to_string()],
    ..Default::default()
};

let node = RaftNode::new(config).await?;
```

### Distributed Memory

```rust
use openrustclaw_distributed::memory::{GossipMemory, MemoryConfig};

let config = MemoryConfig {
    node_id: "node-1".to_string(),
    seeds: vec!["node-2:7946".to_string()],
};

let memory = GossipMemory::new(&config).await?;
memory.set("key", b"value".to_vec(), None).await?;
```

## License

MIT OR Apache-2.0
