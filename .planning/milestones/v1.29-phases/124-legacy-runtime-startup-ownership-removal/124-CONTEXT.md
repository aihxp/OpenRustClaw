# Phase 124: Legacy Runtime Startup Ownership Removal - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how legacy command ownership over worker lifecycle startup is reduced or removed once native runtime-host entrypoints exist.

</domain>

<decisions>
## Implementation Decisions

- Make the legacy startup rules explicit so future implementation can tell the difference between bounded forwarding and still-live ownership.
- Cover `start.rs`, `runtime.rs`, `services.rs`, `schedule.rs`, `mobile.rs`, and `voice_runtime.rs` together because they currently share the startup burden.
- Advance the roadmap to `5/8` only if the removal rules are explicit enough to keep new lifecycle ownership out of the legacy command tree.

</decisions>
