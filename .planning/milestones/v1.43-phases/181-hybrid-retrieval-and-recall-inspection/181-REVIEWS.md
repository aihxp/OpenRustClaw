---
phase: 181
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T20:59:18.949Z
plans_reviewed: [181-01-PLAN.md, 181-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 181

## Claude Review

# Cross-AI Review: Phase 181 — Hybrid Retrieval and Recall Inspection

## Plan 181-01: Hybrid Scoring, Live Vector Path, Bounded Recall Packs

### Summary

A well-scoped plan that addresses real, verified debt (flattened BM25 scores, orphaned `search_with_embedding`, unbounded recall output). The four tasks follow a logical progression from gates → scoring → caller wiring → assembly. The plan stays disciplined about scope guardrails.

### Strengths

- Wave 0 gate task forces explicit decisions before coding — prevents the "assume libSQL works" trap identified in research
- TDD-first approach with concrete test behaviors, not vague "add tests"
- Degraded-state surfacing for missing embeddings is a genuine operational improvement
- Reuses existing `EmbeddingService` and `ToolFactory` seams rather than inventing new plumbing
- Threat model is proportionate — no over-engineering of security for an internal ranking change

### Concerns

- **MEDIUM**: Task 0 (Wave 0 gates) is underspecified as an implementation task. It says "resolve" and "choose" but doesn't define what artifacts prove resolution. A spike that produces no code or tests is hard to verify with `cargo test`. Consider making it produce a concrete type definition (the explanation struct) as its deliverable.
- **MEDIUM**: The plan modifies `crates/gateway/src/server.rs` but the gateway's internal memory search handler is a large, compatibility-heavy file. The plan doesn't specify how narrowly scoped the gateway change is — risk of touching more than needed.
- **LOW**: `ScoredMemory` currently has `pub score: f32` plus `pub explanation: RetrievalExplanation`. The plan's `<interfaces>` section shows the old shape without `explanation`, suggesting the interfaces block is stale relative to the actual codebase (which already has `RetrievalExplanation`, `RetrievalArtifactKind`, `RetrievalVectorLane`, etc.). This matters because Task 1 may duplicate work that's partially landed.
- **LOW**: Batch access-count updates are mentioned in research/concerns but not explicitly tasked. The N+1 write pattern in `memory_store.rs` is a known perf issue that could be fixed alongside the scoring refactor.
- **LOW**: No explicit rollback strategy if the hybrid scoring changes degrade recall quality in practice. Consider a feature flag or weight config.

### Suggestions

- Merge Task 0 into Task 1 — the "gate" is really just "define the explanation type and decide on Rust-side rescoring," which Task 1 does anyway. Two separate tasks for what's effectively one design decision adds overhead.
- Add batch access-count updates to Task 1 since you're already rewriting the scoring path in `memory_store.rs`.
- Update the `<interfaces>` block to reflect that `ScoredMemory` already carries `RetrievalExplanation` — the existing types in `crates/core/src/types.rs` are more advanced than the plan acknowledges.
- Specify the gateway change scope: "only modify `internal_memory_search_handler` to pass embedding service through" rather than leaving it open-ended.

### Risk Assessment: **LOW-MEDIUM**

The core changes (scoring, caller wiring, assembly) are well-understood and local. The main risk is the stale interfaces section causing implementers to re-invent types that already exist, wasting time. The scope guardrails are strong.

---

## Plan 181-02: Inspection, Telemetry, and Operator Surfaces

### Summary

A reasonable Wave 2 plan that extends existing inspect/control/MCP surfaces with retrieval explanation data. Appropriately scoped to rendering and telemetry rather than new subsystems. However, the tasks are vague about what "extend" means concretely for the large `start.rs` file.

### Strengths

- Correctly depends on 181-01 for the typed explanation contracts
- Reuses existing `MemoryViewsService`, `inspect.rs`, and `runtime_events` instead of building new plumbing
- Explicit scope exclusions for Phase 182+ work
- The recall view round-trip testing pattern (render → parse → verify fields) already exists in `memory_views.rs` and the plan builds on it

### Concerns

- **MEDIUM**: Task 2 touches `crates/cli/src/commands/start.rs` — a multi-thousand-line compatibility-heavy file that CLAUDE.md explicitly warns about. The plan says "reuse existing control routes" but doesn't specify which handler or how narrowly the change is bounded. This is the highest-risk file in the plan.
- **MEDIUM**: `Event::MemorySearched` enrichment in `crates/scheduler/src/eventing.rs` could break existing event consumers if the payload shape changes. The plan doesn't address backward compatibility of the event schema.
- **LOW**: Task 1 mentions updating `crates/cli/src/commands/memory.rs` but the test command `cargo test -p openrustclaw-cli memory` may match too broadly. Should target specific test functions.
- **LOW**: The plan doesn't specify whether retrieval explanations in the control API are behind the existing internal-token auth or if new auth consideration is needed. Research says to preserve existing auth — worth making explicit in the task.

### Suggestions

- For Task 2, name the specific handler(s) in `start.rs` that will be modified (e.g., `service_runtime_events_handler`) and commit to not adding new routes.
- Address `Event::MemorySearched` backward compatibility: either make the new fields optional/additive or note that existing consumers will need updates.
- Add an explicit note that all new control/inspect endpoints inherit existing auth middleware — don't leave it implicit.
- Consider whether Task 1 (view model updates) is already partially done: `RecallMemoryItem` in the current codebase already has `score`, `importance`, `confidence`, `artifact_kind`, `source_type`, `source_label`, `vector_lane`, `degraded_reason`, and many other explanation fields. The plan may be proposing work that's already landed.

### Risk Assessment: **LOW-MEDIUM**

The scope is appropriate but the `start.rs` touch and event-schema change carry moderate integration risk. The biggest concern is that the plan may underestimate how much of the view-model work already exists in `memory_views.rs`.

---

## Cross-Plan Observations

1. **Both plans underestimate existing progress.** The codebase already has `RetrievalExplanation`, `RetrievalArtifactKind`, `RetrievalVectorLane`, `RecallPack`, `build_recall_pack()`, degraded-state tracking, and rich `RecallMemoryItem` fields. The plans read as if these don't exist yet. Implementers should inventory what's already landed before writing new code.

2. **The `memory_tools.rs` caller path is partially wired.** `MemorySearchTool` already accepts `Option<Arc<EmbeddingService>>`, calls `embed_query`, falls back to lexical with a warning, and renders recall packs with degraded-state metadata. Task 2 of Plan 01 may be largely complete.

3. **Overall phase risk: LOW.** The remaining real work appears to be: (a) fix the `bm25_score = 1.0` flattening in `memory_store.rs`, (b) batch access-count updates, (c) enrich `Event::MemorySearched` payloads, and (d) wire explanation data into inspect/control surfaces. This is smaller than the plans suggest.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available, and no explicit overall risk label was parsed from the completed review.
