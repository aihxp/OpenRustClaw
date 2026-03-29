# Phase 131: Main Bootstrap Retirement Path - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define how `main.rs` shrinks to binary bootstrap only or is replaced entirely.

</domain>

<decisions>
## Implementation Decisions

- Treat the top-level binary path as part of legacy retirement, not as a separate leftover detail after command-family cleanup.
- Align bootstrap retirement to native delivery entrypoints explicitly so later implementation can narrow `main.rs` without rediscovering routing ownership.
- Keep the milestone focused on the retirement contract rather than the binary rewrite itself.

</decisions>
