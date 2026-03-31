# Research Summary: v1.42 Onboarding Primary LLM Selection

**Summarized:** 2026-03-30

## Stack Additions

- No new third-party dependencies are required for this milestone.
- Reuse the existing onboarding CLI flow, runtime health probes, and `RuntimeProviderSwitchService`.
- Add a small app-layer provider or model onboarding service plus setup-state and handoff fields for selected provider, access mode, and primary model.

## Feature Table Stakes

- Provider selection must represent access mode truthfully.
- Onboarding must verify the chosen provider lane before the model step is considered ready.
- Onboarding must support live model discovery when available and explicit primary-model persistence in the same flow.
- Setup state, repair, handoff, and first-start guidance must reflect the chosen provider and primary model.

## Watch Out For

- Do not treat static docs or `models::scan()` as live catalog truth.
- Do not collapse local-provider and remote-provider verification into one misleading path.
- Do not mark onboarding complete while the primary model is still implicit.
- Do not create a parallel onboarding-only config mutation lane that drifts from runtime mutation semantics.

## Suggested Build Order

1. Define provider descriptors, access modes, and setup-state extensions.
2. Extract or add shared provider verification and model catalog helpers.
3. Rework onboarding model setup around explicit provider, access mode, verification, and model selection.
4. Persist provider and model together through the shared runtime mutation lane.
5. Extend setup handoff, docs, and regression coverage.
