# Phase 135: Compatibility Exception Audit - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the final audit model for any remaining compatibility shims or exceptions so the native-product claim cannot hide them.

</domain>

<decisions>
## Implementation Decisions

- Treat every remaining compatibility surface as classified audit data instead of a fuzzy follow-on caveat.
- Separate `retired`, `native-shimmed`, and `legacy-exception` states so the final claim can stay specific.
- Require ownership, reason, and exit criteria for anything that remains a true exception.

</decisions>
