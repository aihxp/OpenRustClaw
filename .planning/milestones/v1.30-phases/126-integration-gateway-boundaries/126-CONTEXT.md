# Phase 126: Integration Gateway Boundaries - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the infrastructure boundaries for channel providers and external services that still depend on command-local integration wiring.

</domain>

<decisions>
## Implementation Decisions

- Treat provider and external service wiring as explicit gateway ownership instead of preserving it as command-local helper behavior.
- Keep channel providers, registry lookups, scheduler hooks, and service-manager-facing integrations in one planning slice because they share the same side-effect boundary problem.
- Make the boundaries concrete enough that later implementation can wire adapters directly without rediscovering external side-effect ownership.

</decisions>
