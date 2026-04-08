# Stack Research: v1.43 Learning Loop, Memory Depth, and God Mode

**Scope:** deeper memory retrieval and consolidation, durable user and operator modeling, bounded self-learning into lessons and reusable skill improvements, and a distinct God Mode runtime lane
**Researched:** 2026-04-07
**Confidence:** HIGH for repo fit and storage choices, MEDIUM for third-party retrieval vendor defaults

## Current Reusable Stack

### Existing Rust-first seams to keep

- `crates/core/src/traits.rs`
  - Already exposes the right long-lived abstractions: `MemoryStore`, `CoreMemoryStore`, and `EmbeddingProvider`.
  - v1.43 should extend these traits instead of introducing a second memory runtime.
- `crates/db/src/memory_store.rs`
  - Already has the right high-level shape: FTS5 recall, vector embeddings, temporal shaping, and hybrid reranking.
  - The problem is implementation depth, not missing architecture. `search_with_embedding()` still hard-codes BM25 to `1.0` and does per-result access writes, so the current stack is correct but incomplete.
- `crates/db/migrations/001_initial.sql`
  - Already commits the product to SQLite plus libSQL-compatible vectors through `memory_entries`, `memory_fts`, `memory_vectors`, and `memory_archive`.
  - This is the correct persistence center for deeper memory. Do not move memory into a separate vector service.
- `crates/optimization/src/runner.rs`
  - Already provides a bounded mutation and evaluation loop with workspace copy, policy limits, and LangSmith trace hooks.
  - This is the right foundation for skill-improvement proposals and reviewable learning experiments.
- `crates/cli/src/commands/orchestrate.rs`, `crates/cli/src/commands/control.rs`, and `crates/app/src/autonomy_lessons_control.rs`
  - Already provide reflection candidates, decision lessons, and autonomy policy surfaces.
  - v1.43 should add durable candidate storage and better evaluation, not a brand new learning control plane.

### Existing sidecar seams to keep, but tighten

- `sidecar/src/workflows/memory_maintenance.py`
  - Keep LangGraph scheduling and orchestration here because project policy already says scheduling stays in `sidecar/`.
  - Remove mock fallback behavior from the shipped lane. The current summarization and embedding nodes silently degrade to mock summaries and zero vectors.
- `sidecar/src/evaluators/memory_recall.py`
  - Keep this as a lightweight workflow-quality check.
  - Treat it as a smoke evaluator, not the final retrieval metric system.

## Recommended Additions Or Extensions

### 1. Make libSQL-native vector retrieval the primary memory indexing path

Use the existing SQLite plus libSQL split more fully instead of scanning BLOB vectors in Rust.

Recommended stack choice:

- Keep `sqlx` for relational storage and FTS5 filtering in `crates/db`.
- Keep `libsql` as the vector execution path.
- Add a migration that upgrades `memory_vectors` from opaque BLOB scan usage to native vector-indexed query usage.

Implementation choice:

- Candidate generation should become:
  - FTS5 candidate set from `memory_entries` and `memory_fts`
  - vector candidate set from libSQL native vector search
  - fusion in Rust with explicit score components
  - final optional MMR pass in `crates/memory`
- Do not keep the current pattern where `crates/db/src/memory_store.rs` fetches rows, assigns `bm25_score = 1.0`, and computes all vector similarity in-process.

Why:

- This preserves the repo’s existing DB contract.
- It fixes the main retrieval weakness without adding another database.
- Turso and libSQL now expose vector similarity search as a native feature, which makes a libSQL-first upgrade lower-risk than adopting Qdrant, Weaviate, or pgvector in a SQLite product.

Concrete schema direction:

- Keep `memory_entries` as the canonical fact row.
- Replace or augment `memory_vectors` with a vector-native table keyed by memory artifact id.
- Add typed score columns or metadata fields for:
  - lexical score
  - vector score
  - recency score
  - importance boost
  - confidence penalty
  - final fused score

Recommended retrieval artifacts:

- `memory_artifacts`
  - `id`
  - `artifact_kind` (`episodic`, `semantic`, `procedural`, `summary`, `user_fact`, `operator_fact`, `lesson`, `skill_hint`)
  - `review_status` (`candidate`, `approved`, `rejected`, `expired`, `superseded`)
  - `scope_kind` and `scope_id`
  - `canonical_text`
  - `source_run_id`
  - `parent_artifact_id`
  - `importance`
  - `confidence`
  - `created_at`
  - `expires_at`
  - `superseded_by`
- `memory_artifact_vectors`
  - one vector per approved or searchable artifact
  - pinned `model_id`
  - pinned `dimensions`

This keeps raw events, consolidated summaries, durable user facts, and approved lessons queryable through one retrieval surface.

### 2. Add a retrieval-only provider layer for embeddings and reranking

The existing provider stack is enough for generation, but v1.43 needs a retrieval-specialized lane.

Recommended stack choice:

- Add `cohere` as a dependency of `openrustclaw-providers` for retrieval only.
- Keep `async-openai` embeddings as the fallback path when the operator does not configure Cohere.
- Expose a new provider-side trait in `crates/core` or `crates/providers` for reranking, parallel to `EmbeddingProvider`.

Why Cohere:

- The workspace already ships a native Rust Cohere SDK in `crates/cohere`.
- Cohere’s embed API distinguishes `search_document` from `search_query`, which matches OpenRustClaw’s recall use case better than a one-size-fits-all embedding call.
- Cohere also has a native rerank API, which is the simplest way to improve recall precision without pulling in a local cross-encoder service.

Concrete recommendation:

- Default retrieval profile:
  - embeddings: Cohere embed with separate document and query modes
  - rerank: Cohere `v2/rerank`
- Fallback retrieval profile:
  - embeddings: OpenAI `text-embedding-3-small` through the existing `async-openai` crate
  - rerank: disabled

Implementation notes:

- Freeze one embedding dimension per index. Do not mix dimensions in one searchable table.
- Keep all provider calls behind `crates/providers`; do not call provider HTTP APIs from `crates/memory` or `sidecar/`.
- Batch embedding generation in Rust and persist vectors through `crates/db`.

Likely Cargo-level change:

```toml
# crates/providers/Cargo.toml
cohere = { workspace = true, features = ["embeddings", "rerank"] }
```

### 3. Move learned artifacts into DB-backed review queues, not files

Current decision lessons are reviewable, but the durable learning substrate is too file-oriented and too narrow for v1.43.

Recommended stack choice:

- Keep approved operator-facing lessons in the existing control registry surfaces.
- Store raw learning candidates, user-model facts, operator-model facts, and skill-improvement proposals in `crates/db`.

Why:

- Learning candidates need ranking, dedupe, expiry, supersession, and search.
- Those behaviors fit SQLite and libSQL much better than `.claw/...` file manifests.
- File-backed lessons remain useful as the approved, inspectable, operator-edited layer.

Concrete storage pattern:

- `learning_candidates`
  - candidate id
  - source run id
  - candidate kind (`lesson`, `memory_fact`, `user_model_update`, `operator_model_update`, `skill_improvement`)
  - normalized text
  - evidence blob
  - confidence
  - acceptance status
  - trace id
- `learning_acceptances`
  - accepted by
  - accepted at
  - resulting artifact id
  - rollback reference
- `skill_improvement_candidates`
  - target skill id or path
  - proposal kind (`prompt_patch`, `schema_patch`, `wasm_candidate`, `workflow_patch`)
  - mutation payload
  - eval suite id
  - promotion status

This lets v1.43 support self-improvement without hidden self-modification.

### 4. Keep consolidation in LangGraph, but make Rust own persistence and embedding generation

The sidecar should continue to schedule and orchestrate maintenance, but the authoritative memory state should stay in Rust.

Recommended stack choice:

- LangGraph sidecar for workflow control only.
- Rust `crates/db` and `crates/memory` for:
  - durable summary persistence
  - embedding generation requests
  - archive promotion
  - artifact supersession

Why:

- The repo already treats the Python lane as a bounded workflow sidecar.
- The current sidecar memory maintenance flow can silently degrade into mock summaries and mock embeddings, which is the wrong trust model for deeper memory.

Implementation choice:

- Sidecar nodes should request summarization plans and reviewable summaries.
- Rust should persist final archive rows and vectors.
- If required provider dependencies are missing, the workflow should fail closed instead of generating mock outputs.

This keeps the scheduling rule intact while avoiding a second, weaker memory stack.

### 5. Reuse the optimization and LangSmith stack for evaluation instead of adopting a new eval platform

v1.43 needs more evaluation, but it does not need a second experiment system.

Recommended stack choice:

- Keep `crates/optimization` as the bounded experiment runner for skill improvements.
- Keep `openrustclaw_observability` OTLP support as the tracing base.
- Keep LangSmith as the experiment and trace sink.
- Add DB-backed or checked-in eval datasets specific to memory, learning, and God Mode policy behavior.

Why:

- `crates/optimization/src/runner.rs` already enforces mutation boundaries and records evaluations.
- LangSmith now supports OpenTelemetry-based tracing and experiment workflows, which fits the current OTLP setup better than adding another SaaS-only eval framework.
- The repo already has sidecar LangSmith hooks and a memory recall evaluator; the missing piece is dataset quality and metric breadth.

Concrete evaluation additions:

- `memory_eval_cases`
  - query
  - expected artifact ids
  - forbidden artifact ids
  - scope
  - gold answer or gold recall notes
- `learning_eval_cases`
  - candidate input
  - expected acceptance decision
  - expected artifact kind
  - regression tags
- `god_mode_eval_cases`
  - requested action
  - expected warnings
  - expected audit fields
  - expected rollback surface

Concrete metrics to add:

- recall@k
- leakage rate
- summary compression ratio
- lesson adoption precision
- skill proposal acceptance rate
- skill proposal post-eval pass rate
- God Mode audit completeness
- God Mode kill-switch recovery success

Implementation choice:

- Keep fast deterministic metrics in Rust where possible.
- Keep the current `sidecar/src/evaluators/memory_recall.py` for smoke scoring.
- Log final scores and experiments to LangSmith, not just raw traces.

### 6. Build God Mode as a typed execution mode plus append-only audit tables

Do not add a new autonomy framework for God Mode. Add a stronger policy and audit lane to the existing runtime.

Recommended stack choice:

- Extend `AppConfig`, autonomy policy models, and control routes with an explicit `execution_mode` or `power_mode`.
- Persist God Mode activations and actions in append-only DB tables under `crates/db`.
- Reuse existing scheduler, agent runtime, gateway auth, and audit surfaces.

Concrete storage pattern:

- `power_mode_sessions`
  - session id
  - mode (`standard`, `full_autonomy`, `god_mode`)
  - enabled by
  - reason
  - warnings acknowledged
  - started at
  - ended at
  - rollback token or recovery reference
- `power_mode_actions`
  - action id
  - session id
  - tool or workflow name
  - requested capability set
  - outcome
  - trace id
  - audit payload

Why:

- God Mode is mostly a trust, audit, and reversibility problem, not a new model-runtime problem.
- The project already has operator-gated autonomy and broad control surfaces. v1.43 should deepen that lane, not fork it.

Implementation choice:

- All God Mode activity should produce durable receipts in `crates/db`.
- The control plane can still expose summaries and toggles from file-backed or HTTP routes, but the source of truth for action history should be SQLite.
- Reuse existing `audit_log` and `runtime_events` where practical, but add typed tables for God Mode sessions and action receipts instead of overloading generic log blobs.

## Storage And Indexing Pattern Recommendation

Use one three-layer memory substrate inside `crates/db`.

### Layer 1: Immediate recall

- Backing tables: `memory_entries`, `memory_fts`, vector index table
- Content: recent episodic and procedural artifacts
- Indexes:
  - FTS5 over `canonical_text`
  - vector index over approved embeddings
  - namespace and scope filters

### Layer 2: durable profiles and lessons

- Backing tables: `memory_artifacts`, `learning_candidates`, accepted lesson rows
- Content:
  - durable user facts
  - durable operator preferences
  - approved decision lessons
  - accepted skill hints
- Indexes:
  - vector index
  - `scope_kind`, `scope_id`
  - `artifact_kind`
  - `review_status`

### Layer 3: archive and provenance

- Backing tables: `memory_archive`, supersession links, source-run receipts
- Content:
  - compressed summaries
  - evidence pointers
  - artifact lineage
- Indexes:
  - namespace
  - created_at
  - parent and child relationships

This gives the product one searchable memory graph without violating the “recall-only memory” rule.

## What Should Stay Out Of Scope

- External vector databases such as Qdrant, Weaviate, Pinecone, or pgvector-backed Postgres
  - OpenRustClaw is already SQLite plus libSQL-first. Adding a second database would increase operational complexity for marginal gain at this milestone.
- A sidecar-owned memory database
  - Memory persistence should remain in `crates/db`.
- Automatic production self-modification of Rust code, WASM skills, or prompt assets without review
  - learning should produce candidates and bounded experiments, not hidden patching
- AgentFS adoption for God Mode in v1.43
  - AgentFS is interesting for future isolated execution, but it is still beta and is not necessary to ship a first explicit God Mode lane
- A new autonomy agent framework or planner runtime
  - the repo already has autonomy policy, lessons, and scheduler infrastructure
- Model fine-tuning pipelines
  - v1.43 is about better retrieval, better summaries, and better bounded adaptation, not training infrastructure
- Remote execution fleet management
  - Hermes-style remote execution can be a later milestone, but it should not be coupled to memory-depth work

## Recommended Near-Term Dependency Delta

### Add

- `cohere` in `openrustclaw-providers` with `embeddings` and `rerank`
- new DB migrations in `crates/db/migrate.rs` and `crates/db/migrations/`
- typed retrieval or rerank traits in `crates/core` or `crates/providers`

### Keep

- `sqlx`
- `libsql`
- `rusqlite`
- `tokio`
- `openrustclaw-observability`
- LangGraph sidecar scheduling
- LangSmith tracing and experiment export

### Do not add

- Qdrant client crates
- Weaviate client crates
- a separate Redis cache just for memory search
- a local cross-encoder model-serving process
- a new eval SaaS on top of LangSmith

## Sources

### Repo sources

- `crates/core/src/traits.rs`
- `crates/db/src/memory_store.rs`
- `crates/db/migrations/001_initial.sql`
- `crates/optimization/src/runner.rs`
- `sidecar/src/workflows/memory_maintenance.py`
- `sidecar/src/evaluators/memory_recall.py`
- `crates/cli/src/commands/orchestrate.rs`
- `crates/app/src/autonomy_lessons_control.rs`

### Official sources

- Turso AI and Embeddings docs: https://docs.turso.tech/features/ai-and-embeddings
  - Native vector search support in Turso and libSQL. Confidence: HIGH.
- Cohere Embed API docs: https://docs.cohere.com/reference/embed
  - Distinct `search_document` and `search_query` modes plus configurable output dimensions. Confidence: HIGH.
- Cohere Rerank API docs: https://docs.cohere.com/reference/rerank
  - Hosted rerank endpoint appropriate for top-N recall refinement. Confidence: HIGH.
- LangSmith OpenTelemetry tracing docs: https://docs.langchain.com/langsmith/trace-with-opentelemetry
  - Confirms OTLP-based tracing and fan-out support, which aligns with `openrustclaw_observability`. Confidence: HIGH.
- Turso AgentFS docs: https://docs.turso.tech/agentfs/introduction
  - Useful future option for isolated execution, but explicitly beta today. Confidence: HIGH.
