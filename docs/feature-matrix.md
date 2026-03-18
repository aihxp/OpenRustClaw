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
| Rust to sidecar workflow dispatch | partial | Generic dispatch exists; typed workflow helpers are being expanded |
| Durable scheduler schema | real | SQLite schema and retry/dead-letter tables exist |
| Durable scheduler execution loop | partial | Execution and persistence are being completed |
| Observability / LangSmith tracing | partial | Core tracing exists; coverage is not uniform yet |

## Memory and Context

| Area | Status | Notes |
| --- | --- | --- |
| SQLite recall memory store | real | Search/store/dedup/expiry paths exist |
| Core memory store | real | Budgeted key-value memory exists |
| Sidecar memory orchestration | partial | Metadata-based context exists; direct Rust-backed service integration is incomplete |
| Memory maintenance archive pipeline | partial | Workflow exists; persistence integration is being completed |
| RAG pipeline | partial | Workflow exists; storage/retrieval contracts still need production hardening |

## Channels

| Area | Status | Notes |
| --- | --- | --- |
| WebChat | real | Current baseline chat path |
| Telegram | real | Auth probe, outbound send, Bot API polling receive, and local agent/session routing in `openrustclaw start` exist |
| Discord | partial | Auth probe and outbound send path exist; inbound gateway/runtime path remains incomplete |
| Slack | partial | Auth probe, outbound send, and HTTP Events API helper exist; built-in HTTP ingress wiring is still incomplete |
| Matrix | partial | Shape exists; matrix-sdk integration deferred |
| Google Chat | partial | Auth/config shape exists; receive/send coverage incomplete |
| Gmail Pub/Sub | partial | Auth/config shape exists; live Gmail API operations incomplete |
| Teams / WhatsApp / LINE / Viber / WeChat / Messenger / Instagram / iMessage | gated | Present in repo, not part of the current shipped surface |
| Signal / Twilio / X/Twitter | gated | Present in repo, not part of the current shipped surface |

## Tooling and Integrations

| Area | Status | Notes |
| --- | --- | --- |
| MCP stdio server | real | Supported transport |
| MCP remote HTTP/SSE | deferred | Not part of current shipped surface |
| `mcp2-cli` | real | Real stdio/OpenAPI discovery path |
| Cursor integration | partial | Core path exists; full tool/runtime parity still incomplete |
| Skills registry/install flow | partial | Live registry path exists; runtime execution model still evolving |
| WASM skill executor | partial | Real no-import executor exists with JSON ABI, memory limits, and timeout enforcement; host capability surface is still minimal |

## Experience Layers

| Area | Status | Notes |
| --- | --- | --- |
| Voice | gated | Feature-gated and not part of current shipped surface |
| Mobile sync | gated | Repo surface exists, not part of current shipped surface |
| Distributed clustering | gated | Repo surface exists, not part of current shipped surface |
