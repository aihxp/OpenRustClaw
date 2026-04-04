# Codebase Concerns

**Analysis Date:** 2026-04-04

## Tech Debt

**CLI runtime and control surface are concentrated in monolithic modules:**
- Issue: Startup, runtime wiring, control routes, channel bootstrapping, and many tests live in a few multi-thousand-line files.
- Files: `crates/cli/src/commands/start.rs`, `crates/cli/src/main.rs`, `crates/cli/src/commands/skills.rs`, `crates/cli/src/commands/mobile.rs`
- Impact: Small changes have high merge-conflict risk, slow review, large rebuilds, and brittle test locality because runtime code and tests are co-located.
- Fix approach: Split transport setup, control handlers, sidecar wiring, and channel bootstrapping into focused modules with dedicated test files.

**Sidecar RAG implementation mixes prototype behavior with shipped code paths:**
- Issue: `sidecar/src/workflows/rag_pipeline.py` keeps a global in-process `RAG_STORE`, loads files synchronously, emits mock embeddings when `OpenAIEmbeddings` is unavailable, and emits mock answers when `ChatOpenAI` cannot initialize.
- Files: `sidecar/src/workflows/rag_pipeline.py`, `sidecar/src/memory_bridge.py`, `sidecar/pyproject.toml`
- Impact: The sidecar lane can appear healthy while silently degrading to lexical-only, process-local, non-durable behavior.
- Fix approach: Make real dependencies mandatory when the sidecar lane is enabled, move mock behavior behind explicit test-only flags, and keep persistence in `crates/db`.

## Known Bugs

**Marketplace skill verification requests the wrong public-key identity:**
- Symptoms: `ClawHubRegistry::verify_signature` fetches `/api/v1/authors/{name}/key`, where `name` is the skill name rather than `SkillMetadata.author`.
- Files: `crates/skills/src/registry.rs`
- Trigger: Installing a signed marketplace skill whose skill name differs from its author identifier.
- Workaround: Patch verification to use the author field before relying on signed marketplace installs.

**Cron heartbeat tasks can miss their scheduled fire time:**
- Symptoms: `HeartbeatCondition::Cron` only returns true when the next scheduled occurrence is within one second of `Utc::now()`.
- Files: `crates/scheduler/src/heartbeat.rs`, `config/default.toml`
- Trigger: Scheduler loop drift, slow condition evaluation, or any effective poll interval above one second.
- Workaround: Use non-cron conditions for critical automation until cron execution compares against the prior tick window instead of an exact-second match.

## Security Considerations

**Gateway CORS is wildcarded across public and internal routes:**
- Risk: The router applies `CorsLayer::allow_origin(Any)` at the top level, covering `/v1/chat/completions` and `/internal/...` endpoints alike.
- Files: `crates/gateway/src/server.rs`
- Current mitigation: Internal routes still require `x-openrustclaw-internal-token`, and WebSocket origin validation is enforced separately through `OriginValidator`.
- Recommendations: Scope CORS per route group, avoid wildcard CORS on `/internal/...`, and require an explicit allow-list for public HTTP endpoints.

**Marketplace skill packages are unpacked directly from downloaded tarballs:**
- Risk: `install_package` feeds the downloaded archive into `tar::Archive::unpack` without validating member paths, so a malicious archive can attempt path traversal or overwrite unexpected files.
- Files: `crates/skills/src/registry.rs`
- Current mitigation: The CLI applies capability and signature policy before registry installation in `crates/cli/src/commands/skills.rs`.
- Recommendations: Validate archive members before extraction, reject absolute and `..` paths, and fail closed on unsigned or unverifiable packages.

**Unsigned skill installation remains possible by default:**
- Risk: `config/default.toml` sets `skill_signature_required = false`, and `ClawHubRegistry::install_internal` only verifies a package when a signature is present.
- Files: `config/default.toml`, `crates/skills/src/registry.rs`, `crates/cli/src/commands/skills.rs`
- Current mitigation: `enforce_external_skill_policy` blocks unsigned installs that request sensitive capabilities.
- Recommendations: Require signatures by default for marketplace installs and enforce verification in the registry layer, not only in CLI entry points.

## Performance Bottlenecks

**Memory search reranking collapses FTS relevance and does per-result writes:**
- Problem: `search` and `search_with_embedding` fetch `limit * 3` rows, ignore the actual FTS `rank` by hard-coding `bm25_score = 1.0`, rerank in Rust, then issue one `UPDATE` per returned row to increment access counts.
- Files: `crates/db/src/memory_store.rs`
- Cause: FTS rank is selected but not consumed, and access updates are not batched.
- Improvement path: Preserve normalized FTS rank in scoring, batch access-count updates, and push more ranking/filtering into SQL.

**Recursive file watching is implemented as a full filesystem walk on every scheduler tick:**
- Problem: `compute_file_fingerprint` walks whole directories with `walkdir`, and `HeartbeatScheduler::check_and_trigger` evaluates tasks sequentially on every interval.
- Files: `crates/scheduler/src/heartbeat.rs`, `config/default.toml`
- Cause: Polling-based change detection plus a default `poll_interval_ms = 1000`.
- Improvement path: Switch to OS file notifications for watchable paths, widen polling for recursive scans, and isolate expensive conditions from the main scheduler loop.

**RAG collection writes rewrite the entire collection and reads pull whole chunk bodies:**
- Problem: `internal_rag_store_handler` always calls `replace_collection`, which deletes and reinserts the full collection, while `load_collection` returns full chunk payloads up to the request limit.
- Files: `crates/gateway/src/server.rs`, `crates/db/src/rag_store.rs`, `sidecar/src/workflows/rag_pipeline.py`
- Cause: Collection replacement is the only write primitive, and query-only retrieval reloads whole collection snapshots.
- Improvement path: Add incremental upsert/delete APIs, index by `source_id`, and stop reloading entire collections for query-only retrieval.

## Fragile Areas

**Gateway API behavior is exposed as a real surface but still stubbed:**
- Files: `crates/gateway/src/server.rs`, `tests/e2e/tests/vertical/test_api_layer.rs`
- Why fragile: `/v1/chat/completions` returns an `Echo:` response with zeroed token usage, while the transport tests ignore WebSocket and SSE behavior.
- Safe modification: Treat the gateway API as contract work; update handler, fixtures, and vertical tests together.
- Test coverage: HTTP smoke coverage exists, but provider-backed completions, SSE, and WebSocket semantics are not exercised.

**Scheduler semantics rely on wall-clock timing and in-memory state:**
- Files: `crates/scheduler/src/heartbeat.rs`
- Why fragile: Cron checks depend on tick alignment, file/email/system-event state is stored in process memory, and task evaluation is sequential.
- Safe modification: Preserve deterministic time boundaries in tests, add persisted scheduling state before widening use, and benchmark recursive file checks.
- Test coverage: Builder and custom-condition tests exist, but there is no end-to-end test proving cron tasks fire correctly across drift or restart boundaries.

**Sidecar bridge and workflow behavior degrade silently:**
- Files: `sidecar/src/workflows/rag_pipeline.py`, `sidecar/src/memory_bridge.py`, `sidecar/test_sidecar.py`
- Why fragile: Missing dependencies trigger mock embeddings or mock answers, missing bridge env vars fall back to process-local storage, and bridge tests reuse one fake response body for different endpoint contracts.
- Safe modification: Fail fast when the configured sidecar lane lacks required packages or bridge configuration, and separate real contract tests from smoke-level mocked tests.
- Test coverage: Retrieval heuristics are covered, but real bridge contracts and dependency-failure behavior are weakly validated.

## Scaling Limits

**Scheduler throughput is bounded by one sequential polling loop:**
- Current capacity: One `HeartbeatScheduler::run` loop evaluates all tasks every `poll_interval_ms`; the shipped default is `1000` ms in `config/default.toml`.
- Limit: Expensive conditions block the next cycle, and cron/file-change accuracy degrades as task count and watched tree size grow.
- Scaling path: Partition conditions by cost, move long-running checks off the hot loop, and persist scheduler state for resumable execution.

**RAG query-only retrieval is capped by collection snapshot size and process memory:**
- Current capacity: `SqliteRagStore::load_collection` defaults to `1000` chunks per call, and the sidecar fallback store keeps whole collections in a global Python dictionary.
- Limit: Larger collections increase load latency and memory pressure, while the in-memory fallback loses data on restart and does not support multi-process sharing.
- Scaling path: Stream chunk windows from SQLite, keep durable indexes on the Rust side, and remove the global `RAG_STORE` fallback for non-test runs.

## Dependencies at Risk

**The experimental sidecar depends on optional Python packages for real behavior:**
- Risk: `sidecar/src/workflows/rag_pipeline.py` imports `langchain_openai`, `ChatOpenAI`, and `OpenAIEmbeddings` at runtime and falls back to mock behavior when they are unavailable; `sidecar/test_sidecar.py` skips many checks on `ModuleNotFoundError`.
- Impact: CI or local runs can report green smoke coverage while the experimental sidecar is not actually capable of real embedding or generation work.
- Migration plan: Fail sidecar startup when `sidecar.role` is enabled but required packages from `sidecar/pyproject.toml` are unavailable, and keep skip-based tests only for explicitly optional integrations.

## Missing Critical Features

**The OpenAI-compatible HTTP surface is not provider-backed:**
- Problem: The gateway exposes `/v1/chat/completions`, but the handler emits a synthetic echo response instead of routing through `openrustclaw-providers`.
- Files: `crates/gateway/src/server.rs`, `tests/e2e/tests/vertical/test_api_layer.rs`
- Blocks: Using the gateway as a real API surface, validating prompt/tool behavior end-to-end, and shipping truthful token accounting or streaming.

**Real-time gateway transports remain incomplete:**
- Problem: Vertical tests mark WebSocket and SSE coverage as ignored, and the current WebSocket handler only performs handshake validation plus simple ack/echo behavior.
- Files: `crates/gateway/src/server.rs`, `tests/e2e/tests/vertical/test_api_layer.rs`
- Blocks: Reliable browser/live-client integration, streaming output, and meaningful transport-level regression testing.

## Test Coverage Gaps

**Skill registry install verification lacks contract coverage:**
- What's not tested: The author-key lookup path inside `ClawHubRegistry::verify_signature`, including the current `{skill name}` vs `{author}` mismatch.
- Files: `crates/skills/src/registry.rs`, `crates/skills/src/marketplace.rs`
- Risk: Signed marketplace installs can fail only in production flows.
- Priority: High

**Cron scheduling correctness is not covered end-to-end:**
- What's not tested: Whether `HeartbeatCondition::Cron` triggers correctly across timer drift, restarts, and non-1s poll intervals.
- Files: `crates/scheduler/src/heartbeat.rs`
- Risk: Scheduled tasks can be skipped silently.
- Priority: High

**Sidecar bridge tests do not validate real endpoint contracts:**
- What's not tested: Distinct response shapes for `/memory/*` and `/rag/*` endpoints; `sidecar/test_sidecar.py` reuses one fake JSON payload and asserts fields that the Rust handlers do not actually return for several routes.
- Files: `sidecar/test_sidecar.py`, `sidecar/src/memory_bridge.py`, `crates/gateway/src/server.rs`
- Risk: Bridge regressions can pass local smoke tests while failing against the real Rust API.
- Priority: Medium

**Gateway real-time transport coverage is disabled:**
- What's not tested: WebSocket and SSE semantics beyond placeholder behavior; rate limiting is also ignored in vertical E2E.
- Files: `tests/e2e/tests/vertical/test_api_layer.rs`, `tests/e2e/tests/vertical/test_gateway_layer.rs`, `crates/gateway/src/server.rs`
- Risk: Transport regressions and pacing/security regressions ship unnoticed.
- Priority: Medium

---

*Concerns audit: 2026-04-04*
