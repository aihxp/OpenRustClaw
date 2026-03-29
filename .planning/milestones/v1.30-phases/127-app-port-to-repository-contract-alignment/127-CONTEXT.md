# Phase 127: App Port to Repository Contract Alignment - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how app services depend on adapter traits or repositories instead of command-local file helpers.

</domain>

<decisions>
## Implementation Decisions

- Treat app-port alignment as the contract that turns the repository inventory and gateway boundaries into an end-to-end architecture story.
- Map the major app-port families directly to repository and gateway contracts so command-local helpers stop being implied as the persistence layer.
- Keep the milestone focused on contract alignment instead of implementation so the later adapter slices can land incrementally.

</decisions>
