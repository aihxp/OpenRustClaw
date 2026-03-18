# OpenRustClaw Feature Matrix

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
| Durable scheduler schema | real | SQLite schema and retry/dead-letter tables exist |
| Durable scheduler execution loop | real | Due-job polling, leases, retries, dead-letter handling, and sidecar dispatch are persisted and tested |
| Observability / LangSmith tracing | partial | Sidecar workflow traces now preserve trace ids back to Rust and scheduler persistence, but coverage is not uniform across all runtime paths yet |

## Memory and Context

| Area | Status | Notes |
| --- | --- | --- |
| SQLite recall memory store | real | Search/store/dedup/expiry paths exist |
| Core memory store | real | Budgeted key-value memory exists |
| Sidecar memory orchestration | real | Agent and maintenance workflows use the Rust loopback bridge and typed workflow contract for search/store/archive paths |
| Memory maintenance archive pipeline | real | Maintenance workflow can fetch old memories, persist archive summaries, and remove archived originals through Rust-owned storage |
| RAG pipeline | partial | Deterministic collection-backed retrieval, Rust-backed durable chunk storage, and budgeted context assembly exist; retrieval quality and richer indexing still need production hardening |

## Channels

| Area | Status | Notes |
| --- | --- | --- |
| WebChat | real | Current baseline chat path |
| Telegram | real | Auth probe, outbound send, Bot API polling receive, and local agent/session routing in `openrustclaw start` exist |
| Discord | partial | Auth probe, outbound send, verified Interactions HTTP ingress, deferred acknowledgements, and local agent/session routing exist; full Gateway message-event support remains incomplete |
| Slack | real | HTTP mode supports auth probe, outbound send, built-in Events API ingress, and local agent/session routing; Socket Mode remains incomplete |
| Matrix | gated | Repo surface exists, but runtime client support is deferred from the current shipped surface |
| Google Chat | gated | Repo surface exists, but service-account auth and runtime coverage are deferred from the current shipped surface |
| Gmail Pub/Sub | gated | Repo surface exists, but Gmail API runtime coverage is deferred from the current shipped surface |
| Teams / WhatsApp / LINE / Viber / WeChat / Messenger / Instagram / iMessage | gated | Present in repo, not part of the current shipped surface |
| Signal / Twilio / X/Twitter | gated | Present in repo, not part of the current shipped surface |

## Tooling and Integrations

| Area | Status | Notes |
| --- | --- | --- |
| MCP stdio server | real | Supported transport |
| MCP remote HTTP/SSE | deferred | Not part of current shipped surface |
| `mcp2-cli` | real | Real stdio/OpenAPI discovery path |
| Cursor integration | gated | Repo surface exists, but it is not part of the current shipped runtime/tooling surface |
| Skills registry/install flow | real | Workspace installs, marketplace lifecycle sync, capability metadata persistence, and verification-state handling are wired through the CLI |
| WASM skill executor | real | Real no-import executor exists with JSON ABI, memory limits, timeout enforcement, and explicit capability checks |

## Experience Layers

| Area | Status | Notes |
| --- | --- | --- |
| Voice | gated | Feature-gated and not part of current shipped surface |
| Mobile sync | gated | Repo surface exists, not part of current shipped surface |
| Distributed clustering | gated | Repo surface exists, not part of current shipped surface |
