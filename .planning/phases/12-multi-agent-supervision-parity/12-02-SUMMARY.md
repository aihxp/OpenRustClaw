---
phase: 12-multi-agent-supervision-parity
plan: 02
subsystem: orchestration-supervision-control-surface
tags:
  - orchestration
  - control-ui
  - runtime-api
provides:
  - Typed active-run supervision endpoint under `/control/orchestration/active/{run_id}/supervision`
  - Structured Control UI tables for delegations, worker outcomes, active attention signals, and recent events
  - Dashboard contract coverage for the richer supervision surface
affects:
  - Runtime control routing
  - Operator-facing orchestration dashboard visibility
tech-stack:
  added: []
  patterns:
    - Back new dashboard supervision views with typed runtime endpoints instead of raw JSON panes
key-files:
  created: []
  modified:
    - crates/cli/src/commands/start.rs
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - Active-run inspection should answer operator questions in one call rather than forcing separate raw snapshot and event decoding
  - Control UI should remain table-first and compact for supervision instead of adding a new orchestration builder workflow
patterns-established:
  - Orchestration parity surfaces should render directly from shipped typed routes rather than frontend-only derivation
duration: 30min
completed: 2026-03-26
---

# Phase 12: Multi-Agent Supervision Parity Summary

**Surfaced the richer supervision reports through runtime routes and Control UI so operators can inspect delegated work as structured supervision instead of raw orchestration dumps.**

## Performance
- **Duration:** ~30 min
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- Added `/control/orchestration/active/{run_id}/supervision` for typed live supervision inspection.
- Upgraded orchestration run listings to surface approval policy, worker counts, failures, needs-input counts, and escalation hints.
- Replaced raw supervision panes in Control UI with structured tables for delegated tasks, worker outcomes, live attention signals, and recent events.
- Added a Control UI contract test for the new supervision tables and rendering helpers.

## Task Commits
1. **Task 1: Surface richer supervision in runtime APIs and Control UI** - `4f4ca1d` `feat(12-02): surface orchestration supervision`

## Files Created/Modified
- `crates/cli/src/commands/start.rs` - exposed the active-run supervision route
- `crates/cli/src/commands/control_ui.html` - rendered richer orchestration supervision tables and summaries
- `crates/cli/src/commands/control_ui.rs` - locked the supervision rendering contract

## Decisions & Deviations
The dashboard stays focused on operator-readable supervision. Trace, transcript, and resource detail remain available as separate panes instead of being folded into one overloaded supervision view.

## Verification
- `cargo test -p openrustclaw-cli supervision -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_orchestration_supervision_tables -- --nocapture`

## Next Phase Readiness
The supervision surface is now visible from shipped runtime and dashboard entry points. The closeout work can document the operator story and preserve verification evidence for milestone audit.
