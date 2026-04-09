---
status: findings
depth: standard
files_reviewed: 4
files_reviewed_list:
  - README.md
  - docs/src/deployment/remote-connectivity.md
  - docs/src/deployment/production.md
  - crates/distributed/README.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 53 Retroactive Code Review

Reviewed the node-topology contract against the current operator-facing entry surfaces.

### WR-01: Remote-connectivity entry surfaces drifted away from the canonical fallback order

**Files:** `README.md`, `docs/src/deployment/production.md`

**Issue:** The canonical guide at `docs/src/deployment/remote-connectivity.md` now defines the
fallback order as node-first, then Tailscale tailnet, then SSH tunnel, then reverse proxy. The
README and production runbook still jump directly from node-first to SSH tunnel. That breaks the
phase `53` goal of keeping the primary topology entry surfaces aligned on one explicit remote-access
story.

**Fix:** Update the README and production runbook to include the Tailscale tailnet fallback before
SSH tunnel so the operator-facing entry surfaces match the canonical remote-connectivity guide.
