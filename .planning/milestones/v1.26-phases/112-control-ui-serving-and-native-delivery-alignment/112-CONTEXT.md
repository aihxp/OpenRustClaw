# Phase 112: Control UI Serving and Native Delivery Alignment - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Align Control UI serving and wiring to the native gateway-delivery path so the UI is not left behind on the legacy bootstrap contract.

</domain>

<decisions>
## Implementation Decisions

- Treat UI serving and route wiring as delivery ownership, not just as static-file cleanup.
- Preserve the shipped UI contract while moving serving ownership to `openrustclaw-gateway`.
- Require the UI story to line up with the same native gateway route contracts defined for control delivery.

</decisions>
