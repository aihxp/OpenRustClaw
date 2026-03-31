# Phase 177: Provider Access Modes and Resume State - Context

**Gathered:** 2026-03-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the onboarding model step choose a truthful provider path first, represent only the access modes that actually apply to that provider, and persist provider or access-mode context strongly enough that resume can continue the step without losing valid choices. This phase is limited to provider-path selection and resume state. It does not include live verification, model discovery, model persistence, Control UI work, or a generic provider framework.

</domain>

<decisions>
## Implementation Decisions

### Provider Path Truthfulness
- Keep Phase 177 bounded to the existing onboarding-supported providers rather than widening first-run support to every provider crate in the workspace.
- Model provider access explicitly enough that onboarding can show only the valid path types for the selected provider, including remote API-key flows, local-runtime flows, and any bounded combination that the shipped provider lane actually supports.
- Do not ask for irrelevant credentials for a selected provider path; provider-specific input collection must follow the chosen access mode instead of a single hardcoded API-key path.
- Keep provider-path rules close to the onboarding and runtime boundary, not as a new generic provider plugin framework.

### Resume State Continuity
- Extend the durable setup-state contract so the onboarding model step can resume from previously selected provider and access-mode choices when they are still valid.
- Treat provider selection and access-mode selection as part of the model-step state rather than transient wizard memory only.
- Preserve the current repair or resume posture where setup-state evidence drives the next action instead of re-asking every onboarding question.
- Keep later verification and model-selection data out of this phase unless needed only to preserve compatibility for the new provider-path state shape.

### Phase Boundaries
- Reuse the existing onboarding command, setup-state persistence, and runtime provider-switch seam rather than inventing a separate onboarding system.
- Leave provider verification, failure classification, and bootstrap evidence for Phase 178.
- Leave live model discovery, manual model entry fallback, and provider plus model persistence for Phase 179.
- Leave setup handoff, first-launch continuity, and downstream operator guidance for Phase 180.

### the agent's Discretion
The exact Rust type layout, module placement, and prompt wording are at the agent's discretion as long as the shipped behavior stays truthful, resume-aware, and scoped to provider-path selection rather than broader model-management work.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/cli/src/commands/onboard.rs` already owns the interactive onboarding wizard, setup-step selection, setup-state persistence, resume detection, and model-step entrypoint through `run_model_setup()`.
- `SetupState`, `SetupStateManifest`, `load_setup_state()`, `save_setup_state()`, and `update_setup_state()` already provide the durable state lane that Phase 177 should extend.
- `OnboardingState` already carries `model_configured` and `preferred_provider`, which shows there is already an onboarding-local provider memory path to tighten.
- `crates/app/src/runtime_provider_switch.rs` already centralizes provider and model mutation semantics behind `RuntimeProviderSwitchService`.

### Established Patterns
- Large operator flows stay in `crates/cli/src/commands/*.rs`, while reusable business logic is moved into `openrustclaw-app` through typed services and narrow data structures.
- Runtime or operator state is usually persisted as typed serializable structs instead of ad hoc key-value blobs.
- Brownfield phases prefer extracting one bounded seam cleanly instead of widening scope into a generic subsystem redesign.

### Integration Points
- `crates/cli/src/commands/onboard.rs`: `run_model_setup()`, `OnboardingState`, `SetupState`, `load_from_setup_state()`, and resume or repair helpers.
- `crates/app/src/setup_handoff.rs` and `crates/app/src/setup_lifecycle.rs`: downstream consumers of setup-state shape and readiness semantics.
- `crates/cli/src/commands/runtime.rs` and `crates/app/src/runtime_provider_switch.rs`: existing provider mutation boundary that should remain compatible with the onboarding-selected provider path.

</code_context>

<specifics>
## Specific Ideas

- The current onboarding model step hardcodes one provider prompt set and largely assumes remote API-key entry except for Ollama. Phase 177 should replace that with provider-path selection that matches the actual supported access modes.
- Resume should preserve provider-path choices the same way deployment mode and setup path already persist through setup state.
- The phase should not try to solve live provider verification or model catalog truth yet; it should make those later phases possible by giving them a truthful provider-path foundation.

</specifics>

<deferred>
## Deferred Ideas

- Provider connection verification and failure classification belong to Phase 178.
- Live model discovery, manual model entry fallback, and provider-plus-model persistence belong to Phase 179.
- Setup handoff expansion, repair differentiation beyond provider-path state, and first-launch provider-model continuity belong to Phase 180.
- Broad Control UI parity and generic provider-management abstractions remain outside this milestone slice.

</deferred>
