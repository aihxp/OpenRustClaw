# Phase 157: Native Delivery Exit Scorecard - Context

**Gathered:** 2026-03-28
**Status:** Completed

<domain>
## Phase Boundary

Verify the implemented native-delivery scorecard directly against the shipped source tree instead of relying on roadmap memory. The phase must distinguish real native crates and app ownership from surviving legacy delivery hotspots.

</domain>

<decisions>
## Implementation Decisions

### evidence-first scorecard
Use direct codebase evidence instead of restating milestone intent. Ground the scorecard in `cargo metadata`, targeted `wc -l`, and `rg` results for the current entrypoint files.

### bounded claim
Do not interpret the presence of native crates as proof that every legacy surface is deleted. The scorecard must leave room for explicit bounded exceptions.

</decisions>

<code_context>
## Existing Code Insights

- The workspace includes `openrustclaw-app`, `openrustclaw-gateway`, `openrustclaw-mcp`, and `openrustclaw-cli`.
- `crates/app/src/*.rs` totals `20655` lines.
- `crates/cli/src/main.rs` remains `6129` lines and still wires the command tree.

</code_context>

<specifics>
## Specific Ideas

- Verify crate topology and source footprint.
- Use that evidence to define the truthful scorecard for the final claim.

</specifics>

<deferred>
## Deferred Ideas

- Physical deletion of any surviving legacy delivery files

</deferred>
