# Phase 179: Primary Model Discovery and Persistence - Context

**Gathered:** 2026-03-30
**Status:** Ready for planning
**Mode:** Smart discuss

<domain>
## Phase Boundary

Phase 179 should make onboarding capture an explicit primary task model instead of silently inheriting whatever model already lives in runtime config. The implementation must persist the provider-model pair through the existing runtime mutation lane and keep setup state aware of both the selected model and how it was chosen.

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing runtime mutation lane
Provider and model persistence should continue to flow through `runtime::switch_provider(...)` and the underlying `RuntimeProviderSwitchService`, not through a new onboarding-only config writer.

### Use live catalog discovery when available, but keep a manual fallback
Live catalog discovery should happen after provider setup succeeds. When discovery returns usable models, onboarding should offer an explicit selection from that discovered set. When discovery is empty or unavailable, onboarding should fall back to manual entry with a recommended default seeded from the provider's current runtime config.

### Persist model choice and provenance in setup state
Setup state should store the selected primary model and whether it came from live discovery, recommended fallback, or manual entry. That keeps resume and later handoff phases from losing model intent.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` already owns provider selection and calls `runtime::switch_provider(...)`, but it currently passes `model=None`.
- `crates/app/src/runtime_provider_switch.rs` already persists provider and model together when a model is supplied, and it keeps control-plane defaults aligned after a switch.
- `crates/cli/src/commands/runtime.rs` already contains live provider model-catalog fetch logic as part of runtime health checks.
- Setup state, setup lifecycle, inspect, and handoff surfaces currently know about provider and access mode, but they did not originally persist an explicit primary model.

</code_context>

<specifics>
## Specific Ideas

- Add model discovery and explicit model selection inside `run_model_setup(...)`.
- Allow provider verification to continue past a temporary `model_unavailable` warning long enough for onboarding to choose a new model from the live catalog.
- Persist `selected_primary_model` and `selected_primary_model_source` into durable setup state and threaded app mappings.

</specifics>

<deferred>
## Deferred Ideas

- Richer handoff display of the selected model belongs to Phase 180.
- Broad model comparison, ranking, or role-aware recommendations stay out of scope for this milestone.

</deferred>
