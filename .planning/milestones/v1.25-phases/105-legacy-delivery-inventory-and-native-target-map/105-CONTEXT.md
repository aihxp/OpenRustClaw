# Phase 105: Legacy Delivery Inventory and Native Target Map - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Turn the remaining legacy delivery layer into a concrete, complete inventory with an explicit native target map.

</domain>

<decisions>
## Implementation Decisions

- Inventory the remaining delivery layer by shipped family, not by vague “rewrite the CLI” language.
- Ground the target map in the actual current hotspots such as `main.rs`, `start.rs`, the command tree, and the worker bootstraps.
- Treat the output as a migration ledger for future deletion work, not as an aspirational architecture note.

</decisions>
