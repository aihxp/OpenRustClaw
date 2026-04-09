---
status: findings
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/cli/src/commands/enterprise_autonomy.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/control_ui.html
  - crates/app/src/enterprise_admin.rs
  - crates/core/src/types.rs
  - crates/db/src/models.rs
  - crates/db/src/migrate.rs
  - crates/db/src/learning_store.rs
  - crates/db/src/skill_proposal_store.rs
  - crates/app/src/learning_review.rs
  - crates/app/src/skill_proposals.rs
  - crates/cli/src/commands/control.rs
  - crates/cli/src/commands/start.rs
  - crates/cli/src/main.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 185 Code Review

Standard review of the full Phase 185 God Mode overlay, expiry, provenance, and quarantine surfaces across both execution plans.

### WR-01: Re-enabling after expiry can capture the stale God Mode policy as the new baseline

**File:** `crates/cli/src/commands/enterprise_autonomy.rs:273-301`

**Issue:** `enable()` snapshots `current_runtime` before it calls `refresh_manifest_state()`. If the existing manifest has already expired, `refresh_manifest_state()` restores the baseline runtime policy, but `enable()` still records the pre-refresh God Mode override as the new `baseline_policy`. The next disable or expiry can then restore to the stale override instead of the real baseline.

**Fix:** Refresh manifest state before reading the current runtime policy, and add a regression that expires God Mode, re-enables it, and verifies the persisted baseline still matches the restored pre-God-Mode runtime.
