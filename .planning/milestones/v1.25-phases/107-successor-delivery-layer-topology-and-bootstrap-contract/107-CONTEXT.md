# Phase 107: Successor Delivery Layer Topology and Bootstrap Contract - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the successor delivery topology and bootstrap rules that will replace the legacy command tree without breaking shipped entrypoints.

</domain>

<decisions>
## Implementation Decisions

- Reuse existing workspace crates where they already match the target delivery role, especially `openrustclaw-gateway` and `openrustclaw-mcp`.
- Keep `openrustclaw-app` as the application-port home instead of spreading business logic back across delivery crates.
- Make worker startup and infrastructure adaptation first-class native layers instead of hiding them under the CLI rewrite.

</decisions>
