---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/app/src/agent_backend_catalog.rs
  - crates/app/src/agent_backend_control.rs
  - crates/app/src/runtime_provider_switch.rs
  - crates/cli/src/commands/runtime.rs
  - crates/cli/src/commands/onboard.rs
  - crates/cli/src/commands/models.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 191 Code Review

Standard review of the Phase 191 Cursor backend-expansion flow.

### WR-01: Cursor remains classified as `integration_only`, which leaves the new runtime and onboarding path unreachable

**Files:** `crates/app/src/agent_backend_catalog.rs:251-263`, `crates/app/src/runtime_provider_switch.rs:121-123`, `crates/cli/src/commands/runtime.rs:3816-3820`

**Issue:** The phase added runtime-provider switching, delegated-provider creation, and onboarding/model handling for `cursor`, but the backend catalog still marks Cursor as `integration_only` with delegated execution unsupported. That means `discover_delegated_cli_candidates()` still excludes Cursor, policy evaluation still denies it as execution-ineligible, and the new runtime/onboarding Cursor path cannot actually be selected successfully.

**Fix:** Promote Cursor's catalog probe to the delegated backend contract that the phase already built around its documented `cursor agent` auth, model, and headless print surfaces, then update the contract tests to prove Cursor is included as a delegated CLI candidate.
