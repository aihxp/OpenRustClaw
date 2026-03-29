# Phase 128: Adapter Verification and Ownership Exit Criteria - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the verification and ownership rules that make repository and integration-adapter replacement measurable.

</domain>

<decisions>
## Implementation Decisions

- Make direct repository and adapter verification explicit so later implementation can prove progress without relying on command-local persistence tests.
- Define ownership-exit rules for command modules together because verification only matters if the legacy files are no longer allowed to keep the same persistence role.
- Advance the roadmap to `6/8` only if adapter ownership and verification are explicit end to end.

</decisions>
