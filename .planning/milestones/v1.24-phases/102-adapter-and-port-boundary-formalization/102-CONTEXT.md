# Phase 102: Adapter and Port Boundary Formalization - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Make the remaining persistence and external side-effect ownership in the final hotspots explicit as named adapters instead of leaving it buried in helper clusters.

</domain>

<decisions>
## Implementation Decisions

- Formalize the surviving file-system and compiled-skill MCP boundaries with named adapter structs instead of anonymous helper sprawl.
- Keep the new adapter seams in the CLI layer focused on workspace I/O and tool registration while `openrustclaw-app` owns the business rules.
- Treat phase success as readability and ownership clarity, not as raw line migration.

</decisions>
