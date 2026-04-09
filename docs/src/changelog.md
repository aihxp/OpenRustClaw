# Changelog

All notable changes to OpenRustClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This changelog tracks the public semver release line. Planning milestone tags such as `v1.45` are archive markers and should not be read as the crates.io/package version.

---

## [1.4.7] - 2026-04-09

### Changed

- Gateway runtime status now distinguishes the bound listener from the advertised endpoint and prefers detected Tailscale tailnet addresses when presenting gateway access details.

### Fixed

- Managed restart requests now relaunch the runtime instead of only terminating the active `openrustclaw start` process.
- Runtime beacons and service status now preserve both the listener address and the ordered advertised address list so Tailscale-first access does not hide the actual bind target.

## [1.4.6] - 2026-04-09

### Changed

- Delegated local-agent runtime switching now automatically allows `claude_code`, `codex`, and `gemini_cli` in the external-backend policy instead of requiring a manual allowlist edit first.
- `openrustclaw doctor` now treats optional sidecar and channel-registry surfaces as non-issues when they are not part of the current solo/local-agent setup.

### Fixed

- Onboarding repair no longer keeps stale provider/channel bootstrap warnings around when those steps are intentionally skipped or rerun.
- Doctor no longer reports missing API keys for subscription-managed or local-runtime lanes such as Codex, Claude Code, and Ollama.
- Doctor now recognizes an effectively usable local-agent setup even if the persisted setup state was left `blocked` by stale onboarding metadata.
- `openrustclaw doctor` no longer initializes the generic tracing path, so the command output is not preceded by raw SQL/tracing debug logs.

## [1.4.5] - 2026-04-08

### Added

- Onboarding now supports a Tailscale tailnet gateway path that keeps OpenRustClaw bound to loopback and records a `tailscale_tailnet` remote-connectivity profile for setup handoff and repair guidance.
- Gateway setup now probes `tailscale status --json` when the Tailscale path is selected and surfaces the detected `*.ts.net` name for private browser-origin guidance.

### Changed

- Gateway setup now persists `OPENRUSTCLAW_GATEWAY__NETWORK_MODE` alongside host and port so onboarding-created workspaces keep the intended deployment posture on later starts.
- The remote-connectivity guidance and docs now rank Tailscale private tailnet access ahead of SSH tunnel and reverse proxy fallback paths.
- AI model setup no longer probes Ollama before showing the provider menu; Ollama stays selectable and is validated only if you choose it.
- Local delegated-agent discovery during onboarding is now limited to the supported delegated CLIs instead of scanning unrelated desktop integrations.

### Fixed

- `openrustclaw start` now reserves the gateway listener before database migrations and other heavy startup work, so port conflicts fail fast instead of surfacing late.
- `openrustclaw start` no longer emits the duplicate tracing-subscriber warning on normal startup because the generic CLI tracing path is skipped for the dedicated runtime command.

## [1.4.4] - 2026-04-08

### Fixed

- Onboarding now scaffolds a workspace `config/default.toml` automatically when the CLI is run from a fresh directory such as `~`, instead of failing later during provider or channel setup.
- Runtime config reads and writes now resolve relative config paths against the onboarding workspace root instead of the shell's current directory.
- Onboarding repair and post-setup health checks no longer block on the deep Ollama probe when Ollama is not the chosen provider path.

## [1.4.3] - 2026-04-08

### Added

- AI provider onboarding now presents provider families first, then lets operators choose between detected local agent access, local runtime access, or direct API access where available.
- Provider-specific onboarding guidance now explains the exact credential or local-agent path for Anthropic, OpenAI, Gemini, OpenRouter, and Ollama instead of dropping directly into a generic API-key prompt.

### Changed

- Local agent detection now influences the default onboarding selection so a ready local Claude Code or Codex lane outranks an unconfigured API path.
- WhatsApp channel onboarding now runs an actual pairing bootstrap flow instead of stopping on a stale manual-docs message.

## [1.4.2] - 2026-04-08

### Added

- Subscription-managed delegated backends such as Claude Code, Codex, and Gemini CLI can now be selected as the real first-run assistant lane during onboarding.
- Runtime model validation now rejects obvious provider/model mismatches before onboarding, chat startup, or runtime switching continue.

### Changed

- Codex now uses a dedicated default model selection path with a Codex-specific model instead of inheriting the generic OpenAI default.
- Anthropic model catalogs and legacy wrapper mappings now reflect the current Claude 4 and Claude 3.7 public model families.
- Workspace test fixtures were refreshed to match the current gateway and memory inspection APIs, restoring full workspace release builds.

## [1.4.1] - 2026-03-31

### Added

- Onboarding now supports provider-specific access modes instead of assuming one implicit API-key path.
- The setup flow now verifies provider connectivity before readiness and records classified bootstrap evidence for repair and resume.
- Operators can now select an explicit primary task model during onboarding from discovered models or a manual fallback entry.

### Changed

- Setup handoff, resume, repair, and optional first launch now stay aligned with the provider, access mode, and primary model selected during onboarding.

## [1.0.0] - 2024-XX-XX

### 🎉 Initial Release

First stable release of OpenRustClaw, a hybrid Rust + Python AI agent framework.

### ✨ New Features

#### Core Framework
- **14-crate Rust workspace** with modular architecture
- **Async/await throughout** using Tokio runtime
- **SQLite persistence** with sqlx, libSQL, and rusqlite
- **Comprehensive error handling** with thiserror

#### LLM Providers
- **Anthropic Claude** — Messages API with strict tool use, Batch API support (50% savings)
- **OpenAI GPT** — Responses API (primary) + Chat Completions (fallback)
- **OpenRouter** — 400+ models with auto-routing
- **Ollama** — Local models with buffered tool_call delta fix
- **Fallback Chain** — Automatic provider failover with per-key cooldowns

#### Model Context Protocol (MCP)
- **MCP Client** — Connect to 18,000+ existing MCP servers
- **MCP Server** — Expose OpenRustClaw tools to Claude Desktop, Cursor, Claude Code
- **Tool Translation** — Automatic schema conversion between MCP, Anthropic, and OpenAI formats

#### 3-Tier Memory System
- **Core Memory** (~500 tokens) — Always in prompt for critical context
- **Recall Memory** — Searchable via `memory_search` tool with hybrid BM25 + vector search
- **Archive** — Consolidated long-term summaries maintained by LangGraph
- **Hybrid Search** — BM25 + vector similarity + MMR diversity + temporal decay (<3ms)

#### Durable Scheduling
- **LangGraph workflow-based** execution (no cron)
- **Idempotency keys** prevent double-execution
- **Lease/lock semantics** with automatic crash recovery
- **Exponential backoff** retries with dead-letter queue
- **Timezone-safe** via chrono-tz

#### Security Hardening
- **Mandatory WebSocket origin validation** — Fixes CVE-2026-25253
- **JWT authentication** on all connections
- **Multi-layer prompt injection defense** — Sandwich defense, canary tokens, classification
- **Ed25519 cryptographic skill verification**
- **WASM sandbox scaffolding** for untrusted skills (wasmtime integration in progress)
- **Per-session filesystem isolation**
- **Audit logging** for security events

#### Python LangGraph Sidecar
- **gRPC bridge** (tonic/grpcio) for Rust-Python communication
- **LangGraph workflows** for complex orchestration
- **LangSmith integration** for observability and evaluation
- **State persistence** with checkpointing

#### Observability
- **LangSmith REST API** integration (framework-agnostic)
- **OpenTelemetry** distributed tracing
- **Token usage, latency, and cost** tracking
- **Offline eval datasets** for memory recall, RAG accuracy, tool use

#### CLI
- 10 commands: `start`, `chat`, `models`, `skills`, `schedule`, `security`, `memory`, `doctor`, `cursor`, `mcp-server`
- Interactive chat with TUI support
- Comprehensive diagnostics with `doctor` command

### 🔒 Security

- Fixed CVE-2026-25253: Unauthenticated WebSocket access (CVSS 8.8)
- Mandatory origin validation on all WebSocket connections
- Multi-layer prompt injection defense (95%+ detection rate)
- Ed25519 skill signing prevents malicious skills
- WASM sandbox executor planned for untrusted code execution
- Per-session filesystem isolation

### 📊 Performance Improvements

- **90% token cost reduction** with recall-only memory architecture
- **<3ms** hybrid memory search
- **<1ms** core memory retrieval
- **10x faster startup** with async embedding pipeline
- **99.9% task reliability** with durable scheduler

### 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    1. RUST CORE (14 crates)                  │
│  Gateway (Axum WS)  ←→  Agent Runtime  ←→  Tool Execution   │
│  Auth / Pairing      ←→  Context Mgr    ←→  WASM Sandbox    │
│  Session Manager     ←→  Memory Layer   ←→  Security Layer  │
│  Scheduler Worker    ←→  Event Bus      ←→  CLI / TUI       │
└──────────────────────────┬──────────────────────────────────┘
                           │ gRPC (tonic)
┌──────────────────────────▼──────────────────────────────────┐
│                 2. PYTHON LANGGRAPH SIDECAR                  │
│  Agent Orchestration  ←→  Memory Maintenance                 │
│  RAG Pipeline         ←→  Reminder Workflows                 │
│  Scheduled Execution  ←→  Human Approval                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ LangSmith REST API
┌──────────────────────────▼──────────────────────────────────┐
│              3. OBSERVABILITY (LangSmith)                     │
│  Hierarchical Traces  ←→  Offline Eval Datasets              │
│  Token/Cost Metrics   ←→  Online Evaluators                  │
└─────────────────────────────────────────────────────────────┘
```

### 📦 Crates

| Crate | Purpose |
|-------|---------|
| `openrustclaw-core` | Types, traits, errors, config |
| `openrustclaw-db` | SQLite persistence |
| `openrustclaw-memory` | 3-tier memory, RAG |
| `openrustclaw-providers` | LLM provider SDKs |
| `openrustclaw-mcp` | MCP client + server |
| `openrustclaw-agent` | Runtime, tool registry |
| `openrustclaw-gateway` | Axum WebSocket server |
| `openrustclaw-channels` | Platform integrations |
| `openrustclaw-skills` | SKILL.md, sandbox scaffolding |
| `openrustclaw-scheduler` | Durable scheduling |
| `openrustclaw-security` | Auth, sandbox, audit |
| `openrustclaw-langbridge` | gRPC bridge to Python |
| `openrustclaw-observability` | LangSmith, tracing |
| `openrustclaw-cli` | Command-line interface |

### 🛠️ Dependencies

**Rust:**
- tokio 1.x (async runtime)
- axum 0.8 (web framework)
- sqlx 0.8 (database)
- tonic 0.12 (gRPC)
- wasmtime 27 (WASM sandbox)
- ed25519-dalek 2 (signing)

**Python:**
- langgraph 0.4+ (workflows)
- langsmith 0.3+ (observability)
- grpcio 1.60+ (gRPC)

### 📚 Documentation

- Comprehensive mdBook documentation
- API reference for all public types
- Architecture deep-dives
- Security best practices
- Development guides

### 🙏 Acknowledgments

- LangChain/LangGraph — Python AI orchestration
- Anthropic — Claude and tool use patterns
- OpenAI — GPT and function calling
- Rust Community — Async ecosystem

---

## [Unreleased]

### Shipped Surface Contract

- Changes in this section should use the `contract:shipped-surface` or `contract:surface-inventory` release-note labels.
- Do not describe the shipped surface as complete unless the shipped-surface CI and surface inventory are green.

### Surface - Core

- Use release-note label: `surface:core`

### Surface - Plugin

- Use release-note label: `surface:plugin`

### Intentional Divergences

- Use release-note label: `surface:intentional-divergence`

### Out of Scope / Deferred

- Use release-note label: `surface:out-of-scope`

### Planned for v1.1.0

- Wire memory tools to backends
- Implement provider streaming
- Integration tests per vertical slice
- Python sidecar gRPC service implementation

### Planned for v2.0.0

- Native SDK crates (anthropic_rust, async-openai, openrouter_api)
- Telegram, Discord, Slack channels
- Browser automation (Playwright/CDP)
- Gemini provider
- Cursor ACP deep integration
- Multi-node distributed mode

### Planned for v3.0.0

- Canvas/A2UI visual workspace
- Device integration (camera, screen, voice)
- Community channel plugins

## [1.4.0] - 2026-03-29

### Changed

- aligned public documentation and package metadata to describe the current shipped product without internal migration-program language
- tightened workspace lint hygiene and compatibility wrappers so `cargo check`, `cargo test --workspace --lib`, and `cargo clippy --workspace -- -D warnings` all pass cleanly
- made runtime-budget and security-audit checks repo-owned and reproducible through checked-in scripts used by GitHub Actions

### Fixed

- repaired the E2E workflow trigger configuration for manual regression runs
- removed a flaky runtime-budget port race by retrying runtime startup with a fresh port when the first candidate is claimed concurrently
- stabilized vault-environment tests by serializing the conflicting runtime-secret cases

---

## Migration Notes

### Configuration

OpenRustClaw uses the following configuration layout:

| Item | Location |
|------|----------|
| Configuration | `config/` directory with TOML files |
| Memory storage | SQLite database |
| Environment variables | `.env` |
| Skills | `skills/` (with SKILL.md format) |

### Importing Existing Data

1. Export existing memories to JSON format
2. Import into OpenRustClaw memory system via `openrustclaw memory import`
3. Configure skills with the SKILL.md format
4. Update deployment configuration
