---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 185 Code Review Fix

Applied a manual fix for the Phase 185 review finding.

## Outcome

- `WR-01` was fixed in `crates/cli/src/commands/enterprise_autonomy.rs`.

## Fix Summary

- `enable()` now refreshes expired manifest state before snapshotting the current runtime policy for the next baseline.
- Added a regression that expires God Mode, re-enables it, and verifies disable restores the original pre-God-Mode runtime instead of the stale override.
- This keeps the TTL expiry and baseline-restore guarantees intact across repeated God Mode cycles.
