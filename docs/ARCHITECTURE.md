# OpenRustClaw Architecture

## Overview

OpenRustClaw is a multi-modal AI agent runtime built with a hybrid Rust/Python architecture. This document describes the high-level architecture, key components, and design decisions.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              OpenRustClaw                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Gateway   │  │   Agents    │  │   Skills    │  │  Scheduler  │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         └─────────────────┴─────────────────┴─────────────────┘             │
│                                    │                                        │
│                           ┌────────┴────────┐                              │
│                           │   Core Runtime  │                              │
│                           │   (Rust Core)   │                              │
│                           └────────┬────────┘                              │
│                                    │                                        │
├────────────────────────────────────┼────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  │  ┌─────────────┐  ┌─────────────┐     │
│  │  Providers  │  │   Memory    │◄─┘  │  Channels   │  │  Security   │     │
│  └─────────────┘  └─────────────┘     └─────────────┘  └─────────────┘     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Distributed Layer (Optional)                     │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │   │
│  │  │ Cluster │  │  Raft   │  │GossipMem│  │  Load   │  │ Discovery│   │   │
│  │  │ Manager │  │Consensus│  │  ory   │  │Balancer │  │          │   │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Sidecar (Python)                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │   │
│  │  │  MCP Bridge │  │ Tool Exec   │  │ File System │                  │   │
│  │  │  (gRPC)     │  │             │  │  Sandbox    │                  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Core Runtime (`crates/core`)
The heart of OpenRustClaw, written in Rust for performance and safety.

**Key Responsibilities:**
- Message routing and dispatch
- Agent lifecycle management
- Configuration management
- Error handling and recovery

**Key Types:**
- `Agent`: Core agent trait and implementations
- `Message`: Inter-agent communication
- `Tool`: Extensible tool system
- `Task`: Async task management

### 2. Gateway (`crates/gateway`)
HTTP/WebSocket API server for external integrations.

**Features:**
- RESTful API for agent control
- WebSocket for real-time streaming
- Webhook handling
- Rate limiting and authentication

### 3. Providers (`crates/providers` + SDK crates)
LLM provider integrations with unified interface.

**Supported Providers:**
- OpenAI (GPT-4, GPT-3.5)
- Anthropic (Claude)
- Azure OpenAI
- AWS Bedrock
- Google Gemini
- Local models (Ollama, vLLM, llama.cpp)
- And 15+ more providers

**Architecture Pattern:**
```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> Result<Stream>;
    fn name(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;
}
```

### 4. Memory System (`crates/memory`)
Hierarchical memory architecture for context management.

**Layers:**
- **Core Memory**: Active conversation context
- **Recall Memory**: Recent history (configurable TTL)
- **Archive Memory**: Long-term storage with vector search

**Backends:**
- In-memory (development)
- SQLite (embedded)
- Redis (production)
- Vector DBs (Pinecone, Qdrant, pgvector)

### 5. Channels (`crates/channels`)
Multi-platform messaging integrations.

**Supported Channels:**
- WebChat (built-in)
- Telegram, Discord, Slack
- Microsoft Teams, Google Chat
- WhatsApp, Matrix, iMessage
- LINE, Viber, WeChat
- Meta (Messenger, Instagram)

### 6. Security (`crates/security`)
Enterprise security features.

**Components:**
- JWT/OAuth2 authentication
- SAML 2.0 / OIDC SSO
- RBAC with fine-grained permissions
- Input sanitization and prompt injection detection
- Skill sandboxing (WASM)

### 7. Distributed Mode (`crates/distributed`)
Horizontal scaling capabilities.

**Features:**
- Raft consensus for leader election
- Gossip protocol for service discovery
- Distributed memory (Redis/etcd)
- Load balancing (round-robin, consistent hashing)
- Session affinity

## Data Flow

### Typical Request Flow

```
1. User Request
   └─► Channel (e.g., Slack)
       └─► Gateway
           └─► Core Runtime
               ├─► Security (Auth/Authz)
               ├─► Memory (Load context)
               ├─► Provider (LLM call)
               ├─► Skills (Tool execution)
               └─► Memory (Save response)
                   └─► Channel Response
```

### Tool Execution Flow

```
1. LLM requests tool call
   └─► Core Runtime
       ├─► Built-in tool? → Execute directly
       └─► MCP tool? → Sidecar
               └─► MCP Server
                   └─► Result
                       └─► LLM
                           └─► Final response
```

## Technology Stack

### Backend (Rust)
- **Runtime**: Tokio (async)
- **HTTP**: Axum, reqwest
- **Serialization**: serde, prost (gRPC)
- **Database**: sqlx, redis
- **ML/AI**: Candle (optional local inference)

### Sidecar (Python)
- **MCP**: Model Context Protocol SDK
- **Sandbox**: RestrictedPython, seccomp
- **gRPC**: grpcio

### Infrastructure
- **Container**: Docker, Kubernetes
- **Observability**: Prometheus, Grafana, OpenTelemetry
- **Messaging**: gRPC, NATS (optional)

## Design Principles

1. **Performance**: Rust core for hot paths, Python for flexibility
2. **Extensibility**: Plugin architecture with WASM sandboxing
3. **Reliability**: Structured concurrency, graceful degradation
4. **Observability**: OpenTelemetry tracing throughout
5. **Security**: Defense in depth, zero-trust architecture

## Related Documentation

- [Architecture Decision Records](./adr/)
- [API Reference](./API_REFERENCE.md)
- [Deployment Guide](./DEPLOYMENT.md)
- [Security Policy](../SECURITY.md)
