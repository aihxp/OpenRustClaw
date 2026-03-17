# ADR-001: Rust + Python Hybrid Architecture

## Status
Accepted

## Context
OpenRustClaw needed to balance performance with developer productivity and ecosystem access:

- AI/ML ecosystems are predominantly Python-based
- Rust offers superior performance, safety, and concurrency
- System-critical components need maximum reliability
- Tool integrations require rapid iteration

## Decision
We adopted a hybrid architecture:

### Rust Core (80%)
- Message routing and agent runtime
- HTTP/gRPC servers (gateway)
- Provider SDKs (HTTP clients)
- Memory management
- Security-critical code
- Distributed consensus

### Python Sidecar (20%)
- MCP (Model Context Protocol) bridge
- File system sandboxing
- Python-specific tool execution
- Rapid prototyping of new integrations

### Communication
- gRPC with Protocol Buffers
- Unix sockets (local) or TCP (remote)
- Shared memory for large data transfers

## Consequences

### Positive
- **Performance**: Rust core handles 100K+ concurrent connections
- **Safety**: Memory safety guarantees in critical paths
- **Ecosystem**: Full access to Python AI/ML libraries
- **Deployment**: Single binary + optional Python sidecar

### Negative
- **Complexity**: Two languages to maintain
- **Debugging**: Cross-language stack traces
- **Build**: More complex CI/CD pipeline

## Alternatives Considered

### Pure Python
- Rejected: Performance concerns for high-throughput scenarios

### Pure Rust
- Rejected: Would lose access to Python AI ecosystem

### WebAssembly for Python
- Rejected: WASI not mature enough for Python

## References
- [Architecture Overview](../ARCHITECTURE.md)
- [Sidecar Implementation](../../sidecar/)
