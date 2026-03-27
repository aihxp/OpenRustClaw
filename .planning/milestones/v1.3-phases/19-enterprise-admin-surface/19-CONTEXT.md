# Phase 19: Enterprise Admin Surface - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 19 should make the enterprise access, enterprise policy, audit export, and supervised-autonomy controls actually operable from the shipped control surface. The backend groundwork already exists across Phases 16 through 18, but operators still have to know individual API routes and header rules by hand.

This phase should close that gap without expanding scope into new enterprise products:

- keep the work inside the existing Rust control plane and `/control/ui`
- stay grounded in typed reports and existing protected routes
- make the current enterprise and supervised-autonomy baseline usable, not broader

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing enterprise routes
Do not invent a second admin API. Phase 19 should compose the shipped access, policy, and supervision surfaces rather than fork them.

### Make operator identity usable from the browser
Protected enterprise writes already require scoped operator headers. The control surface now needs a safe, explicit place to persist and send those headers for real operator workflows.

### Keep the admin summary typed
If the UI needs a consolidated admin view, expose it through one typed runtime report instead of rebuilding sensitive state entirely on the client.

### Treat supervised autonomy as part of the same operator loop
Phase 18 already made active-run lifecycle controls usable in Control UI. Phase 19 should connect that work into the same enterprise admin/operator surface rather than creating a new autonomy console.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/enterprise_access.rs` already owns the file-backed operator registry and protected-route contract.
- `crates/cli/src/commands/enterprise_policy.rs` already owns the typed policy summary, updates, and durable audit export bundle.
- `crates/cli/src/commands/orchestrate.rs` already exposes supervised lifecycle state plus decision history for active runs.
- `crates/cli/src/commands/control_ui.html` already renders enterprise foundations, enterprise access, and orchestration supervision, but it does not yet provide an operator workflow for enterprise bootstrap, policy updates, or audit export.

</code_context>

<specifics>
## Specific Ideas

- add a typed enterprise admin summary that combines access, policy, and supervised-run attention state
- persist enterprise operator headers in Control UI and reuse them on protected writes
- add Control UI actions for bootstrap, operator upsert, policy update, and audit export
- refresh the shipped enterprise and supervision panels after admin mutations
- close the phase with docs and verification focused on the operator loop

</specifics>

<deferred>
## Deferred Ideas

- SSO, SCIM, or external IAM integration
- multi-tenant enterprise administration
- a separate enterprise web product outside the shipped Control UI

</deferred>
