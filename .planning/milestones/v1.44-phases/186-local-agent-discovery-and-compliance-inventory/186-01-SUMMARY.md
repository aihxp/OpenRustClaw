---
phase: 186-local-agent-discovery-and-compliance-inventory
plan: "01"
subsystem: agent-discovery
tags: [agents, discovery, onboarding, models, compliance]
requires: []
provides:
  - shared local-agent discovery catalog
  - truthful readiness and auth metadata for installed local agent binaries
  - first shipped operator-facing catalog surface in `openrustclaw models`
affects: [phase-186-plan-02, onboarding, inspect, provider-catalog]
tech-stack:
  added: []
  patterns: [shared catalog service, safe local CLI probing, detection-only classification]
key-files:
  created:
    - crates/app/src/agent_backend_catalog.rs
  modified:
    - crates/app/src/lib.rs
    - crates/cli/src/commands/models.rs
key-decisions:
  - "Kept delegated local agent discovery distinct from direct API providers instead of flattening everything into one provider type."
  - "Classified Cursor as detection-only until a documented programmable backend surface is confirmed."
patterns-established:
  - "Shared app-layer catalog services should feed operator surfaces instead of adding more static command-local inventories."
  - "Vendor auth status can be surfaced truthfully through documented status commands without importing credentials."
requirements-completed: [DISC-01, DISC-02, DISC-03]
duration: n/a
completed: 2026-04-08
---

# Phase 186: Local Agent Discovery and Compliance Inventory Summary

**OpenRustClaw now has a shared app-layer catalog for installed local agent backends, and `openrustclaw models` is the first shipped surface to render it alongside direct providers.**

## Accomplishments

- Added `AgentBackendCatalogService` in `openrustclaw-app` to probe installed local agent binaries safely and classify auth, readiness, policy, and notes.
- Added typed parsing for `claude auth status` and `codex login status`.
- Classified ambiguous or unsupported surfaces truthfully, including Cursor as detection-only.
- Updated `openrustclaw models` to show both direct providers and detected local agent backends from the shared catalog.
- Added Gemini to the direct provider list so the models surface is less stale.

## Verification

- `cargo fmt --all`
- `cargo test -p openrustclaw-app agent_backend_catalog -- --nocapture`
- `cargo test -p openrustclaw-cli models -- --nocapture`

## Remaining Work

- Reuse the shared catalog in onboarding, inspect, and control surfaces.
- Add policy-aware persistence and richer readiness reasons for delegated backends.
- Define the boundary between detection-only surfaces and execution-capable delegated backends more explicitly in runtime policy.

---
*Phase: 186-local-agent-discovery-and-compliance-inventory*
*Completed: 2026-04-08*
