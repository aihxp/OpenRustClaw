---
status: findings
depth: standard
files_reviewed: 3
files_reviewed_list:
  - docs/src/contributing/development.md
  - .planning/codebase/GREENFIELD.md
  - CLAUDE.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 60 Retroactive Code Review

Reviewed the contributor-defaults surface against the current development guidance and greenfield
contract.

### WR-01: The contributor setup guide still points at the retired repository URL

**File:** `docs/src/contributing/development.md`

**Issue:** The development guide still tells contributors to clone
`https://github.com/openrustclaw/openrustclaw.git`. The workspace metadata and public crate surfaces
now use `https://github.com/aihxp/OpenRustClaw`. That stale clone URL breaks the contributor default
story this phase was meant to harden.

**Fix:** Update the development guide to clone `https://github.com/aihxp/OpenRustClaw.git` so the
contributor entry point matches the live repository contract.
