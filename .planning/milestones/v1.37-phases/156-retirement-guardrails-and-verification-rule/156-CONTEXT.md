# Phase 156: Retirement Guardrails and Verification Rule - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the direct guardrails and verification rules for the first legacy command-tree retirement slice.

</domain>

<decisions>
## Implementation Decisions

- Make retirement verification explicit so future implementation can prove regressions are blocked, not just hoped away.
- Pair guardrails with ownership-exit rules because retired files stay retired only if new work is forced toward the native path.
- Advance the roadmap to `5/6` only if retirement states, delete-or-shim rules, and regression checks are explicit end to end.

</decisions>
