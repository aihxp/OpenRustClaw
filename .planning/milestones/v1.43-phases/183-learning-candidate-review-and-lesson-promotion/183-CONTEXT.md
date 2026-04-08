# Phase 183: Learning Candidate Review and Lesson Promotion - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Turn successful runs, reflection candidates, and audit evidence into reviewable learning candidates that can promote into bounded active lessons only with explicit evidence, review state, and rollback discipline.

This phase is about the candidate queue and lesson-promotion controls. It is not yet about skill proposal generation or God Mode authority changes.

</domain>

<decisions>
## Implementation Decisions

### Candidate-first learning loop
- **D-01:** Phase 183 should introduce a durable learning-candidate queue instead of promoting reflection output directly into live lessons.
- **D-02:** Learning candidates must preserve provenance to the originating run, memory artifact, runtime event, or audit record.
- **D-03:** Candidate state should be explicit and durable: pending review, approved, rejected, superseded, rolled back, and promoted.

### Reuse existing lesson seams
- **D-04:** Promoted lessons should reuse the existing decision-lesson/control path under `.claw/control/lessons` and the autonomy lesson service surfaces rather than inventing a parallel active-lesson runtime.
- **D-05:** Phase 183 should separate candidate storage from active-lesson storage: candidates in SQLite, promoted lessons in the existing lesson/control lane unless a stronger reason appears during planning.

### Evidence and review discipline
- **D-06:** High-impact lesson promotion requires replay, evaluation, or equivalent review evidence before activation.
- **D-07:** Successful runs alone are not enough evidence; the promoted lesson must capture why it is safe and where it applies.
- **D-08:** Candidate approval must remain bounded: routing, recall, tool choice, and other scoped runtime hints are in; silent authority widening is out.

### Scope guardrails
- **D-09:** Phase 183 should not generate skill proposals yet; that belongs to Phase 184.
- **D-10:** Phase 183 should not widen default autonomy or God Mode permissions; that belongs to Phase 185.

### the agent's Discretion
- Exact candidate schema boundaries between reflection, audit, optimization, and runtime-event evidence
- Whether replay/eval evidence lives on the candidate row or in a linked evidence table
- Which existing control or inspect surfaces should own candidate review first

</decisions>

<specifics>
## Specific Ideas

- Reuse `crates/app/src/orchestration_reporting.rs` reflection candidates as a primary seed source.
- Reuse the existing decision-lesson control lane in `crates/app/src/autonomy_lessons_control.rs`, `crates/cli/src/commands/control.rs`, and the lesson routes already exposed in `crates/cli/src/commands/start.rs`.
- Reuse optimization/evaluation evidence where it helps satisfy the “replay or equivalent review evidence” requirement.
- Keep candidate review typed and auditable so operators can see source ids, confidence, review state, and any resulting lesson id.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone and phase contract
- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md` - `LEAR-01` through `LEAR-04`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`

### Prior phase outputs
- `.planning/phases/182-structured-memory-artifacts-and-model-control/182-01-SUMMARY.md`
- `.planning/phases/182-structured-memory-artifacts-and-model-control/182-02-SUMMARY.md`

### Existing code seams
- `crates/app/src/orchestration_reporting.rs` - reflection candidate generation
- `crates/app/src/autonomy_lessons_control.rs` - active lesson service seam
- `crates/app/src/tool_execution_audit.rs` - durable operator/audit evidence
- `crates/cli/src/commands/control.rs` - lesson registry and mutation path
- `crates/cli/src/commands/start.rs` - lesson and orchestration control routes
- `crates/db/src/migrate.rs` - existing optimization/evaluation tables that may inform candidate evidence storage
- `crates/memory/src/model_artifacts.rs` - Phase 182 artifact control and projection seam

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Reflection candidates already exist and are operator-visible, but they do not yet flow through a durable review queue.
- Active lessons already have a shipped control-plane surface and file-backed runtime representation.
- Optimization candidate/evaluation tables prove the repo already tolerates reviewable promote-or-reject queues backed by SQLite.

### Established Patterns
- Reviewable state should be explicit, typed, and durable.
- Promotion should preserve provenance and remain reversible.
- Active runtime guidance should stay bounded and operator-inspectable.

### Integration Points
- Learning candidates should bridge orchestration reporting, audit evidence, and the lesson-control lane.
- Promoted lessons must remain distinct from structured model artifacts and from future skill proposals.

</code_context>

<deferred>
## Deferred Ideas

- Skill proposal generation and installation flow - Phase 184
- God Mode-specific lesson weighting or quarantine rules - Phase 185
- Automatic self-activation of lessons with no review state - out of scope

</deferred>

---

*Phase: 183-learning-candidate-review-and-lesson-promotion*
*Context gathered: 2026-04-08*
