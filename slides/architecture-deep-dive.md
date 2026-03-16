---
marp: true
theme: default
paginate: true
class: invert
header: 'Architecture Deep Dive'
footer: '© 2026 OpenRustClaw Project'
---

<!--
Speaker Notes: This deck provides a technical deep dive into OpenRustClaw's architecture. Target audience: engineers and architects. Plan for 30 minutes.
-->

<style>
section {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}
h1, h2 {
  color: #e67e22;
}
strong {
  color: #3498db;
}
table {
  font-size: 0.8em;
}
code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.85em;
}
</style>

# 🏗️ Architecture Deep Dive

## Understanding OpenRustClaw's Design

### A 3-Layer Hybrid Architecture

---

<!--
Speaker Notes: Start with the high-level view. The hybrid approach is key—Rust for performance, Python for AI workflows.
-->

## 🎯 3-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         LAYER 1                                  │
│                     🦀 RUST CORE (14 crates)                     │
│                                                                  │
│    High-performance, memory-safe, concurrent execution          │
│    • WebSocket gateway    • Agent runtime    • Memory layer     │
│    • Tool execution       • Security         • Persistence      │
└────────────────────────────┬────────────────────────────────────┘
                             │ gRPC (tonic)
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                         LAYER 2                                  │
│              🐍 PYTHON LANGGRAPH SIDECAR                         │
│                                                                  │
│    AI orchestration, workflow management, maintenance tasks     │
│    • Agent workflows      • Memory consolidation                │
│    • Scheduled execution  • Human-in-the-loop                   │
└────────────────────────────┬────────────────────────────────────┘
                             │ REST API
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                         LAYER 3                                  │
│               📊 OBSERVABILITY (LangSmith)                       │
│                                                                  │
│    Tracing, metrics, evaluation, cost tracking                  │
└─────────────────────────────────────────────────────────────────┘
```

---

<!--
Speaker Notes: Explain why this split makes sense. Rust handles I/O and performance-critical paths; Python handles complex AI reasoning workflows.
-->

## 🦀 Rust Core — 14 Crates Breakdown

### Dependency Graph

```
                    ┌─────────┐
                    │  core   │ ← Types, traits, errors
                    └────┬────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
    ┌────▼────┐    ┌─────▼─────┐   ┌────▼────┐
    │   db    │    │  memory   │   │providers│
    └────┬────┘    └─────┬─────┘   └────┬────┘
         │               │               │
         │         ┌─────▼─────┐         │
         │         │ security  │◄────────┘
         │         └─────┬─────┘
         │               │
    ┌────▼───────────────▼────┐
    │        agent            │ ← Runtime, tools, streaming
    └───────────┬─────────────┘
                │
    ┌───────────┼───────────┬───────────┐
    │           │           │           │
┌───▼───┐  ┌────▼────┐ ┌────▼────┐ ┌───▼────┐
│gateway│  │ channels│ │ skills  │ │scheduler│
└───┬───┘  └─────────┘ └────┬────┘ └────┬───┘
    │                       │           │
    └───────────┬───────────┴───────────┘
                │
          ┌─────▼──────┐
          │ langbridge │ ← gRPC to Python
          └─────┬──────┘
                │
          ┌─────▼─────┐
          │    cli    │ ← 10 CLI commands
          └───────────┘
```

---

<!--
Speaker Notes: Go through each crate's responsibility. Mention that this modular design allows for independent testing and deployment.
-->

## 📦 Crate Responsibilities

### Foundation Layer

| Crate | Responsibility | Key Dependencies |
|-------|---------------|------------------|
| `core` | Types, traits, errors, config | `thiserror`, `serde` |
| `db` | SQLite persistence, migrations | `sqlx`, `libsql`, `rusqlite` |

### Service Layer

| Crate | Responsibility | Key Technologies |
|-------|---------------|------------------|
| `memory` | 3-tier memory, RAG, search | FTS5, vectors, BM25 |
| `providers` | LLM clients, fallback chain | Native SDKs, `reqwest` |
| `security` | Auth, origin check, injection | Ed25519, WASMtime |
| `mcp` | MCP protocol client + server | JSON-RPC, schema transform |

### Application Layer

| Crate | Responsibility | Key Technologies |
|-------|---------------|------------------|
| `agent` | Runtime, tool registry, streaming | `tokio`, `futures` |
| `gateway` | Axum WebSocket server | `axum`, `tokio-tungstenite` |

---

<!--
Speaker Notes: Continue with the remaining crates. Emphasize the scheduler's durability guarantees and the langbridge's role.
-->

## 📦 More Crate Responsibilities

### Integration Layer

| Crate | Responsibility | Key Technologies |
|-------|---------------|------------------|
| `channels` | Chat platform integrations | WebChat (v1), future: Discord, Slack |
| `skills` | SKILL.md parser, WASM sandbox | `wasmtime`, Ed25519 verify |
| `scheduler` | Durable job scheduling | LangGraph integration |
| `langbridge` | gRPC bridge to Python | `tonic`, Protocol Buffers |
| `observability` | Tracing, metrics, LangSmith | OpenTelemetry |
| `cli` | Command-line interface | `clap`, `anyhow` |

### Why 14 Crates?

```rust
// Independent versioning
// Separate test suites
// Selective deployment
// Clear dependency boundaries
```

---

<!--
Speaker Notes: Explain how the Python sidecar extends capabilities without sacrificing Rust's performance for critical paths.
-->

## 🐍 Python Sidecar — LangGraph Workflows

### Architecture

```python
# sidecar/src/server.py
class LangBridgeServicer(langbridge_pb2_grpc.LangBridgeServicer):
    """gRPC server for Rust ↔ Python communication"""
    
    async def ExecuteWorkflow(self, request, context):
        # LangGraph StateGraph execution
        graph = self.build_workflow_graph(request.workflow_type)
        result = await graph.ainvoke(request.state)
        return langbridge_pb2.WorkflowResult(data=result)
```

### Workflow Types

```
sidecar/src/workflows/
├── agent_workflow.py      # Main agent reasoning
├── memory_maintenance.py  # Consolidation & cleanup
├── reminder_executor.py   # Durable scheduled tasks
├── human_approval.py      # HITL workflows
└── rag_pipeline.py        # Document processing
```

### Why Python for This?

- 🧠 **LangGraph ecosystem** — StateGraph, checkpoints, persistence
- 📚 **RAG libraries** — LangChain document loaders, embeddings
- 🤖 **Model integrations** — Easy access to all LLM providers

---

<!--
Speaker Notes: Trace a message through the system. This helps understand the data flow end-to-end.
-->

## 🔄 Data Flow Diagram

### Message Processing Pipeline

```
┌──────────┐     ┌──────────────┐     ┌───────────────┐
│  Client  │────►│   Gateway    │────►│ Auth/Origin   │
│ (WebChat)│     │  (Axum WS)   │     │   Validation  │
└──────────┘     └──────────────┘     └───────┬───────┘
                                              │
┌──────────┐     ┌──────────────┐     ┌───────▼───────┐
│  Client  │◄────│   Response   │◄────│ Agent Runtime │
│ (Stream) │     │   (Stream)   │     │  (Tool Exec)  │
└──────────┘     └──────────────┘     └───────┬───────┘
                                              │
                         ┌────────────────────┼────────────────────┐
                         │                    │                    │
                   ┌─────▼─────┐       ┌──────▼──────┐      ┌─────▼─────┐
                   │  Memory   │       │  gRPC Call  │      │  Provider │
                   │  (Local)  │       │  (Sidecar)  │      │  (LLM)    │
                   └───────────┘       └─────────────┘      └───────────┘
```

### Flow Steps

1. **Receive** → WebSocket message arrives at Gateway
2. **Authenticate** → JWT validation + origin check
3. **Enrich** → Load core memory, inject context
4. **Execute** → Agent runtime processes with tools
5. **Stream** → Token-by-token response to client

---

<!--
Speaker Notes: The fallback chain is critical for production reliability. Explain the cooldown mechanism and how it prevents cascading failures.
-->

## 🔗 Provider Chain with Fallback

### Fallback Architecture

```rust
// crates/providers/src/fallback.rs
pub struct FallbackChain {
    providers: Vec<Box<dyn LlmProvider>>,
    cooldowns: HashMap<String, Instant>,
    health_checks: HashMap<String, ProviderHealth>,
}

impl FallbackChain {
    pub async fn complete(&self, request: Request) -> Result<Response> {
        for provider in self.available_providers() {
            match provider.complete(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    self.record_failure(&provider.name(), e);
                    continue; // Try next provider
                }
            }
        }
        Err(Error::AllProvidersFailed)
    }
}
```

### Provider Priority

```
Primary:    Anthropic (Claude) ─────┐
                                     ├──► Automatic Fallback
Secondary:  OpenAI (GPT-4) ─────────┤
                                     │
Tertiary:   OpenRouter (400+ models)┘

Local:      Ollama (offline mode)
```

---

<!--
Speaker Notes: Deep dive into the memory architecture. This is where OpenRustClaw really differentiates from other frameworks.
-->

## 🧠 Memory System Architecture

### 3-Tier Design

```rust
// crates/memory/src/lib.rs
pub struct MemoryManager {
    core: CoreMemory,        // ~500 tokens, always loaded
    recall: RecallMemory,    // Searchable, on-demand
    archive: ArchiveMemory,  // Consolidated summaries
}

impl MemoryManager {
    pub async fn search(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
        // Hybrid search: BM25 + Vector + MMR
        let bm25_results = self.recall.bm25_search(query, limit * 2);
        let vector_results = self.recall.vector_search(query, limit * 2);
        
        // Merge and diversify with MMR
        hybrid_merge(bm25_results, vector_results)
            .diversify(MMR_lambda)
            .temporal_decay()
            .take(limit)
            .collect()
    }
}
```

### Storage Layer

```
SQLite Database
├── core_memory (key-value, always cached)
├── memory_entries (content + metadata)
├── memory_fts (FTS5 full-text index)
├── memory_vectors (libSQL vector embeddings)
└── memory_archive (consolidated summaries)
```

---

<!--
Speaker Notes: Continue with the search algorithm details. The hybrid approach gives both semantic and lexical matching.
-->

## 🔍 Hybrid Search Algorithm

### BM25 + Vector + MMR

```python
# Pseudocode for hybrid search
def hybrid_search(query: str, k: int = 10) -> List[MemoryEntry]:
    # 1. Lexical search (exact matches)
    bm25_scores = bm25_search(query, top_k=k*2)
    
    # 2. Semantic search (meaning similarity)
    query_embedding = embed(query)
    vector_scores = vector_search(query_embedding, top_k=k*2)
    
    # 3. Score fusion (Reciprocal Rank Fusion)
    fused = rrf_fusion(bm25_scores, vector_scores)
    
    # 4. Diversity with MMR
    results = []
    candidates = fused
    while len(results) < k and candidates:
        # Max Marginal Relevance
        best = max(candidates, 
                   key=lambda d: lambda_param * relevance(d) 
                               - (1-lambda_param) * max_sim(d, results))
        results.append(best)
        candidates.remove(best)
    
    # 5. Temporal decay (recent = more relevant)
    return apply_temporal_decay(results)
```

### Performance

- **Query latency**: <3ms (p99)
- **Embedding generation**: ~50ms (batched)
- **Index update**: Async, non-blocking

---

<!--
Speaker Notes: Security is layered. Each layer provides defense in depth. If one fails, others protect the system.
-->

## 🔒 Security Layers

### Layer 1: Transport & Auth

```rust
// crates/security/src/origin_check.rs
pub async fn validate_origin(
    request: &Request,
    allowed_origins: &[String],
) -> Result<(), SecurityError> {
    let origin = request.headers()
        .get("origin")
        .ok_or(SecurityError::MissingOrigin)?;
    
    if !allowed_origins.contains(&origin.to_str()?) {
        // Log and reject
        audit_log.record(AuditEvent::InvalidOrigin { origin });
        return Err(SecurityError::InvalidOrigin);
    }
    Ok(())
}
```

### Layer 2: Prompt Injection Defense

```
┌───────────────────────────────────────────────┐
│  Defense Stack                                │
│                                               │
│  1. Input Sanitization                        │
│     • HTML/JS stripping                       │
│     • Known pattern matching                  │
│                                               │
│  2. Sandwich Defense                          │
│     • User input wrapped in delimiters        │
│     • Clear separation from system            │
│                                               │
│  3. Canary Tokens                             │
│     • Hidden markers in prompts               │
│     • Detection of prompt leakage             │
│                                               │
│  4. Classification                            │
│     • ML-based injection detection            │
│     • Real-time scoring                       │
└───────────────────────────────────────────────┘
```

---

<!--
Speaker Notes: Explain skill verification and sandboxing. This is how OpenRustClaw handles untrusted code safely.
-->

## 🔐 Skill Verification & Sandboxing

### Ed25519 Signature Verification

```rust
// crates/security/src/skill_verifier.rs
pub struct SkillVerifier {
    trusted_keys: HashSet<Ed25519PublicKey>,
}

impl SkillVerifier {
    pub fn verify(&self, skill: &Skill) -> Result<(), VerifyError> {
        // Check manifest signature
        let manifest_json = serde_json::to_string(&skill.manifest)?;
        let signature = Ed25519Signature::from_bytes(&skill.signature)?;
        
        for key in &self.trusted_keys {
            if key.verify(&manifest_json, &signature).is_ok() {
                return Ok(());
            }
        }
        Err(VerifyError::UntrustedSignature)
    }
}
```

### WASM Sandboxing

```rust
// crates/skills/src/wasm_runtime.rs
pub struct WasmSandbox {
    engine: wasmtime::Engine,
    store: wasmtime::Store<SandboxState>,
    limits: ResourceLimits,
}

impl WasmSandbox {
    pub fn instantiate(&mut self, wasm_bytes: &[u8]) -> Result<Instance> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        
        // Apply resource limits
        self.store.limiter(|state| &mut state.limits);
        
        // Pre-instantiate with restricted imports
        let instance = Instance::new(&mut self.store, &module, &[])?;
        Ok(instance)
    }
}
```

---

<!--
Speaker Notes: Cover deployment options. OpenRustClaw is designed to be flexible—single binary, Docker, or cloud-native.
-->

## 🚀 Deployment Options

### Option 1: Single Binary

```bash
# Build standalone binary
cargo build --release --bin openrustclaw

# Run everything
./openrustclaw start
```

**Pros**: Simple, minimal dependencies, edge deployment  
**Cons**: Single process, vertical scaling only

### Option 2: Docker Compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  openrustclaw:
    image: openrustclaw:latest
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - DATABASE_URL=sqlite:/data/openrustclaw.db
    volumes:
      - ./data:/data
    ports:
      - "3000:3000"
  
  sidecar:
    image: openrustclaw-sidecar:latest
    environment:
      - LANGSMITH_API_KEY=${LANGSMITH_API_KEY}
```

---

<!--
Speaker Notes: Kubernetes deployment for production. Mention the Helm chart availability.
-->

## 🚀 Deployment Options (Continued)

### Option 3: Kubernetes

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: openrustclaw
spec:
  replicas: 3  # Horizontal scaling
  selector:
    matchLabels:
      app: openrustclaw
  template:
    spec:
      containers:
        - name: gateway
          image: openrustclaw/gateway:v1.0
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
        - name: sidecar
          image: openrustclaw/sidecar:v1.0
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: openrustclaw-pvc
```

### Scaling Characteristics

| Deployment | Scale | Best For |
|------------|-------|----------|
| Single Binary | 1 instance | Development, edge |
| Docker Compose | 1 host | Small teams, demos |
| Kubernetes | 100+ pods | Production, enterprise |

---

<!--
Speaker Notes: Summary slide. Emphasize the modular design and how it enables future extensibility.
-->

## 🎯 Architecture Summary

### Key Design Decisions

1. **Hybrid Rust/Python**
   - Rust for performance-critical I/O and security
   - Python for AI orchestration and workflow complexity

2. **Modular Crate Structure**
   - 14 focused crates with clear boundaries
   - Independent testing, versioning, deployment

3. **3-Tier Memory**
   - Core (~500 tokens) for speed
   - Recall for comprehensive search
   - Archive for long-term consolidation

4. **Defense in Depth**
   - Multiple security layers
   - No single point of failure
   - Audit everything

### Future Extensibility

```
New Provider?   → Add to crates/providers/
New Channel?    → Add to crates/channels/
New Security?   → Add to crates/security/
New Workflow?   → Add to sidecar/src/workflows/
```

---

## 📚 Additional Resources

### Code Locations

| Component | Path |
|-----------|------|
| Rust Core | `crates/` |
| Python Sidecar | `sidecar/src/` |
| Protocol Buffers | `proto/` |
| Database Migrations | `crates/db/migrations/` |
| CLI Commands | `crates/cli/src/commands/` |

### Documentation

- 📖 Architecture Decision Records: `docs/src/architecture/`
- 🔐 Security Guide: `docs/src/security/`
- 🧠 Memory System: `docs/src/memory/`

### Next Steps

1. 📊 See "Memory System" deck for RAG details
2. 🔒 See "Security Features" deck for hardening guide
3. 🔌 See "Provider Ecosystem" deck for LLM integration
