# Phase 146: Runtime Startup Boundary Contracts - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the runtime startup-boundary contracts needed to move worker lifecycle startup off the legacy command layer truthfully.

</domain>

<decisions>
## Implementation Decisions

- Treat startup boundaries as explicit successor-entry contracts instead of preserving them as runtime helper clusters inside legacy command files.
- Keep service-manager, probes, maintenance, and scheduler in one planning slice because they are the core runtime-host boot dependencies.
- Make the contracts concrete enough that later implementation work can wire native runtime-host entrypoints without rediscovering startup ownership.

</decisions>
