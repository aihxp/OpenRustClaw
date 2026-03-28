# OpenRustClaw Distributed

Distributed systems support for OpenRustClaw.

## Overview

Provides distributed capabilities for horizontal scaling:

- **Consensus**: Raft-based consensus for cluster coordination
- **Memory**: Distributed memory with eventual consistency
- **Load Balancing**: Request distribution across nodes
- **Clustering**: Node discovery and membership

This crate is the distributed cluster lane. It is not the whole remote-connectivity story by itself.

- a `distributed node` is an advanced cluster member for multi-machine coordination
- a `mobile node` is a bounded device-side participant with its own operator surface
- an `SSH tunnel` is a transport fallback for reaching a protected local runtime when a node-first path is unavailable
- a `reverse proxy` is a bounded last-resort remote-access path, not the preferred topology

The local runtime remains the primary production anchor unless a deployment explicitly opts into the distributed lane.

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
