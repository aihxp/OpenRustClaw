---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 21 Code Review Fix

Applied a manual fix for the Phase 21 audit-review retention drift.

## Outcome

- `WR-01` was fixed in `crates/cli/src/commands/enterprise_policy.rs`.

## Fix Summary

- `list_recent_exports()` now filters export bundles against the configured retention window before
  building the review summary.
- Export pruning still happens during `export_audit_bundle()`, but the review path no longer lies
  about expired bundles that remain on disk awaiting the next prune cycle.
- Added a lightweight regression test that writes one stale and one fresh export bundle and verifies
  only the in-window bundle appears in the review list.

## Verification

- `cargo test -p openrustclaw-cli list_recent_exports_ignores_exports_past_retention_window -- --nocapture`
- Attempted the broader existing audit-review tests, but the higher-level `enterprise`/`review_summary`
  slice remained long-running in this workspace, so I used the focused regression to verify the
  fixed retention filter directly.
