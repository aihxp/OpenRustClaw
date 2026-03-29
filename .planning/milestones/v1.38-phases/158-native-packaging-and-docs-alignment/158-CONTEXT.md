# Phase 158: Native Packaging and Docs Alignment - Context

**Gathered:** 2026-03-28
**Status:** Completed

<domain>
## Phase Boundary

Align the canonical packaging and documentation story to the implemented native delivery surfaces and the verified source-tree evidence gathered in Phase 157.

</domain>

<decisions>
## Implementation Decisions

### canonical-doc alignment
Treat the planning and milestone docs as the canonical place to capture the final exit claim and package layout truthfully.

### no cosmetic overreach
Do not rewrite broad user-facing docs just to make the claim look cleaner. Only align the canonical claim surfaces that govern milestone truthfulness.

</decisions>

<code_context>
## Existing Code Insights

- The workspace crate graph already exposes the native crates through `cargo metadata`.
- The live planning docs still describe `v1.38` as active before closeout and need shipment normalization.

</code_context>

<specifics>
## Specific Ideas

- Align the roadmap, project, state, and implementation-roadmap docs to the shipped `v1.38` result.
- Preserve the distinction between completed roadmap denominators and bounded source exceptions.

</specifics>

<deferred>
## Deferred Ideas

- Any broader public-site or marketing-language rewrite

</deferred>
