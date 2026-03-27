# Plan 28-01 Summary: Add Durable Setup-State Contract

## What Changed

- Added a durable setup-state manifest under `.claw/control/setup-state.json`.
- The manifest now records deployment mode, deployment path, setup path, selected steps, completed steps, blockers, current step, and next action.
- Updated `doctor` to treat unfinished setup state as a first-start readiness blocker instead of ignoring it.

## Why It Matters

The onboarding flow no longer depends on transient in-memory choices alone. Setup is now a first-class workspace contract the runtime can inspect and resume.

## Verification Notes

- `cargo test -p openrustclaw-cli onboard -- --nocapture`
- `cargo test -p openrustclaw-cli doctor -- --nocapture`
