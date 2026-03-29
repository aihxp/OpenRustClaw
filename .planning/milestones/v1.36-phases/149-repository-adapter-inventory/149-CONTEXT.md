# Phase 149: Repository Adapter Inventory - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first repository-adapter inventory and successor ownership slice so persistence-heavy command-local helpers no longer remain a broad future replacement claim.

</domain>

<decisions>
## Implementation Decisions

- Treat the first repository-adapter inventory as explicit implementation-contract data instead of leaving persistence ownership as a broad future idea.
- Tie the slice directly to the real persistence-heavy hotspots in `start.rs`, `runtime.rs`, `skills.rs`, `inspect.rs`, and related command modules so the repository-lift handoff is grounded in the current codebase.
- Keep the phase at the planning-contract level so later source work can build one concrete repository-lift slice first.

</decisions>
