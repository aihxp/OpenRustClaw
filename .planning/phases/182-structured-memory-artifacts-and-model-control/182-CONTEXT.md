# Phase 182: Structured Memory Artifacts and Model Control - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Promote durable user, operator, project, and archive artifacts through bounded policy-gated memory flows. This phase is about structuring durable memory artifacts, projecting only a bounded high-signal subset into active runtime context, and giving operators typed control over those artifacts. It is not yet about learning-candidate promotion, skill proposal generation, or God Mode authority changes.

</domain>

<decisions>
## Implementation Decisions

### Structured artifact modeling
- **D-01:** Phase 182 should reuse the Rust-owned memory and artifact seams that already exist instead of creating a parallel profile store or dashboard-only model layer.
- **D-02:** User model, operator model, project memory, and archive summaries should become distinct durable artifact classes with explicit kind, source lineage, and lifecycle state.
- **D-03:** Durable model artifacts should be promoted from existing memory evidence or summaries through explicit policy-gated promotion, not written ad hoc from prompt text.

### Projection and prompt boundaries
- **D-04:** Only a bounded, high-signal subset of structured artifacts may project into core memory or prompt context.
- **D-05:** Phase 181's recall-only contract remains in force: raw recall rows, raw archive blobs, and bulk artifact dumps stay out of the system prompt.
- **D-06:** Projection logic should prefer typed summaries and explicit selection rules over a blended "profile blob".

### Operator control and inspection
- **D-07:** Operators need typed inspect, correct, deactivate, and remove paths for structured artifacts through existing CLI, control, or MCP surfaces.
- **D-08:** Artifact mutation should preserve source lineage and status so later phases can safely review, learn from, or supersede artifacts.

### Scope guardrails
- **D-09:** Phase 182 should not promote learning candidates into active lessons; that belongs to Phase 183.
- **D-10:** Phase 182 should not generate skill proposals or widen runtime authority; Phase 184 and Phase 185 depend on artifact control first.

### the agent's Discretion
- Exact artifact schema boundaries between user, operator, project, and archive summary artifacts
- Whether projection is best implemented via core-memory shaping, view-model projection, or a dedicated artifact projection service
- Which existing inspect/control surfaces should own artifact correction and deactivation first

</decisions>

<specifics>
## Specific Ideas

- Build on the shared `RecallPack` and retrieval explanation contracts from Phase 181 rather than inventing a second inspection format.
- Reuse `crates/memory/src/artifacts.rs` and file-backed artifact concepts where they help, but keep durable model artifacts in the Rust-owned memory/control path.
- Preserve source lineage back to the memories, summaries, or inspections that justified a promoted artifact.
- Keep operator-facing edits typed and bounded so corrections do not require editing raw memory exports or archive files directly.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone and phase contract
- `.planning/PROJECT.md` - Current milestone intent and active requirements for v1.43
- `.planning/REQUIREMENTS.md` - `MODL-01` through `MODL-04` define the committed structured-memory scope
- `.planning/ROADMAP.md` - Phase 182 goal, dependency boundary, and success criteria
- `.planning/STATE.md` - Current milestone position after Phase 181 completion

### Prior phase outputs
- `.planning/phases/181-hybrid-retrieval-and-recall-inspection/181-01-SUMMARY.md` - retrieval contracts, hybrid scoring, and vector-aware caller path
- `.planning/phases/181-hybrid-retrieval-and-recall-inspection/181-02-SUMMARY.md` - bounded recall packs, runtime-event inspection, and recall-view metadata

### Existing codebase seams
- `crates/memory/src/artifacts.rs` - existing workspace artifact registry and artifact classification logic
- `crates/memory/src/policies.rs` - memory write and policy gating seam
- `crates/memory/src/context.rs` - bounded projection boundary and recall-pack shaping
- `crates/memory/src/core_memory.rs` - core-memory persistence and shaping seam
- `crates/app/src/memory_views.rs` - existing typed memory view models that now carry bounded explanation metadata
- `crates/cli/src/commands/memory.rs` - operator-facing memory import/export and file-backed view hooks
- `crates/cli/src/commands/inspect.rs` - typed inspection surfaces for memory and runtime state
- `crates/cli/src/commands/start.rs` - existing control and MCP registration points for memory inspection

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/memory/src/artifacts.rs`: already knows about artifact classes and model-aware artifact resolution, which may inform Phase 182 artifact categorization
- `crates/memory/src/policies.rs`: natural place to enforce promotion and mutation policy boundaries
- `crates/memory/src/context.rs`: now owns bounded recall-pack assembly and should remain the prompt-boundary enforcement seam
- `crates/app/src/memory_views.rs`: can carry structured artifact metadata without dumping raw state
- `crates/cli/src/commands/inspect.rs` and `crates/cli/src/commands/start.rs`: already expose typed inspection and MCP/control surfaces that can be extended for artifact control

### Established Patterns
- Rust owns durable state and trust boundaries
- Bounded typed payloads are preferred over raw DB or file dumps
- Retrieval and inspection surfaces now share one explanation contract and durable runtime-event lane

### Integration Points
- Structured artifact promotion should connect memory policies, durable storage, and control/inspection surfaces in one Rust-owned path
- Projection into active context should compose with core memory and bounded recall-pack patterns instead of bypassing them

</code_context>

<deferred>
## Deferred Ideas

- Learning candidate creation, approval, and promotion - Phase 183
- Skill proposal generation and verification - Phase 184
- God Mode overlay, audit, and recovery - Phase 185
- Automatic self-modifying model artifacts without explicit policy or operator control - out of scope

</deferred>

---

*Phase: 182-structured-memory-artifacts-and-model-control*
*Context gathered: 2026-04-08*
