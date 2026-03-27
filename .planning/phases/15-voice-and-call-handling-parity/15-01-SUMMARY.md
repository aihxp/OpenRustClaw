---
phase: 15-voice-and-call-handling-parity
plan: 01
subsystem: voice-operator-report
tags:
  - voice
  - talk
  - bounded-calls
  - runtime
  - verification
provides:
  - Typed voice operator report across voice runtime, talk receipts, and bounded voice-call receipts
  - Shipped `/control/voice/operator-summary` route for the aggregated report
  - Focused integration coverage for stale sessions, talk errors, and stale bounded calls
affects:
  - Voice and call operator visibility
  - Control API parity surface
  - Voice lifecycle verification evidence
tech-stack:
  added: []
  patterns:
    - Aggregate persisted voice, talk, and bounded call evidence into one typed operator report before rendering the dashboard
key-files:
  created:
    - .planning/phases/15-voice-and-call-handling-parity/15-01-SUMMARY.md
    - tests/integration/src/voice_operator_report_test.rs
  modified:
    - crates/cli/src/commands/inspect.rs
    - crates/cli/src/commands/start.rs
    - crates/cli/src/commands/skills.rs
    - tests/integration/src/lib.rs
key-decisions:
  - Use a typed runtime summary instead of teaching Control UI to stitch several voice lanes together on the client
  - Reuse persisted voice session, talk receipt, and bounded voice-call evidence instead of inventing new state
patterns-established:
  - Voice parity work should start with a runtime summary that exposes readiness, attention signals, and recent activity across the shipped voice lanes
duration: 20min
completed: 2026-03-27
---

# Phase 15 Plan 01 Summary

**Added a typed voice operator report so operators can inspect voice runtime, talk receipts, and bounded voice-call receipts from one coherent shipped route.**

## Performance
- **Duration:** ~20 min
- **Tasks:** 2 completed
- **Files modified:** 5

## Accomplishments
- Added `voice_operator_report_summary(...)` in `inspect.rs` with provider coverage, voice lane, talk lane, bounded call lane, attention signals, and recent activity.
- Exposed the report through `/control/voice/operator-summary` in `start.rs`.
- Made bounded voice-call summaries reusable by workspace path in `skills.rs` so the operator report can aggregate them truthfully outside `cwd` assumptions.
- Added focused integration coverage proving stale voice sessions, talk errors, and stale bounded voice calls surface in the report.

## Task Commits
1. **Task 1: Add a typed voice operator report** - pending commit

## Files Created/Modified
- `crates/cli/src/commands/inspect.rs` - added the typed voice operator report and its derived signals/activity helpers
- `crates/cli/src/commands/start.rs` - exposed `/control/voice/operator-summary`
- `crates/cli/src/commands/skills.rs` - added workspace-root-aware bounded voice-call summary helpers
- `tests/integration/src/voice_operator_report_test.rs` - locked the operator report attention-signal contract
- `tests/integration/src/lib.rs` - included the new integration test module

## Decisions & Deviations
The operator report intentionally stays summary-first. Session transcript, event, and artifact drill-down already exist, so the new route focuses on readiness, attention pressure, and recent activity instead of duplicating all detail payloads.

## Verification
- `cargo test -p openrustclaw-cli inspect -- --nocapture`
- `cargo test -p openrustclaw-integration-tests voice_operator_report -- --nocapture`

## Next Step Readiness
Phase 15 can now wire the richer voice operator story into `/control/ui` without inventing a second client-side aggregation layer.
