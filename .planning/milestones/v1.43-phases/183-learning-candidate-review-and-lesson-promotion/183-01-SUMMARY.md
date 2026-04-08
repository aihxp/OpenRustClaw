---
phase: 183-learning-candidate-review-and-lesson-promotion
plan: "01"
subsystem: learning
tags: [learning-candidates, provenance, evidence, review, storage]
requires: []
provides:
  - typed durable learning-candidate contracts
  - SQLite-backed candidate, evidence, and history storage
  - app-layer review and promotion gating service
affects: [phase-183-plan-02, phase-184, god-mode]
tech-stack:
  added: []
  patterns: [candidate-first learning queue, linked review evidence, promotion history]
key-files:
  created:
    - crates/db/src/learning_store.rs
    - crates/app/src/learning_review.rs
  modified:
    - crates/core/src/types.rs
    - crates/db/src/lib.rs
    - crates/db/src/migrate.rs
    - crates/db/src/models.rs
    - crates/app/src/lib.rs
key-decisions:
  - "Added a first-class learning-candidate queue in SQLite instead of promoting reflection output directly into active lessons."
  - "Kept lesson activation behind an app-layer service that requires approval and evidence before high-impact promotion."
patterns-established:
  - "Learning candidates now carry durable provenance, explicit review state, linked evidence, and rollback-ready history."
  - "Promotion gating lives in one reusable service seam instead of being reimplemented by CLI or MCP callers."
requirements-completed: [LEAR-01, LEAR-04]
duration: n/a
completed: 2026-04-08
---

# Phase 183: Learning Candidate Review and Lesson Promotion Summary

**OpenRustClaw now has a durable candidate-first learning loop with explicit provenance, review state, and evidence-aware promotion gating.**

## Performance

- **Duration:** n/a
- **Started:** 2026-04-08T05:41:47Z
- **Completed:** 2026-04-08T06:04:07Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added shared learning-candidate, source, evidence, review, promotion, and rollback contracts to `openrustclaw-core`.
- Added `learning_candidates`, `learning_candidate_evidence`, and `learning_candidate_history` tables plus `SqliteLearningStore`.
- Added `LearningReviewService` with approval checks, high-impact evidence gating, lesson promotion, and rollback logic.

## Verification

- `cargo test -p openrustclaw-core --lib`
- `cargo test -p openrustclaw-db learning_store -- --nocapture`
- `cargo test -p openrustclaw-app learning_review -- --nocapture`

## Decisions Made

- High-impact candidates now require replay, evaluation, or audit evidence before they can become active lessons.
- Candidate lifecycle and history stay in SQLite, while active lessons still reuse the existing `.claw/control/lessons` lane.

## Deviations from Plan

None - the work stayed inside the durable queue, service, and promotion-gating seam.

## Next Phase Readiness

- Phase 183 plan 02 can now route operator CLI, runtime control routes, and MCP tooling through the shared review service without inventing a second lesson path.

---
*Phase: 183-learning-candidate-review-and-lesson-promotion*
*Completed: 2026-04-08*
