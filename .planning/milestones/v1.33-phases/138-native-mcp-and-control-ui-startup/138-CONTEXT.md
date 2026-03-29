# Phase 138: Native MCP and Control UI Startup - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the first source-level native MCP bootstrap slice plus the Control UI serving alignment needed for the successor startup path.

</domain>

<decisions>
## Implementation Decisions

- Treat MCP startup and Control UI serving as part of one shared successor startup slice instead of splitting them into disconnected future work.
- Tie the MCP slice directly to `openrustclaw-mcp` and the UI-serving handoff directly to the gateway successor path.
- Keep the phase at the planning-contract level so later source work can implement MCP and Control UI startup without rediscovering bootstrap alignment.

</decisions>
