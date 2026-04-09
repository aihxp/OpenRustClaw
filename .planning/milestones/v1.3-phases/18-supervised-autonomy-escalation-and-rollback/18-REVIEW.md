---
status: findings
depth: standard
files_reviewed: 7
files_reviewed_list:
  - crates/cli/src/commands/orchestrate.rs
  - crates/cli/src/commands/start.rs
  - crates/cli/src/commands/enterprise_access.rs
  - crates/cli/src/commands/control_ui.rs
  - crates/cli/src/commands/control_ui.html
  - README.md
  - docs/src/deployment/production.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 18 Retroactive Code Review

Reviewed the historical Phase 18 supervised-autonomy lifecycle patch against the current `HEAD`
implementation, using the original phase summaries and commit range to reconstruct scope.

### WR-01: Authenticated operator attribution is dropped for pause, resume, and kill actions

**File:** `crates/cli/src/commands/start.rs:9345-9468`, `crates/cli/src/commands/orchestrate.rs:1542-1584`

**Issue:** Phase 18 introduced durable intervention history so operators can see who escalated,
rolled back, or otherwise intervened in an active orchestration run. In the shipped flow,
`escalate` and `rollback` preserved the authenticated operator ID, but `pause`, `resume`, and
`kill` still called the orchestration helpers without any intervention request payload. That
caused the decision ledger to record the literal fallback value `"operator"` instead of the
authenticated enterprise operator, breaking the phase’s explicit intervention-evidence contract for
three of the five lifecycle controls.

**Fix:** Thread `ActiveRunInterventionRequest` through the pause, resume, and kill surfaces as well,
populate it from the authenticated operator when present, and add a regression test proving those
three actions now retain the real operator ID in decision history.
