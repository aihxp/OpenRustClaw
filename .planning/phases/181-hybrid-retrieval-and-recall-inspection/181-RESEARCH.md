# Phase 181: Hybrid Retrieval and Recall Inspection - Research

**Researched:** 2026-04-08
**Domain:** Rust-owned recall ranking, bounded recall assembly, and operator inspection for OpenRustClaw memory retrieval [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/memory/src/context.rs]
**Confidence:** MEDIUM [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/agent/src/memory_tools.rs]

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### Retrieval ranking
- **D-01:** Phase 181 should harden the existing Rust-owned retrieval path instead of introducing a second memory subsystem.
- **D-02:** Ranking should combine lexical, vector, recency, confidence, and importance signals rather than flattening BM25 or relying on one opaque score.
- **D-03:** Retrieval quality and explainability come before retrieval breadth; do not widen default recall depth until ranking debt is reduced.

### Recall assembly and prompt boundaries
- **D-04:** Retrieved memory should be assembled into concise, deduplicated output with provenance, freshness, and artifact-type metadata.
- **D-05:** The recall-only memory contract remains fixed: raw memory files, raw archive blobs, and large undifferentiated recall dumps must not be injected into the system prompt.
- **D-06:** Recall output should stay tool-driven and bounded, with compact summaries rather than raw storage payloads.

### Inspection and observability
- **D-07:** Operators need an inspectable “why this memory appeared” lane that exposes ranking factors and contributing source artifacts.
- **D-08:** Retrieval changes should produce durable inspection or telemetry artifacts that make regressions debuggable across sessions.

### Scope guardrails
- **D-09:** Phase 181 should not add structured user/operator/project model stores; that belongs to Phase 182.
- **D-10:** Phase 181 should not activate learning-candidate promotion, skill proposals, or God Mode behavior; later phases depend on trustworthy retrieval first.

### Claude's Discretion
- Exact fusion formula, weighting, and normalization strategy across ranking signals
- The most appropriate operator-facing inspection surface shape across CLI, Control UI, or control APIs
- Whether retrieval telemetry lands as an existing memory inspection extension or a new dedicated retrieval-event artifact

### Deferred Ideas (OUT OF SCOPE)
- Structured user, operator, project, and archive model artifacts — Phase 182
- Learning candidate capture, promotion, and rollback — Phase 183
- Skill proposal generation and verification — Phase 184
- God Mode overlay, audit, and recovery — Phase 185
- Graph-memory browsing or dashboard-heavy retrieval UX — out of scope for this milestone slice
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RETR-01 | Recall retrieval combines lexical, vector, recency, confidence, and importance signals instead of relying on a flattened or opaque rank. | Preserve real FTS5 rank, activate an actual vector-aware caller path, and fuse explicit score components in `crates/db/src/memory_store.rs` instead of `bm25_score = 1.0`. [VERIFIED: crates/db/src/memory_store.rs][CITED: https://sqlite.org/fts5.html] |
| RETR-02 | Retrieved memory results are assembled into concise, deduplicated recall output with provenance, freshness, and artifact-type metadata. | Add a bounded recall assembly layer in the existing Rust memory/app seam and return structured summaries instead of the current plain-text memory list. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/app/src/memory_views.rs] |
| RETR-03 | Operators can inspect why a memory was surfaced, including the ranking factors and source artifacts that contributed to the result. | Reuse `inspect.rs`, control routes, MCP handlers, and `runtime_events` for typed retrieval explanations instead of a parallel inspection system. [VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/start.rs][VERIFIED: crates/scheduler/src/eventing.rs] |
| RETR-04 | The runtime keeps the recall-only memory contract by using bounded recall summaries and never injecting raw memory files or raw archive blobs into the system prompt. | Preserve the current core-only prompt injection boundary and keep deeper recall tool-driven. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/prompt.rs][VERIFIED: crates/memory/src/recall.rs][VERIFIED: AGENTS.md] |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Prefer provider crates and `openrustclaw-providers` abstractions instead of ad hoc raw HTTP in feature code. [VERIFIED: ./CLAUDE.md]
- Keep database access routed through the established persistence crates and typed runtime services. [VERIFIED: ./CLAUDE.md]
- Memory writes go through the memory-policy layer so dedupe, confidence, and TTL rules stay consistent. [VERIFIED: ./CLAUDE.md][VERIFIED: crates/memory/src/policies.rs]
- No cron jobs; use the durable scheduler in `crates/scheduler` and shipped scheduling surfaces. [VERIFIED: ./CLAUDE.md]
- Security-sensitive surfaces must preserve origin validation, token checks, and bounded runtime trust by default. [VERIFIED: ./CLAUDE.md]
- The active memory model remains Core -> Recall -> Archive. [VERIFIED: ./CLAUDE.md][VERIFIED: AGENTS.md]
- Avoid injecting full memory files directly into prompts when a bounded memory surface already exists. [VERIFIED: ./CLAUDE.md][VERIFIED: AGENTS.md]
- Prefer `openrustclaw-app` for new application-level business logic. [VERIFIED: ./CLAUDE.md]
- Treat large CLI adapter files such as `crates/cli/src/commands/start.rs` as compatibility-heavy surfaces; keep new logic bounded and pushed down into app/db/memory layers. [VERIFIED: ./CLAUDE.md]
- `cargo fmt`, `cargo clippy -- -D warnings`, and strict warning-free Rust builds are project conventions. [VERIFIED: ./CLAUDE.md][VERIFIED: Cargo.toml]

## Summary

Phase 181 should be planned as a hardening pass over the current Rust memory path, not a new retrieval subsystem. The existing seams already route recall through `SqliteMemoryStore`, `openrustclaw-memory`, `AgentRuntime`, CLI inspection, control APIs, and MCP handlers, and the context file explicitly locks that direction in as a decision. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: crates/cli/src/commands/start.rs][VERIFIED: crates/agent/src/memory_tools.rs]

The main implementation debt is concrete and local. `MemoryStore::search` is documented as hybrid, but both `search()` and `search_with_embedding()` currently flatten lexical relevance with `bm25_score = 1.0`, omit any MMR pass, and increment access counts one row at a time. The only vector-aware path is `search_with_embedding()`, and no current caller invokes it, which means the shipped tool, gateway, CLI, and MCP memory-search flows are still effectively lexical plus importance/recency shaping. [VERIFIED: crates/core/src/traits.rs][VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs][VERIFIED: crates/cli/src/commands/start.rs]

The plan should therefore center on four deliverables: real score fusion inside the existing DB seam, a bounded recall assembly layer that preserves the recall-only prompt contract, an inspectable explanation object that existing operator surfaces can render, and durable retrieval telemetry that fits the current `runtime_events`/inspection model. The only seam that needs explicit pre-plan validation is libSQL: the workspace depends on `libsql`, but the current runtime path uses `sqlx::SqlitePool` and has no live `libsql` call sites today. [VERIFIED: Cargo.toml][VERIFIED: crates/db/src/lib.rs][VERIFIED: crates/db/src/pool.rs][VERIFIED: crates/scheduler/src/eventing.rs][VERIFIED: crates/cli/src/commands/inspect.rs]

**Primary recommendation:** Extend `crates/db/src/memory_store.rs`, `openrustclaw-memory`, `openrustclaw-app`, and the existing CLI/control/MCP inspection routes; do not add a parallel retrieval lane. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: ./CLAUDE.md]

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `openrustclaw-db` | Workspace `1.4.1`. [VERIFIED: Cargo.toml] | Owns recall persistence, FTS tables, archive rows, and the current retrieval hotspot in `SqliteMemoryStore`. [VERIFIED: crates/db/src/lib.rs][VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/db/src/migrate.rs] | The runtime already wires this store into chat, sessions, gateway, and start-up code, so Phase 181 should deepen it instead of replacing it. [VERIFIED: crates/cli/src/commands/chat.rs][VERIFIED: crates/cli/src/commands/session.rs][VERIFIED: crates/cli/src/commands/start.rs] |
| `sqlx` | Workspace pin `0.8`; crates.io stable `0.8.6` updated 2025-10-15. [VERIFIED: Cargo.toml][VERIFIED: crates.io API] | Current async SQLite pool, migrations, and query execution path. [VERIFIED: crates/db/src/pool.rs][VERIFIED: crates/db/src/migrate.rs] | All live runtime DB access in the current path goes through `sqlx::SqlitePool`. [VERIFIED: crates/db/src/pool.rs][VERIFIED: crates/db/src/memory_store.rs] |
| SQLite FTS5 | Built into the current schema through `memory_fts` and `memory_fts_mapping`. [VERIFIED: crates/db/src/migrate.rs][VERIFIED: crates/db/migrations/001_initial.sql] | Lexical candidate generation with configurable `bm25()` and `rank` ordering. [CITED: https://sqlite.org/fts5.html] | The repo already stores recall text in FTS5; the debt is that the current code ignores the returned lexical score. [VERIFIED: crates/db/src/memory_store.rs][CITED: https://sqlite.org/fts5.html] |
| `openrustclaw-memory` | Workspace `1.4.1`. [VERIFIED: Cargo.toml] | Owns recall/core/archive policy, context boundaries, and the explicit “never auto-inject recall” contract. [VERIFIED: crates/memory/src/recall.rs][VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/memory/src/policies.rs] | Phase 181 must preserve this boundary while adding bounded recall assembly. [VERIFIED: AGENTS.md][VERIFIED: crates/agent/src/prompt.rs] |
| `openrustclaw-app` | Workspace `1.4.1`. [VERIFIED: Cargo.toml] | Existing typed application-service seam and current memory view renderer. [VERIFIED: crates/app/src/memory_views.rs] | `CLAUDE.md` explicitly says new application-level logic should prefer `openrustclaw-app`, which fits retrieval assembly and explanation rendering. [VERIFIED: ./CLAUDE.md] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `libsql` | Workspace pin `0.6`; crates.io stable `0.9.30` updated 2026-03-19. [VERIFIED: Cargo.toml][VERIFIED: crates.io API] | Declared vector seam for native libSQL/Turso execution. [VERIFIED: crates/db/src/lib.rs][VERIFIED: crates/db/Cargo.toml] | Use only after a Phase 181 spike proves the live Rust path can call native vector primitives; the repo currently has no runtime `libsql` call sites. [VERIFIED: crates/db/src/lib.rs][VERIFIED: crates/db/src/pool.rs][VERIFIED: `rg -n "libsql::|use libsql"` codebase search] |
| `rusqlite` | Workspace pin `0.32`; crates.io stable `0.39.0` updated 2026-03-15. [VERIFIED: Cargo.toml][VERIFIED: crates.io API] | CLI-oriented sync SQLite fallback. [VERIFIED: crates/db/Cargo.toml][VERIFIED: crates/db/src/lib.rs] | Keep it out of the runtime ranking path; use only for existing CLI fallback surfaces. [VERIFIED: AGENTS.md][VERIFIED: crates/db/src/lib.rs] |
| `DurableEventBus` + `runtime_events` | Existing repo seam. [VERIFIED: crates/scheduler/src/eventing.rs][VERIFIED: crates/db/src/migrate.rs] | Durable telemetry for retrieval/search events. [VERIFIED: crates/scheduler/src/eventing.rs] | Use for retrieval-event logging before inventing a new logging store. [VERIFIED: crates/core/src/types.rs][VERIFIED: crates/cli/src/commands/services.rs] |
| CLI/control/MCP memory inspection surfaces | Existing repo seam. [VERIFIED: crates/cli/src/commands/memory.rs][VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/start.rs] | Operator-facing rendering and APIs. [VERIFIED: crates/app/src/memory_views.rs] | Extend these surfaces for ranking explanations and bounded recall previews. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: crates/app/src/memory_views.rs] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Extending the existing Rust retrieval path. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md] | A second sidecar-owned or external retrieval subsystem. [VERIFIED: .planning/codebase/CONCERNS.md][VERIFIED: sidecar/src/workflows/rag_pipeline.py] | This conflicts with locked decision `D-01` and repeats the repo’s documented silent-degradation risk in sidecar RAG paths. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: .planning/codebase/CONCERNS.md] |

**Installation:** No new dependency should be assumed at plan time; Phase 181 can start by reusing current workspace crates and only add connection-seam work if the libSQL spike proves necessary. [VERIFIED: Cargo.toml][VERIFIED: crates/db/src/pool.rs]

**Version verification:** `sqlx` is pinned to `0.8` locally while crates.io lists stable `0.8.6`; `libsql` is pinned to `0.6` locally while crates.io lists stable `0.9.30`; `rusqlite` is pinned to `0.32` locally while crates.io lists stable `0.39.0`. Planning should prefer repo-pinned versions unless Phase 181 explicitly budgets an upgrade. [VERIFIED: Cargo.toml][VERIFIED: crates.io API]

## Architecture Patterns

### Recommended Project Structure

```text
crates/db/src/memory_store.rs          # Preserve FTS rank, add fused score components, batch access updates
crates/app/src/memory_retrieval.rs     # New app-layer assembly/explanation service [ASSUMED]
crates/memory/src/context.rs           # Keep prompt-boundary enforcement and bounded assembly helpers
crates/cli/src/commands/inspect.rs     # Reuse for explainability reports
crates/cli/src/commands/start.rs       # Route wiring only; avoid new business logic here
```

### Pattern 1: FTS-First Candidate Generation With Explainable Fusion

**What:** Keep FTS5 as the lexical candidate generator, preserve its actual `rank`, and fuse lexical, vector, recency, confidence, and importance as separate components rather than flattening them into one opaque number. [VERIFIED: crates/db/src/memory_store.rs][CITED: https://sqlite.org/fts5.html]

**When to use:** Every default recall search path used by the agent tool, gateway, CLI, and MCP memory search surfaces. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs][VERIFIED: crates/cli/src/commands/memory.rs][VERIFIED: crates/cli/src/commands/start.rs]

**Example:**

```sql
SELECT *
FROM memory_fts
WHERE memory_fts MATCH ?
AND rank MATCH 'bm25(10.0)'
ORDER BY rank;
```

Source: [CITED: https://sqlite.org/fts5.html]

### Pattern 2: Bounded Recall Assembly Outside the System Prompt

**What:** Convert scored memories into a compact recall pack with deduped content, provenance, freshness, artifact type, and score reasons, then expose that pack to tools and inspection surfaces without injecting raw recall rows or archive blobs into the system prompt. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/prompt.rs][VERIFIED: crates/memory/src/recall.rs][VERIFIED: AGENTS.md]

**When to use:** Tool-driven memory recall and operator-facing preview surfaces. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/app/src/memory_views.rs]

**Example:**

```rust
if tool_names.contains("memory_search") {
    prompt.push_str("You have a memory_search tool. Use it when you need to recall facts, ");
}
```

Source: [VERIFIED: crates/agent/src/prompt.rs]

### Pattern 3: Inspection Through Existing Typed Surfaces

**What:** Add retrieval explanation reports to existing app/inspect/control/MCP seams instead of scraping DB rows directly in delivery code. [VERIFIED: crates/app/src/memory_views.rs][VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/start.rs][VERIFIED: ./CLAUDE.md]

**When to use:** Operator “why did this surface?” views, regression debugging, and API/MCP inspection payloads. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md]

**Example:**

```rust
pub async fn memory_timeline(
    store: &SqliteMemoryStore,
    namespace: Option<&str>,
    limit: usize,
) -> Result<MemoryTimelineReport>
```

Source: [VERIFIED: crates/cli/src/commands/inspect.rs]

### Anti-Patterns to Avoid

- **Treating the current `search()` path as already hybrid:** `search_with_embedding()` exists, but nothing in the repo calls it today. [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: `rg -n "search_with_embedding\\(" codebase search]`
- **Flattening lexical relevance:** the current code selects `rank` from FTS5 and then sets `bm25_score = 1.0`, which defeats lexical explainability. [VERIFIED: crates/db/src/memory_store.rs][CITED: https://sqlite.org/fts5.html]
- **N+1 writes during recall:** both search paths issue one `UPDATE` per returned row for access counts. [VERIFIED: crates/db/src/memory_store.rs]
- **Opaque recall strings:** the agent tool and gateway currently emit plain lists with score and importance, but not provenance, freshness, or factor explanations. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs]
- **Prompt stuffing:** core memory is the only always-injected tier; recall must stay tool-driven and bounded. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/prompt.rs][VERIFIED: AGENTS.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Vector retrieval acceleration | A custom ANN engine or unbounded in-Rust full-vector scans. [VERIFIED: crates/db/src/memory_store.rs] | Native libSQL/Turso vector primitives when the seam is proven, or a small candidate set scored inside the existing store until then. [CITED: https://docs.turso.tech/sdk/ts/orm/drizzle][VERIFIED: crates/db/src/memory_store.rs] | Official libSQL/Turso docs expose `F32_BLOB`, vector indexes, `vector_distance_cos`, and `vector_top_k`, while the repo currently only stores raw BLOB vectors and scans fetched candidates. [CITED: https://docs.turso.tech/sdk/ts/orm/drizzle][VERIFIED: crates/db/src/memory_store.rs] |
| Retrieval telemetry | A new ad hoc log file for recall events. [VERIFIED: crates/scheduler/src/eventing.rs] | `runtime_events` and `DurableEventBus`, optionally with richer payloads or a dedicated retrieval projection later. [VERIFIED: crates/db/src/migrate.rs][VERIFIED: crates/scheduler/src/eventing.rs] | The repo already persists durable runtime events and has `MemorySearched` in the shared event type. [VERIFIED: crates/core/src/types.rs][VERIFIED: crates/scheduler/src/eventing.rs] |
| Operator inspection rendering | Raw SQLite dumps in CLI/UI code. [VERIFIED: crates/cli/src/commands/memory.rs] | `openrustclaw-app` report/render helpers plus `inspect.rs` and existing control/MCP routes. [VERIFIED: crates/app/src/memory_views.rs][VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/start.rs] | Project guidance explicitly prefers typed app services over direct adapter logic. [VERIFIED: ./CLAUDE.md] |
| Recall prompt injection | Raw memory rows, raw archive summaries, or memory-view markdown dumped into the system prompt. [VERIFIED: crates/cli/src/commands/memory.rs][VERIFIED: crates/memory/src/context.rs] | Bounded recall summaries plus on-demand `memory_search`. [VERIFIED: crates/memory/src/recall.rs][VERIFIED: crates/agent/src/prompt.rs] | This is a locked project rule and a phase requirement. [VERIFIED: AGENTS.md][VERIFIED: .planning/REQUIREMENTS.md] |

**Key insight:** Phase 181 does not need a broader memory stack; it needs the current memory stack to become truthful about ranking, assembly, and inspection. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][VERIFIED: crates/db/src/memory_store.rs]

## Common Pitfalls

### Pitfall 1: Preserving FTS5 but throwing away its score

**What goes wrong:** Retrieval still looks lexical because it uses FTS5, but rank explanations and lexical quality are lost because the code hard-codes `bm25_score = 1.0`. [VERIFIED: crates/db/src/memory_store.rs]

**Why it happens:** The query selects `rank`, but the implementation never normalizes or records that returned value. [VERIFIED: crates/db/src/memory_store.rs]

**How to avoid:** Preserve the raw lexical score, normalize it explicitly, and include it in the explanation payload returned with the final fused score. [VERIFIED: crates/db/src/memory_store.rs][CITED: https://sqlite.org/fts5.html]

**Warning signs:** Search results reorder when importance changes, but there is still no operator-visible lexical rationale. [VERIFIED: crates/db/src/memory_store.rs]

### Pitfall 2: Planning around a vector path that is not currently exercised

**What goes wrong:** The plan assumes shipped hybrid retrieval already exists because `search_with_embedding()` and `memory_vectors` exist. [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/db/src/migrate.rs]

**Why it happens:** The vector-aware method is present, but no current caller uses it, and `store_vector()` is only referenced in store-local tests. [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: `rg -n "search_with_embedding\\(|store_vector\\(" codebase search]`

**How to avoid:** Budget a caller-path task explicitly: either generate query embeddings before `MemoryStore::search`, or widen the trait/store interface so the default search path becomes genuinely hybrid. [VERIFIED: crates/core/src/traits.rs][ASSUMED]

**Warning signs:** Plans talk about “hybrid fusion” without a concrete source for query embeddings or a live vector index/query path. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs][ASSUMED]

### Pitfall 3: Improving recall breadth without an assembly contract

**What goes wrong:** More memories are returned, but the runtime still emits opaque strings or oversized payloads. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs]

**Why it happens:** The current tool surface returns free-form text lines, and `MemoryViewsService` only renders simple markdown headers without provenance/freshness fields. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/app/src/memory_views.rs]

**How to avoid:** Add a typed recall pack with compact excerpts, dedupe, provenance, freshness, artifact type, and factor scores before increasing default depth. [VERIFIED: .planning/REQUIREMENTS.md][ASSUMED]

**Warning signs:** Search output grows in length, but there is still no structured explanation object or bounded preview lane. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/cli/src/commands/memory.rs]

### Pitfall 4: Building new inspection plumbing instead of extending existing surfaces

**What goes wrong:** Phase 181 spends time on a new retrieval dashboard or bespoke API instead of fixing the ranking seam. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md]

**Why it happens:** The current memory timeline/archive views are too thin, so it is tempting to replace them rather than extend them. [VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/app/src/memory_views.rs]

**How to avoid:** Keep new business logic in `openrustclaw-app`, expose it through existing inspect/control/MCP routes, and leave major dashboard work deferred. [VERIFIED: ./CLAUDE.md][VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md]

**Warning signs:** A plan introduces a parallel retrieval service or dashboard-heavy surface before explaining how CLI/control/MCP consumers will reuse the same report type. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md][ASSUMED]

## Code Examples

Verified patterns from official and local sources:

### Weighted FTS5 ranking without losing explainability

```sql
SELECT * FROM ft
WHERE ft MATCH ?
AND rank MATCH 'bm25(10.0, 5.0)'
ORDER BY rank;
```

Source: [CITED: https://sqlite.org/fts5.html]

### Native libSQL/Turso vector search primitives to validate before use

```sql
CREATE INDEX IF NOT EXISTS vector_index
ON vector_table(vector)
USING vector_cosine(3);

SELECT id, distance
FROM vector_top_k('vector_index', vector32('[2.2,3.3,4.4]'), 5);
```

Source: [CITED: https://docs.turso.tech/sdk/ts/orm/drizzle]

### Existing durable retrieval-event seam

```rust
Event::MemorySearched { query, result_count }
```

Source: [VERIFIED: crates/core/src/types.rs]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Current repo path: FTS candidates plus `bm25_score = 1.0`, optional local cosine over fetched BLOB vectors, and per-result access updates. [VERIFIED: crates/db/src/memory_store.rs] | Phase 181 target: preserve FTS score, add a real vector-aware caller path, fuse explicit signals, batch writes, and return explanation objects. [VERIFIED: .planning/REQUIREMENTS.md][ASSUMED] | Planned for Phase 181. [VERIFIED: .planning/STATE.md][ASSUMED] | This is the minimum needed to satisfy `RETR-01` and `RETR-03` truthfully. [VERIFIED: .planning/REQUIREMENTS.md] |
| Current repo path: tool/gateway search outputs are free-form strings or shallow JSON with no provenance/freshness/artifact-type metadata. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs] | Phase 181 target: bounded recall packs with provenance, freshness, artifact type, and score reasons. [VERIFIED: .planning/REQUIREMENTS.md][ASSUMED] | Planned for Phase 181. [VERIFIED: .planning/STATE.md][ASSUMED] | This closes `RETR-02` and keeps recall debuggable. [VERIFIED: .planning/REQUIREMENTS.md] |
| Current repo path: prompt boundary is correct, but only because recall is still shallow and tool-driven. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/prompt.rs] | Phase 181 target: preserve the same boundary even after recall quality improves. [VERIFIED: .planning/REQUIREMENTS.md] | No change in boundary; only the assembly surface changes. [VERIFIED: AGENTS.md][ASSUMED] | This avoids raw-memory prompt stuffing while improving retrieval quality. [VERIFIED: AGENTS.md][VERIFIED: .planning/REQUIREMENTS.md] |

**Deprecated/outdated:**

- Treating the trait/comment-level phrase “hybrid BM25 + vector + MMR + temporal decay” as already implemented is outdated; the current implementation does not apply MMR and does not preserve real BM25 values. [VERIFIED: crates/core/src/traits.rs][VERIFIED: crates/db/src/memory_store.rs]
- Treating `memory_vectors` BLOB storage alone as evidence of live vector search is outdated; the current runtime never calls `search_with_embedding()` and never calls `store_vector()` outside tests. [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: `rg -n "search_with_embedding\\(|store_vector\\(" codebase search]`

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The live Rust retrieval path can be taught to call libSQL native vector primitives without a broader storage redesign. [ASSUMED] | Standard Stack; Open Questions | If false, the plan must either keep vector rescoring in Rust for Phase 181 or budget connection-seam work before hybrid fusion. |
| A2 | `artifact_type` metadata for Phase 181 can be represented from current memory classes and metadata without waiting for Phase 182 model stores. [ASSUMED] | Summary; Common Pitfalls | If false, `RETR-02` needs an interim artifact taxonomy task before assembly work. |
| A3 | A new app-layer retrieval assembly service is the cleanest place for explanation shaping, instead of only extending formatter functions. [ASSUMED] | Recommended Project Structure; Architecture Patterns | If false, work may need to stay inside `crates/memory` plus `inspect.rs`, which changes task slicing but not overall scope. |

## Open Questions

1. **Can the current Rust runtime actually execute native libSQL vector search on the live database path?**
   - What we know: the workspace depends on `libsql`, official docs expose `Builder` setup and native vector primitives, and the repo currently uses `sqlx::SqlitePool` with no runtime `libsql` calls. [VERIFIED: Cargo.toml][VERIFIED: crates/db/src/pool.rs][VERIFIED: `rg -n "libsql::|use libsql"` codebase search][CITED: https://docs.rs/libsql/latest/libsql/][CITED: https://docs.turso.tech/sdk/rust/quickstart][CITED: https://docs.turso.tech/sdk/ts/orm/drizzle]
   - What's unclear: whether Phase 181 can call vector indexes directly on the existing path or needs a new connection seam. [ASSUMED]
   - Recommendation: make this a Wave 0 spike and do not lock the fusion implementation until it is answered. [ASSUMED]

2. **Where should retrieval explanations live first: extended memory timeline reports, or a dedicated retrieval report type?**
   - What we know: existing operator surfaces already include `inspect.rs`, memory CLI commands, control memory routes, and MCP handlers. [VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/memory.rs][VERIFIED: crates/cli/src/commands/start.rs]
   - What's unclear: whether those consumers can share one typed explanation payload cleanly without adding a new service type. [ASSUMED]
   - Recommendation: design one typed explanation/report struct in `openrustclaw-app`, then let CLI/control/MCP render it differently. [VERIFIED: ./CLAUDE.md][ASSUMED]

3. **What is the minimal artifact taxonomy that satisfies `RETR-02` without pre-building Phase 182?**
   - What we know: current runtime has `memory_type`, source/source_type, archive rows, and metadata fields, but not structured user/operator/project model stores yet. [VERIFIED: crates/core/src/types.rs][VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/db/src/migrate.rs]
   - What's unclear: whether `artifact_type` should be a new retrieval-only enum or a derived display field. [ASSUMED]
   - Recommendation: keep it retrieval-facing and derived in Phase 181 unless that proves too lossy in the spike. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | Build, unit tests, integration tests, workspace validation | ✓ [VERIFIED: local command] | `1.94.0` [VERIFIED: local command] | — |
| `rustc` | Compile and clippy gates | ✓ [VERIFIED: local command] | `1.94.0` [VERIFIED: local command] | — |
| `python3` | Optional sidecar or local research helpers only | ✓ [VERIFIED: local command] | `3.14.2` [VERIFIED: local command] | Phase 181 can still proceed without using sidecar changes. [VERIFIED: .planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md] |

**Missing dependencies with no fallback:**
- None identified for planning or Rust-side implementation. [VERIFIED: local command][VERIFIED: Cargo.toml]

**Missing dependencies with fallback:**
- Native libSQL vector execution was not probed live; Phase 181 can fall back to improving lexical fusion and bounded assembly first if the spike fails. [VERIFIED: crates/db/src/pool.rs][ASSUMED]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Cargo workspace tests with `#[tokio::test]` across unit, integration, and E2E crates. [VERIFIED: Cargo.toml][VERIFIED: tests/integration/src/memory_policy_test.rs][VERIFIED: tests/e2e/tests/regression/test_memory_recall.rs] |
| Config file | None beyond Cargo workspace manifests. [VERIFIED: `rg --files` test-config scan] |
| Quick run command | `cargo test -p openrustclaw-db memory_store -- --nocapture` or a targeted crate/test command per task. [VERIFIED: crates/db/src/memory_store.rs] |
| Full suite command | `cargo test --workspace` [VERIFIED: AGENTS.md][VERIFIED: ./CLAUDE.md] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RETR-01 | Hybrid ranking preserves lexical/vector/recency/confidence/importance components and ordering. [VERIFIED: .planning/REQUIREMENTS.md] | unit + integration | `cargo test -p openrustclaw-db hybrid_retrieval -- --nocapture` [ASSUMED] | ❌ Wave 0 [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: tests/e2e/tests/regression/test_memory_recall.rs] |
| RETR-02 | Recall assembly is bounded, deduped, and includes provenance/freshness/artifact-type metadata. [VERIFIED: .planning/REQUIREMENTS.md] | unit | `cargo test -p openrustclaw-app memory_views -- --nocapture` plus new assembly tests. [VERIFIED: crates/app/src/memory_views.rs][ASSUMED] | ❌ Wave 0 [VERIFIED: crates/app/src/memory_views.rs] |
| RETR-03 | Operators can inspect why a memory surfaced through CLI/control/MCP. [VERIFIED: .planning/REQUIREMENTS.md] | integration | `cargo test -p openrustclaw-cli inspect::memory -- --nocapture` or targeted control-route tests. [VERIFIED: crates/cli/src/commands/inspect.rs][ASSUMED] | ❌ Wave 0 [VERIFIED: crates/cli/src/commands/inspect.rs][VERIFIED: crates/cli/src/commands/start.rs] |
| RETR-04 | No raw recall/archive payloads are injected into the system prompt; recall remains tool-driven. [VERIFIED: .planning/REQUIREMENTS.md] | unit + integration | `cargo test -p openrustclaw-agent prompt_describes_strict_memory_store_boundary -- --nocapture` plus new prompt-boundary tests. [VERIFIED: crates/agent/src/prompt.rs][ASSUMED] | ✅ partial [VERIFIED: crates/agent/src/prompt.rs][VERIFIED: crates/memory/src/context.rs] |

### Sampling Rate

- **Per task commit:** Run the smallest affected crate/test target plus at least one ranking or prompt-boundary test. [VERIFIED: Cargo.toml][ASSUMED]
- **Per wave merge:** Run `cargo test --workspace` or the full impacted workspace subset if runtime cost forces staging. [VERIFIED: AGENTS.md][ASSUMED]
- **Phase gate:** Full suite green before `/gsd-verify-work`. [VERIFIED: .planning/config.json]

### Wave 0 Gaps

- [ ] `crates/db/src/memory_store.rs` targeted tests for preserved FTS rank, signal normalization, and batch access updates. [VERIFIED: crates/db/src/memory_store.rs]
- [ ] New assembly tests for bounded recall pack shaping and dedupe. [VERIFIED: crates/app/src/memory_views.rs][ASSUMED]
- [ ] Control/API tests for retrieval explanations returned by inspect/control/MCP surfaces. [VERIFIED: crates/cli/src/commands/start.rs][ASSUMED]
- [ ] A regression test that proves the default prompt still excludes raw recall and raw archive payloads after Phase 181 changes. [VERIFIED: crates/memory/src/context.rs][VERIFIED: crates/agent/src/prompt.rs]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes [VERIFIED: gateway/control internal routes already validate tokens] | Preserve existing internal-token and gateway auth checks on any new inspection/control surface. [VERIFIED: crates/gateway/src/server.rs][VERIFIED: ./CLAUDE.md] |
| V3 Session Management | no direct session-issuance change in this phase. [VERIFIED: .planning/REQUIREMENTS.md] | Keep current session behavior untouched; avoid coupling retrieval explainability to session mutation. [ASSUMED] |
| V4 Access Control | yes [VERIFIED: namespace-scoped queries and internal routes already exist] | Keep namespace/user scoping in `MemoryQuery` and enforce control-surface auth for inspection APIs. [VERIFIED: crates/core/src/types.rs][VERIFIED: crates/gateway/src/server.rs] |
| V5 Input Validation | yes [VERIFIED: typed queries, serde payloads, tool schemas exist] | Use typed request structs and avoid raw SQL string interpolation for user-controlled filter values beyond the current bounded enums. [VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs][VERIFIED: crates/db/src/memory_store.rs] |
| V6 Cryptography | no new crypto should be introduced in this phase. [VERIFIED: .planning/REQUIREMENTS.md] | Reuse existing hashing/policy behavior only; never hand-roll new cryptography for retrieval metadata. [VERIFIED: crates/memory/src/policies.rs] |

### Known Threat Patterns for Rust/SQLite Recall

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Cross-namespace recall leakage through inspection surfaces | Information Disclosure | Always scope searches and retrieval explanations by namespace/user and preserve current internal-token protection on control/internal routes. [VERIFIED: crates/core/src/types.rs][VERIFIED: crates/gateway/src/server.rs] |
| Raw archive or raw memory payload injection into prompts | Information Disclosure | Keep recall tool-driven and inject only bounded summaries or core memory. [VERIFIED: crates/memory/src/recall.rs][VERIFIED: crates/memory/src/context.rs][VERIFIED: AGENTS.md] |
| Memory poisoning through low-confidence or speculative content | Tampering | Respect existing write policy and include confidence/provenance in retrieval explanations so suspicious items are inspectable. [VERIFIED: crates/memory/src/policies.rs][VERIFIED: crates/core/src/types.rs] |
| Opaque ranking decisions that hide unsafe or stale matches | Repudiation | Log retrieval events durably and return component scores/source references in explainability payloads. [VERIFIED: crates/scheduler/src/eventing.rs][ASSUMED] |

## Sources

### Primary (HIGH confidence)

- Local phase context and requirements:
  - `.planning/phases/181-hybrid-retrieval-and-recall-inspection/181-CONTEXT.md`
  - `.planning/REQUIREMENTS.md`
  - `.planning/STATE.md`
  - `.planning/research/SUMMARY.md`
  - `.planning/research/FEATURES.md`
  - `.planning/research/ARCHITECTURE.md`
  - `.planning/research/PITFALLS.md`
  - `.planning/research/STACK.md`
  - `.planning/codebase/CONCERNS.md`
  - `.planning/codebase/ARCHITECTURE.md`
- Local implementation seams:
  - `crates/db/src/memory_store.rs`
  - `crates/db/src/pool.rs`
  - `crates/db/src/migrate.rs`
  - `crates/core/src/traits.rs`
  - `crates/core/src/types.rs`
  - `crates/memory/src/context.rs`
  - `crates/memory/src/recall.rs`
  - `crates/memory/src/policies.rs`
  - `crates/agent/src/prompt.rs`
  - `crates/agent/src/memory_tools.rs`
  - `crates/app/src/memory_views.rs`
  - `crates/cli/src/commands/memory.rs`
  - `crates/cli/src/commands/inspect.rs`
  - `crates/cli/src/commands/start.rs`
  - `crates/gateway/src/server.rs`
  - `crates/scheduler/src/eventing.rs`
- Official documentation:
  - SQLite FTS5 docs: https://sqlite.org/fts5.html
  - libSQL Rust docs: https://docs.rs/libsql/latest/libsql/
  - Turso Rust quickstart: https://docs.turso.tech/sdk/rust/quickstart
  - Turso vector/native embeddings examples: https://docs.turso.tech/sdk/ts/orm/drizzle

### Secondary (MEDIUM confidence)

- crates.io API for version verification:
  - `https://crates.io/api/v1/crates/libsql`
  - `https://crates.io/api/v1/crates/sqlx`
  - `https://crates.io/api/v1/crates/rusqlite`

### Tertiary (LOW confidence)

- None. All operational claims in this research were either verified locally or cited to official documentation. [VERIFIED: local research session]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - the phase should reuse the existing repo-owned DB/memory/app/inspect stack, and those seams are directly verified in code and docs. [VERIFIED: Cargo.toml][VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/app/src/memory_views.rs]
- Architecture: MEDIUM - the extension points are clear, but the live libSQL vector seam and final explanation-surface shape still need a short spike. [VERIFIED: crates/db/src/pool.rs][VERIFIED: `rg -n "libsql::|use libsql"` codebase search][ASSUMED]
- Pitfalls: HIGH - the ranking debt, prompt boundary, unused vector path, and thin inspection surfaces are directly observable in the current code. [VERIFIED: crates/db/src/memory_store.rs][VERIFIED: crates/agent/src/memory_tools.rs][VERIFIED: crates/gateway/src/server.rs][VERIFIED: crates/memory/src/context.rs]

**Research date:** 2026-04-08
**Valid until:** 2026-05-08 for local-code findings; re-verify crate/doc version claims if planning starts after that date. [VERIFIED: local research session][ASSUMED]
