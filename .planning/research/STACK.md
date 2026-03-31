# Stack Research: v1.42 Onboarding Primary LLM Selection

**Scope:** onboarding-time provider access-mode handling, live provider verification, model discovery or scan, and persisted primary task model selection
**Researched:** 2026-03-30
**Confidence:** HIGH for repo integration points, MEDIUM for provider catalog behavior

## Current Reusable Stack

### Existing onboarding lane to reuse

- `crates/cli/src/commands/onboard.rs`
  - Already owns the interactive first-run flow with `dialoguer`, setup-state persistence, bootstrap outcomes, and readiness handoff.
  - Already records provider bootstrap results via `validate_provider_bootstrap()` and `record_bootstrap_outcome()`.
- `SetupState` and `SetupBootstrapOutcome`
  - Already give a durable place to persist onboarding progress and readiness evidence under `.claw/control/setup-state.json`.
  - Good fit for storing the chosen provider, chosen model, selected access mode, and discovery outcome summary for handoff.

### Existing runtime lane to reuse

- `crates/cli/src/commands/runtime.rs`
  - Already owns config mutation, provider construction, runtime health scans, and persisted runtime-health output.
  - `switch_provider(...)` and `switch_model(...)` already persist provider/model selection into `config/default.toml`.
  - `runtime_health_status(...)` already does live provider verification and checks whether the configured model appears in the provider catalog.
- `crates/app/src/runtime_provider_switch.rs`
  - Already centralizes provider/model mutation semantics behind `RuntimeProviderSwitchService`.
  - Best place to extend provider-selection semantics without growing more onboarding-only config write logic in CLI.

### Existing config and dependency surface to reuse

- `openrustclaw-core::config::AppConfig`
  - Already persists provider choice, per-provider model fields, fallback chain, control-plane provider, and provider-specific env var references.
- Existing crates already cover the milestone:
  - `reqwest`, `serde`, `serde_json`, `tokio`
  - `dialoguer`, `console`, `indicatif`
  - `openrustclaw-app`, `openrustclaw-core`, `openrustclaw-providers`
- Existing workspace SDK crates are available if needed later:
  - `ollama-sdk`
  - `openrouter-api`
  - native provider crates for request execution

## Recommended Additions Or Extensions

### 1. Add a small typed onboarding provider-selection layer

Add a repo-local type and service, preferably in `openrustclaw-app`, for:

- `ProviderAccessMode`
  - `ApiKeyOnly`
  - `SubscriptionOnly`
  - `ApiKeyOrSubscription`
- `OnboardingProviderDescriptor`
  - provider id
  - supported access modes
  - whether live catalog listing is supported
  - whether manual model entry is allowed
  - default recommended model fallback

Reason:

- `onboard.rs` currently hardcodes one path per provider and assumes API-key entry for all remote providers except Ollama.
- v1.42 needs truthful provider-specific access-mode handling without embedding more branchy provider policy directly in the CLI step.

This should stay local to onboarding/runtime. Do not add a generic plugin registry or a repo-wide provider metadata framework for this milestone.

### 2. Extract provider verification and model discovery into a shared service

Promote the live probe and catalog-fetch logic out of `crates/cli/src/commands/runtime.rs` into an app-layer service used by both onboarding and runtime health.

Recommended shape:

- `ProviderVerificationService`
  - verify configured access for one provider
  - return auth/access/quota/rate-limit/unavailable classification
- `ProviderModelCatalogService`
  - fetch available model ids for one provider when supported
  - normalize to a small `DiscoveredModel` struct
  - return `catalog_is_account_scoped` and `fetched_at`

Reason:

- onboarding now needs more than “is the configured model healthy”
- runtime health already has the only real live verification code
- keeping catalog logic CLI-local will duplicate probe code and drift fast

Dependency impact:

- no new third-party dependency required
- use existing `reqwest` + `serde_json` first

### 3. Extend persisted setup state and handoff with explicit primary-model fields

Add milestone-scoped fields to setup state or handoff-derived state for:

- `selected_provider`
- `selected_access_mode`
- `selected_model`
- `model_selection_source` (`catalog`, `manual_entry`, `default_fallback`)
- optional `provider_verified_at`

Reason:

- current onboarding marks model setup complete after provider switch, but it does not preserve an explicit onboarding-time primary model choice
- later steps should not require a separate post-setup `runtime switch-model`

This is a local data-shape extension, not a new persistence backend. Keep it in setup state and `config/default.toml`.

### 4. Extend runtime provider mutation to support one-shot provider + model persistence

Prefer extending `RuntimeProviderSwitchRequest` rather than adding a parallel onboarding-only config writer.

Likely additions:

- access mode metadata if needed for handoff
- explicit primary-model persistence in the same transaction as provider selection
- optional “preserve existing fallback lane” flag if onboarding should avoid resetting unrelated runtime config

Reason:

- provider and model selection are one logical commit in v1.42
- config mutation rules already live behind `RuntimeProviderSwitchService`

### 5. Replace `models::scan()` for onboarding use

`crates/cli/src/commands/models.rs` is currently a static recommendation printer for Groq/OpenRouter/SiliconFlow/Ollama and is not aligned with the onboarding provider set.

Recommendation:

- do not extend `models::scan()` into the onboarding discovery engine
- either add a new onboarding-facing discovery function or refactor `models` commands to call the shared catalog service

Reason:

- current scan output is role-assignment advice, not account-aware model discovery
- it encodes providers that are outside the v1.42 onboarding scope

## Integration Points

### CLI and app boundaries

- `crates/cli/src/commands/onboard.rs`
  - keep prompts, selections, and setup-state updates here
  - replace the current provider branch logic in `run_model_setup()` with calls into a typed onboarding/model-selection service
- `crates/app/src/runtime_provider_switch.rs`
  - extend the request shape and keep config mutation rules here
- new app-layer module
  - recommended: `crates/app/src/provider_onboarding.rs` or similar
  - own provider descriptors, access-mode rules, verification results, and model catalog normalization

### Runtime reuse

- `crates/cli/src/commands/runtime.rs`
  - reuse `runtime_health_status(...)` classification logic
  - factor `probe_ollama_provider(...)`, `probe_remote_provider_health(...)`, and `extract_provider_model_ids(...)` into shared code instead of copying them into onboarding
- `create_provider_from_config(...)`
  - remains the validation path for “can we instantiate the selected provider from current config and env”

### Persistence reuse

- `config/default.toml`
  - remains the source of truth for selected provider and selected model
- `.claw/control/setup-state.json`
  - remains the onboarding handoff source of truth for what was chosen and what was verified

### Crates/APIs to reuse directly

- Reuse now:
  - `reqwest::Client`
  - `serde_json::Value` or a tiny typed response model
  - `RuntimeProviderSwitchService`
  - `runtime::runtime_health_status(...)`
  - `runtime::switch_provider(...)` / `runtime::switch_model(...)`
- Use only if the new shared service benefits from typed responses:
  - `ollama-sdk` for local model listing
  - `openrouter-api` for typed OpenRouter model listing

For v1.42, prefer keeping Anthropic and OpenAI catalog reads as thin `reqwest` calls unless a typed client is already needed elsewhere. Adding direct CLI dependencies on more SDK crates is not justified just for onboarding.

### Unstable provider catalog assumptions

- Provider model catalogs are not stable product metadata.
- Catalog results are account-scoped and access-mode-scoped.
- A model appearing in public docs does not mean it is available to the operator’s account.
- `docs/src/guides/providers.md` and `crates/cli/src/commands/models.rs` contain static model examples and should not be treated as truth for onboarding.
- OpenRouter catalog breadth is especially unstable and should be treated as a live snapshot, not a curated default list.

Implication:

- onboarding should prefer live discovery when available
- if discovery fails or returns zero usable models, allow manual model entry plus a recommended default fallback

## Avoid

- Do not add a new database table or migration for onboarding model choice.
- Do not add a new external provider-catalog service or background sync job.
- Do not introduce a generic provider plugin system.
- Do not make `models::scan()` the canonical onboarding source.
- Do not expand first-run support to every provider crate in the workspace; keep v1.42 bounded to the current onboarding providers unless requirements change.
- Do not persist large catalog snapshots in setup state; store only the selected model and concise verification metadata.
- Do not assume subscription access can be verified through the same API-key probe path; model the access mode explicitly and branch truthfully.
