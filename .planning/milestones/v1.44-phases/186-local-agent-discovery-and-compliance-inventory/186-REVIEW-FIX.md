---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 186 Code Review Fix

Applied a manual fix for the Phase 186 review finding.

## Outcome

- `WR-01` was fixed in `crates/app/src/agent_backend_catalog.rs`.

## Fix Summary

- Reclassified Cursor from `delegated_cli_candidate` to `integration_only` in the shared discovery catalog.
- Cursor now resolves to detection-only readiness while still exposing discoverable model metadata for operator visibility.
- Updated the catalog regression so the shipped behavior matches the phase contract that Cursor stays visible without being treated as a delegated execution lane.
