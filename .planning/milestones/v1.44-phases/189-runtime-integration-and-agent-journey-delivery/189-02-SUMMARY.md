---
phase: 189-runtime-integration-and-agent-journey-delivery
plan: "02"
subsystem: delegated-control-registry-integration
tags: [control, model-profiles, delegated-backends, inspect, runtime]
requires: [189-01]
provides:
  - delegated backend model-profile templates in control init
  - vendor-managed delegated runtime profiles with explicit fallback chains
  - operator-visible delegated runtime receipts through existing inspect surfaces
affects: [phase-190, control, runtime, inspect]
tech-stack:
  added: []
  patterns: [vendor-managed templates, control-init seeding, inspect reuse]
key-files:
  created: []
  modified:
    - crates/cli/src/commands/control.rs
    - crates/cli/src/commands/runtime.rs
key-decisions:
  - "Seeded delegated backend model profiles only when the local backend is execution-eligible."
  - "Used `vendor-managed` as the model placeholder so control init does not guess unsupported model ids."
  - "Kept inspect reuse implicit by writing delegated receipts into the same shipped audit surfaces enterprise and operator flows already read."
patterns-established:
  - "Control initialization can safely expose delegated runtime lanes by seeding honest vendor-managed templates instead of hard-coding guessed model catalogs."
  - "Delegated runtime and inspect surfaces should converge through shared receipt stores before any UI-specific polish work."
requirements-completed: [ROUT-02, ROUT-04]
duration: n/a
completed: 2026-04-08
---

# Phase 189: Runtime Integration and Agent Journey Delivery Summary

**Phase 189 is now complete: delegated local-agent execution is available as a bounded runtime lane, control init can seed operator-facing templates for eligible backends, and the resulting receipts land in existing inspectable audit surfaces.**

## Accomplishments

- Extended `control init` so eligible delegated backends create vendor-managed model-profile templates automatically.
- Kept those templates truthful by using delegated backend ids directly and avoiding guessed model names.
- Preserved fallback chains back to direct providers and local runtime profiles so delegated lanes can fail safely.
- Completed the runtime and control integration without bypassing the existing policy, audit, or inspect surfaces.

## Verification

- `cargo check -p openrustclaw-cli --tests`
- `cargo test -p openrustclaw-cli runtime -- --nocapture`
- `cargo test -p openrustclaw-cli control -- --nocapture`

## Remaining Work

- Audit the install-to-onboard-to-first-task-to-inspect journey across docs and runtime strings.
- Repair disconnected terminology and dead-end guidance across product surfaces.
- Align the public story with the now-shipped delegated runtime boundary.

---
*Phase: 189-runtime-integration-and-agent-journey-delivery*
*Completed: 2026-04-08*
