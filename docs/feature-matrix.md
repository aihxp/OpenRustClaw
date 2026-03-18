# OpenRustClaw Feature Matrix

Companion planning docs:

- [roadmap.md](roadmap.md)
- [parity-matrix.md](parity-matrix.md)
- [parity-positioning.md](parity-positioning.md)

Status values:

- `real`: usable end to end
- `partial`: exists but not complete enough to claim as production-ready
- `gated`: present in the repo but intentionally out of the shipped surface
- `deferred`: planned, not currently part of the shipped surface

## Core Runtime

| Area | Status | Notes |
| --- | --- | --- |
| Gateway health and WebSocket entry | real | Core runtime path exists and is tested |
| Python sidecar process management | real | Source-tree execution and readiness checks exist |
| Rust to sidecar workflow dispatch | real | Workflow dispatch now preserves typed configurable metadata through the bridge contract |
| Workflow execution tier model | real | Execution tiers are explicit and operator-visible: `rust_native` for production-critical paths, `compat_sidecar` for bounded legacy migration only when `sidecar.role=compatibility`, and `experimental_langgraph` for prototyping only; LangGraph is no longer part of the production-critical runtime path |
| Durable scheduler schema | real | SQLite schema and retry/dead-letter tables exist |
| Durable scheduler execution loop | real | Due-job polling, leases, retries, dead-letter handling, event-triggered dispatch, session lifecycle hooks, hook execution policies, and Rust-native reminder delivery are persisted and tested |
| File-backed task manifests / task registry | real | Standard `.claw/tasks/`-style manifests, task priority, filesystem<->scheduler sync, export, inspect/reprioritize/disable controls, CLI scaffolding, and MCP task inspection/control now sit on top of the durable SQLite scheduler |
| Autonomous optimization framework | real | Rust-native target registry, candidate store, mutation policy enforcement, temp-workspace runner, evaluation history, promotion history, CLI operator controls, and MCP inspection/promotion tools exist |
| Observability / LangSmith tracing | partial | Sidecar workflow traces now preserve trace ids back to Rust, and scheduler dispatch, channel message handling, persisted Slack/Discord ingress handling, richer channel trace metadata, gateway chat completions, internal memory/RAG endpoints, plus MCP tool calls can emit Rust-side LangSmith runs when enabled via env; coverage is still not uniform across all runtime paths |

## Memory and Context

| Area | Status | Notes |
| --- | --- | --- |
| SQLite recall memory store | real | Search/store/dedup/expiry paths exist |
| Core memory store | real | Budgeted key-value memory exists |
| Sidecar memory orchestration | real | Agent and maintenance workflows use the Rust loopback bridge and typed workflow contract for search/store/archive paths, and the bridge/internal API plus default agent tools now support both rendering and setting core memory |
| Memory maintenance archive pipeline | real | Maintenance workflow can fetch old memories, persist archive summaries, and remove archived originals through Rust-owned storage |
| RAG pipeline | partial | Deterministic collection-backed retrieval, Rust-backed durable chunk storage, budgeted context assembly, source-type filters, stopword-aware lexical scoring, configurable per-source diversity limits, configurable minimum-score filtering, configurable top_k/preferred/required/excluded source and source-type controls, minimum-overlap filtering, duplicate suppression, retrieval summaries, score-aware/metadata-aware context shaping, and collection stats inspection exist; richer indexing still needs production hardening |
| Model-aware memory/artifact rehydration | real | Workspace guidance now resolves through a Rust-native artifact registry, provider/model-family preference matching, preferred-target sync, runtime prompt rebudgeting, and session-safe rehydration on model/provider swaps |
| Instruction/context artifact registry | real | A canonical Rust-native registry now scans and normalizes universal and vendor-specific instruction files such as `AGENTS.md`, `AI.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `CLAUDE.md`, `GEMINI.md`, Copilot instruction files, Cursor/Continue rules, persona files, memory files, and `Modelfile`, with precedence, visibility, and sync handling |

## Channels

| Area | Status | Notes |
| --- | --- | --- |
| WebChat | real | Current baseline chat path |
| Telegram | real | Auth probe, outbound send, Bot API polling receive, reply threading, mention-aware group activation metadata, and local agent/session routing exist |
| Discord | partial | Auth probe, outbound send, edit/reaction operations, verified Interactions HTTP ingress, Gateway `MESSAGE_CREATE` receive, reconnect plus session resume handling, stale-heartbeat recovery, invalid-session recycling, thread-aware session routing, thread-preferred outbound replies, reply-reference propagation aliases, bot/webhook/self-message suppression, DM markers, mention detection, attachment/embed metadata capture, deferred acknowledgements, and local agent/session routing exist; deeper gateway polish remains incomplete |
| Slack | real | HTTP mode supports auth probe, outbound send, edit/reaction operations, built-in Events API ingress, mention activation metadata, and local agent/session routing; Socket Mode remains incomplete |
| Teams | partial | Bot Framework channel is now on the shipped startup path instead of being hard-gated, but deeper webhook/runtime parity still trails the tier-1 channels |
| Channel account/binding registry | real | `.claw/channels/` manifests now back pending/approved accounts, account activation mode, workspace/account/channel bindings, operator CLI control, and typed HTTP operator APIs for Control UI reuse |
| Matrix | gated | Repo surface exists, but runtime client support is deferred from the current shipped surface |
| Google Chat | partial | Shipped runtime now supports webhook ingress plus token-backed outbound sends and local agent/session routing; fuller service-account auth and richer operator parity remain open |
| Gmail Pub/Sub | gated | Repo surface exists, but Gmail API runtime coverage is deferred from the current shipped surface |
| WhatsApp | real | Baileys bridge runtime, pairing/QR support, reconnect handling, DM/group routing, mentions, replies, media send/receive, and local agent/session routing are part of the shipped surface |
| iMessage | partial | BlueBubbles/macOS direct send, BlueBubbles inbound webhook ingress, tapbacks, contact routing, and local agent/session routing are on the shipped runtime path; richer attachment/group mapping still trails OpenClaw |
| Teams / LINE / Viber / WeChat / Messenger / Instagram | gated | Present in repo, not part of the current shipped surface |
| Signal / Twilio / X/Twitter | gated | Present in repo, not part of the current shipped surface |

## Tooling and Integrations

| Area | Status | Notes |
| --- | --- | --- |
| MCP stdio server | real | Supported transport with workspace, memory, scheduling, and RAG collection/chunk inspection tools |
| MCP remote HTTP/SSE | deferred | Not part of current shipped surface |
| `mcp2-cli` | real | Real stdio/OpenAPI discovery path |
| Control-plane registry | real | `.claw/control/` now provides agent profiles, model profiles, Claw manifests, runtime-mode/task/category bindings, MCP inspection tools, onboarding scaffolding, and doctor validation |
| Native web access / browser automation | deferred | Roadmap now includes a Rust-native stack for read-only fetch/extract, crawl-for-RAG, interactive browser control, optional vision verification, MCP/browser tool surfaces, and optional compatibility bridges for external browser runtimes where Rust remains the durable source of truth |
| Multi-model orchestration / quarterback model | deferred | Roadmap now includes model profiles, model-per-task routing, a planner-orchestrator model mode, structured inter-model delegation, and a control-plane fallback model so model changes and outages do not take the platform down |
| Multi-claw execution modes | partial | Solo/task/category/orchestrated modes, task/category bindings, runtime self-description, and Claw-visible runtime context are file-backed and inspectable; full orchestrated execution remains open |
| Provider/model availability scanning | partial | `openrustclaw models scan` plus onboarding-time recommendation output now validate configured lanes at a basic level; recurring health scans and provider-native limit introspection remain open |
| Onboarding journey / config settings / doctor | partial | Onboarding now scaffolds solo-vs-multi-claw control state and model-role recommendations, while `doctor --repair/--deep` validates the new control registry; broader OpenClaw-style repair/config parity remains open |
| Cursor integration | gated | Repo surface exists, but it is not part of the current shipped runtime/tooling surface |
| Skills registry/install flow | real | Workspace installs, marketplace lifecycle sync, discovery-time capability normalization, capability metadata persistence, sensitive-capability classification, verification policy summaries, and verification-state handling are wired through the CLI; failed verification clears stale verified state |
| WASM skill executor | real | Real no-import executor exists with JSON ABI, memory limits, timeout enforcement, explicit capability checks, declared-capability sandbox config helpers, verification-aware declared-capability policy enforcement helpers, and sensitive-capability classification helpers |

## Experience Layers

| Area | Status | Notes |
| --- | --- | --- |
| Voice | gated | Feature-gated and not part of current shipped surface |
| Mobile sync | gated | Repo surface exists, not part of current shipped surface |
| Distributed clustering | gated | Repo surface exists, not part of current shipped surface |
