---
phase: 14-control-ui-surface-completion
plan: 01
subsystem: voice-and-talk-control-ui-renderers
tags:
  - control-ui
  - voice
  - talk
provides:
  - Typed Control UI renderers for voice session list, detail, metrics, transcript, artifact, and event surfaces
  - Typed Control UI renderers for talk receipt list, detail, metrics, and event surfaces
  - Dashboard contract coverage for the upgraded voice and talk renderer hooks
affects:
  - Control UI operator inspection for voice runtime and talk runtime
tech-stack:
  added: []
  patterns:
    - Replace raw JSON panes with compact summary cards plus tables over existing typed runtime payloads
key-files:
  created: []
  modified:
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - The first batch should target voice and talk because those are high-value trust-sensitive panes already backed by stable runtime contracts
  - This slice upgrades renderer depth only and does not change the underlying voice or talk backend behavior
patterns-established:
  - Control UI parity work can ship by reusing existing typed routes and tightening renderer contracts in `control_ui.rs`
duration: 30min
completed: 2026-03-27
---

# Phase 14: Control UI Surface Completion Summary

**Replaced the highest-value raw voice and talk inspection panes with typed summaries and tables so operators can read those flows without decoding JSON.**

## Performance
- **Duration:** ~30 min
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- Upgraded the voice runtime session list into a typed table with direct inspect actions.
- Replaced raw voice session detail and metrics dumps with summary cards.
- Replaced raw voice transcript, artifact, and event dumps with typed tables.
- Upgraded the talk receipt list, detail, metrics, and event panes to the same typed renderer pattern.
- Added a Control UI contract test that locks the new voice/talk renderer hooks into the dashboard shell.

## Task Commits
1. **Task 1: Upgrade the first batch of high-value raw detail panes** - pending commit

## Files Created/Modified
- `crates/cli/src/commands/control_ui.html` - replaced raw voice and talk inspection panes with typed summary-card and table renderers
- `crates/cli/src/commands/control_ui.rs` - added dashboard contract coverage for the new voice/talk renderer hooks

## Decisions & Deviations
This slice stays intentionally narrow. It upgrades the operator-critical voice and talk surfaces first while leaving skill detail and the remaining mobile sub-detail panes for the next batch.

## Verification
- `node - <<'NODE' ... new Function(match[1]) ... NODE`
- `cargo test -p openrustclaw-cli control_ui -- --nocapture`

## Next Phase Readiness
The Control UI now has a clear typed pattern for replacing raw runtime panes. The next slice can apply the same renderer contract to skill detail and the remaining mobile sub-detail surfaces.
