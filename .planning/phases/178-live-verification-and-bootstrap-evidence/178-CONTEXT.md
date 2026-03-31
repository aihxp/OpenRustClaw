# Phase 178: Live Verification and Bootstrap Evidence - Context

**Gathered:** 2026-03-30
**Status:** Ready for planning
**Mode:** Smart discuss

<domain>
## Phase Boundary

Phase 178 should make the onboarding model lane prove that the chosen provider path is actually usable before the step is treated as ready. The implementation should stay inside the existing onboarding, setup-state, setup-lifecycle, and handoff surfaces. It should not introduce a second readiness framework separate from the runtime health scan that already exists.

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing runtime health probe
The current onboarding flow already calls `runtime::runtime_health_status(...)` after provider setup. Phase 178 should keep that live check as the source of truth and shape its results into onboarding-specific verification evidence instead of adding duplicate provider-specific probes.

### Make failure categories explicit and durable
Provider verification should classify failures into operator-meaningful buckets such as authentication or access, missing local runtime, model unavailable, billing or quota, rate limiting, or generic provider reachability. Those categories should be persisted into durable bootstrap evidence so resume, repair, inspect, and handoff surfaces can reason about the exact failed verification lane.

### Keep repair routing grounded in the model step
Even with richer verification evidence, the repair flow should stay bounded to the existing onboarding model step. The new evidence should help explain whether the operator needs to re-enter provider setup or verification, not create a brand-new top-level onboarding step.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` already persists `bootstrap_outcomes`, records verification results, and resumes or repairs onboarding from durable setup state.
- `validate_provider_bootstrap(...)` currently collapses provider verification to `ready` or `blocked` with a freeform detail string, even though the runtime health report already exposes richer `issue_kind` data.
- `crates/cli/src/commands/runtime.rs` already classifies provider health outcomes into categories such as `auth`, `billing`, `rate_limited`, `model_unavailable`, `provider_unavailable`, `probe_failed`, and `not_configured`.
- `crates/app/src/setup_lifecycle.rs`, `crates/app/src/setup_handoff.rs`, and `crates/cli/src/commands/inspect.rs` already propagate bootstrap outcomes and derive repair plans from them.

</code_context>

<specifics>
## Specific Ideas

- Extend durable bootstrap outcome records with enough metadata to distinguish verification failures from generic setup notes.
- Thread provider verification issue kind through lifecycle and handoff reporting without breaking existing setup-state compatibility.
- Improve onboarding and repair messaging so a failed provider verification can say what failed and what the operator should do next.

</specifics>

<deferred>
## Deferred Ideas

- Full provider-model selection belongs to Phase 179.
- Broader setup handoff presentation polish belongs to Phase 180 unless required for truthful verification routing in this phase.

</deferred>
