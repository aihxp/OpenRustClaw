---
phase: 183-learning-candidate-review-and-lesson-promotion
plan: "02"
subsystem: api
tags: [cli, mcp, control-plane, orchestration, lesson-promotion]
requires:
  - phase: 183-01
    provides: durable candidate queue, evidence storage, and review service
provides:
  - reflection-candidate queueing instead of direct lesson activation
  - operator CLI review and rollback commands
  - MCP/runtime control handlers for candidate review and lesson promotion
affects: [phase-184, phase-185]
tech-stack:
  added: []
  patterns: [service-backed candidate review, compatibility-preserving route repurpose, rollback-aware promotion]
key-files:
  created: []
  modified:
    - crates/cli/src/commands/control.rs
    - crates/cli/src/commands/orchestrate.rs
    - crates/cli/src/commands/start.rs
    - crates/cli/src/main.rs
key-decisions:
  - "Repurposed the existing reflection-candidate promote path to queue a learning candidate instead of silently writing a live lesson."
  - "Extended existing control and MCP surfaces rather than introducing a separate learning dashboard or bespoke review interface."
patterns-established:
  - "Operator review, promotion, and rollback now share one workspace-backed learning-review source across CLI, HTTP, and MCP."
  - "Promoted lessons retain a durable candidate link so rollback updates both the active lesson lane and candidate state."
requirements-completed: [LEAR-02, LEAR-03]
duration: n/a
completed: 2026-04-08
---

# Phase 183: Learning Candidate Review and Lesson Promotion Summary

**Operators can now queue, review, promote, and roll back learning candidates through the existing orchestration, control, and MCP surfaces.**

## Performance

- **Duration:** n/a
- **Started:** 2026-04-08T05:41:47Z
- **Completed:** 2026-04-08T06:04:07Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Reworked orchestration reflection promotion so it queues a durable learning candidate instead of writing a decision lesson immediately.
- Added control CLI commands for listing, queueing, reviewing, promoting, and rolling back learning candidates.
- Added `/control/learning/candidates/...` routes and MCP tools for candidate review and bounded lesson promotion.

## Verification

- `cargo test -p openrustclaw-cli orchestrate -- --nocapture`
- `cargo test -p openrustclaw-cli control -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_control_tools -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Decisions Made

- Preserved compatibility by keeping the old reflection-candidate route name while changing its behavior to queue reviewable candidates.
- Kept manual operator access in the existing control lane so lesson promotion and rollback remain auditable and reversible.

## Deviations from Plan

- `cargo fmt --all` normalized some adjacent Rust formatting in files outside the core Phase 183 logic. The behavior change stayed scoped to the learning-review path.

## Next Phase Readiness

- Phase 184 can now build skill proposals from approved candidates and promoted lessons instead of scraping raw orchestration receipts.

---
*Phase: 183-learning-candidate-review-and-lesson-promotion*
*Completed: 2026-04-08*
