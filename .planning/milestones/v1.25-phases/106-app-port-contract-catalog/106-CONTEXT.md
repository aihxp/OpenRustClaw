# Phase 106: App Port Contract Catalog - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the app-port families needed to let native delivery layers replace the legacy command tree.

</domain>

<decisions>
## Implementation Decisions

- Group ports by real delivery families and repository responsibilities instead of creating one giant catch-all port.
- Cover CLI, control, MCP, worker-host, and repository-facing concerns in the first catalog.
- Make the catalog specific enough to drive future implementation milestones without rediscovering boundaries.

</decisions>
