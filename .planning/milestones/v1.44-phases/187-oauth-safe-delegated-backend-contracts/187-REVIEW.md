---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/app/src/agent_backend_control.rs
  - crates/app/src/lib.rs
  - crates/app/src/agent_backend_catalog.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/enterprise_policy.rs
  - crates/cli/src/commands/browser.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 187 Code Review

Standard review of the full Phase 187 delegated-backend contract and policy-facing surfaces.

### WR-01: Enterprise policy summary does not apply the actual allowlist or wrapper policy to delegated backends

**File:** `crates/cli/src/commands/enterprise_policy.rs:312-315`

**Issue:** `summary()` publishes `delegated_agent_backends` directly from discovery contracts via `contracts_from_catalog()`. Those contracts describe intrinsic readiness, but they are never run through `evaluate_execution()` with the real enterprise backend policy. As a result, operator-facing policy surfaces can still report a delegated backend as execution-eligible even when `allow_local_cli_wrappers` is disabled or the backend is not allowlisted.

**Fix:** Apply the existing `AgentBackendControlService::evaluate_execution()` policy check when building the summary report and downgrade reported execution eligibility with the blocking reason whenever enterprise policy would deny the backend.
