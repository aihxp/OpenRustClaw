---
phase: 06-deployment-runtime-and-operator-ops
plan: 02
subsystem: runtime-ops-docs
tags:
  - docs
  - runtime
  - deployment
  - recovery
provides:
  - Truthful deploy-run-recover loop in top-level and deployment docs
  - Installation guide bridge into managed runtime operations
  - Documentation for the new operator-ops summary surface
affects:
  - Top-level operator guidance
  - Installation documentation
  - Production deployment runbook
tech-stack:
  added: []
  patterns:
    - Align operator docs around the exact shipped CLI and control-plane runtime commands
key-files:
  created: []
  modified:
    - README.md
    - docs/src/getting-started/installation.md
    - docs/src/deployment/production.md
key-decisions:
  - The canonical MVP operations story is install-status -> health -> backup -> upgrade or rollback -> browser confirmation, not a generic infrastructure essay
  - The browser-facing `Operator Ops Summary` should be documented as the same contract the CLI surfaces expose
patterns-established:
  - Operator docs should point to one concrete control endpoint or UI panel whenever a runtime trust loop is introduced
duration: 15min
completed: 2026-03-26
---

# Phase 6: Deployment, Runtime, and Operator Ops Summary

**Aligned README, installation, and production docs around the actual Rust runtime maintenance path instead of leaving restart and recovery scattered across command references.**

## Performance
- **Duration:** ~15 min
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- Updated README with the runtime operator loop around install status, health, upgrade planning, backup, rollback, and `/control/runtime/operator-ops`.
- Added an installation-guide step that validates the runtime operations path before an operator treats the workspace as production-ready.
- Expanded the production guide with one canonical deploy-run-recover sequence tied to the shipped runtime commands and the Control UI summary panel.

## Verification
- Manual cross-check against shipped commands and control endpoints:
  - `openrustclaw runtime services install-status`
  - `openrustclaw runtime health`
  - `openrustclaw runtime backup`
  - `openrustclaw runtime upgrade-plan --config config/default.toml`
  - `GET /control/runtime/operator-ops`

## Next Phase Readiness
Plan 03 can now lock the documented runtime operator contract with integration coverage.
