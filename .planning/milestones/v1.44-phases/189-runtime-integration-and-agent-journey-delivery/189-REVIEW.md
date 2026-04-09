---
status: findings
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/app/src/agent_backend_control.rs
  - crates/cli/src/commands/runtime.rs
  - crates/cli/src/commands/control.rs
  - crates/cli/src/commands/inspect.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 189 Code Review

Standard review of the Phase 189 delegated runtime and control-registry integration flow.

### WR-01: Control init seeds delegated model profiles without applying the configured backend policy

**File:** `crates/cli/src/commands/control.rs:708-749`

**Issue:** `write_detected_delegated_model_profiles()` filters discovery contracts only by `contract.execution_eligible`, which reflects intrinsic catalog readiness but not the workspace's configured external-backend policy. If local CLI wrappers are disabled or a backend is removed from `allowed_backends`, control initialization can still write `delegated-*` model profiles for backends that runtime execution will later deny.

**Fix:** Load the workspace runtime config during control initialization, derive the actual delegated-backend policy, and only seed delegated model profiles for contracts that remain allowed after `evaluate_execution()`.
