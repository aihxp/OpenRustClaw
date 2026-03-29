# Phase 130: Compatibility Shim and Delete Boundaries - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how still-live legacy surfaces become thin compatibility shims or hard deletes.

</domain>

<decisions>
## Implementation Decisions

- Treat compatibility survival as an explicit bounded state, not as permission for the legacy tree to remain a second permanent routing layer.
- Define shim-versus-delete rules centrally so future implementation can decide retirement state from criteria rather than from habit.
- Keep the milestone focused on retirement boundaries rather than implementation so later deletion work can land incrementally.

</decisions>
