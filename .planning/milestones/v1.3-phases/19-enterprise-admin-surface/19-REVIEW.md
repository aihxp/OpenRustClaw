---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/cli/src/commands/control_ui.html
  - crates/cli/src/commands/control_ui.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/start.rs
  - README.md
  - docs/src/deployment/production.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 19 Retroactive Code Review

Reviewed the historical Phase 19 enterprise-admin surface against the current `HEAD`
implementation, using the original phase summaries and commit range to reconstruct scope.

### WR-01: Enterprise admin tokens are accepted from the URL and persisted in long-lived browser storage

**File:** `crates/cli/src/commands/control_ui.html:1728-1750`, `crates/cli/src/commands/control_ui.html:2856-2905`

**Issue:** The enterprise admin panel seeds `operator_token` and `approver_token` from the control
UI query string and writes both secrets into persistent `localStorage`. That exposes protected-write
credentials through browser history, copied links, and long-lived client-side storage, which is a
real secret-handling regression for the enterprise admin loop this phase introduced.

**Fix:** Stop accepting enterprise tokens from URL query params, keep only non-secret operator and
approver IDs in `localStorage`, move the tokens to `sessionStorage`, and add a UI regression test so
future control UI changes cannot silently reintroduce persistent token storage.
