---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 189 Code Review Fix

Applied a manual fix for the Phase 189 review finding.

## Outcome

- `WR-01` was fixed in `crates/cli/src/commands/control.rs`.

## Fix Summary

- Control initialization now loads the workspace runtime config and derives the effective delegated-backend policy before seeding delegated model profiles.
- Delegated `delegated-*` model profiles are written only for backends that remain allowed after the same policy evaluation used by runtime execution.
- Added a regression proving that a detected `codex` backend does not get a delegated model profile when local CLI wrappers are disabled by policy.
