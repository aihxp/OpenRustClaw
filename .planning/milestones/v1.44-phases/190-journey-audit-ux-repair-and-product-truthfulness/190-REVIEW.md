---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - README.md
  - crates/cli/src/commands/onboard.rs
  - crates/cli/src/commands/models.rs
  - crates/cli/src/commands/inspect.rs
  - crates/cli/src/commands/control_ui.html
  - .planning/PROJECT.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 190 Code Review

Standard review of the Phase 190 operator-facing journey and truthfulness surfaces.

### WR-01: Shipped docs still describe Cursor as a supported delegated execution lane

**Files:** `README.md:52`, `.planning/PROJECT.md:53`

**Issue:** The current product copy still groups Cursor with supported delegated local-agent execution lanes and claims "truthful Cursor execution support." That no longer matches the shipped backend contract: Cursor is surfaced for detection and model discovery, but the delegated-backend policy classifies it as discovery-only rather than execution-eligible.

**Fix:** Update the README and planning summary language so Cursor is described as a discovered local-agent surface, not as a supported delegated execution lane.
