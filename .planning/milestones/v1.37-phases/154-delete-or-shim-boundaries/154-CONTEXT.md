# Phase 154: Delete-or-Shim Boundaries - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how still-live legacy surfaces move toward explicit native ownership through bounded delete-or-shim rules.

</domain>

<decisions>
## Implementation Decisions

- Treat compatibility survival as an explicit bounded state, not as permission for the legacy tree to remain a second permanent routing layer.
- Define delete-or-shim rules centrally so future implementation can decide shutdown state from criteria rather than habit.
- Keep the milestone focused on retirement boundaries rather than implementation so later deletion work can land incrementally.

</decisions>
