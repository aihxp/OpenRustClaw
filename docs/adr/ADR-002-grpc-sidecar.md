# ADR-002: gRPC for Sidecar Communication

## Status
Accepted

## Context
The Rust core and Python sidecar needed a communication protocol that is:
- Fast and efficient
- Type-safe
- Language-agnostic
- Supports streaming

## Decision
We chose gRPC with Protocol Buffers:

```protobuf
service Sidecar {
  rpc ExecuteTool(ToolRequest) returns (ToolResponse);
  rpc StreamLogs(LogRequest) returns (stream LogEntry);
  rpc HealthCheck(HealthRequest) returns (HealthStatus);
}
```

### Transport
- **Local**: Unix domain sockets (lower latency)
- **Remote**: TCP with TLS (for distributed deployments)

### Why gRPC over alternatives?

| Alternative | Pros | Cons |
|-------------|------|------|
| REST/HTTP | Simple, universal | Verbose, no streaming |
| GraphQL | Flexible | Overkill for internal API |
| Raw TCP | Fast | No type safety |
| Message Queue | Decoupling | Added complexity |

## Consequences

### Positive
- **Performance**: HTTP/2 multiplexing, binary serialization
- **Type Safety**: Protocol Buffer definitions
- **Streaming**: Bidirectional streaming for logs
- **Tooling**: Excellent code generation

### Negative
- **Complexity**: Requires protobuf compilation
- **Debugging**: Binary protocol harder to inspect
- **Browser**: Not directly browser-accessible

## Implementation

### Code Generation
```bash
# Rust
protoc --prost_out=src/proto sidecar.proto

# Python
protoc --python_out=sidecar/proto sidecar.proto
```

### Connection Management
- Connection pooling for reuse
- Automatic reconnection with backoff
- Health checks every 30 seconds

## References
- [Sidecar README](../../sidecar/README.md)
- [Protocol Buffers](https://protobuf.dev/)
