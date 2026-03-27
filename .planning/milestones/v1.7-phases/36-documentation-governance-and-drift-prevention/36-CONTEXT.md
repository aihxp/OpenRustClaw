# Phase 36: Documentation Governance and Drift Prevention - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 36 makes the documentation rewrite durable. The repo now needs an explicit maintenance workflow, an updated audit artifact that can catch future drift, and the cleanup of any leftover redundant surface that no longer serves the product.

</domain>

<decisions>
## Implementation Decisions

### Reuse the documentation contract as the governance anchor
- Extend `docs/documentation-contract.md` with the ongoing maintenance workflow.
- Keep the audit and maintenance story anchored in the same canonical doc set instead of creating another parallel policy page.

### Turn the docs audit into an ongoing drift check
- Keep the shipped-feature-family mapping.
- Add explicit maintenance expectations so future milestones know what must update together.

### Remove docs that no longer carry unique truth
- Delete stale redirect or placeholder docs when they are not referenced and no longer help navigation.

### the agent's Discretion
The governance rules can stay lightweight as long as they are explicit enough to stop the repo from falling back into parallel, unsynchronized docs.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/documentation-contract.md` already names canonical docs and merge/delete rules.
- `docs/docs-audit.md` already maps feature families to docs and tests.
- `docs/src/planning/docs-audit.md` already serves as the mdBook pointer to the canonical audit.

### Established Patterns
- Root planning docs are canonical.
- mdBook planning pages are stubs or mirrors.

### Integration Points
- `docs/documentation-contract.md`
- `docs/docs-audit.md`
- `docs/src/planning/docs-audit.md`
- `docs/src/contributing/development.md`
- `docs/reengineering-backlog.md`

</code_context>

<specifics>
## Specific Ideas

- add a maintenance checklist for future milestones
- make the docs audit useful beyond the original Phase 8 gate
- remove the stale reengineering backlog redirect

</specifics>

<deferred>
## Deferred Ideas

- automated link checking beyond the current mdBook build step
- a dedicated contributor docs guide beyond a small development-note addition

</deferred>
