# Phase 111: Gateway Bootstrap Split and Start.rs Retirement Slice - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first truthful split between `start.rs` and the future native gateway or MCP startup path so legacy-bootstrap retirement can become incremental instead of rhetorical.

</domain>

<decisions>
## Implementation Decisions

- Treat startup ownership as a distinct retirement concern from route or tool logic.
- Split control, websocket, webhook, and MCP startup responsibilities into native targets before talking about deleting `start.rs`.
- Allow `start.rs` to survive temporarily only as a bounded compatibility shell instead of a continuing assembly hotspot.

</decisions>
