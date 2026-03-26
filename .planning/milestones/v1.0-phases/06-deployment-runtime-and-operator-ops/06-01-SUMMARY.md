---
phase: 06-deployment-runtime-and-operator-ops
plan: 01
subsystem: runtime-operator-summary
tags:
  - runtime
  - control-ui
  - operator-ops
  - recovery
provides:
  - Typed runtime operator-ops summary surface
  - `/control/runtime/operator-ops` control endpoint
  - `Operator Ops Summary` panel in Control UI
affects:
  - Runtime control plane
  - Browser operator dashboard
  - Runtime recovery visibility
tech-stack:
  added: []
  patterns:
    - Compose multiple existing runtime primitives into one operator-facing summary instead of re-implementing recovery logic in the UI
key-files:
  created: []
  modified:
    - crates/cli/src/commands/runtime.rs
    - crates/cli/src/commands/start.rs
    - crates/cli/src/commands/control_ui.html
    - crates/cli/src/commands/control_ui.rs
key-decisions:
  - Runtime operations should start from one summary that combines health, beacon, service-install state, lock state, reload posture, and actionable guidance
  - Control UI should consume the same summary payload as other control surfaces rather than inventing frontend-only recovery logic
patterns-established:
  - Later trust phases can add browser-first summary panels by composing existing typed control-plane reports
duration: 20min
completed: 2026-03-26
---

# Phase 6: Deployment, Runtime, and Operator Ops Summary

**Added one coherent operator-ops summary so deploy, restart, and recovery decisions no longer require hopping between unrelated runtime endpoints.**

## Performance
- **Duration:** ~20 min
- **Tasks:** 3 completed
- **Files modified:** 4

## Accomplishments
- Added `runtime_operator_ops_summary(...)` in `runtime.rs` to compose runtime health, beacon, reload state, service-install status, runtime-lock status, backup roots, and recommended operator actions.
- Exposed the summary through `/control/runtime/operator-ops`.
- Added `Operator Ops Summary` to Control UI so browser operators can see the managed-service and recovery posture alongside the existing runtime panels.

## Verification
- `cargo test -p openrustclaw-cli runtime_operator_ops_summary_reports_recovery_guidance -- --nocapture`
- `cargo test -p openrustclaw-cli dashboard_includes_runtime_operator_ops_panel -- --nocapture`

## Next Phase Readiness
Plan 02 can now align the docs around the same operator summary and recovery contract.
