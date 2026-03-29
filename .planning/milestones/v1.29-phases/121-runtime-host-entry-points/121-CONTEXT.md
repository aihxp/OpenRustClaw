# Phase 121: Runtime Host Entry Points - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the dedicated runtime-host and background-worker entrypoints over app ports so worker startup can leave the legacy command-layer bootstrap path.

</domain>

<decisions>
## Implementation Decisions

- Treat runtime-host bootstrap and background-worker bootstrap as delivery ownership, not as leftover command helpers.
- Map startup ownership to app ports explicitly so later implementation can move incrementally without rediscovering which contracts belong in `openrustclaw-app`.
- Keep the milestone at the planning-contract level so future runtime-host binaries or modules can land without bundling repository-adapter work into the same slice.

</decisions>
