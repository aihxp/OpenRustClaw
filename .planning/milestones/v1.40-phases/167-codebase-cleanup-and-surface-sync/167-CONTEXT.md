# Phase 167: Codebase Cleanup and Surface Sync - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Apply the safe deletion, merge, and simplification work identified in the cleanup inventory while preserving shipped product behavior and keeping docs and source structure aligned.

</domain>

<decisions>
## Implementation Decisions

### cleanup under verification
Only land cleanup that can be bounded by compile, test, docs, or script verification. Do not treat churn as progress.

### simplify over rewrite
Prefer targeted lint fixes, stale-helper removal, and small structure cleanup over broad rewrites of stable operator surfaces.

### delete public-surface drift
Remove obsolete public files when a clearer replacement already exists, rather than leaving both in place.

</decisions>

<code_context>
## Existing Code Insights

- The workspace still carried enough dead code, stale helper shape, and lint debt to fail `cargo clippy --workspace -- -D warnings`.
- A stale public architecture page remained alongside the clearer replacement until it was deleted.
- Runtime-budget automation had a real local bind race that made release verification flaky.

</code_context>

<specifics>
## Specific Ideas

- Clean lint debt in adapter-style CLI and app surfaces without changing behavior.
- Delete the stale public architecture page after the replacement lands.
- Keep runtime-budget automation in sync with the cleaned source and release surfaces.

</specifics>

<deferred>
## Deferred Ideas

- Broad internal module renames for historical names like `greenfield_progress`, because those do not improve the public product surface enough to justify churn in this milestone

</deferred>
