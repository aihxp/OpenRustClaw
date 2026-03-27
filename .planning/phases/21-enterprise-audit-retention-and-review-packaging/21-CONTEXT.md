# Phase 21: Enterprise Audit Retention and Review Packaging - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 21 should turn the existing enterprise audit export into a real retention and review contract. Phase 17 already created a durable JSON export, and Phase 20 added richer governance state, but operators still have to inspect raw bundle files and generic tool history if they want to understand what governance and supervised-autonomy evidence exists.

This phase should close that gap without overreaching into compliance products:

- keep enterprise audit policy inside the existing enterprise policy/admin surface
- retain richer governance and supervision evidence in the export bundle
- give operators one typed review surface for recent exports and high-signal audit evidence

</domain>

<decisions>
## Implementation Decisions

### Extend the current enterprise policy manifest
Retention belongs beside audit export policy, not in a new retention file. Keep the export root and review limits under one enterprise policy surface.

### Reuse typed Rust summaries instead of scraping files in the UI
If operators need a review package, expose it through one typed runtime summary and let Control UI render that contract.

### Prefer bounded retention and recent-history packaging
This phase should make review easier and evidence more durable, not attempt a full compliance archive or external GRC integration.

### Keep governance and supervised-autonomy evidence in the same audit story
Phase 20 governance and Phase 18 supervision are now part of the enterprise trust boundary. The review package should show both rather than splitting them into unrelated exports.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/enterprise_policy.rs` already owns the enterprise policy manifest and audit export bundle path.
- `crates/cli/src/commands/inspect.rs` already exposes enterprise foundations, tool execution history, enterprise access, and enterprise admin summaries.
- `crates/cli/src/commands/start.rs` already exposes `/control/enterprise/policy` and `/control/enterprise/audit/export`.
- `crates/cli/src/commands/control_ui.html` already exposes enterprise policy mutation and audit export from the shipped `Enterprise Admin` surface, but it has no review panel for prior exports or richer enterprise audit context.

</code_context>

<specifics>
## Specific Ideas

- add retention settings and recent-export review limits to enterprise audit policy
- enrich the audit export bundle with governance and supervision context, not only foundations plus generic tool history
- add a typed enterprise audit review summary for recent exports and recent governance or supervision events
- surface that review summary in `/control/ui` so operators can inspect the audit contract without reading raw bundle files
- close with docs and verification focused on the stronger audit handoff story

</specifics>

<deferred>
## Deferred Ideas

- legal hold, immutable storage, or compliance certifications
- third-party SIEM, GRC, or archive connectors
- long-term multi-tenant enterprise retention policy management

</deferred>
