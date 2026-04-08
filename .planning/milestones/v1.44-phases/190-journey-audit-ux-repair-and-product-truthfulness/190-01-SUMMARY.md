---
phase: 190-journey-audit-ux-repair-and-product-truthfulness
plan: "01"
subsystem: onboarding-and-docs-journey-repair
tags: [journey, onboarding, docs, providers, delegated-backends]
requires: []
provides:
  - truthful README language for provider lanes and delegated local agents
  - onboarding copy that distinguishes first-run bootstrap from later delegated execution
  - model-list output that explains direct lanes versus delegated local agent lanes
affects: [phase-190-plan-02, onboarding, docs, model-selection]
tech-stack:
  added: []
  patterns: [truthful-copy, lane-language, journey-alignment]
key-files:
  created: []
  modified:
    - README.md
    - crates/cli/src/commands/onboard.rs
    - crates/cli/src/commands/models.rs
key-decisions:
  - "Kept delegated local agent lanes visible during onboarding without pretending they are raw token-import provider paths."
  - "Explained that first-run bootstrap still validates the documented provider lane while later delegated execution uses the installed CLI directly."
  - "Aligned the README and `openrustclaw models` output to the same provider-lane versus delegated-agent language as onboarding."
patterns-established:
  - "Operator-facing copy should describe delegated local agents as visible runtime lanes, not hidden credentials or fake API providers."
  - "Onboarding can preserve delegated lane identity while still using a truthful provider bootstrap contract underneath."
requirements-completed: [JOUR-01, JOUR-04]
duration: n/a
completed: 2026-04-08
---

# Phase 190: Journey Audit, UX Repair, and Product Truthfulness Summary

**Phase 190 plan 01 repaired the entry journey from README through onboarding and model selection so OpenRustClaw now describes direct providers, local runtimes, and delegated local agents with one truthful vocabulary.**

## Accomplishments

- Updated the repo entrypoint to explain delegated local agent lanes without implying token scraping or unsupported OAuth reuse.
- Tightened onboarding copy so delegated local agent selections stay visible while first-run bootstrap still uses the documented provider path.
- Reframed `openrustclaw models` around provider and runtime lanes, with explicit notes about delegated local agent boundaries and installed-CLI execution.

## Verification

- `cargo fmt --all`
- `cargo test -p openrustclaw-cli onboard -- --nocapture`
- `cargo test -p openrustclaw-cli models -- --nocapture`

## Remaining Work

- Surface the same lane language and delegated runtime cues in inspect and Control UI.
- Make delegated runtime receipts easier to understand once work has actually been routed externally.

---
*Phase: 190-journey-audit-ux-repair-and-product-truthfulness*
*Completed: 2026-04-08*
