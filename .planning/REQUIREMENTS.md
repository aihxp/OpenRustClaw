# Requirements: OpenRustClaw v1.42 Onboarding Primary LLM Selection

**Defined:** 2026-03-30
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Provider Access

- [ ] **ACCS-01**: Operator can choose the primary LLM provider from the onboarding-supported provider list during the model setup step.
- [ ] **ACCS-02**: Operator sees only the access modes that are actually supported for the selected provider, including API-key, subscription-managed, local-runtime, or bounded combinations where applicable.
- [ ] **ACCS-03**: Onboarding collects only the credential or runtime input required for the chosen provider and access mode instead of asking for irrelevant secrets.
- [ ] **ACCS-04**: Setup state persists the selected provider and access mode so the model setup step can resume without losing prior valid choices.

### Verification

- [ ] **VERF-01**: Onboarding verifies the chosen provider connection before the model setup step is marked ready.
- [ ] **VERF-02**: Operator sees whether verification failed because of authentication or access, missing local runtime, model unavailability, billing or quota, rate limiting, or generic provider reachability issues.
- [ ] **VERF-03**: Verification outcomes are recorded in durable bootstrap evidence so repair and resume can target the failed sub-step directly.

### Model Selection

- [ ] **MODL-01**: Onboarding can discover or scan available models for the chosen provider when that provider exposes a usable catalog for the selected access mode.
- [ ] **MODL-02**: Operator can choose the primary task model explicitly from the discovered model list when discovery succeeds.
- [ ] **MODL-03**: Operator can enter a primary task model manually when live model discovery is unavailable, account-scoped, empty, or unsupported for the selected provider or access mode.
- [ ] **MODL-04**: The selected provider and primary task model are persisted into the runtime configuration in the same onboarding flow without requiring a separate post-setup runtime switch step.
- [ ] **MODL-05**: Onboarding records whether the selected primary model came from live discovery, recommended fallback, or manual entry.

### Handoff and Resume

- [ ] **HNDF-01**: Setup handoff surfaces show the selected provider, access mode, primary task model, and current readiness outcome for the onboarding model lane.
- [ ] **HNDF-02**: Resume and repair flows can distinguish incomplete provider selection, failed verification, and incomplete model selection when re-entering the onboarding model step.
- [ ] **HNDF-03**: First assistant launch after onboarding uses the provider and primary task model established during onboarding when the workspace is otherwise ready.

## v2 Requirements

### Provider Guidance

- **GUID-01**: Operator can compare available models by capability, context, or cost inside onboarding before choosing the primary task model.
- **GUID-02**: Onboarding can recommend different primary models for distinct roles such as task lane, fallback lane, and control-plane lane.

### Surface Expansion

- **SRFC-01**: Control UI exposes the same onboarding-time provider access-mode and primary-model selection flow as the CLI wizard.
- **SRFC-02**: Onboarding expands the first-run provider catalog beyond the current supported provider set without widening the milestone boundary implicitly.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Generic provider plugin framework | Too broad for a bounded onboarding milestone; reuse the existing runtime and onboarding lanes instead |
| Multi-provider benchmarking or auto-ranking | Adds opinionated scoring and drift risk beyond the core first-run trust problem |
| Broad Control UI redesign for model setup | The scoped gap is in the CLI onboarding lane, setup handoff, and related docs or verification surfaces |
| Universal first-run support for every provider crate in the workspace | The milestone should stay bounded to the currently shipped onboarding providers |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ACCS-01 | Unmapped | Pending |
| ACCS-02 | Unmapped | Pending |
| ACCS-03 | Unmapped | Pending |
| ACCS-04 | Unmapped | Pending |
| VERF-01 | Unmapped | Pending |
| VERF-02 | Unmapped | Pending |
| VERF-03 | Unmapped | Pending |
| MODL-01 | Unmapped | Pending |
| MODL-02 | Unmapped | Pending |
| MODL-03 | Unmapped | Pending |
| MODL-04 | Unmapped | Pending |
| MODL-05 | Unmapped | Pending |
| HNDF-01 | Unmapped | Pending |
| HNDF-02 | Unmapped | Pending |
| HNDF-03 | Unmapped | Pending |

**Coverage:**
- v1 requirements: 15 total
- Mapped to phases: 0
- Unmapped: 15 ⚠️

---
*Requirements defined: 2026-03-30*
*Last updated: 2026-03-30 after initial definition for v1.42*
