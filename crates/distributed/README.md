# OpenRustClaw Distributed

Multi-node distributed mode for OpenRustClaw, enabling horizontal scaling across multiple machines.

## Features

- **Cluster Formation**: Automatic node discovery and cluster formation
- **Leader Election**: Raft consensus algorithm for reliable leader election
- **Request Distribution**: Consistent hashing and load balancing across workers
- **Distributed Memory**: Shared state via Redis, etcd, or gossip replication
- **Session Management**: Distributed session routing with sticky session support
- **Task Distribution**: Job scheduling and execution across the cluster
- **Health Monitoring**: Automatic failover and recovery

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Leader    │────▶│  Worker 1   │     │  Worker 2   │
│  (API GW)   │     │ (Agents)    │     │ (Agents)    │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       └───────────────────┴───────────────────┘
                    Shared State
              (etcd / Consul / Redis)
```

## Quick Start

### Leader Node

```rust
use openrustclaw_distributed::{ClusterManager, ClusterManagerBuilder};

let manager = ClusterManagerBuilder::new()
    .node_id("leader-1")
    .bootstrap_leader()
    .with_gossip()
    .build()
    .await?;

manager.start().await?;
```

### Worker Node

```rust
use openrustclaw_distributed::ClusterManagerBuilder;
use std::net::SocketAddr;

let leader_addr: SocketAddr = "10.0.0.1:50051".parse()?;

let manager = ClusterManagerBuilder::new()
    .node_id("worker-1")
    .join_worker(leader_addr)
    .build()
    .await?;

manager.start().await?;
```

## Configuration

### TOML Configuration

```toml
[distributed]
enabled = true
node_id = "node-1"
listen_addr = "0.0.0.0:50051"
api_addr = "0.0.0.0:8080"
bootstrap_leader = true

[discovery]
backend = "etcd"
etcd_endpoints = ["etcd-1:2379", "etcd-2:2379", "etcd-3:2379"]

[consensus]
election_timeout_ms = 1000
heartbeat_interval_ms = 100

[load_balancer]
strategy = "consistent_hash"
sticky_sessions = true
virtual_nodes = 150

[memory]
backend = "redis"
redis_url = "redis://localhost:6379"
```

## Components

### Cluster Management

- `Cluster`: Manages cluster membership and node state
- `ClusterManager`: Main entry point for distributed operations

### Consensus (Raft)

- `RaftNode`: Implements Raft consensus for leader election
- Handles log replication and split-brain protection

### Service Discovery

- `Discovery` trait with implementations:
  - `EtcdDiscovery`: Uses etcd for service registration
  - `GossipDiscovery`: Uses mDNS for gossip-based discovery
  - `StaticDiscovery`: Static configuration only

### Load Balancing

- `LoadBalancer`: Distributes requests across workers
- Strategies: Round-robin, Least connections, Consistent hashing
- `SessionRouter`: Routes sessions to appropriate nodes

### Session Management

- `SessionManager`: Manages distributed sessions
- Session migration and replication support
- Sticky session affinity

### Task Distribution

- `TaskManager`: Manages task queues on workers
- `TaskScheduler`: Schedules tasks on the leader
- `TaskExecutor` trait for custom task execution

### Distributed Memory

- `DistributedMemory` trait with implementations:
  - `RedisMemory`: Redis-backed distributed state
  - `GossipMemory`: Gossip-based replication
  - `EtcdMemory`: etcd-backed state

## gRPC Services

The distributed crate provides several gRPC services:

- `ClusterService`: Cluster membership management
- `RaftService`: Raft consensus RPCs
- `SessionService`: Session management
- `TaskService`: Task distribution

## CLI Integration

```bash
# Start leader node
openrustclaw start --leader --cluster-addr 0.0.0.0:50051

# Start worker node
openrustclaw start --worker --leader-addr 10.0.0.1:50051

# Check cluster status
openrustclaw cluster status

# Scale workers
openrustclaw cluster scale --workers 5
```

## Docker Compose Example

```yaml
version: '3'
services:
  leader:
    image: openrustclaw
    command: --leader --cluster-addr 0.0.0.0:50051
    ports:
      - "8080:8080"
      - "50051:50051"
    environment:
      - RUST_LOG=info
  
  worker-1:
    image: openrustclaw
    command: --worker --leader-addr leader:50051
    depends_on:
      - leader
  
  worker-2:
    image: openrustclaw
    command: --worker --leader-addr leader:50051
    depends_on:
      - leader
  
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
```

## Testing

```bash
# Run unit tests
cargo test -p openrustclaw-distributed

# Run integration tests
cargo test -p openrustclaw-distributed --test integration_tests
```

## License

MIT License - See LICENSE file for details.
