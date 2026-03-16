---
marp: true
theme: default
paginate: true
header: 'OpenRustClaw'
footer: '2026'
---

# OpenRustClaw

### A Hybrid Rust + Python AI Agent Framework

Combining Rust's performance and safety with LangGraph's AI orchestration

---

## The Problem: OpenClaw's Gaps

| Issue | Severity |
|-------|----------|
| CVE-2026-25253: Unauthenticated WebSocket (CVSS 8.8) | Critical |
| MEMORY.md injected every turn (~15-20K tokens, 93.5% waste) | High |
| Prompt injection defense: only 17% success rate | High |
| 1000+ malicious skills in marketplace (no verification) | High |
| Synchronous memory indexing blocks startup | Medium |
| Basic cron scheduler, missed reminders | Medium |
| Single-writer SQLite, no session isolation | Medium |

---

## The Solution: Hybrid Architecture

```
┌──────────────────────────────────────────────┐
│              1. RUST CORE (14 crates)         │
│                                                │
│  Gateway (Axum WS)  ←→  Agent Runtime         │
│  Auth / Security     ←→  Tool Execution       │
│  Memory Layer        ←→  WASM Sandbox          │
│  Scheduler Worker    ←→  CLI / TUI             │
│                                                │
│  SQLite (sqlx async | libSQL vectors)          │
└────────────────────┬─────────────────────────┘
                     │ gRPC (tonic)
┌────────────────────▼─────────────────────────┐
│          2. PYTHON LANGGRAPH SIDECAR          │
│                                                │
│  Agent Orchestration  ←→  RAG Pipeline         │
│  Memory Maintenance   ←→  Scheduled Workflows  │
└────────────────────┬─────────────────────────┘
                     │ REST API
┌────────────────────▼─────────────────────────┐
│          3. OBSERVABILITY (LangSmith)         │
│                                                │
│  Traces  ←→  Evals  ←→  Metrics               │
└──────────────────────────────────────────────┘
```

---

## Why Hybrid? Why Not Pure Rust?

- **LangGraph** is Python/JS only — no Rust runtime exists
- **LangSmith** is framework-agnostic via REST API
- Reimplementing LangGraph from scratch wastes effort

### The Clean Split

| Rust Owns | Python Owns |
|-----------|-------------|
| Performance, safety, persistence | AI orchestration, graph flows |
| Gateway, tools, scheduling, security | LangGraph workflows, RAG |
| SQLite, embeddings, CLI | LangSmith integration, evals |

---

## LLM Providers (Native SDKs)

| Provider | SDK | Key Features |
|----------|-----|-------------|
| **Anthropic** | `anthropic_rust` | Strict tool use, fine-grained streaming, Batch API (50% savings) |
| **OpenAI** | `async-openai` | Responses API + Chat Completions fallback, strict tools |
| **OpenRouter** | `openrouter_api` | 400+ models, auto-routing, zero logging |
| **Ollama** | Raw HTTP | Local models, buffered tool_call delta fix |

**Fallback Chain**: Anthropic -> OpenAI -> OpenRouter (configurable)

---

## Model Context Protocol (MCP)

### OpenRustClaw is both MCP Client AND Server

**MCP Client** — Connect to 18,000+ existing MCP servers
- Filesystem, databases, web search, APIs

**MCP Server** — Expose tools to external clients
- Claude Desktop, Claude Code, Cursor

**Tool Translation** — Automatic schema conversion
- MCP <-> Anthropic <-> OpenAI formats

---

## 3-Tier Memory (Database-First, Recall-Only)

```
┌────────────────────────────────────────────┐
│  Tier 1: CORE MEMORY (~500 tokens)          │
│  Always in prompt. User identity, prefs.    │
│  Storage: core_memory table (<=20 entries)  │
└──────────────────┬─────────────────────────┘
                   │ memory_search (on-demand)
┌──────────────────▼─────────────────────────┐
│  Tier 2: RECALL MEMORY (never injected)     │
│  Semantic, episodic, procedural memories.   │
│  Hybrid BM25 + vector + MMR search (<3ms)  │
└──────────────────┬─────────────────────────┘
                   │ consolidation workflow
┌──────────────────▼─────────────────────────┐
│  Tier 3: ARCHIVE (compressed summaries)     │
│  Monthly digests, merged fact clusters.     │
└────────────────────────────────────────────┘
```

**Result**: ~2K token system prompt vs OpenClaw's ~15-20K

---

## Security Hardening

| OpenClaw Gap | OpenRustClaw Fix |
|-------------|-----------------|
| Unauthenticated WebSocket | Mandatory origin validation + JWT auth |
| 17% prompt injection defense | Multi-layer: sandwich, canary, classification |
| No skill verification | Ed25519 cryptographic signatures |
| No sandboxing | WASM sandbox (wasmtime) |
| No session isolation | Per-session filesystem namespaces |
| Unbounded memory growth | TTL + consolidation + archival |

---

## Durable Scheduling (No Cron)

- **LangGraph workflow-based** execution
- **Idempotency keys** prevent double-execution
- **Lease/lock semantics** with automatic crash recovery
- **Exponential backoff** retries with dead-letter queue
- **Timezone-safe** via chrono-tz

```
Rust Worker Loop → Poll SQLite → Acquire Lease
→ Dispatch to LangGraph → Record Result → Next
```

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `openrustclaw start` | Start gateway + sidecar |
| `openrustclaw chat` | Interactive chat |
| `openrustclaw models list` | List LLM models |
| `openrustclaw skills list` | List installed skills |
| `openrustclaw schedule list` | List scheduled jobs |
| `openrustclaw security audit` | Run security audit |
| `openrustclaw memory stats` | Memory statistics |
| `openrustclaw doctor` | Run diagnostics |
| `openrustclaw cursor setup` | Cursor IDE integration |
| `openrustclaw mcp-server` | Start MCP server |

---

## Project Structure

```
OpenRustClaw/
├── Cargo.toml              # Workspace (14 crates)
├── crates/
│   ├── core/               # Types, traits, errors
│   ├── db/                 # SQLite (12 migrations)
│   ├── memory/             # 3-tier memory, RAG
│   ├── providers/          # 4 LLM providers
│   ├── mcp/                # MCP client + server
│   ├── agent/              # Runtime, tools
│   ├── gateway/            # Axum WebSocket
│   ├── scheduler/          # Durable scheduler
│   ├── security/           # 6 security modules
│   ├── langbridge/         # gRPC bridge
│   ├── observability/      # LangSmith + tracing
│   └── cli/                # 10 CLI commands
├── sidecar/                # Python LangGraph
├── proto/                  # Protobuf definitions
└── docs/                   # mdBook documentation
```

---

## Roadmap

### v1 (Current)
- 14-crate Rust workspace
- 4 LLM providers with fallback chain
- MCP client + server
- 3-tier recall-only memory
- Durable scheduler
- Security hardening

### v2
- Telegram, Discord, Slack channels
- Browser automation
- Gemini provider
- Cursor ACP integration

### v3+
- Canvas/A2UI visual workspace
- Device integration
- Community plugins

---

## Getting Started

```bash
# Clone
git clone https://github.com/hprincivil/OpenRustClaw.git
cd OpenRustClaw

# Build
cargo build --workspace

# Configure
cp .env.example .env
# Edit .env with your API keys

# Run
cargo run --bin openrustclaw -- start
cargo run --bin openrustclaw -- chat --provider anthropic
```

---

## Links

- **Repository**: github.com/hprincivil/OpenRustClaw
- **Documentation**: mdBook at `docs/`
- **License**: MIT

### Built With

Rust + Tokio + Axum + SQLite + LangGraph + LangSmith + MCP

---

<!-- _class: lead -->

# Thank You

**OpenRustClaw** — Performance meets Intelligence

github.com/hprincivil/OpenRustClaw
