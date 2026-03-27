---
phase: 15-voice-and-call-handling-parity
plan: 02
subsystem: voice-control-ui-parity
tags:
  - control-ui
  - voice
  - talk
  - bounded-calls
provides:
  - Top-level voice operator report surface in `/control/ui`
  - Summary-card renderers for voice status, provider coverage, metrics, session health, voice catalog, talk status, and talk metrics
  - Dashboard contract coverage for the new voice operator surface
affects:
  - Control UI voice and talk readability
  - Operator trust loop for voice surfaces
tech-stack:
  added: []
  patterns:
    - Render report-driven voice operator summaries before session-level drill-down
key-files:
  created:
    - .planning/phases/15-voice-and-call-handling-parity/15-02-SUMMARY.md
  modified:
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - Use the new operator report as the top-level voice story while leaving detailed session and call actions intact
  - Replace the remaining top-level raw voice and talk `pre` panes with typed summary cards instead of adding more raw dumps
patterns-established:
  - Voice parity in Control UI should be summary-first, then drill into session, talk receipt, or bounded call detail
duration: 20min
completed: 2026-03-27
---

# Phase 15 Plan 02 Summary

**Surfaced the typed voice operator report in Control UI and replaced the remaining top-level voice and talk raw panes with summary-card renderers.**

## Performance
- **Duration:** ~20 min
- **Tasks:** 1 completed
- **Files modified:** 2

## Accomplishments
- Added a top-level `Voice Operator Surface` card plus `attention signals` and `recent activity` tables in `/control/ui`.
- Replaced the top-level voice and talk raw `pre` blocks with typed summary-card renderers for voice status, provider readiness, metrics, session health, voice catalog, talk status, and talk metrics.
- Wired the new surface to refresh through the existing voice session, talk session, and bounded voice-call flows instead of inventing a separate client-side model.
- Added dashboard contract coverage for the new operator report rendering.

## Task Commits
1. **Task 1: Surface the voice operator report in Control UI** - pending commit

## Files Created/Modified
- `crates/cli/src/commands/control_ui.html` - added the report-driven voice operator surface and summary-card renderers for top-level voice/talk panes
- `crates/cli/src/commands/control_ui.rs` - locked the voice operator report surface in dashboard contract tests

## Decisions & Deviations
The UI stays intentionally operational rather than decorative. Phase 15 improves operator comprehension at the top of the voice surface while preserving the existing session transcript, artifact, event, and bounded call drill-down panes underneath.

## Verification
- `node - <<'NODE' ... new Function(match[1]) ... NODE`
- `cargo test -p openrustclaw-cli control_ui -- --nocapture`

## Next Step Readiness
Phase 15 now only needs doc alignment, verification capture, and roadmap/state sync to close truthfully.
