---
phase: 187-oauth-safe-delegated-backend-contracts
plan: "01"
subsystem: delegated-backend-contracts
tags: [agents, oauth, delegated-backends, policy, compliance]
requires: []
provides:
  - typed delegated local-agent backend contracts
  - truthful model-catalog labels for delegated backends
  - policy-ready allow or deny evaluation for local vendor-agent CLIs
affects: [phase-187-plan-02, inspect, enterprise-policy, routing]
tech-stack:
  added: []
  patterns: [typed contract layer, vendor-managed labeling, allowlist policy evaluation]
key-files:
  created:
    - crates/app/src/agent_backend_control.rs
  modified:
    - crates/app/src/lib.rs
key-decisions:
  - "Modeled local vendor-agent CLIs as delegated backend contracts rather than direct provider aliases."
  - "Labeled model availability as `vendor_managed` whenever the CLI is usable but does not expose trustworthy model discovery."
  - "Reused the external-backend allowlist shape and local-wrapper gate instead of introducing a separate shadow policy lane."
patterns-established:
  - "Discovery metadata should be normalized into policy-aware contracts before any routing or onboarding flow treats a backend as executable."
  - "Detection-only or unsupported tools remain visible but ineligible through explicit contract evaluation rather than hidden heuristics."
requirements-completed: [AUTH-01, AUTH-02, AUTH-04]
duration: n/a
completed: 2026-04-08
---

# Phase 187: OAuth-Safe Delegated Backend Contracts Summary

**Phase 187 plan 01 now gives OpenRustClaw a typed delegated-backend contract layer, so local vendor-agent CLIs can be treated as policy-gated external execution lanes instead of ambiguous discovery rows.**

## Accomplishments

- Added `AgentBackendControlService` to turn local discovery entries into delegated backend contracts with explicit provider mapping, transport shape, readiness, execution eligibility, and model-catalog labeling.
- Introduced truthful contract labels for model-catalog behavior: `supported`, `vendor_managed`, `unknown`, and `unsupported`.
- Added allowlist-based execution policy evaluation for delegated local agent backends using the same boundary shape as existing external backend controls.
- Kept detection-only surfaces ineligible for execution while leaving them visible in the contract layer.

## Verification

- `cargo fmt --all`
- `cargo test -p openrustclaw-app agent_backend_control -- --nocapture`
- `cargo check -p openrustclaw-app`

## Remaining Work

- Reuse delegated backend contracts in inspect and enterprise policy surfaces.
- Normalize allowlist updates so browser and delegated local-agent backends share one truthful operator policy story.
- Carry policy decisions and contract readiness into later onboarding and runtime routing phases.

---
*Phase: 187-oauth-safe-delegated-backend-contracts*
*Completed: 2026-04-08*
