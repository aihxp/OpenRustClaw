# ADR-005: Raft for Distributed Consensus

## Status
Accepted

## Context
For distributed deployments, OpenRustClaw needed:
- Leader election (single coordinator)
- State replication across nodes
- Fault tolerance (tolerate node failures)
- Consistency guarantees

## Decision
We chose Raft consensus algorithm:

### Why Raft over Paxos?
- **Understandability**: Easier to reason about
- **Implementation**: Simpler to implement correctly
- **Performance**: Good enough for our use case

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      Cluster                             │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐         │
│  │  Node 1 │◄────►│  Node 2 │◄────►│  Node 3 │         │
│  │ (Leader)│      │(Follower)│     │(Follower)│        │
│  └────┬────┘      └─────────┘      └─────────┘         │
│       │                                                  │
│       ▼                                                  │
│  ┌─────────┐                                            │
│  │  Raft   │  Log Replication                           │
│  │  State  │  (Leader → Followers)                      │
│  └─────────┘                                            │
└─────────────────────────────────────────────────────────┘
```

### Components

1. **RaftNode**: Core consensus logic
2. **Log Entry**: Replicated state machine commands
3. **Leader Election**: Timeout-based voting
4. **Log Replication**: AppendEntries RPC

## Consequences

### Positive
- **Strong Consistency**: All nodes see same state
- **Fault Tolerance**: Tolerates (n-1)/2 failures
- **Leader-Based**: Simple client interaction

### Negative
- **Latency**: Requires majority for writes
- **Complexity**: Leader election edge cases
- **Split-Brain**: Requires odd number of nodes

## Implementation

```rust
pub struct RaftNode {
    id: NodeId,
    state: RaftState,
    current_term: u64,
    voted_for: Option<NodeId>,
    log: Vec<LogEntry>,
    commit_index: u64,
    last_applied: u64,
}

impl RaftNode {
    pub async fn append_entries(&mut self, req: AppendEntries) -> Result<AppendResponse>;
    pub async fn request_vote(&mut self, req: VoteRequest) -> Result<VoteResponse>;
}
```

## Deployment

**Recommended**: 3 or 5 nodes
- 3 nodes: Tolerates 1 failure
- 5 nodes: Tolerates 2 failures

## References
- [Raft Paper](https://raft.github.io/raft.pdf)
- [Distributed Crate](../../crates/distributed/)
- [Raft Implementation](../../crates/distributed/src/consensus.rs)
