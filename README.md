# OpenRustClaw

**A hybrid Rust + Python AI agent framework** that combines Rust's performance and safety with LangGraph's AI orchestration capabilities.

OpenRustClaw is a ground-up reimagining of the [OpenClaw](https://github.com/openclaw) AI agent framework, addressing its documented security vulnerabilities, performance bottlenecks, and architectural limitations while preserving feature parity.

---

## Why OpenRustClaw?

| Problem in OpenClaw | OpenRustClaw Fix |
|---------------------|------------------|
| CVE-2026-25253: Unauthenticated WebSocket access (CVSS 8.8) | Mandatory origin validation + token auth on ALL connections |
| MEMORY.md injected every turn (~15-20K tokens, 93.5% waste) | 3-tier recall-only memory: Core (~500 tokens) + on-demand search |
| Prompt injection: only 17% defense rate | Multi-layer: sandwich defense, canary tokens, classification |
| 1000+ malicious skills in marketplace | Ed25519 cryptographic signatures + WASM sandboxing |
| Synchronous memory indexing blocks startup | Fully async embedding pipeline with bounded concurrency |
| Basic cron scheduler, missed reminders | Durable scheduler: idempotency, leases, dead-letter, timezone-safe |
| Single-writer SQLite, no isolation | WAL mode + per-session filesystem namespaces |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    1. RUST CORE (14 crates)                  │
│                                                               │
│  Gateway (Axum WS)  ←→  Agent Runtime  ←→  Tool Execution   │
│  Auth / Pairing      ←→  Context Mgr    ←→  WASM Sandbox    │
│  Session Manager     ←→  Memory Layer   ←→  Security Layer  │
│  Scheduler Worker    ←→  Event Bus      ←→  CLI / TUI       │
│                                                               │
│  SQLite Persistence (sqlx async | libSQL vectors | rusqlite) │
└──────────────────────────┬──────────────────────────────────┘
                           │ gRPC (tonic)
┌──────────────────────────▼──────────────────────────────────┐
│                 2. PYTHON LANGGRAPH SIDECAR                  │
│                                                               │
│  Agent Orchestration  ←→  Memory Maintenance                 │
│  RAG Pipeline         ←→  Reminder Workflows                 │
│  Scheduled Execution  ←→  Human Approval                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ LangSmith REST API
┌──────────────────────────▼──────────────────────────────────┐
│              3. OBSERVABILITY (LangSmith)                     │
│                                                               │
│  Hierarchical Traces  ←→  Offline Eval Datasets              │
│  Token/Cost Metrics   ←→  Online Evaluators                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Features

### LLM Providers (Native SDKs)
- **Anthropic** — Messages API with strict tool use, fine-grained streaming, Batch API (50% savings)
- **OpenAI** — Responses API (primary) + Chat Completions (fallback), strict tools
- **OpenRouter** — 400+ models, auto-routing (price/throughput/quality), zero logging
- **Ollama** — Local models with buffered tool_call delta fix
- **Fallback Chain** — Automatic provider failover with per-key cooldowns

### Model Context Protocol (MCP)
- **MCP Client** — Connect to 18,000+ existing MCP servers
- **MCP Server** — Expose OpenRustClaw tools to Claude Desktop, Claude Code, and Cursor
- **Tool Translation** — Automatic schema conversion between MCP, Anthropic, and OpenAI formats

### 3-Tier Memory System (Database-First, Recall-Only)
- **Core Memory** (~500 tokens) — Always in prompt. User identity, project context, preferences
- **Recall Memory** — Searchable via `memory_search` tool. Never auto-injected
- **Archive** — Consolidated long-term summaries. Auto-maintained by LangGraph workflow
- **Hybrid Search** — BM25 + vector similarity + MMR diversity + temporal decay (<3ms)
- **Zero MD File Sprawl** — SQLite is the canonical store, not markdown files

### Durable Scheduling (No Cron)
- LangGraph workflow-based execution
- Idempotency keys prevent double-execution
- Lease/lock semantics with automatic crash recovery
- Exponential backoff retries with dead-letter queue
- Timezone-safe via chrono-tz

### Security Hardening
- Mandatory WebSocket origin validation (fixes CVE-2026-25253)
- JWT authentication on all connections
- Multi-layer prompt injection defense
- Ed25519 cryptographic skill verification
- WASM sandboxing for untrusted skills (wasmtime)
- Per-session filesystem isolation

### Observability
- LangSmith REST API integration (framework-agnostic)
- Distributed tracing (OpenTelemetry)
- Token usage, latency, and cost tracking
- Offline eval datasets for memory recall, RAG accuracy, tool use

---

## Project Structure

```
OpenRustClaw/
├── Cargo.toml                  # Workspace root (14 crates)
├── crates/
│   ├── core/                   # Types, traits, errors, config
│   ├── db/                     # SQLite persistence (12 migrations)
│   ├── memory/                 # 3-tier memory, RAG, context manager
│   ├── providers/              # Anthropic, OpenAI, OpenRouter, Ollama
│   ├── mcp/                    # MCP client + server
│   ├── agent/                  # Runtime, tool registry, streaming
│   ├── gateway/                # Axum WebSocket server
│   ├── channels/               # WebChat (v1)
│   ├── skills/                 # SKILL.md, WASM sandbox, marketplace
│   ├── scheduler/              # Durable scheduler (no cron)
│   ├── security/               # Auth, origin check, injection defense
│   ├── langbridge/             # gRPC bridge to Python sidecar
│   ├── observability/          # LangSmith, tracing, metrics
│   └── cli/                    # 10 CLI commands
├── sidecar/                    # Python LangGraph sidecar
│   ├── src/
│   │   ├── server.py           # gRPC server
│   │   ├── workflows/          # LangGraph StateGraph definitions
│   │   └── evaluators/         # Offline eval datasets
│   └── pyproject.toml
├── proto/                      # Shared protobuf definitions
├── docs/                       # mdBook documentation
├── slides/                     # Marp presentation decks
├── config/                     # Default configuration
├── .cursor/rules/              # Cursor IDE integration (5 MDC files)
└── CLAUDE.md                   # Claude Code project instructions
```

---

## Getting Started

### Prerequisites

- **Rust** 1.85+ (stable)
- **Python** 3.11+ (for sidecar)
- **protoc** (Protocol Buffers compiler)

### Build

```bash
# Clone the repository
git clone https://github.com/hprincivil/OpenRustClaw.git
cd OpenRustClaw

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run the CLI
cargo run --bin openrustclaw -- --help
```

### Configuration

```bash
# Copy the example environment file
cp .env.example .env

# Edit with your API keys
# ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# OPENROUTER_API_KEY=sk-or-...
```

### Quick Start

```bash
# Start the gateway and sidecar
cargo run --bin openrustclaw -- start

# Interactive chat
cargo run --bin openrustclaw -- chat --provider anthropic

# Run diagnostics
cargo run --bin openrustclaw -- doctor

# Set up Cursor IDE integration
cargo run --bin openrustclaw -- cursor setup
```

### Docker Deployment

```bash
# Build and run with Docker
docker build -t openrustclaw .
docker run -p 18789:18789 -e ANTHROPIC_API_KEY=sk-ant-... openrustclaw

# Or use Docker Compose
cp .env.docker .env
# Edit .env with your API keys
docker-compose up -d

# View logs
docker-compose logs -f

# See docs/src/deployment/docker.md for complete guide
```

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `openrustclaw start` | Start gateway server + Python sidecar |
| `openrustclaw chat` | Interactive chat with the agent |
| `openrustclaw models list` | List available LLM models |
| `openrustclaw skills list` | List installed skills |
| `openrustclaw schedule list` | List scheduled jobs |
| `openrustclaw security audit` | Run security audit |
| `openrustclaw security generate-keys` | Generate Ed25519 keypair |
| `openrustclaw memory export` | Export memory to markdown |
| `openrustclaw memory stats` | Show memory statistics |
| `openrustclaw doctor` | Run diagnostics |
| `openrustclaw cursor setup` | Set up Cursor IDE integration |
| `openrustclaw mcp-server` | Start MCP server for external clients |

---

## Cursor IDE Integration

OpenRustClaw provides first-class Cursor support:

```bash
# Auto-generate .cursor/mcp.json and .cursor/rules/
openrustclaw cursor setup
```

This sets up:
- **5 MDC rule files** — Rust conventions, crate architecture, provider patterns, memory patterns, testing patterns
- **MCP server connection** — Access OpenRustClaw tools directly from Cursor
- **CLAUDE.md** — Project instructions for Claude Code

---

## Documentation

```bash
# Build mdBook documentation
mdbook build docs/

# Generate API docs
cargo doc --workspace --no-deps --open

# Build presentation slides (requires marp-cli)
npx @marp-team/marp-cli slides/openrustclaw-overview.md -o slides/output/overview.pdf
```

---

## Provider SDK Compliance

| Provider | SDK | Required Headers | TOS |
|----------|-----|-----------------|-----|
| Anthropic | `anthropic_rust` (planned) | `x-api-key`, `anthropic-version`, `content-type` | No competing AI products |
| OpenAI | `async-openai` (planned) | `Authorization: Bearer` | No competing AI models |
| OpenRouter | `openrouter_api` (planned) | `Authorization: Bearer`, `HTTP-Referer` | Zero logging default |
| Ollama | Raw HTTP | None (local) | N/A |

---

## Database Schema

12 SQLite migrations managing:
- **sessions** / **conversations** — Session and message history
- **memory_entries** / **memory_fts** / **memory_vectors** — 3-tier memory with FTS5 + vector search
- **core_memory** — Always-loaded key-value memory (~500 tokens)
- **memory_archive** — Consolidated long-term summaries
- **skills** — Skill registry with verification status
- **audit_log** — Security event audit trail
- **scheduled_jobs** / **job_runs** / **dead_letter_queue** — Durable scheduler
- **workflow_checkpoints** — LangGraph state persistence

---

## Roadmap

### v1.0 - Complete ✅

All v1 features are now implemented:

- [x] 15-crate Rust workspace (added mcp2cli)
- [x] 4 LLM providers with fallback chain
- [x] Provider streaming (Anthropic, OpenAI, OpenRouter, Ollama)
- [x] MCP client + server
- [x] mcp2cli integration (96-99% token savings)
- [x] 3-tier recall-only memory (Core/Recall/Archive)
- [x] Memory tools wired to SQLite backends
- [x] Durable scheduler (idempotent, with retry/lease)
- [x] Security hardening (6 modules)
- [x] CLI with 11 commands (+ mcp2cli)
- [x] Python sidecar with LangGraph workflows
- [x] Integration tests (160+ tests)

### v1.1 - Polish & Hardening (Next)
- [ ] Production-ready error handling review
- [ ] Performance benchmarks and optimization
- [ ] End-to-end testing with real providers
- [x] Docker deployment configuration
- [ ] Metrics and observability dashboards
- [ ] Documentation complete (API reference)

### v2.0 - Ecosystem Expansion ✅ COMPLETE

All v2.0 features are now implemented:

- [x] **Native SDK crates** - `anthropic_rust`, `async_openai`, `openrouter_api`
- [x] **Telegram Bot** - Full bot API integration
- [x] **Discord Bot** - Gateway + slash commands
- [x] **Slack App** - Socket Mode + HTTP mode
- [x] **Browser Automation** - Playwright + CDP backends
- [x] **Gemini Provider** - Google Gemini API support
- [x] **Cursor ACP** - Deep IDE integration with code/terminal/git tools
- [x] **Multi-node Distributed** - Raft consensus, horizontal scaling

### v2.1 - Platform Hardening (Next)
- [ ] Production deployment guides
- [ ] Kubernetes Helm charts
- [ ] Terraform modules
- [ ] AWS/GCP/Azure marketplace
- [ ] SOC 2 compliance documentation
- [ ] Enterprise SSO (OIDC/SAML)

### v3+ - Advanced Features
- [ ] Canvas/A2UI visual workspace
- [ ] Device integration (camera, screen, voice)
- [ ] Community channel plugins

### v2
- [ ] Native SDK crates (anthropic_rust, async-openai, openrouter_api)
- [ ] Telegram, Discord, Slack channels
- [ ] Browser automation (Playwright/CDP)
- [ ] Gemini provider
- [ ] Cursor ACP deep integration
- [ ] Multi-node distributed mode

### v3+
- [ ] Canvas/A2UI visual workspace
- [ ] Device integration (camera, screen, voice)
- [ ] Community channel plugins

---

## Contributing

See [docs/src/contributing/development.md](docs/src/contributing/development.md) for setup instructions.

```bash
# Run the full test suite
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt --all
```

---

## License

MIT License. See [LICENSE](LICENSE) for details.
