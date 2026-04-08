# Phase 188: Onboarding and Model Selection Cohesion - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Converge onboarding, model selection, inspect, and setup handoff around one truthful provider-or-agent lane. This phase turns discovery plus delegated-backend contracts into a coherent first-run and resume experience, while keeping actual delegated task routing for Phase 189.

</domain>

<decisions>
## Implementation Decisions

### Truthfulness boundary
- **D-01:** The onboarding menu must distinguish direct API providers, local runtimes, and delegated local agents without flattening them into one fake provider type.
- **D-02:** Delegated local agent lanes must carry truthful compatibility notes when they are detected but not yet fully routable.
- **D-03:** Existing direct-provider onboarding paths should stay intact unless they can be replaced by a shared lane descriptor without regressing resilience.

### Persistence boundary
- **D-04:** Selected lane identity, access mode, and model choice must persist through setup state, repair, resume, and inspect without losing whether the lane was direct, local-runtime, or delegated.
- **D-05:** Phase 188 may add new typed metadata to setup handoff or onboarding state, but should not break existing durable setup manifests.

### the agent's Discretion
- The exact typed shape of a shared onboarding lane descriptor
- Whether onboarding persistence stores a backend id, provider id, or both
- Which surface should become the canonical renderer for the shared provider-or-agent lane list

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/phases/186-local-agent-discovery-and-compliance-inventory/186-02-SUMMARY.md`
- `.planning/phases/187-oauth-safe-delegated-backend-contracts/187-01-SUMMARY.md`
- `.planning/phases/187-oauth-safe-delegated-backend-contracts/187-02-SUMMARY.md`
- `crates/cli/src/commands/onboard.rs`
- `crates/cli/src/commands/models.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/app/src/agent_backend_catalog.rs`
- `crates/app/src/agent_backend_control.rs`
- `crates/app/src/setup_handoff.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets
- Onboarding now sees local-agent discovery hints and Gemini is fully wired through the runtime path.
- Setup handoff already exposes both raw agent-backend discovery entries and delegated backend contracts.
- Delegated backend contracts can now separate direct providers from delegated local agents and label model catalogs truthfully.

### Current disconnects
- Onboarding still uses static provider descriptors instead of one shared provider-or-agent lane catalog.
- Setup state persists provider id and access mode, but not a first-class delegated backend lane identity.
- `openrustclaw models`, onboarding, inspect, and enterprise policy all reference the same reality, but through different shapes.

</code_context>

<deferred>
## Deferred Ideas

- Actual delegated task execution and receipts
- Control-registry model profile integration for delegated agent lanes
- Final docs and UX audit polish

</deferred>

---

*Phase: 188-onboarding-and-model-selection-cohesion*
*Context gathered: 2026-04-08*
