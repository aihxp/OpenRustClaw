# Phase 119: CLI Family Dependency Removal - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how the remaining CLI families stop depending on command-to-command orchestration so native delivery modules can own their flows directly.

</domain>

<decisions>
## Implementation Decisions

- Treat command-to-command orchestration as an architectural smell that the second CLI slice must forbid explicitly.
- Route shared behavior to app ports or explicit delivery helpers rather than back through legacy command files.
- Keep compatibility shims bounded to forwarding and translation so they cannot become another hidden orchestration layer.

</decisions>
