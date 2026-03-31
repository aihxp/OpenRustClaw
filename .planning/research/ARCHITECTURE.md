# Architecture Research: v1.42 Onboarding Primary LLM Selection

**Scope:** onboarding-time provider access-mode handling, live provider verification, model discovery or scan, and persisted primary task model selection
**Researched:** 2026-03-30

## Existing Architecture

- `crates/cli/src/commands/onboard.rs` owns the first-run wizard, setup-state persistence, bootstrap outcomes, resume, repair, and post-onboarding handoff.
- `crates/cli/src/commands/runtime.rs` already owns provider health scanning, model-availability checks, and persisted runtime config mutation.
- `crates/app/src/runtime_provider_switch.rs` already centralizes provider and model persistence rules behind `RuntimeProviderSwitchService`.
- `crates/app/src/setup_handoff.rs` exposes durable onboarding state to later operator surfaces.

## Needed Changes

### Modify onboarding state and setup state

- Extend `OnboardingState` and `SetupState` to carry:
  - selected provider
  - selected access mode
  - selected primary model
  - model selection source
- Keep bootstrap outcomes as the evidence layer for verification and discovery results.

### Add a shared onboarding provider-model service

- Add a new app-layer service, likely in `crates/app/src`, that owns:
  - provider descriptors for onboarding
  - access-mode rules
  - provider verification results
  - model catalog discovery normalization
- `onboard.rs` should keep the prompts and flow control, but call this service rather than embedding more provider-specific branching.

### Reuse and refactor runtime health logic

- Factor provider probe and catalog extraction helpers out of `runtime.rs` into reusable code or a shared service boundary.
- Keep runtime health as the canonical classification source for auth, billing, rate-limit, and model-unavailable outcomes.

### Extend config mutation in one transaction

- Persist provider and primary model together through `RuntimeProviderSwitchService` or a close sibling service.
- Avoid an onboarding-only config write path that can drift from the runtime mutation rules already used elsewhere.

### Extend setup handoff surfaces

- Update `SetupHandoffState` and `SetupHandoffReport` to surface the chosen provider and primary model.
- Preserve readiness semantics: onboarding is not “ready” until the selected lane is both verified and model-complete.

## Data and State Flow

1. Operator chooses provider.
2. Onboarding resolves provider descriptor and supported access modes.
3. Operator chooses or confirms access mode.
4. Onboarding gathers the required credential or local-runtime input for that mode.
5. Shared verification logic probes the provider and returns a classified result.
6. Shared catalog logic discovers models when supported.
7. Operator selects the primary model or enters one manually.
8. Runtime config is updated with provider plus model together.
9. Setup state and bootstrap outcomes persist the selected provider, access mode, model, and verification summary.
10. Setup handoff reports the resulting lane without requiring a later runtime switch step.

## Build Order

1. Define provider or access-mode descriptors and new setup-state fields.
2. Extract or add shared provider verification and model catalog services.
3. Replace `run_model_setup()` in onboarding with the new provider or model flow.
4. Extend runtime provider-switch persistence to support one-shot provider-plus-model writes.
5. Extend setup handoff and any operator-facing summaries that describe the selected model lane.
6. Add targeted CLI, app-service, and resume or repair regression tests.
7. Update onboarding and provider docs to match shipped behavior.

## Verification Hooks

- CLI tests around `run_model_setup()` and setup-state round trips.
- App-layer tests for provider descriptor logic, verification classification, and model catalog normalization.
- Runtime mutation tests proving provider and model persist together.
- Setup handoff tests proving selected provider and model appear in the final report.
- Regression tests for resume or repair after failed verification, failed catalog lookup, and successful manual-entry fallback.
