---
phase: 05-email-and-voice-communications
plan: 02
subsystem: voice-outcome-diagnostics
tags:
  - voice
  - control-ui
  - diagnostics
  - audit
provides:
  - Typed recent voice outcome summaries derived from persisted session receipts
  - Control endpoint for recent voice outcomes
  - Control UI visibility for stale, paused, interrupted, ended, and active voice sessions
affects:
  - Voice runtime inspection
  - Browser operator diagnostics
  - Communications trust path for persisted voice sessions
tech-stack:
  added: []
  patterns:
    - Derive operator-facing communication summaries from persisted runtime receipts rather than changing the underlying artifact format
key-files:
  created:
    - tests/integration/src/voice_outcomes_test.rs
  modified:
    - crates/cli/src/commands/voice_runtime.rs
    - crates/cli/src/commands/start.rs
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
    - tests/integration/src/lib.rs
key-decisions:
  - Voice trust should land as a typed summary over existing session receipts before any broader telephony expansion
  - Stale, paused, interrupted, ended, and artifact-gap states should be surfaced as operator-readable outcome labels
patterns-established:
  - Communications diagnostics should follow the same Phase 2/3/4 trust pattern: summary first, raw detail still available underneath
duration: 35min
completed: 2026-03-26
---

# Phase 5: Email and Voice Communications Summary

**Hardened the voice lane by turning persisted session receipts into operator-readable recent outcomes instead of leaving voice diagnostics buried in raw JSON blobs.**

## Performance
- **Duration:** ~35 min
- **Tasks:** 3 completed
- **Files modified:** 6

## Accomplishments
- Added a typed `voice_session_outcomes` summary that classifies recent sessions as active, stale, paused, interrupted, ended, or artifact-gap.
- Exposed those summaries through `/control/voice/outcomes` so voice diagnostics stay in the shipped control plane.
- Added a `Recent Voice Outcomes` table to Control UI that shows session state, attention-needed signals, activity totals, and human-readable detail.
- Added regression coverage for the new operator diagnostics path.

## Task Commits
1. **Task 1: Harden the voice communication trust path** - pending commit in current checkpoint

## Files Created/Modified
- `crates/cli/src/commands/voice_runtime.rs` - Added recent voice outcome summaries derived from persisted session receipts
- `crates/cli/src/commands/start.rs` - Added the control endpoint for voice outcomes
- `crates/cli/src/commands/control_ui.html` - Added the recent voice outcomes panel and loader
- `crates/cli/src/commands/control_ui.rs` - Added dashboard regression coverage for the new panel
- `tests/integration/src/lib.rs` - Registered the new voice outcomes integration test
- `tests/integration/src/voice_outcomes_test.rs` - Added regression coverage for stale, paused, and ended outcome classification

## Decisions & Deviations
This slice deliberately derived summaries from the existing persisted session receipts instead of redefining the voice storage model. That keeps the runtime durable path stable while still giving operators an outcome-first view suited for MVP support and diagnosis.

## Verification
- `cargo test -p openrustclaw-cli dashboard_includes_voice_outcomes_panel -- --nocapture`
- `cargo test -p openrustclaw-integration-tests voice_outcomes_distinguish_stale_paused_and_ended_sessions -- --nocapture`

## Next Phase Readiness
Phase 5 now has durable email evidence and readable voice outcome diagnostics. The remaining communications slice is docs and cross-surface verification.
