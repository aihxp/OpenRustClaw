# Phase 152: Repository Lift Compatibility and Verification Rule - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the direct compatibility and verification rules for the first repository and integration adapter lift slice.

</domain>

<decisions>
## Implementation Decisions

- Treat verification and compatibility as part of the fourth source-level handoff slice instead of later cleanup.
- Require direct successor-entry verification rather than command-local helper fallback proof.
- Keep compatibility boundaries explicit so the first repository-lift implementation claim cannot hide fallback ownership.

</decisions>
