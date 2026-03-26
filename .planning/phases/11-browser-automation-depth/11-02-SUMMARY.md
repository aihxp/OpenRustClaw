---
phase: 11-browser-automation-depth
plan: 02
subsystem: browser-workflow-control-surface
tags:
  - browser
  - control-ui
  - runtime-api
provides:
  - Typed `/control/browser/workflow-history` runtime endpoint
  - `Recent Browser Workflows` panel in Control UI
  - Dashboard contract coverage for the new browser workflow history surface
affects:
  - Runtime control routing
  - Browser operator dashboard visibility
tech-stack:
  added: []
  patterns:
    - Reuse typed runtime endpoints to render new Control UI parity surfaces directly
key-files:
  created: []
  modified:
    - crates/cli/src/commands/start.rs
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - Browser workflow history should be exposed as one typed runtime endpoint instead of requiring direct filesystem inspection
  - Control UI should render recent browser workflows beside existing browser sessions and artifacts
patterns-established:
  - Browser parity surfaces should follow the same ledger -> runtime endpoint -> dashboard pattern used by earlier trust-focused phases
duration: 25min
completed: 2026-03-26
---

# Phase 11: Browser Automation Depth Summary

**Surfaced the new browser workflow ledger through the runtime and Control UI so operators can inspect recent browser runs from shipped surfaces.**

## Performance
- **Duration:** ~25 min
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- Added `/control/browser/workflow-history` with limit, action, and backend filtering.
- Added a `Recent Browser Workflows` table to Control UI with backend, session, destination, step count, status, and artifact visibility.
- Updated browser action refresh flows so successful browser runs also refresh recent workflow history.
- Added a Control UI contract test for the new panel.

## Task Commits
1. **Task 1: Surface browser workflow history in runtime APIs and Control UI** - `89b4b26` `feat(11-02): surface browser workflow history`

## Files Created/Modified
- `crates/cli/src/commands/start.rs` - exposed `/control/browser/workflow-history`
- `crates/cli/src/commands/control_ui.html` - rendered `Recent Browser Workflows` and wired refresh behavior
- `crates/cli/src/commands/control_ui.rs` - added dashboard contract coverage for the new panel

## Decisions & Deviations
The dashboard view stays table-first and intentionally simple. The goal in this phase is trustworthy recent-history inspection, not a new browser workflow builder.

## Verification
- `cargo test -p openrustclaw-cli dashboard_includes_browser_workflow_history_panel -- --nocapture`
- `cargo test -p openrustclaw-cli browser_workflow_history -- --nocapture`

## Next Phase Readiness
The browser workflow ledger is now visible from shipped runtime surfaces. The closeout work can update docs and preserve the phase verification artifact.
