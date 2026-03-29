# Phase 100: Transition Helper Cleanup and Adapter Convergence - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Clean up transition-era helper duplication and normalize the affected adapters after the setup and secondary command-surface extractions land.

</domain>

<decisions>
## Implementation Decisions

- Remove or reduce helper duplication that became unnecessary once the app-side services existed.
- Keep the remaining CLI helpers narrow and adapter-specific instead of recreating shared business rules locally.
- Use the same verification loop as the extracted phases so cleanup is only accepted if the migrated contracts still pass.

</decisions>
