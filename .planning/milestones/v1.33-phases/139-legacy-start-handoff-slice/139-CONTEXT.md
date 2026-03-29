# Phase 139: Legacy Start Handoff Slice - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first bounded startup handoff that reduces `start.rs` ownership truthfully while preserving compatibility coverage.

</domain>

<decisions>
## Implementation Decisions

- Treat the first `start.rs` handoff as explicit bounded ownership transfer instead of a broad retirement promise.
- Keep compatibility coverage explicit wherever startup forwarding remains necessary.
- Tie the handoff directly to the first gateway, MCP, and Control UI successor slices so `start.rs` stops being the implied permanent bootstrap owner.

</decisions>
