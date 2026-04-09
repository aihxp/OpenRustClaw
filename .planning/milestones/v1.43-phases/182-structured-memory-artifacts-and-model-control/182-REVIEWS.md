---
phase: 182
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T21:00:19.626Z
plans_reviewed: [182-01-PLAN.md, 182-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 182

## Claude Review

# Cross-AI Review: Phase 182 — Structured Memory Artifacts and Model Control

## 182-01-PLAN: Durable Typed Artifact Storage and Projection

### Summary

Solid plan that introduces a well-scoped artifact layer with clear boundaries. The decision to project into reserved core-memory keys rather than widening the prompt builder is the right call — it preserves the existing trust boundary without introducing a second injection path. The two-task split (contracts first, then storage) is clean.

### Strengths

- Reuses existing `MemoryPolicies` seam instead of inventing a parallel policy path
- Projection bounded to exactly four reserved keys — prevents prompt bloat by construction
- Superseding semantics (new promotion deactivates old) avoids unbounded artifact accumulation
- Lineage preservation through mutations enables safe downstream learning in Phase 183
- Threat model is concrete and maps directly to mitigations

### Concerns

- **MEDIUM**: No migration rollback story. If `memory_model_artifacts` schema needs revision in a later phase, there's no mention of how migrations compose. The repo uses `crates/db/src/migrate.rs` — confirm it supports incremental migrations cleanly.
- **MEDIUM**: Projection sync timing is underspecified. If promotion and core-memory sync aren't transactional (single SQLite transaction), a crash between them leaves stale projection. The plan says "sync" but doesn't say "in the same transaction."
- **LOW**: `clip_excerpt` in `context.rs` already clips recall content. The plan introduces summary clipping for projection — confirm these use the same clipping logic or document why they differ.
- **LOW**: The plan lists `crates/memory/src/model_artifacts.rs` as a new file but doesn't specify whether it re-exports through `crates/memory/src/lib.rs` as a public module or stays internal. The `lib.rs` is in `files_modified` but the action text is vague.

### Suggestions

- Specify that promotion + superseding + core-memory sync happen in a single SQLite transaction to prevent partial-write inconsistency
- Add a test for the race case: two concurrent promotions of the same artifact kind/namespace should result in exactly one active artifact
- Consider adding a `projected_at` timestamp on the artifact row so operators can verify projection freshness during inspection (Phase 182-02 will need this)
- The `evaluate_model_artifact_promotion` function in `policies.rs` already exists (lines 224-269) — the plan should explicitly note this is being extended, not created from scratch

### Risk Assessment

**LOW**. The scope is well-contained, the projection boundary is sound, and the existing policy seam already has the promotion evaluation stub. Main risk is the transactional gap in projection sync.

---

## 182-02-PLAN: Operator CLI, Inspect, and MCP Surfaces

### Summary

Reasonable extension plan that wires the 182-01 artifact layer into existing operator surfaces. The dependency on 182-01 is correctly declared. The main concern is scope — touching four CLI files plus MCP registration in `start.rs` is a lot of surface for one plan, and `start.rs` is flagged as a compatibility-heavy hotspot in CLAUDE.md.

### Strengths

- Extends existing surfaces rather than creating new command families
- Mutations immediately resync projection — prevents stale-context drift
- Typed payloads for MCP responses keep the bounded-response contract intact
- Tests validate the full promote → correct → deactivate → remove lifecycle

### Concerns

- **HIGH**: `crates/cli/src/commands/start.rs` is explicitly called out in CLAUDE.md as a "compatibility-heavy surface" that should be treated carefully. The plan adds MCP handlers there but doesn't acknowledge this constraint or explain why the change is bounded enough to be safe. This needs explicit justification.
- **MEDIUM**: The test targets reference specific test names (`mcp_server_memory_tools_persist_and_render`) that may not exist yet. If 182-01 doesn't create that test fixture, 182-02 verification will fail. The dependency handoff is underspecified.
- **MEDIUM**: No error UX specification. What does the operator see when they try to deactivate an already-deactivated artifact, or remove one that's currently projected? The plan says "without dropping lineage" but doesn't define the error contract.
- **LOW**: `crates/cli/src/main.rs` is listed in `files_modified` for new subcommand registration, but the action text only mentions memory.rs and inspect.rs. The main.rs change should be explicit about what's added (e.g., new `MemoryAction` variants).

### Suggestions

- Add explicit error cases to the test behavior: duplicate deactivation, removal of non-existent artifact, promotion with invalid lineage references
- Acknowledge the `start.rs` hotspot constraint and keep the MCP handler additions isolated (e.g., register handlers through a separate function that `start.rs` calls, rather than inlining logic)
- Specify the CLI output format for artifact listing — JSON, table, or markdown — so it's consistent with existing `inspect` output patterns
- Consider splitting Task 2 (MCP handlers in `start.rs`) into its own plan if the `start.rs` changes grow beyond ~50 lines of handler registration

### Risk Assessment

**MEDIUM**. The `start.rs` hotspot risk is real — this file is large and compatibility-sensitive. The plan would benefit from an explicit strategy for keeping the MCP additions isolated. The rest is straightforward surface wiring.

---

## Overall Phase Assessment

The two plans together achieve the Phase 182 goals: durable typed artifacts, policy-gated promotion, bounded projection, and operator control. The wave ordering (01 → 02) is correct. The main gaps are:

1. **Transactional integrity** of projection sync (182-01)
2. **Hotspot management** for `start.rs` (182-02)
3. **Error contract** for operator-facing mutation failures (182-02)

None of these are blocking — they're refinements that should be addressed before execution begins.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available, and no explicit overall risk label was parsed from the completed review.
