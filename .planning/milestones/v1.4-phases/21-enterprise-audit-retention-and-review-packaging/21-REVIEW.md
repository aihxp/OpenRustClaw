---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/cli/src/commands/enterprise_policy.rs
  - crates/cli/src/commands/start.rs
  - crates/cli/src/commands/control_ui.html
  - crates/cli/src/commands/control_ui.rs
  - README.md
  - docs/src/deployment/production.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 21 Retroactive Code Review

Reviewed the historical Phase 21 enterprise-audit retention and review packaging slice against the
current `HEAD` implementation, using the original phase summaries and commit range to reconstruct
scope.

### WR-01: Audit review can report expired bundles as still retained until another export happens

**File:** `crates/cli/src/commands/enterprise_policy.rs:550-575`

**Issue:** The retention contract only prunes expired bundles during `export_audit_bundle()`, but
`review_summary()` calls `list_recent_exports()` which previously only sorted and truncated JSON
files under the export root. If no new export had run since old bundles aged out, the review path
would still surface those expired files as “retained export(s)” in `GET /control/enterprise/audit/review`
and the `Enterprise Audit Review` panel. That makes the review surface untruthful about what is
actually inside the retention window.

**Fix:** Apply the same retention-window filter when building the recent-export review list, and
cover the path with a regression test that proves stale bundles are excluded even before the next
export-triggered prune.
