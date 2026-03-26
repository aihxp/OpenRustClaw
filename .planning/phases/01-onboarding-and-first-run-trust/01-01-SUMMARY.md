---
phase: 01-onboarding-and-first-run-trust
plan: 01
subsystem: onboarding
tags:
  - onboarding
  - doctor
  - first-run
provides:
  - First-start readiness policy for onboarding health checks
  - Regression coverage for readiness blocking rules
affects:
  - Guided onboarding launch handoff
  - CLI diagnostics expectations
tech-stack:
  added: []
  patterns:
    - Use a dedicated readiness policy for first-start gating instead of relying on raw failed-count health
key-files:
  created: []
  modified:
    - crates/cli/src/commands/doctor.rs
    - crates/cli/src/commands/onboard.rs
    - docs/src/getting-started/quickstart.md
key-decisions:
  - Missing provider credentials are blocking for onboarding first start even if generic doctor severity stays at warning
  - No-channel setups remain non-blocking for the default assistant path
patterns-established:
  - Post-onboarding launch decisions should use explicit readiness rules rather than generic health summaries
duration: 90min
completed: 2026-03-26
---

# Phase 1: Onboarding and First-Run Trust Summary

**Hardened the onboarding post-check so the wizard no longer offers the persisted assistant when first-start prerequisites are still missing.**

## Performance
- **Duration:** ~90 min
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- Added a dedicated first-start readiness policy in `doctor.rs` that treats missing provider credentials, missing onboarding state, missing control registry, and broken enabled-channel readiness as blocking for onboarding handoff.
- Updated `onboard.rs` to use that readiness policy and print concrete blocking items instead of only relying on generic failed-count health.
- Added targeted regression tests for the new readiness behavior and aligned quickstart wording with the stricter launch gate.

## Task Commits
1. **Task 1: Implement onboarding readiness gate and tests** - `7db966c`

## Files Created/Modified
- `crates/cli/src/commands/doctor.rs` - Added first-start readiness evaluation and tests
- `crates/cli/src/commands/onboard.rs` - Switched post-onboarding launch gating to the explicit readiness policy
- `docs/src/getting-started/quickstart.md` - Clarified that the wizard only offers assistant launch after provider and workspace readiness pass

## Decisions & Deviations
Kept the generic `doctor` command severity model intact and added a narrower readiness policy for onboarding instead. That avoids redefining every warning globally while still fixing the broken first-run launch path.

## Next Phase Readiness
Phase 1 is not complete yet. `01-02` remains for workflow-level onboarding regression coverage, and `01-03` remains for broader installation and README alignment.
