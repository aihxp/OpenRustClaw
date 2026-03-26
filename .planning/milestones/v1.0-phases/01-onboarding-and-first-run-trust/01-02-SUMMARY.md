---
phase: 01-onboarding-and-first-run-trust
plan: 02
subsystem: onboarding-tests
tags:
  - onboarding
  - integration-tests
  - first-run
provides:
  - Workflow-level onboarding regression coverage
  - Pure launch-gate helper for non-interactive verification
affects:
  - Onboarding launch-handoff confidence
  - Existing-workspace detection coverage
tech-stack:
  added: []
  patterns:
    - Keep first-run launch policy testable via pure helpers instead of TTY-coupled logic
key-files:
  created:
    - tests/integration/src/onboarding_test.rs
  modified:
    - crates/cli/src/commands/onboard.rs
    - tests/integration/src/lib.rs
key-decisions:
  - Extracted assistant-launch gating into a pure helper so workflow-level tests can exercise ready and blocked first-start outcomes
  - Covered workspace-state detection through the integration crate instead of relying only on CLI unit tests
patterns-established:
  - Onboarding workflow decisions should have integration coverage in addition to command-module unit tests
duration: 75min
completed: 2026-03-26
---

# Phase 1: Onboarding and First-Run Trust Summary

**Added workflow-level onboarding regression coverage so first-run launch and existing-workspace detection are no longer guarded only by helper-level unit tests.**

## Performance
- **Duration:** ~75 min
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- Added `should_offer_assistant_launch(...)` so onboarding launch gating can be tested without live terminal prompts.
- Added integration tests for existing workspace-state detection, ready first-start handoff, and blocked first-start handoff.
- Kept the shipped CLI behavior unchanged while making the first-run decision path easier to verify.

## Task Commits
1. **Task 1: Add onboarding workflow test seam and integration coverage** - `84cd542`

## Files Created/Modified
- `crates/cli/src/commands/onboard.rs` - Extracted a pure launch-gate helper and added unit coverage
- `tests/integration/src/lib.rs` - Registered onboarding integration tests
- `tests/integration/src/onboarding_test.rs` - Added workflow-level onboarding regression tests

## Decisions & Deviations
Stayed inside the existing CLI and integration crates rather than building a heavier end-to-end terminal harness. The key Phase 1 risk was missing regression coverage around first-run decisions, and the new seam plus integration tests cover that directly.

## Next Phase Readiness
Phase 1 now has 2 of 3 plans complete. `01-03` remains for broader installation and README alignment before the phase can be closed.
