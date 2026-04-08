# Phase 187: OAuth-Safe Delegated Backend Contracts - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Turn discovered local vendor-agent CLIs into explicit delegated-backend contracts with policy-aware allow or deny semantics, truthful model-catalog labels, and audit-ready decisions. This phase is about the safe contract layer between discovery and future runtime routing, not yet full task execution through those backends.

</domain>

<decisions>
## Implementation Decisions

### Compliance boundary
- **D-01:** OpenRustClaw must not scrape vendor OAuth tokens, browser sessions, or local credential stores.
- **D-02:** Account-backed local vendor CLIs are delegated execution backends, not direct API providers.
- **D-03:** When a vendor does not expose trustworthy non-interactive model discovery, the contract must say `vendor_managed` or `unknown` instead of guessing models.

### Policy boundary
- **D-04:** Existing `external_backends` policy should remain the primary allowlist and environment-boundary source unless a concrete conflict forces a new config lane.
- **D-05:** Delegated local agent backend execution must remain bounded by local-wrapper policy and allowlists before any future routing calls it.
- **D-06:** Detection-only surfaces such as Cursor must stay visible but ineligible for delegated execution.

### the agent's Discretion
- The exact shape of the delegated backend contract and policy decision types
- Whether audit-entry generation lives alongside policy evaluation or remains caller-owned
- Which inspect or policy surfaces should expose the new contracts first

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/phases/186-local-agent-discovery-and-compliance-inventory/186-01-SUMMARY.md`
- `.planning/phases/186-local-agent-discovery-and-compliance-inventory/186-02-SUMMARY.md`
- `crates/app/src/agent_backend_catalog.rs`
- `crates/app/src/browser_backend_control.rs`
- `crates/cli/src/commands/browser.rs`
- `crates/cli/src/commands/enterprise_policy.rs`
- `crates/core/src/config.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets
- `agent_backend_catalog.rs` already has truthful detection, readiness, auth metadata, and provider mapping helpers.
- `browser_backend_control.rs` already models an allowlist-based external-backend policy with audit-ready deny decisions.
- `config.external_backends` already carries allowlists, local-wrapper gating, cloud-execution gating, audit-log paths, and env allowlists.

### Current disconnects
- Discovery entries are still just metadata; nothing yet says which local agent backends are execution-capable under current policy.
- Browser external-backend policy and local agent discovery live in parallel with no shared delegated-backend contract layer.
- Model availability for local vendor agents is still only implied by notes rather than normalized as `supported`, `vendor_managed`, `unknown`, or `unsupported`.

</code_context>

<deferred>
## Deferred Ideas

- Full delegated task execution through local vendor-agent backends
- Persisted onboarding selection of delegated local agents
- End-to-end route attribution and session receipts for delegated runs

</deferred>

---

*Phase: 187-oauth-safe-delegated-backend-contracts*
*Context gathered: 2026-04-08*
