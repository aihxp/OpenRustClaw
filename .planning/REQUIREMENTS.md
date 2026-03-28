# Requirements: v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces

**Started:** 2026-03-28
**Historical greenfield baseline:** retired `18/18` ranked seam ledger complete, or `100%`
**Full-conversion roadmap baseline:** `4/6` milestones shipped, or about `67%`
**Target after shipment:** `5/6` milestones shipped, or about `83%`

## Scope

This milestone continues the post-`18/18` full-conversion program by targeting the remaining setup-lifecycle and secondary command-module seams that still own mixed business logic outside the major hotspots already reduced in `start.rs`, `mobile.rs`, `voice_runtime.rs`, `orchestrate.rs`, and `browser.rs`.

## Milestone Requirements

### Setup Lifecycle

- [ ] **GFC-37**: Operator-facing onboarding, repair, and resume orchestration compose through `openrustclaw-app`, with `onboard.rs` reduced toward an adapter around setup state, workspace I/O, and bounded runtime probes.

### Secondary Lifecycle Commands

- [ ] **GFC-38**: The targeted residual lifecycle seams in `channels.rs`, `schedule.rs`, `services.rs`, and `control.rs` compose through `openrustclaw-app`, with those command modules reduced toward adapters around transport, persistence, and bounded side effects.

### Secondary Operator Helpers

- [ ] **GFC-39**: The targeted residual operator, media, tools, and memory helper seams compose through `openrustclaw-app`, with the affected secondary command modules reduced toward adapters around artifact, runtime, and workspace inputs.

### Adapter Cleanup

- [ ] **GFC-40**: Transition-era helper duplication created during the brownfield-to-greenfield migration is removed, consolidated, or explicitly bounded so the affected command modules expose clearer adapter-only ownership.

## Future Requirements

- Final adapter-only enforcement, architecture guardrails, and the exit audit remain deferred to `v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement`.

## Out of Scope

- Defining a new greenfield percentage denominator beyond the six-milestone full-conversion program
- Broad new product capabilities unrelated to shrinking setup and secondary command ownership
- Final architecture guardrails and exit-scorecard claims before the `v1.24` enforcement milestone

## Traceability

- `GFC-37` -> Phase 97
- `GFC-38` -> Phase 98
- `GFC-39` -> Phase 99
- `GFC-40` -> Phase 100
