# Phase 93: Orchestration Request Routing, Override Validation, and Lifecycle-State Transition Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move orchestration request routing, override validation, and lifecycle-state transition business logic out of `orchestrate.rs` so those flows compose through `openrustclaw-app` instead of route-local command orchestration.

</domain>

<decisions>
## Implementation Decisions

- Keep workspace file I/O, background worker spawning, and active-run persistence in `crates/cli/src/commands/orchestrate.rs`.
- Move route selection, autonomy override validation, and operator intervention state transitions into a dedicated `openrustclaw-app` service.
- Preserve the current CLI and control-plane contracts while reducing orchestration-specific business rules in the legacy command module.

</decisions>
