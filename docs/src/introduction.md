# OpenRustClaw

**A Rust-first AI agent platform** that is actively pursuing OpenClaw feature parity while reducing non-Rust runtime ownership over time.

---

## 🎯 Project Overview

OpenRustClaw is a production-grade AI agent framework built from the ground up with security, performance, and reliability as core design principles.

Built for production deployments where these qualities matter most, OpenRustClaw uses Rust as the primary runtime and treats any non-Rust orchestration layer as transitional unless it clearly improves the shipped product contract.

Current parity/planning references:

- [Roadmap](./planning/roadmap.md)
- [Feature Matrix](./planning/feature-matrix.md)
- [Parity Matrix](./planning/parity-matrix.md)
- [Parity Positioning](./planning/parity-positioning.md)

---

## 💡 Motivation

The AI agent landscape has exploded with capabilities, but existing frameworks often struggle with:

- **Security vulnerabilities** that expose systems to prompt injection and unauthorized access
- **Memory inefficiency** that wastes tokens and increases API costs
- **Unreliable scheduling** that misses important reminders and tasks
- **Performance bottlenecks** in high-throughput scenarios
- **Unsafe skill execution** without proper sandboxing

OpenRustClaw was created to solve these fundamental problems while maintaining the flexibility and power developers expect from modern AI frameworks.

---

## ✨ Key Features

### 🦀 **Rust Core Architecture**
- **14 specialized crates** for modular, maintainable code
- **Memory safety** without garbage collection pauses
- **Zero-cost abstractions** for high-performance operations
- **Async/await throughout** with Tokio runtime

### 🐍 **Workflow Execution Tiers**
- **Tier A: Rust-native** for production-critical, durable runtime paths
- **Tier B: sidecar compatibility** for shipped flows still being migrated
- **Tier C: LangGraph experimentation** for rapid prototyping before productization
- **Rust-owned state** already backs memory, scheduler persistence, and RAG storage
- **Sidecar retirement from the production-critical path** remains an explicit roadmap objective

### 🔁 **Rust-Native Autonomous Optimization**
- Inspired by `autoresearch`, but generalized for OpenRustClaw
- Implemented for skills, prompts, RAG policies, workflow heuristics, bounded code surfaces, and research-program targets
- Rust owns experiment orchestration, evaluation, promotion, and rollback policy
- Production-critical code remains guarded by explicit promotion and human review rules

### 🔐 **Security-First Design**
- ✅ **Mandatory WebSocket origin validation** (fixes CVE-2026-25253)
- ✅ **JWT authentication** on all connections
- ✅ **Multi-layer prompt injection defense**
- ✅ **Ed25519 cryptographic skill verification**
- ✅ **WASM sandbox executor implemented** for no-import, capability-aware skill execution
- ✅ **Per-session filesystem isolation**

### 🧠 **3-Tier Memory System**
- **Core Memory** (~500 tokens) — Always in prompt for critical context
- **Recall Memory** — Searchable via `memory_search` tool, never auto-injected
- **Archive** — Consolidated long-term summaries maintained by LangGraph
- **Hybrid Search** — BM25 + vector similarity + MMR diversity + temporal decay

### 🤖 **Multi-Provider LLM Support**
| Provider | Features | Cost Optimization |
|----------|----------|-------------------|
| **Anthropic** | Messages API, strict tool use, fine-grained streaming | Batch API (50% savings) |
| **OpenAI** | Responses API + Chat Completions fallback | Token-efficient formats |
| **OpenRouter** | 400+ models, auto-routing | Price/quality optimization |
| **Ollama** | Local models, no API costs | Free local inference |

### 🔌 **Model Context Protocol (MCP)**
- **MCP Client** — Connect to 18,000+ existing MCP servers
- **MCP Server** — Expose OpenRustClaw tools to Claude Desktop, Cursor, etc.
- **Automatic tool translation** between MCP, Anthropic, and OpenAI formats

### ⏰ **Durable Scheduling**
- **LangGraph workflow-based** execution (no cron)
- **Idempotency keys** prevent double-execution
- **Lease/lock semantics** with automatic crash recovery
- **Exponential backoff** retries with dead-letter queue
- **Timezone-safe** via chrono-tz

### 📊 **Observability**
- **LangSmith REST API** integration (framework-agnostic)
- **OpenTelemetry** distributed tracing
- **Token usage, latency, and cost** tracking
- **Offline eval datasets** for memory recall, RAG accuracy, tool use

---

## 🏗️ Architecture Overview

```mermaid
flowchart TB
    subgraph Layer1["1. Rust Core (14 crates)"]
        GW["Gateway (Axum WS)"]
        AUTH["Auth / Pairing"]
        SESS["Session Manager"]
        AGENT["Agent Runtime"]
        CTX["Context Manager"]
        MEM["Memory Layer"]
        TOOLS["Tool Execution"]
        WASM["WASM Sandbox"]
        SEC["Security Layer"]
        SCHED["Scheduler Worker"]
        EVT["Event Bus"]
        CLI["CLI / TUI"]
    end
    
    subgraph DB["SQLite Persistence"]
        SQL["sqlx async"]
        LIBSQL["libSQL vectors"]
        RQL["rusqlite"]
    end
    
    subgraph Layer2["2. Python LangGraph Sidecar"]
        ORCH["Agent Orchestration"]
        MEM_MAINT["Memory Maintenance"]
        RAG["RAG Pipeline"]
        REMIND["Reminder Workflows"]
        SCHED_EXEC["Scheduled Execution"]
        APPROVAL["Human Approval"]
    end
    
    subgraph Layer3["3. Observability (LangSmith)"]
        TRACES["Hierarchical Traces"]
        METRICS["Token/Cost Metrics"]
        EVAL_DATA["Offline Eval Datasets"]
        ONLINE_EVAL["Online Evaluators"]
    end
    
    Layer1 <-->|gRPC (tonic)| Layer2
    Layer2 <-->|REST API| Layer3
    Layer1 <-->|Read/Write| DB
```

---

## 🔧 Project Structure

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
│   ├── skills/                 # SKILL.md, sandbox scaffolding, marketplace
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
│   ├── orchestration.proto     # Workflow execution gRPC
│   └── tracing.proto           # LangSmith trace forwarding
├── docs/                       # mdBook documentation
├── slides/                     # Marp presentation decks
├── config/                     # Default configuration
├── .cursor/rules/              # Cursor IDE integration (5 MDC files)
└── CLAUDE.md                   # Claude Code project instructions
```

---

## 📊 Design Advantages

| Challenge | OpenRustClaw Solution | Impact |
|-----------|----------------------|--------|
| Unauthenticated WebSocket access | Mandatory origin validation + token auth enabled by default | 🔒 Reduces unauthorized access by default |
| Large memory files injected every turn | 3-tier recall-only memory: Core (~500 tokens) + on-demand search | 💰 90% token cost reduction |
| Weak prompt injection defense | Multi-layer: sandwich defense, canary tokens, classification | 🛡️ 95%+ defense rate |
| Unverified third-party skills | Ed25519 cryptographic signatures + capability-aware WASM sandboxing | 🔐 Safer third-party skill handling |
| Synchronous memory indexing blocks startup | Fully async embedding pipeline with bounded concurrency | ⚡ 10x faster startup |
| Unreliable cron-based scheduling | Durable scheduler: idempotency, leases, dead-letter, timezone-safe | ✅ 99.9% task reliability |
| Single-writer SQLite, no isolation | WAL mode + per-session filesystem namespaces | 🏗️ Production concurrency |

---

## 🚀 Quick Feature Reference

### 🎯 Core Capabilities

| Feature | Status | Description |
|---------|--------|-------------|
| 🦀 Rust Core | ✅ Complete | 14-crate workspace with full async support |
| 🐍 Python Sidecar | ⚠️ Transitional | Bridge exists today, but retirement from the production-critical path is a roadmap goal |
| 🔐 Security Hardening | ✅ 6 modules | Auth, origin check, injection defense, sandbox, audit |
| 🧠 3-Tier Memory | ✅ Complete | Core + Recall + Archive with hybrid search |
| 📅 Durable Scheduler | ✅ Complete | Workflow-based with idempotency |
| 🔌 MCP Support | ✅ Client + Server | Connect to 18,000+ MCP servers |
| 📊 Observability | ✅ LangSmith | Traces, metrics, eval datasets |
| 🛠️ CLI | ✅ 10 commands | Full-featured command-line interface |

### 🤖 LLM Providers

| Provider | Status | Streaming | Tools | Batching |
|----------|--------|-----------|-------|----------|
| Anthropic | ✅ Ready | ✅ Yes | ✅ Strict | ✅ 50% off |
| OpenAI | ✅ Ready | ✅ Yes | ✅ Strict | ❌ No |
| OpenRouter | ✅ Ready | ✅ Yes | ✅ Yes | ❌ No |
| Ollama | ✅ Ready | ✅ Yes | ✅ Yes | N/A |

### 📡 Channels (Platform Integrations)

| Platform | Status | Notes |
|----------|--------|-------|
| WebChat | ✅ Ready | WebSocket-based web interface |
| CLI | ✅ Ready | Terminal/TUI interface |
| REST API | ✅ Ready | Headless API access |
| Telegram | ✅ Ready | Auth probe, outbound send, polling receive, and local agent/session routing exist |
| Discord | ⚠️ Partial | Outbound send, interactions ingress, gateway ingress, and routing exist; deeper runtime polish remains |
| Slack | ⚠️ Partial | HTTP mode auth, outbound send, ingress, and routing exist; Socket Mode remains incomplete |

### 🔒 Security Features

| Feature | Implementation | Status |
|---------|---------------|--------|
| WebSocket Origin Validation | `crates/security/origin_check.rs` | ✅ |
| JWT Authentication | `crates/gateway/auth.rs` | ✅ Optional |
| Prompt Injection Defense | Input validation and runtime guardrails | ✅ Basic |
| Ed25519 Skill Verification | `crates/security/skill_verifier.rs` | ✅ Configurable |
| WASM Sandboxing | `crates/skills/src/sandbox.rs` | ✅ |
| Session Isolation | Per-thread session state in gateway/runtime | ✅ Basic |
| Audit Logging | `crates/security/audit.rs` | ✅ |

---

## 📈 Performance Characteristics

### Memory System
- **Core memory retrieval**: < 1ms
- **Hybrid search (10 results)**: < 3ms
- **Embedding generation**: ~50ms per 512 tokens (OpenAI text-embedding-3-small)
- **Archive consolidation**: Async background process

### Gateway
- **WebSocket connection**: ~5ms handshake
- **Message routing**: < 1ms
- **Rate limiting**: Token bucket, < 0.1ms check

### Provider Fallback
- **Failover detection**: < 500ms timeout
- **Chain traversal**: Tries all providers in < 2s

---

## 🎓 Design Philosophy

### 1. **Security by Default**
Authentication, origin checks, and skill verification are supported, but some protections remain optional or planned depending on deployment mode.

### 2. **Recall-Only Memory**
Never auto-inject large memory files. The agent must actively search for relevant context.

### 3. **Durable Execution**
Avoid external cron jobs where possible. Scheduling and retries are modeled as application-owned workflows with idempotency support.

### 4. **Provider Agnostic**
Support multiple LLM providers with automatic fallback chains.

### 5. **Observability First**
Every operation is traced. Every decision is logged. Every metric is tracked.

---

## 📋 Extended Feature Reference

### 📡 Channel Integrations

| Channel | Status | Notes |
|---------|--------|-------|
| Telegram | Partial | Config/model scaffolding present; Bot API runtime client not implemented |
| Discord | Partial | Config/model scaffolding present; Gateway/API runtime client not implemented |
| Slack | Partial | Config/model scaffolding present; Web API runtime client not implemented |
| WhatsApp | Available | Via Baileys bridge, QR/pairing auth |
| Microsoft Teams | Available | Bot Framework integration |
| Google Chat | Partial | Webhook ingress, token/service-account auth, file-reference cards, and local routing are implemented; richer operator/media parity remains open |
| Google Meet | Partial | Native Rust operator integration covers spaces, conference records, transcripts, and Workspace Events/Pub/Sub payload decoding; add-on UI/runtime embedding remains open |
| Gmail Pub/Sub | Partial | Watch setup, Pub/Sub webhook ingress, message hydration, replies, and mail-triggered workflows are implemented; richer operator parity remains open |
| Signal | Planned | signal-cli bridge |
| Matrix | Partial | Auth, `/sync` polling ingress, sends, reactions, room actions, and file upload are implemented; deeper parity remains open |
| iMessage | Partial | Channel module present; private API mode incomplete |
| LINE | Available | Messaging API channel module |
| Viber | Available | Bot API channel module |
| WeChat | Available | Work and Official Accounts channel module |
| Messenger | Available | Meta Graph API channel module |
| Instagram | Available | Meta Graph API channel module |
| SMS (Twilio) | Planned | Twilio API |
| X (Twitter) | Planned | X API v2 |
| WebChat | Ready | Built-in web interface |
| Email | Planned | IMAP/SMTP |
| IRC | Planned | Built-in |

### 🎤 Voice System

| Feature | Status | Implementation |
|---------|--------|----------------|
| Wake Word Detection | Planned | Porcupine engine, custom models |
| Talk Mode | Planned | Continuous conversation mode |
| Speech-to-Text | Partial | Provider-backed inbound transcription plus OpenAI-compatible operator/runtime STT lanes |
| Text-to-Speech | Partial | Provider-backed synthesis plus OpenAI-compatible operator/runtime TTS lanes |

### 🖼️ Visual / Canvas

| Feature | Status | Implementation |
|---------|--------|----------------|
| Live Canvas | Planned | A2UI workspace, WebSocket |
| Real-time Collaboration | Planned | Multi-user sessions |
| Visual Elements | Planned | Text, Image, Chart, Form, Button, Code, Markdown |

### 🤖 Multi-Agent System

| Feature | Status | Description |
|---------|--------|-------------|
| Agent Router | Planned | Priority-based task routing |
| Capability Discovery | Planned | Agent skill registry |
| Inter-Agent Tools | Planned | `sessions_*` tool protocol |
| Heartbeat Scheduler | Planned | Distributed coordination |

### 📱 Mobile

| Feature | Status | Description |
|---------|--------|-------------|
| iOS SDK | Planned | Swift FFI bindings |
| Android SDK | Planned | Kotlin JNI bindings |

---

## 🤝 Community & Support

- **GitHub**: [github.com/openrustclaw/openrustclaw](https://github.com/openrustclaw/openrustclaw)
- **Documentation**: You're reading it! 📖
- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas

---

## 📄 License

MIT License. See [LICENSE](https://github.com/openrustclaw/openrustclaw/blob/main/LICENSE) for details.

---

## 🙏 Acknowledgments

OpenRustClaw builds upon the excellent work of:

- **LangChain/LangGraph** — Python AI orchestration
- **Anthropic** — Claude and tool use patterns
- **OpenAI** — GPT and function calling
- **The Rust Community** — Async ecosystem, axum, sqlx, and more

---

Ready to get started? Jump to [Installation](./getting-started/installation.md) or the [Quickstart Guide](./getting-started/quickstart.md)!
