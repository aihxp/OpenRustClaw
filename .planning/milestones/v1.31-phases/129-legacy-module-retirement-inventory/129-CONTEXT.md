# Phase 129: Legacy Module Retirement Inventory - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define which legacy command modules leave the main product path first and what retirement state each one enters.

</domain>

<decisions>
## Implementation Decisions

- Treat retirement state as explicit architecture data instead of leaving the command tree as vague future cleanup.
- Group the major command-tree hotspots and bootstrap surfaces into one inventory slice because they now share the same retirement problem.
- Keep the milestone at the planning-contract level so later implementation can retire modules incrementally without rediscovering the file-by-file shutdown story.

</decisions>
