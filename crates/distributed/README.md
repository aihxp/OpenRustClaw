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

```ignore
use std::sync::Arc;

use openrustclaw_distributed::{Cluster, ConsensusConfig, DistributedConfig, LocalNode, NodeInfo, NodeRole, RaftNode};

let addr = "127.0.0.1:50051".parse().unwrap();
let local = Arc::new(LocalNode::new(NodeInfo::new("node-1", "Node 1", addr, addr, NodeRole::Worker)));
let cluster = Cluster::new(local.clone(), DistributedConfig::default());
let node = RaftNode::new(local, cluster, ConsensusConfig::default());
```

### Distributed Memory

```ignore
use openrustclaw_distributed::{DistributedMemory, GossipConfig, GossipMemory, MemoryBackend, MemoryConfig};

let config = MemoryConfig {
    backend: MemoryBackend::Gossip,
    gossip: GossipConfig::default(),
    ..MemoryConfig::default()
};

let memory = GossipMemory::new(&config)?;
memory.set("key", b"value".to_vec(), None).await?;
```

## License

MIT OR Apache-2.0
