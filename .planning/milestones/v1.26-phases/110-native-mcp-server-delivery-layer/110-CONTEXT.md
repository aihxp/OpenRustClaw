# Phase 110: Native MCP Server Delivery Layer - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the native MCP server delivery path so tool-catalog and invocation ownership no longer depend on the `start.rs` bootstrap hotspot.

</domain>

<decisions>
## Implementation Decisions

- Treat MCP as its own native transport lane in `openrustclaw-mcp`, even when legacy startup still couples it to the control bootstrap.
- Use `McpServerPort` as the app-side contract for capability exposure, tool invocation, and compiled-skill MCP behavior.
- Allow compatibility forwarding only after the native MCP delivery path is explicit and named.

</decisions>
