# Phase 122: Runtime Startup Boundary Contracts - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native startup boundaries for service-manager, probe-runner, runtime-maintenance, and scheduler flows needed by the runtime-host delivery path.

</domain>

<decisions>
## Implementation Decisions

- Treat startup boundaries as explicit delivery or infrastructure contracts instead of preserving them as command-local helper clusters.
- Keep service manager, probes, maintenance, and scheduler in one planning slice because they are the core runtime-host boot dependencies.
- Make the contracts concrete enough that later implementation work can wire adapters directly without rediscovering startup ownership.

</decisions>
