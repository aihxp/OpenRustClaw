# OpenRustClaw Re-engineering Backlog

This backlog replaces ad hoc placeholder cleanup with a phased execution plan.
The target architecture remains:

- Rust core for gateway, channels, scheduler, persistence, auth, tools, MCP
- Python sidecar for LangGraph workflows, evaluators, and orchestration
- SQLite as the operational system of record
- LangSmith for tracing and evals

## Rules

- No feature is considered shipped unless startup, runtime behavior, tests, and docs all exist.
- No product path may return fake success.
- Partial subsystems must be explicitly marked `gated`, `dev_only`, or `deferred`.
- Work lands as vertical slices, not isolated placeholder rewrites.

## Phase 1: Product Contract

Goal: define the real supported surface and gate everything else.

- Create and maintain a feature matrix for runtime support.
- Align CLI help, README, docs, and health output with the matrix.
- Gate non-shipping features behind explicit docs/status rather than silent stubs.

Status:
- In progress

## Phase 2: Rust-Sidecar Contract

Goal: make the Rust/Python boundary explicit and testable.

- Replace implicit metadata conventions with documented workflow contracts.
- Add typed Rust helpers for sidecar workflows.
- Add integration tests for request/response semantics and workflow metadata.

Status:
- Pending

## Phase 3: Durable Scheduler

Goal: make scheduler execution real end to end.

- Poll due jobs from SQLite.
- Acquire leases atomically.
- Dispatch workflows to the Python sidecar.
- Persist job runs, retries, dead-letter entries, and next-run scheduling.
- Add restart-safe tests around lease, retry, and run persistence.

Status:
- In progress

## Phase 4: Memory Service

Goal: make sidecar memory operations use the Rust memory system instead of mocks.

- Standardize memory search/store/archive contracts.
- Route sidecar memory tools to Rust-backed services.
- Persist memory maintenance archive results into SQLite.
- Add eval coverage for recall and archive quality.

Status:
- Pending

## Phase 5: Tier 1 Channels

Goal: finish the primary operator-facing channels.

- WebChat
- Telegram
- Discord
- Slack

Each channel must have:

- config validation
- real auth/connect
- inbound receive path
- outbound send path
- retry/reconnect behavior
- tests for nominal and failure paths

Status:
- Pending

## Phase 6: MCP Surface

Goal: keep one honest supported transport and make it useful.

- Keep stdio MCP server as the supported runtime path.
- Expand MCP server tools for workspace health, memory, and scheduling.
- Keep `mcp2-cli` as the operator/debug path.

Status:
- Pending

## Phase 7: Skills Runtime

Goal: move from claims to a usable secure execution model.

- Require signature verification for external skills.
- Keep manifest/capability enforcement real.
- Either implement WASM execution for real or continue to document it as deferred.
- Build install/update/remove lifecycle on top of the live registry path.

Status:
- Pending

## Phase 8: RAG and Context

Goal: separate retrieval from memory and make context assembly deterministic.

- Distinct corpus ingestion for docs/code/config/conversation summaries
- Retrieval evaluation before tuning
- Token-budgeted context assembly and compaction

Status:
- Pending

## Current Execution Order

1. Product contract and feature matrix
2. Durable scheduler execution path
3. Rust-sidecar typed helpers
4. Memory service integration
5. Tier 1 channels
6. MCP feature completion
7. Skills runtime
8. RAG/context service

## Definition of Done

The re-engineering effort is complete when:

- shipped features are real end to end
- no placeholder success paths remain in shipped features
- sidecar workflows consume Rust-owned state/services where required
- scheduler and memory behavior are durable and restart-safe
- docs and runtime surface match exactly
