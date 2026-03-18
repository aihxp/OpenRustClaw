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
- Mostly complete

## Phase 2: Rust-Sidecar Contract

Goal: make the Rust/Python boundary explicit and testable.

- Replace implicit metadata conventions with documented workflow contracts.
- Add typed Rust helpers for sidecar workflows.
- Add integration tests for request/response semantics and workflow metadata.

Status:
- Mostly complete

## Phase 3: Durable Scheduler

Goal: make scheduler execution real end to end.

- Poll due jobs from SQLite.
- Acquire leases atomically.
- Dispatch workflows to the Python sidecar.
- Persist job runs, retries, dead-letter entries, and next-run scheduling.
- Add restart-safe tests around lease, retry, and run persistence.

Status:
- Mostly complete

## Phase 4: Memory Service

Goal: make sidecar memory operations use the Rust memory system instead of mocks.

- Standardize memory search/store/archive contracts.
- Route sidecar memory tools to Rust-backed services.
- Persist memory maintenance archive results into SQLite.
- Add eval coverage for recall and archive quality.

Status:
- Mostly complete

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
- Mostly complete

## Phase 6: MCP Surface

Goal: keep one honest supported transport and make it useful.

- Keep stdio MCP server as the supported runtime path.
- Expand MCP server tools for workspace health, memory, and scheduling.
- Keep `mcp2-cli` as the operator/debug path.

Status:
- Mostly complete

## Phase 7: Skills Runtime

Goal: move from claims to a usable secure execution model.

- Require signature verification for external skills.
- Keep manifest/capability enforcement real.
- Either implement WASM execution for real or continue to document it as deferred.
- Build install/update/remove lifecycle on top of the live registry path.

Status:
- In progress

## Phase 8: RAG and Context

Goal: separate retrieval from memory and make context assembly deterministic.

- Distinct corpus ingestion for docs/code/config/conversation summaries
- Retrieval evaluation before tuning
- Token-budgeted context assembly and compaction

Status:
- In progress

## Current Snapshot

- Product contract and runtime/docs honesty work is largely complete.
- Scheduler, MCP stdio, marketplace access, and the no-import WASM executor are real.
- Telegram and Slack HTTP mode are end-to-end runtime paths.
- Discord now has verified Interactions HTTP ingress, Gateway `MESSAGE_CREATE` receive, reconnect plus session resume handling, stale-heartbeat recovery, invalid-session recycling, deferred acknowledgements, and follow-up replies; deeper gateway polish is still incomplete.
- Memory maintenance now uses Rust-backed archive persistence and removes archived recall entries through the loopback bridge.
- The Rust loopback memory API now supports both rendering and setting core-memory entries, not just read-only rendering.
- Rust-to-sidecar workflow dispatch now preserves typed configurable metadata through a reserved contract key instead of flattening everything to strings.
- The skills CLI now resolves real `SKILL.md` workspace paths, validates and normalizes declared capabilities, syncs marketplace installs back into the main skills table, and keeps managed verification state aligned with install/update flows.
- Marketplace installs no longer auto-mark signed skills as verified, and unsigned skills requesting sensitive capabilities are rejected during install/update.
- Workspace skill discovery now normalizes declared capabilities, skips invalid `SKILL.md` capability sets, and the WASM sandbox config layer can be built directly from declared capabilities.
- Skill verification now clears stale `verified` state on missing signatures or failed signature checks instead of leaving outdated verification records behind.
- The sidecar RAG workflow now supports deterministic query-only retrieval against Rust-backed durable collections and budgeted context assembly with stable source ids.
- The sidecar RAG workflow now applies source-aware scoring and optional source-type filters on top of the durable collection store.
- The sidecar RAG workflow now applies stopword-aware lexical scoring and configurable per-source diversity limits on top of the durable collection store.
- The sidecar RAG workflow now supports configurable minimum-score filtering so weak lexical matches can be dropped instead of filling the retrieval set.
- The Rust loopback RAG API now supports collection listing and deletion in addition to replace/load operations.
- The Rust loopback RAG API now returns richer collection stats, including source breadth and content volume, in addition to replace/load/list/delete operations.
- The Python sidecar memory bridge now supports both rendering and setting durable core-memory entries through the Rust loopback API.
- The default sidecar agent tools now expose durable `set_core_memory` writes through the Rust loopback bridge.
- Sidecar LangSmith traces now preserve workflow trace ids back through the gRPC boundary so scheduler runs can persist them.
- Scheduler dispatch can now create Rust-side LangSmith parent runs when `[observability].langsmith_enabled = true` and LangSmith env vars are present.
- Channel message handling can now create Rust-side LangSmith runs when `[observability].langsmith_enabled = true` and LangSmith env vars are present.
- Gateway chat completions and internal memory/RAG endpoints can now emit Rust-side LangSmith runs when `[observability].langsmith_enabled = true` and LangSmith env vars are present.
- MCP stdio tool calls can now emit Rust-side LangSmith runs when `[observability].langsmith_enabled = true` and LangSmith env vars are present.
- Slack and Discord ingress handlers can now emit Rust-side LangSmith runs when `[observability].langsmith_enabled = true` and LangSmith env vars are present.
- Discord gateway normalization now suppresses bot/system/webhook/self-authored events, preserves thread/reply/timestamp metadata, and keeps thread-aware session scope separate from channel scope.
- Discord outbound sends now propagate reply references when channel-originated metadata includes a referenced message id.
- Skill metadata now exposes sensitive capability classification and whether privileged execution should require verification.
- The WASM sandbox now exposes verification-aware declared-capability policy validation and execution helpers for privileged skill execution paths.
- The RAG retrieval workflow now supports configurable `top_k`, preferred source ids, required source ids, and retrieval summaries in addition to source-type/diversity/min-score controls.
- MCP stdio now exposes durable RAG collection and chunk inspection through `list_rag_collections` and `load_rag_chunks`.
- `openrustclaw start` now gates non-shipping channel modules instead of advertising them through the active runtime surface.
- The biggest remaining engineering gaps are:
  - broader observability/LangSmith trace coverage across all runtime paths
  - deeper Discord Gateway runtime polish beyond the current message receive, reconnect, and session resume path
  - fuller skills host-capability enforcement beyond the current no-import WASM boundary
  - deeper retrieval quality and richer indexing beyond the current durable lexical collection store

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
