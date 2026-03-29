# Phase 132: Guardrails and Retirement Verification - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Define the guardrails and verification rules proving new product entrypoints no longer depend on retired delivery files.

</domain>

<decisions>
## Implementation Decisions

- Make retirement verification explicit so future implementation can prove regressions are blocked, not just hoped away.
- Pair guardrails with ownership-exit rules because retired files stay retired only if new work is forced toward the native path.
- Advance the roadmap to `7/8` only if retirement states, shim rules, and regression checks are explicit end to end.

</decisions>
