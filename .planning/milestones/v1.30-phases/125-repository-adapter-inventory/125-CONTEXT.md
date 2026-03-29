# Phase 125: Repository Adapter Inventory - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the repository and gateway adapter inventory for sqlite, workspace files, audit logs, runtime config, compiled-skill cache, and registries.

</domain>

<decisions>
## Implementation Decisions

- Treat repository and gateway ownership as the next native-delivery boundary rather than leaving persistence layout implicit in command modules.
- Map the major persistence-heavy families to explicit repository contracts so later implementation can move incrementally without rediscovering ownership.
- Keep the milestone at the planning-contract level so the next implementation slice can create adapters without bundling legacy-module retirement into the same work.

</decisions>
