# Phase 98: Secondary Lifecycle Command Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the targeted residual lifecycle seams in `channels.rs`, `schedule.rs`, `services.rs`, and `control.rs` behind `openrustclaw-app` so those command modules keep shrinking toward adapter-only ownership.

</domain>

<decisions>
## Implementation Decisions

- Keep workspace metadata reads, persistence, route transport, and bounded side effects in the CLI command modules.
- Move channel routing, schedule trigger shaping, channel health composition, and control-registry validation into dedicated `openrustclaw-app` services.
- Preserve the current CLI and operator-facing contracts while reducing lifecycle and control business rules in the legacy command modules.

</decisions>
