---
status: findings
depth: standard
files_reviewed: 7
files_reviewed_list:
  - crates/app/src/agent_backend_catalog.rs
  - crates/app/src/lib.rs
  - crates/cli/src/commands/models.rs
  - crates/cli/src/commands/onboard.rs
  - crates/cli/src/commands/inspect.rs
  - crates/app/src/setup_lifecycle.rs
  - crates/app/src/control_registry.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 186 Code Review

Standard review of the full Phase 186 local-agent discovery and onboarding reuse surfaces.

### WR-01: Cursor is still mislabeled as a delegated backend instead of detection-only

**File:** `crates/app/src/agent_backend_catalog.rs:251-264`

**Issue:** The phase contract said Cursor should remain visible as detection-only, but the shared catalog still classifies it as `delegated_cli_candidate` with supported delegated execution and `Ready` readiness. That lets downstream surfaces present Cursor like a shippable delegated agent lane instead of an ambiguous integration surface.

**Fix:** Reclassify Cursor as `integration_only` in the shared catalog so readiness resolves to detection-only while still preserving any discoverable model metadata for operator visibility.
