# Architecture Overview

This document provides a comprehensive overview of OpenRustClaw's architecture, explaining the design decisions, data flow, and component interactions.

---

## 🏗️ High-Level Architecture

OpenRustClaw follows a **three-layer architecture** that combines Rust's performance with Python's AI ecosystem:

```mermaid
flowchart TB
    subgraph Clients["Client Applications"]
        Web["Web Chat"]
        CLI["CLI / TUI"]
        API["REST API"]
        Cursor["Cursor IDE"]
        Telegram["Telegram (v2)"]
    end
    
    subgraph Layer1["Layer 1: Rust Core"]
        direction TB
        GW["Gateway<br/>Axum WebSocket"]
        AUTH["Auth Service<br/>JWT + Origin Check"]
        SESS["Session Manager"]
        AGENT["Agent Runtime"]
        MEM["Memory System<br/>3-Tier Architecture"]
        TOOLS["Tool Registry"]
        SEC["Security Layer<br/>6 Modules"]
        SCHED["Durable Scheduler"]
    end
    
    subgraph DB["Persistence Layer"]
        SQLITE["SQLite<br/>WAL Mode"]
        SQLX["sqlx (async)"]
        LIBSQL["libSQL (vectors)"]
    end
    
    subgraph Layer2["Layer 2: Python Sidecar"]
        GRPC["gRPC Server"]
        LANG["LangGraph Workflows"]
        RAG["RAG Pipeline"]
        EVAL["Evaluators"]
    end
    
    subgraph Layer3["Layer 3: Observability"]
        LANGSMITH["LangSmith"]
        OTEL["OpenTelemetry"]
        PROM["Prometheus"]
    end
    
    Clients -->|WebSocket/HTTP| GW
    GW --> AUTH
    GW --> SESS
    SESS --> AGENT
    AGENT --> MEM
    AGENT --> TOOLS
    AGENT --> SEC
    AGENT --> SCHED
    
    Layer1 <-->|gRPC| Layer2
    Layer1 <-->|SQL| DB
    Layer2 -->|REST API| Layer3
    Layer1 -->|Traces| Layer3
```

---

## 📊 Layer 1: Rust Core (14 Crates)

The Rust Core is the heart of OpenRustClaw, handling all synchronous operations, WebSocket connections, and high-throughput tasks.

```mermaid
flowchart LR
    subgraph CoreCrates["Core Crates"]
        CORE["core<br/>Types & Traits"]
        DB["db<br/>Persistence"]
        CONFIG["config<br/>Settings"]
    end
    
    subgraph FeatureCrates["Feature Crates"]
        MEM["memory<br/>3-Tier System"]
        PROV["providers<br/>LLM Clients"]
        MCP["mcp<br/>Protocol"]
        SEC["security<br/>Hardening"]
        OBS["observability<br/>Tracing"]
    end
    
    subgraph RuntimeCrates["Runtime Crates"]
        AGENT["agent<br/>Runtime"]
        GATE["gateway<br/>WebSocket"]
        CHAN["channels<br/>Integrations"]
        SKILL["skills<br/>WASM Sandbox"]
        SCHED["scheduler<br/>Durable Jobs"]
        BRIDGE["langbridge<br/>gRPC"]
    end
    
    subgraph Interface["Interface"]
        CLI["cli<br/>Commands"]
    end
    
    CORE --> DB
    CORE --> CONFIG
    DB --> MEM
    CORE --> PROV
    CORE --> MCP
    CORE --> SEC
    CORE --> OBS
    
    MEM --> AGENT
    PROV --> AGENT
    MCP --> AGENT
    SEC --> AGENT
    OBS --> AGENT
    
    AGENT --> GATE
    AGENT --> CHAN
    AGENT --> SKILL
    AGENT --> SCHED
    AGENT --> BRIDGE
    
    GATE --> CLI
    SCHED --> CLI
    MEM --> CLI
```

### Crate Responsibilities

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| `core` | Shared types, traits, errors | `serde`, `uuid`, `chrono` |
| `db` | SQLite persistence | `sqlx`, `libsql`, `rusqlite` |
| `memory` | 3-tier memory + RAG | `db`, `tiktoken-rs` |
| `providers` | LLM provider SDKs | `reqwest`, `tokio` |
| `mcp` | MCP client/server | `jsonschema`, `tower` |
| `security` | Auth, sandbox, audit | `ed25519-dalek`, `wasmtime` |
| `agent` | Runtime, tool registry | All above |
| `gateway` | Axum WebSocket server | `axum`, `tower-http` |
| `scheduler` | Durable job scheduling | `cron`, `chrono-tz` |
| `langbridge` | gRPC to Python | `tonic`, `prost` |
| `observability` | LangSmith, tracing | `opentelemetry`, `tracing` |
| `cli` | Command-line interface | `clap`, `ratatui` |

---

## 🐍 Layer 2: Python LangGraph Sidecar

The Python sidecar handles AI orchestration, complex workflows, and maintenance tasks.

```mermaid
flowchart TB
    subgraph Sidecar["Python Sidecar"]
        GRPC["gRPC Server<br/>grpcio"]
        
        subgraph Workflows["LangGraph Workflows"]
            AGENT_WF["Agent Workflow<br/>StateGraph"]
            MEM_WF["Memory Maintenance<br/>Consolidation"]
            SCHED_WF["Scheduled Execution<br/>Reminders"]
            RAG_WF["RAG Pipeline<br/>Retrieval"]
        end
        
        subgraph Eval["Evaluators"]
            MEM_EVAL["Memory Recall<br/>Accuracy"]
            RAG_EVAL["RAG Precision<br/>@K"]
            TOOL_EVAL["Tool Use<br/>Correctness"]
        end
    end
    
    subgraph External["External Services"]
        LANGSMITH["LangSmith<br/>Tracing"]
        OPENAI["OpenAI<br/>Embeddings"]
    end
    
    GRPC --> Workflows
    Workflows --> Eval
    Workflows --> External
```

### Workflow Types

1. **Agent Workflow** — Main conversation loop with tool use
2. **Memory Maintenance** — Consolidation, deduplication, pruning
3. **Scheduled Execution** — Durable job processing
4. **RAG Pipeline** — Document ingestion and retrieval

---

## 🔄 Data Flow

### Typical Request Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant GW as Gateway
    participant SESS as SessionMgr
    participant AGENT as Agent Runtime
    participant MEM as Memory
    participant PROV as Provider
    participant SIDE as Python Sidecar
    
    Client->>GW: WebSocket: message
    GW->>GW: Origin validation
    GW->>GW: JWT verification
    GW->>SESS: Get/Create session
    SESS-->>GW: Session context
    
    GW->>AGENT: Process message
    AGENT->>MEM: Load core memory
    MEM-->>AGENT: ~500 tokens
    
    AGENT->>PROV: Completion request
    PROV-->>AGENT: Tool call requested
    
    AGENT->>MEM: Search relevant context
    MEM-->>AGENT: Top-K results
    
    AGENT->>PROV: Re-request with context
    PROV-->>AGENT: Response + tool calls
    
    AGENT->>AGENT: Execute tools
    AGENT->>SIDE: Log trace (async)
    AGENT->>MEM: Store new memories
    
    AGENT-->>GW: Response + events
    GW-->>Client: WebSocket: reply
```

### Memory Write Flow

```mermaid
sequenceDiagram
    participant AGENT as Agent Runtime
    participant POLICY as Memory Policies
    participant DB as Database
    participant SIDE as Python Sidecar
    
    AGENT->>POLICY: Proposed memory
    POLICY->>POLICY: Deduplication check
    POLICY->>POLICY: Importance scoring
    POLICY->>POLICY: Confidence validation
    
    alt Passes all checks
        POLICY->>DB: Store entry
        DB-->>POLICY: Success
        POLICY->>SIDE: Generate embedding (async)
        POLICY-->>AGENT: Stored successfully
    else Duplicate found
        POLICY->>DB: Update existing
        POLICY-->>AGENT: Duplicate merged
    else Low confidence
        POLICY-->>AGENT: Rejected
    end
```

---

## 🔒 Security Boundaries

```mermaid
flowchart TB
    subgraph Perimeter["Network Perimeter"]
        ORIGIN["Origin Check<br/>Whitelist validation"]
        RATE["Rate Limiting<br/>Token bucket"]
    end
    
    subgraph AuthLayer["Authentication Layer"]
        JWT["JWT Validation<br/>Token verification"]
        SESSION["Session Binding<br/>User isolation"]
    end
    
    subgraph AppLayer["Application Layer"]
        INJECT["Prompt Injection<br/>Defense"]
        SANITIZE["Input Sanitization<br/>Validation"]
    end
    
    subgraph ExecutionLayer["Execution Layer"]
        CAPS["Capability Check<br/>Permission validation"]
        WASM["WASM Sandbox<br/>wasmtime"]
        ISOLATION["Filesystem Isolation<br/>Per-session chroot"]
    end
    
    subgraph Audit["Audit Layer"]
        LOG["Audit Logging<br/>Immutable records"]
    end
    
    Perimeter --> AuthLayer
    AuthLayer --> AppLayer
    AppLayer --> ExecutionLayer
    ExecutionLayer --> Audit
```

---

## 🗄️ Persistence Architecture

### Database Schema Overview

```mermaid
erDiagram
    SESSIONS {
        uuid id PK
        string session_type
        string user_id
        string platform
        datetime created_at
        datetime updated_at
    }
    
    CONVERSATIONS {
        uuid id PK
        uuid session_id FK
        string role
        text content
        json tool_calls
        datetime created_at
    }
    
    MEMORY_ENTRIES {
        uuid id PK
        string memory_type
        text content
        string content_hash
        string namespace
        float importance
        float confidence
        datetime created_at
        datetime expires_at
    }
    
    MEMORY_VECTORS {
        uuid entry_id FK
        vector embedding
    }
    
    CORE_MEMORY {
        string user_id PK
        string key PK
        text value
        int token_count
        float importance
        datetime updated_at
    }
    
    SCHEDULED_JOBS {
        string id PK
        string name
        string workflow_name
        json trigger_config
        string status
        datetime next_run
    }
    
    AUDIT_LOG {
        uuid id PK
        string event_type
        string user_id
        uuid session_id
        json details
        datetime timestamp
    }
    
    SESSIONS ||--o{ CONVERSATIONS : has
    MEMORY_ENTRIES ||--o| MEMORY_VECTORS : has
```

### Database Access Patterns

| Crate | Access Pattern | Library |
|-------|---------------|---------|
| `gateway` | Read/write sessions | `sqlx` |
| `memory` | Read/write entries, search | `sqlx` + `libsql` |
| `db` | Migrations, core memory | `rusqlite` |
| `scheduler` | Job scheduling | `sqlx` |
| `security` | Audit logging | `sqlx` |

---

## 🔌 Component Interactions

### Provider Fallback Chain

```mermaid
flowchart LR
    REQ[Request] --> P1[Anthropic]
    P1 -->|Rate limit| P2[OpenAI]
    P1 -->|Success| RESP[Response]
    P2 -->|Error| P3[OpenRouter]
    P2 -->|Success| RESP
    P3 -->|Error| P4[Ollama]
    P3 -->|Success| RESP
    P4 -->|Error| ERR[All Failed]
    P4 -->|Success| RESP
```

### Tool Execution Flow

```mermaid
flowchart TB
    CALL[Tool Call] --> REGISTRY[Tool Registry]
    REGISTRY --> LOOKUP{Tool Found?}
    LOOKUP -->|No| ERR1[ToolError::NotFound]
    LOOKUP -->|Yes| CAPS[Capability Check]
    
    CAPS --> PERM{Permitted?}
    PERM -->|No| ERR2[ToolError::CapabilityDenied]
    PERM -->|Yes| EXEC{Execution Type}
    
    EXEC -->|Native| NATIVE[Native Tool]
    EXEC -->|WASM| SANDBOX[WASM Sandbox]
    EXEC -->|MCP| MCP[MCP Client]
    
    NATIVE --> RESULT[ToolOutput]
    SANDBOX --> RESULT
    MCP --> RESULT
    
    RESULT --> AUDIT[Audit Log]
    AUDIT --> RETURN[Return to Agent]
```

---

## 📊 Scalability Considerations

### Vertical Scaling (Single Node)

- **SQLite WAL mode** enables concurrent reads
- **Async throughout** with Tokio runtime
- **Bounded concurrency** for embeddings
- **Connection pooling** via `sqlx`

### Horizontal Scaling (Future)

```mermaid
flowchart TB
    subgraph LB["Load Balancer"]
        NGINX["Nginx/HAProxy"]
    end
    
    subgraph Nodes["OpenRustClaw Nodes"]
        N1["Node 1<br/>SQLite"]
        N2["Node 2<br/>SQLite"]
        N3["Node 3<br/>SQLite"]
    end
    
    subgraph Shared["Shared Services"]
        REDIS["Redis<br/>Session Store"]
        POSTGRES["PostgreSQL<br/>Shared State"]
    end
    
    LB --> N1
    LB --> N2
    LB --> N3
    
    N1 --> Shared
    N2 --> Shared
    N3 --> Shared
```

---

## 🎯 Design Principles

### 1. **Security First**
- All connections authenticated
- All inputs sanitized
- All skills verified
- All actions audited

### 2. **Recall-Only Memory**
- Never inject large files into prompts
- Agent must actively search for context
- Reduces token waste by ~90%

### 3. **Durable Execution**
- No cron jobs
- Distributed leases for scheduling
- Idempotency keys prevent duplicates

### 4. **Provider Agnostic**
- Support multiple LLM providers
- Automatic fallback chains
- Native SDK compliance

### 5. **Observability Built-In**
- Every operation traced
- Every decision logged
- Offline eval datasets

---

## 🛠️ Technology Stack

### Backend (Rust)
- **Runtime**: Tokio (async)
- **HTTP**: Axum, reqwest
- **Serialization**: serde, prost (gRPC)
- **Database**: sqlx, libSQL, rusqlite
- **Sandbox**: wasmtime (WASM)
- **Crypto**: ed25519-dalek

### Sidecar (Python)
- **Orchestration**: LangGraph
- **Observability**: LangSmith
- **gRPC**: grpcio

### Infrastructure
- **Container**: Docker, Kubernetes
- **Observability**: Prometheus, Grafana, OpenTelemetry
- **Messaging**: gRPC, NATS (optional)

---

## 🌐 Distributed Mode (Future)

Horizontal scaling capabilities planned for future releases:
- Raft consensus for leader election
- Gossip protocol for service discovery
- Distributed memory (Redis/etcd)
- Load balancing (round-robin, consistent hashing)
- Session affinity

---

## 📚 Related Documentation

- [Rust Core Deep Dive](./rust-core.md) — All 14 crates explained
- [LangGraph Sidecar](./langgraph-sidecar.md) — Python orchestration
- [Memory & RAG](./memory-rag.md) — 3-tier memory system
- [Security Architecture](../guides/security.md) — Security deep dive
