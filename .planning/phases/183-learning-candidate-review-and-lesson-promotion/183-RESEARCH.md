# Phase 183 Research: Learning Candidate Review and Lesson Promotion

## Current state

OpenRustClaw already has:
- reflection-candidate generation in `crates/app/src/orchestration_reporting.rs`
- a shipped active-lesson control lane in `crates/app/src/autonomy_lessons_control.rs`
- file-backed lesson persistence through `crates/cli/src/commands/control.rs`
- orchestration promotion plumbing in `crates/cli/src/commands/orchestrate.rs`
- operator-facing control routes in `crates/cli/src/commands/start.rs`
- durable reviewable queue patterns in `crates/db/src/migrate.rs` for optimization candidates and evaluations

OpenRustClaw does not yet have:
- a durable learning-candidate queue with explicit review state
- provenance-rich candidate storage that bridges reflection, audit, and replay/evaluation evidence
- a service seam that separates candidate review from direct lesson creation
- rollback or supersede history between a candidate and the lesson it produced

## Phase 183 decisions

### D-183-01: Add a first-class SQLite learning-candidate store

Phase 183 should add Rust-owned durable candidate storage instead of writing lessons directly from orchestration receipts.

Why:
- `promote_reflection_candidate` currently bypasses review and writes straight into the live lesson lane
- candidates need durable state, provenance, and evidence history that do not belong in YAML lesson files
- the repo already uses SQLite-backed review queues for adjacent control flows

### D-183-02: Keep active lessons in the existing control lane

Promoted lessons should continue to land in `.claw/control/lessons` through the existing lesson service and control registry.

Why:
- the active runtime already knows how to consume decision lessons
- Phase 183 is about adding a candidate-review bridge, not replacing active lesson storage
- this keeps promoted runtime guidance bounded and operator-legible

### D-183-03: Make candidate lifecycle explicit and durable

Candidates should move through typed states such as `pending_review`, `approved`, `rejected`, `superseded`, `promoted`, and `rolled_back`.

Why:
- `LEAR-02` requires review and rollback discipline before and after promotion
- the queue must explain whether a lesson is still waiting, blocked, replaced, active, or revoked
- explicit state avoids overloading lesson `active` with review semantics it does not represent

### D-183-04: Preserve evidence separately from the candidate summary

Replay, evaluation, audit, and runtime-event evidence should be stored as linked records or structured attachments rather than flattened into one text field.

Why:
- `LEAR-04` requires promotability to depend on review evidence, not only a human-written note
- evidence needs to remain inspectable even when the candidate summary changes
- linked evidence makes it easier to distinguish low-impact hints from higher-impact guidance

### D-183-05: Centralize review logic in an app service

Candidate ingestion, approval, rejection, promotion, and rollback should flow through an app-layer learning-review service instead of being scattered across CLI handlers.

Why:
- the CLI and MCP surfaces should stay thin
- promotion needs consistent gating around confidence, provenance, and required evidence
- a shared service gives Phase 184 a stable seam for future skill proposals

## Implementation shape

### Durable candidate contract

Add shared types for:
- learning candidate kind and source kind
- review status
- provenance references
- linked evidence records
- promotion and rollback reports

### Storage

Add SQLite tables for:
- `learning_candidates`
- `learning_candidate_evidence`
- `learning_candidate_promotions`

These should track:
- stable candidate id
- namespace or workspace scope
- source receipt or audit ids
- confidence and impact
- review status
- reviewer metadata
- linked lesson id
- created, updated, reviewed, promoted, and rolled-back timestamps

### Review service

Add an app-layer service that can:
- ingest reflection candidates into the durable queue
- list and inspect candidates with evidence
- approve, reject, supersede, and roll back candidates
- enforce replay/evaluation evidence for higher-impact promotions
- create or deactivate active lessons through the existing lesson-control seam

### Operator surfaces

Extend existing CLI and MCP control surfaces so operators can:
- queue a reflection candidate
- review candidate state and evidence
- approve and promote a candidate
- reject, supersede, or roll back a promoted lesson

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Direct promotion path remains reachable and bypasses review | Breaks candidate-first learning | Re-route orchestration promotion to candidate creation and explicit review actions |
| Candidate provenance becomes lossy across receipts and audits | Weakens trust in learned artifacts | Persist typed source refs and evidence attachments, not only human-readable notes |
| Promotion broadens runtime authority silently | Violates `LEAR-03` | Limit promoted lessons to existing bounded decision-lesson fields and reject unsafe scopes |
| Rollback only deactivates lessons without updating candidate history | Leaves review state inconsistent | Persist promotion history and candidate rollback state in SQLite |

## Recommended plan split

- `183-01`: shared learning-candidate contracts, durable queue storage, evidence linkage, and app-layer review service
- `183-02`: orchestration, CLI, and MCP review flows that promote approved candidates into the existing lesson lane and support rollback
