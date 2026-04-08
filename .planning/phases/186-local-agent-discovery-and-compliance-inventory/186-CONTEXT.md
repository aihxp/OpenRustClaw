# Phase 186: Local Agent Discovery and Compliance Inventory - Context

**Gathered:** 2026-04-08
**Status:** In progress

<domain>
## Phase Boundary

Detect installed local agent tools and expose truthful capability, auth, and policy metadata through one shared catalog that later onboarding, inspect, and routing surfaces can reuse. This phase is about discovery truthfulness and catalog convergence, not full delegated execution, onboarding persistence, or end-to-end journey closeout.

</domain>

<decisions>
## Implementation Decisions

### Discovery truthfulness
- **D-01:** Detection must distinguish direct API providers from delegated local agent backends instead of flattening them into one provider type.
- **D-02:** Detected binaries that lack a documented programmable execution path must remain visible as detection-only or unsupported instead of being mislabeled as ready.
- **D-03:** Discovery should use safe local probes such as `--help`, `--version`, or documented auth-status commands only.

### Compliance boundary
- **D-04:** OpenRustClaw must not scrape cached OAuth tokens, browser sessions, or vendor credential stores.
- **D-05:** Subscription-managed access is only usable through supported vendor CLI or API surfaces.
- **D-06:** Cursor support stays conservative until a documented programmable backend surface is confirmed.

### Catalog convergence
- **D-07:** One shared app-layer catalog should feed CLI, onboarding, inspect, and control instead of multiplying static provider lists.
- **D-08:** Phase 186 can ship in slices, starting with the shared discovery service and one shipped operator-facing surface.

### the agent's Discretion
- The exact typed shape of the catalog entries and readiness states
- Which current operator surface should be the first catalog consumer
- Whether model discovery is marked unsupported or unknown per backend when no trustworthy probe exists

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/research/STACK.md`
- `.planning/research/ARCHITECTURE.md`
- `.planning/research/PITFALLS.md`
- `crates/cli/src/commands/onboard.rs`
- `crates/cli/src/commands/models.rs`
- `crates/core/src/config.rs`
- `crates/app/src/browser_backend_control.rs`
- `crates/app/src/control_registry.rs`
- `crates/app/src/tool_host_service.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets
- `crates/cli/src/commands/onboard.rs` already models `subscription_managed` access conceptually.
- `crates/core/src/config.rs` already exposes `external_backends` policy and audit-log configuration.
- `crates/app/src/tool_host_service.rs` already contains host IDs plus help and version parsing utilities that can seed discovery.
- `crates/cli/src/commands/models.rs` is a good first consumer because it already owns the user-facing provider catalog, even though it is still too static.

### Current disconnects
- Onboarding and `models` use different source-of-truth tables.
- Gemini support exists in config and providers but is missing from the `models` surface.
- Local vendor agent CLIs are installed on this machine but not visible anywhere inside the product catalog.

</code_context>

<deferred>
## Deferred Ideas

- Policy-persisted delegated backend allow or deny settings beyond current discovery classification
- Onboarding persistence for delegated local agents
- Runtime routing and execution receipts for delegated local agent runs
- Full user-journey and agent-journey repair across docs and control surfaces

</deferred>

---

*Phase: 186-local-agent-discovery-and-compliance-inventory*
*Context gathered: 2026-04-08*
