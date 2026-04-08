---
phase: 188-onboarding-and-model-selection-cohesion
plan: "01"
subsystem: shared-lane-catalog
tags: [onboarding, models, lanes, delegated-backends, ux]
requires: []
provides:
  - shared onboarding and model lane catalog
  - truthful delegated lane rendering for detected local agents only
  - canonical lane labels reused across onboarding and models list
affects: [phase-188-plan-02, onboarding, models, inspect]
tech-stack:
  added: []
  patterns: [shared typed catalog, truthful lane rendering, delegated compatibility labeling]
key-files:
  created:
    - crates/app/src/onboarding_lane_catalog.rs
  modified:
    - crates/app/src/lib.rs
    - crates/cli/src/commands/models.rs
    - crates/cli/src/commands/onboard.rs
key-decisions:
  - "Centralized onboarding and model rows behind one typed lane catalog instead of duplicating command-local provider tables."
  - "Only surfaced delegated local-agent lanes when the matching backend is actually detected on the machine."
  - "Kept delegated lanes visibly distinct and compatibility-labeled instead of pretending they are ordinary direct providers."
patterns-established:
  - "Provider-or-agent menus should consume shared lane descriptors rather than re-encoding provider metadata per CLI command."
  - "Delegated local-agent lanes can reuse direct provider bootstrap paths while still preserving truthful UX boundaries."
requirements-completed: [ONBR-01, ONBR-04]
duration: n/a
completed: 2026-04-08
---

# Phase 188: Onboarding and Model Selection Cohesion Summary

**Phase 188 plan 01 converged onboarding and `openrustclaw models` onto one shared provider-or-agent lane catalog, so the first menu an operator sees now matches the model inventory the product reports later.**

## Accomplishments

- Added `OnboardingLaneCatalogService` to assemble direct API lanes, the local Ollama runtime lane, and detected delegated local-agent lanes into one typed catalog.
- Updated onboarding to render selection rows from that shared lane catalog instead of a command-local static provider table.
- Updated `openrustclaw models` to reuse the same lane catalog for direct providers and detected delegated agent backends.
- Kept delegated local-agent lanes honest by surfacing vendor-managed or compatibility notes rather than implying full routing support before Phase 189.

## Verification

- `cargo test -p openrustclaw-app onboarding_lane_catalog -- --nocapture`
- `cargo test -p openrustclaw-cli onboard -- --nocapture`
- `cargo test -p openrustclaw-cli models -- --nocapture`

## Remaining Work

- Persist the selected lane identity, lane type, and delegated backend metadata through setup handoff and inspect.
- Preserve delegated-lane choice across repair and resume flows.
- Introduce actual delegated runtime routing and execution receipts in Phase 189.

---
*Phase: 188-onboarding-and-model-selection-cohesion*
*Completed: 2026-04-08*
