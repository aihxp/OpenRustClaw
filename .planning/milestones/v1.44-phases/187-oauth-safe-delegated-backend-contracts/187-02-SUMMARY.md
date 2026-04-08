---
phase: 187-oauth-safe-delegated-backend-contracts
plan: "02"
subsystem: inspect-and-policy
tags: [agents, inspect, policy, enterprise, delegated-backends]
requires: [187-01]
provides:
  - inspect setup handoff exposure of delegated backend contracts
  - enterprise policy visibility for delegated local agent backends
  - external-backend allowlist normalization that accepts delegated agent backend ids
affects: [phase-188, inspect, enterprise-policy, setup-handoff]
tech-stack:
  added: []
  patterns: [shared contract reuse, policy normalization, inspect enrichment]
key-files:
  created: []
  modified:
    - crates/app/src/setup_handoff.rs
    - crates/cli/src/commands/enterprise_policy.rs
    - crates/cli/src/commands/inspect.rs
key-decisions:
  - "Reused setup handoff as the first inspect-facing surface for delegated backend contracts instead of inventing a disconnected one-off report."
  - "Kept delegated local agent allowlists under the existing external-backend policy lane so browser and agent wrappers share one operator boundary."
  - "Accepted delegated agent backend ids in enterprise-policy normalization without pretending detection-only surfaces are execution-ready."
patterns-established:
  - "Typed delegated backend contracts should be surfaced through existing operator reports before routing depends on them."
  - "External backend allowlists can cover multiple bounded wrapper families as long as normalization remains truthful and explicit."
requirements-completed: [AUTH-03]
duration: n/a
completed: 2026-04-08
---

# Phase 187: OAuth-Safe Delegated Backend Contracts Summary

**Phase 187 is now complete: delegated local vendor-agent backends have typed contracts, policy-ready allow or deny evaluation, and visible operator-facing surfaces through setup handoff and enterprise policy.**

## Accomplishments

- Extended setup handoff reports so inspect consumers can see delegated backend contracts derived from live local-agent discovery.
- Extended enterprise policy reports so delegated local agent backends appear alongside the existing external-backend boundary.
- Updated enterprise policy normalization to accept delegated local agent backend ids such as `claude_code`, `codex`, and `gemini_cli`.
- Kept the allowlist and local-wrapper gate unified under the existing external-backend policy lane instead of creating a separate hidden policy surface.

## Verification

- `cargo fmt --all`
- `cargo test -p openrustclaw-cli inspect -- --nocapture`
- `cargo test -p openrustclaw-cli enterprise_policy -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Remaining Work

- Use delegated backend contracts to drive truthful onboarding and model-selection menus.
- Preserve delegated backend choices through repair, resume, and first launch.
- Introduce bounded runtime routing and receipts for actual delegated task execution.

---
*Phase: 187-oauth-safe-delegated-backend-contracts*
*Completed: 2026-04-08*
