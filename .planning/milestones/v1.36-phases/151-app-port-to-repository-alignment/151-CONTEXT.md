# Phase 151: App-Port to Repository Alignment - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define app-port to repository-adapter alignment needed to move persistence-heavy and side-effect-heavy ownership off command-local helpers.

</domain>

<decisions>
## Implementation Decisions

- Group the app-port to repository-adapter alignment into one implementation slice because persistence and integration handoff need to target the same native infrastructure model.
- Define compatibility shims explicitly so command-local helper entrypoints can remain bounded while repository-lift ownership becomes the real owner.
- Keep alignment focused on successor ownership, not legacy-tree retirement, so the milestone stays incremental.

</decisions>
