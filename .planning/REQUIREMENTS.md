# Requirements: v1.22 Full Greenfield Conversion: Orchestration and Browser Services

**Status:** Drafted 2026-03-28
**Full-conversion baseline:** `3/6` milestones shipped, or about `50%`
**Target after shipment:** `4/6` milestones shipped, or about `67%`

## Milestone Goal

Move the next orchestration and browser business-logic queues behind `openrustclaw-app` so `orchestrate.rs` and the browser command surfaces continue shrinking toward adapter-only ownership.

## Active Requirements

- `GFC-33` Orchestration request routing, override validation, and lifecycle-state transition services
- `GFC-34` Orchestration checkpoint, transcript, trace, reflection, and supervision summary services
- `GFC-35` Browser backend policy and audit services
- `GFC-36` Browser session persistence, workflow execution, inspection, and sequence-orchestration services

## Constraints

- Preserve current CLI and control-surface contracts while extracting application ownership.
- Keep the retired historical `18/18` seam ledger fixed and use the six-milestone full-conversion roadmap for current percentage reporting.
- Leave persistence, transport wiring, and external runtime I/O in adapters unless a phase explicitly extracts a narrower boundary.

## Out of Scope

- redefining the retired historical seam ledger
- broad onboarding or setup migrations scheduled for later full-conversion milestones
- adapter-only exit enforcement work reserved for `v1.24`
