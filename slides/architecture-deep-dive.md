---
marp: true
theme: default
paginate: true
header: 'OpenRustClaw — Architecture Deep Dive'
footer: '2026'
---

# OpenRustClaw
## Architecture Deep Dive

A technical walkthrough of the hybrid Rust + Python AI agent framework

---

## Crate Dependency Graph

```
cli ──→ gateway ──→ agent ──→ providers
  │        │          │          │
  │        ▼          ▼          ▼
  │     security   memory      mcp
  │        │          │          │
  │        ▼          ▼          ▼
  ├──→ scheduler ──→ db ──→── core
  │        │
  └──→ langbridge ──→ observability
```

**Rule**: Dependencies flow downward. `core` has zero internal dependencies.

---

## Core Crate (`crates/core`)

### Key Traits

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: CompletionRequest)
        -> Result<CompletionResponse>;
    async fn stream(&self, req: CompletionRequest)
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>>>>>;
    fn model_id(&self) -> &str;
    fn provider_name(&self) -> &str;
    fn native_tool_format(&self) -> ToolFormat;
}
```

All providers implement this unified interface.

---

## Provider SDK Architecture

```
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│   Anthropic    │  │    OpenAI     │  │  OpenRouter   │
│                │  │               │  │               │
│ anthropic_rust │  │ async-openai  │  │ openrouter_api│
│ Messages API   │  │ Responses API │  │ 400+ models   │
│ Strict tools   │  │ Strict tools  │  │ Auto-routing  │
│ Batch (50%)    │  │ Built-in web  │  │ Zero logging  │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                   │
        ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────┐
│           Unified LlmProvider Trait                  │
│  + ProviderChain (fallback, cooldowns, key rotation) │
└─────────────────────────────────────────────────────┘
```

---

## Anthropic Integration Details

**SDK**: `anthropic_rust` (async, type-safe, streaming)

| Feature | Implementation |
|---------|---------------|
| Strict tool use | `strict: true` on tool definitions |
| Fine-grained streaming | `eager_input_streaming: true` |
| Batch API | 50% cost reduction for non-real-time ops |
| Token counting | Pre-flight context budget checks |

**Required Headers**:
- `x-api-key` — API key
- `anthropic-version` — `2023-06-01`
- `content-type` — `application/json`

---

## OpenAI Integration Details

**SDK**: `async-openai` (comprehensive, actively maintained)

### Responses API (Primary)
- Server-side conversation state
- Built-in tools: web search, file search, code interpreter
- Native agentic loop (multiple tools per request)
- 40-80% better cache utilization

### Chat Completions (Fallback)
- Lightweight, stateless
- Broad model compatibility

---

## OpenRouter Integration Details

**SDK**: `openrouter_api` (type-state builder, auto key zeroing)

### Route Strategies
```rust
pub enum RouteStrategy {
    Price,       // :floor — cheapest provider
    Throughput,  // :nitro — fastest provider
    Quality,     // default — best quality
    WebSearch,   // :online — with web search
}
```

- 400+ models through one API
- Auto-fallback on provider failure
- Zero logging by default
- API key auto-zeroed on drop (`zeroize` crate)

---

## Tool Format Translation

MCP, Anthropic, and OpenAI each have different tool schemas:

```rust
pub enum ToolFormat {
    Mcp,        // inputSchema, JSON Schema 2020-12
    Anthropic,  // input_schema, strict mode
    OpenAi,     // function.parameters, additionalProperties:false
}

pub fn translate_tool(
    tool: &ToolDefinition,
    target: ToolFormat,
) -> serde_json::Value;
```

**Key constraints**:
- MCP: no `$ref` pointers (must be self-contained)
- OpenAI strict: `additionalProperties: false` on every object
- Anthropic strict: `strict: true` at tool level

---

## Fallback Chain

```rust
pub struct ProviderChain {
    providers: Vec<(Box<dyn LlmProvider>, ProviderConfig)>,
    cooldowns: DashMap<String, Instant>,
}
```

**Behavior**:
1. Try providers in order (default: Anthropic -> OpenAI -> OpenRouter)
2. On 429/500: cooldown provider for 60s, try next
3. On success: return result + provider name
4. On all exhausted: `ProviderError::AllProvidersExhausted`

---

## MCP Architecture

### Dual Role: Client AND Server

```
External MCP Servers          OpenRustClaw          External Clients
(filesystem, DB, etc.)        MCP Layer             (Claude Desktop, etc.)

  ┌──────────┐            ┌──────────────┐         ┌──────────┐
  │ Server A │◄──stdio──►│  MCP Client  │         │ Claude   │
  └──────────┘            │              │         │ Desktop  │
  ┌──────────┐            │  MCP Server  │◄─stdio─►│          │
  │ Server B │◄──stdio──►│              │         └──────────┘
  └──────────┘            └──────────────┘         ┌──────────┐
                                                    │ Cursor   │
                                                    └──────────┘
```

---

## MCP Client

```rust
pub struct McpClient {
    transport: StdioTransport,  // JSON-RPC over stdio
}

impl McpClient {
    // Connect to external MCP server
    async fn connect(cmd: &str, args: &[&str]) -> Result<Self>;

    // Discover available tools
    async fn discover_tools(&self) -> Result<Vec<McpToolDef>>;

    // Invoke a tool
    async fn call_tool(&self, name: &str, args: Value)
        -> Result<ToolOutput>;
}
```

Access 18,000+ existing MCP servers.

---

## MCP Server

```rust
pub struct McpServer {
    tool_registry: Arc<ToolRegistry>,
}

// Exposed tools:
// - memory_search: Search agent memory
// - memory_store: Store information
// - schedule_task: Create scheduled workflow
// - rag_search: Search documentation corpus
// - list_skills: List available skills
// - run_security_audit: Security check
```

Exposes OpenRustClaw tools to Claude Desktop, Claude Code, Cursor.

---

## Memory Architecture: 3 Tiers

### The Problem with OpenClaw

| OpenClaw | Tokens | Waste |
|----------|--------|-------|
| MEMORY.md injected every turn | ~15-20K | 93.5% |
| OpenRustClaw core memory | ~500 | 0% |

### The Fix: Recall-Only

- **Core Memory**: ~500 tokens, always in prompt (identity, prefs)
- **Recall Memory**: Searchable via `memory_search` tool, NEVER injected
- **Archive**: Consolidated long-term summaries

---

## Memory Write Pipeline

```
New Memory
    │
    ▼
┌─────────────────────┐
│  1. Content Hash     │  SHA-256 deduplication
│  2. Dedupe Check     │  Cosine > 0.92 = merge
│  3. Score Importance  │  1.0 explicit ... 0.5 background
│  4. Score Confidence  │  Source reliability
│  5. Set TTL           │  Episodic: 90d, Semantic: none
│  6. Async Embed       │  Bounded semaphore (max 4)
│  7. Store             │  memory_entries + memory_vectors
└─────────────────────┘
```

---

## Hybrid Search (<3ms)

```
Query
  │
  ├──► BM25 (FTS5)  ──────┐
  │                         │
  ├──► Vector (libSQL) ────┤──► Reciprocal Rank Fusion
  │                         │
  └──► Metadata filter ────┘
                │
                ▼
         MMR Diversity
                │
                ▼
         Temporal Decay
                │
                ▼
         Top-K Results
```

---

## Context Window Manager

### Write-Select-Compress-Isolate

**1. WRITE** — Build lean system prompt (~2K tokens)
- Core memory (~500 tokens)
- Tool schemas (names + descriptions only)
- Runtime metadata (~50 tokens)

**2. SELECT** — Recent N conversation turns

**3. COMPRESS** — At 85% capacity
- Summarize older turns
- Replace large tool outputs with summaries

**4. ISOLATE** — For overflow
- Spawn sub-agent with focused context

---

## SQLite Triple-Driver Strategy

| Driver | Role | Layer |
|--------|------|-------|
| **sqlx** | Async. Sessions, conversations, skills, audit, jobs. Compile-time checked. | General persistence |
| **libSQL** | Vector operations. Native vector search. Future Turso cloud. | Embeddings |
| **rusqlite** | Sync fallback. CLI queries, migrations, diagnostics. | CLI |

**All share** the same WAL-mode SQLite database file.

**Rule**: sqlx owns non-vector writes, libSQL owns vector writes, rusqlite is read-only except migrations.

---

## Database Schema (12 Migrations)

```
sessions ← conversations
memory_entries ← memory_fts (FTS5)
               ← memory_vectors (libSQL)
core_memory
memory_archive
skills
audit_log
scheduled_jobs ← job_runs
               ← dead_letter_queue
workflow_checkpoints
```

---

## Scheduler Architecture

### Rust-Owned Worker Loop (No Cron)

```rust
loop {
    // 1. Poll: due jobs WHERE next_run_at <= now()
    // 2. Lease: atomic UPDATE (prevents double-exec)
    // 3. Idempotency: check job_runs for same key
    // 4. Dispatch: gRPC to LangGraph sidecar
    // 5. Success: record, calc next_run_at, release
    // 6. Failure: backoff retry or dead-letter
    tokio::time::sleep(poll_interval).await;
}
```

---

## Scheduler Properties

| Property | Implementation |
|----------|---------------|
| Idempotency | `idempotency_key` UNIQUE constraint |
| Lease/Lock | `lease_owner` + `lease_expires_at` columns |
| Retries | Exponential: `base * 2^count`, capped |
| Dead-letter | After max retries, full payload saved |
| Timezone | chrono-tz, all stored as UTC |
| Crash recovery | Expired leases auto-released |
| Durability | SQLite WAL mode survives restarts |

---

## Security Layers

```
Incoming Request
       │
       ▼
┌──────────────┐
│ Origin Check  │  CVE-2026-25253 fix
└──────┬───────┘
       ▼
┌──────────────┐
│ JWT Auth      │  Token validation
└──────┬───────┘
       ▼
┌──────────────┐
│ Input Sanitize│  Prompt injection defense
└──────┬───────┘
       ▼
┌──────────────┐
│ Skill Verify  │  Ed25519 signatures
└──────┬───────┘
       ▼
┌──────────────┐
│ WASM Sandbox  │  Capability enforcement
└──────┬───────┘
       ▼
┌──────────────┐
│ Isolation     │  Per-session namespaces
└──────────────┘
```

---

## Prompt Injection Defense

### Multi-Layer Approach

1. **Pattern Matching** — 36 known injection patterns
2. **Sandwich Defense** — System instructions at start AND end of prompt
3. **Canary Tokens** — Unique markers that trigger alert if echoed
4. **Content Classification** — Score text for injection likelihood

**OpenClaw**: 17% defense rate
**OpenRustClaw**: Multi-layer with canary detection alerts

---

## gRPC Bridge (`crates/langbridge`)

### Rust <-> Python Communication

```protobuf
service OrchestrationService {
    rpc ExecuteWorkflow(WorkflowRequest)
        returns (WorkflowResponse);
    rpc ExecuteWorkflowStream(WorkflowRequest)
        returns (stream WorkflowEvent);
    rpc GetWorkflowStatus(StatusRequest)
        returns (StatusResponse);
}
```

**Sidecar lifecycle**: Auto-start, health monitoring, restart on crash.

---

## Python Sidecar Workflows

| Workflow | Purpose |
|----------|---------|
| `agent_orchestrator.py` | Main agent graph (decide -> retrieve -> tool -> respond) |
| `memory_maintenance.py` | Expire, dedupe, consolidate, archive, reindex |
| `rag_pipeline.py` | Query -> retrieve -> grade -> web fallback -> generate |
| `reminder.py` | Parse reminder -> wait -> send message |
| `scheduler.py` | Generic scheduled workflow executor |

All defined as LangGraph `StateGraph` with checkpointing.

---

## Observability Stack

### LangSmith (Framework-Agnostic REST API)

```rust
pub struct LangSmithClient {
    api_key: String,
    project_name: String,
}

impl LangSmithClient {
    async fn trace_run(&self, run: &TraceRun) -> Result<()>;
    async fn update_run(&self, id: &str, ...) -> Result<()>;
    async fn log_feedback(&self, run_id: &str, ...) -> Result<()>;
}
```

**Traced**: Every LLM call, tool execution, RAG retrieval, scheduler job, memory operation.

---

## Offline Eval Datasets

| Dataset | Test Cases | Purpose |
|---------|-----------|---------|
| `reminder_timing` | 50+ | Timezone handling, time parsing |
| `memory_recall` | 100+ | Cross-session fact retrieval |
| `rag_accuracy` | 100+ | Retrieval relevance, citations |
| `tool_use` | 50+ | Tool selection, parameter extraction |

### Online Evaluators
- Memory recall accuracy
- RAG precision scoring
- Reminder timing accuracy
- Failure classification taxonomy

---

## Agent Runtime Loop

```rust
pub async fn process(&self, messages: Vec<Message>)
    -> Result<CompletionResponse>
{
    let mut current = messages;
    for _ in 0..MAX_ITERATIONS {
        let response = self.provider.complete(request).await?;

        if response.tool_calls.is_empty() {
            return Ok(response);  // Done
        }

        for call in &response.tool_calls {
            let output = self.tools.execute(
                &call.name, call.arguments.clone(), &ctx
            ).await?;
            current.push(Message::tool(output));
        }
    }
}
```

---

## Streaming with Ollama Fix

### The Problem
Ollama sends tool_call deltas across multiple chunks.
Naive forwarding breaks JSON parsing.

### The Fix
```rust
pub struct ToolCallBuffer {
    partial: HashMap<String, PartialToolCall>,
}

impl ToolCallBuffer {
    fn accumulate(&mut self, chunk: &StreamChunk);
    fn flush(&mut self) -> Vec<ToolCall>;  // On Done
}
```

Buffer deltas, emit only complete tool calls.

---

## Cursor IDE Integration

### Three Levels

| Level | What | Status |
|-------|------|--------|
| **Context** | `.cursor/rules/` (5 MDC files) | v1 |
| **Tools** | MCP server exposed to Cursor | v1 |
| **Agent** | Cursor CLI as subprocess tool | v2 |

```bash
# Auto-generate Cursor configuration
openrustclaw cursor setup
# Creates .cursor/mcp.json + .cursor/rules/
```

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Memory search latency | < 3ms (hybrid BM25 + vector) |
| System prompt size | ~2K tokens (vs 15-20K) |
| Concurrent sessions | 10K+ (Axum + tokio) |
| Embedding concurrency | 4 max (bounded semaphore) |
| Scheduler poll interval | 1s (configurable) |
| Provider failover | < 100ms cooldown check |

---

## Build & Test

```bash
# Build all 14 crates
cargo build --workspace

# Run tests (75+ tests)
cargo test --workspace

# Python sidecar tests
pytest sidecar/tests/

# Generate API documentation
cargo doc --workspace --no-deps --open

# Build documentation book
mdbook build docs/
```

---

<!-- _class: lead -->

# Architecture Summary

**Rust Core** — Performance, safety, persistence
**Python Sidecar** — AI orchestration, workflows
**LangSmith** — Observability, evals

**14 crates | 4 providers | 3-tier memory | MCP | Durable scheduling**

github.com/hprincivil/OpenRustClaw
