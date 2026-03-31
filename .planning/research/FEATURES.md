# Feature Research: v1.42 Onboarding Primary LLM Selection

**Scope:** onboarding-time provider access-mode handling, live provider verification, model discovery or scan, and persisted primary task model selection
**Researched:** 2026-03-30

## Category 1: Provider Access and Account Mode

### Table Stakes

- Prompt for a provider first, using only providers the shipped onboarding flow already supports.
- Show the provider's supported access modes truthfully: API key, subscription-managed account, local runtime, or a bounded combination.
- Avoid asking for credentials or tokens that do not apply to the selected access mode.
- Record enough state to resume the model step without repeating prior valid choices.

### Differentiators

- Explain why a given access mode affects model discovery, verification, or manual entry behavior.
- Preserve a short explanation in setup handoff so later operator surfaces reflect what was actually chosen.

### Deferred Ideas

- Expanding first-run selection to every provider crate in the workspace.
- A generic provider capability registry shared across all runtime surfaces.

### Anti-Features

- A fake unified account flow that hides whether the provider is API-key-backed, subscription-backed, or local.
- Asking for an API key unconditionally even when the selected flow does not use one.

## Category 2: Verification and Readiness

### Table Stakes

- Verify provider connectivity during onboarding before the model step is marked ready.
- Surface failure classes clearly enough to distinguish auth or access problems, unavailable local runtime, rate limiting, billing, and generic outages.
- Record the verification result in bootstrap outcomes so repair and resume flows can reuse it.

### Differentiators

- Reuse existing runtime-health classification language so onboarding and runtime status agree.
- Allow the flow to continue only when the chosen provider lane is truthful about readiness.

### Deferred Ideas

- Background provider polling or long-lived verification daemons during onboarding.
- Multi-provider benchmarking or latency-based ranking.

### Anti-Features

- Treating config writes alone as proof that the provider is usable.
- Hiding the difference between “reachable but wrong model” and “provider unavailable.”

## Category 3: Model Discovery and Primary Selection

### Table Stakes

- Discover or scan available models for the selected provider where the provider exposes a usable catalog.
- Let the operator choose a primary task model explicitly instead of accepting only provider defaults.
- Provide a bounded manual-entry fallback when catalog discovery is unavailable, account-scoped, or empty.
- Persist the chosen model into runtime config in the same onboarding flow.

### Differentiators

- Label how the model choice was produced: live catalog, recommended fallback, or manual entry.
- Prefer provider-scoped recommendations over static global model lists.

### Deferred Ideas

- Rich model comparison UX with price, context, and capability scoring across providers.
- Operator-defined model profiles or role-routing presets during first-run onboarding.

### Anti-Features

- Treating `docs` examples or `models::scan()` output as authoritative catalog truth.
- Forcing live catalog discovery when a local or subscription path cannot provide one.

## Category 4: Handoff, Resume, and Operator Guidance

### Table Stakes

- Setup state and setup handoff should show the selected provider and primary model.
- Resume and repair should know whether the model step failed because access mode, verification, or model selection was incomplete.
- The rest of onboarding should continue without requiring a separate `runtime switch-model` step.

### Differentiators

- Reuse bootstrap-outcome language so the handoff stays concise and operator-readable.
- Make first assistant launch use the provider and model established during onboarding.

### Deferred Ideas

- Broad Control UI redesign for model setup.
- Post-onboarding optimization workflows for fallback chains or multi-model routing.

### Anti-Features

- Marking onboarding complete while forcing the operator to finish core model selection elsewhere.
- Persisting an ambiguous “provider configured” state without the selected primary model.
