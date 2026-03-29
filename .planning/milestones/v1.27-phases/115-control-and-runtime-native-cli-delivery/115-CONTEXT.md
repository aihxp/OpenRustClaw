# Phase 115: Control and Runtime Native CLI Delivery - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native CLI delivery path for control and runtime entrypoints so those operator flows stop defaulting to the legacy command hubs.

</domain>

<decisions>
## Implementation Decisions

- Treat CLI control and CLI runtime as distinct from the already-defined control HTTP path.
- Route those flows through `ControlPlanePort` and `RuntimeOperationsPort` instead of leaving orchestration in legacy command modules.
- Preserve compatibility through bounded forwarding only after the native CLI delivery path is explicit.

</decisions>
