# Phase 195: Guided First-Task Orchestration and Fallbacks - Context

**Gathered:** 2026-04-08
**Status:** Complete

<domain>
## Phase Boundary

Turn onboarding and repair handoff into a real first-task launch path. This phase is about prefilled first-task guidance, orchestration-ready launch requests, and actionable fallback choices when the preferred lane is not ready. It is the final journey-polish layer on top of the routing console, not a new routing-policy system.

</domain>

<decisions>
## Implementation Decisions

- **D-01:** First-task guidance should reuse setup handoff and routing-console truth rather than inventing a separate onboarding-only planner.
- **D-02:** The first-task path must stay explicit and inspectable: it should show the prompt, route preview, orchestration request, and fallback choices before execution.
- **D-03:** Direct-provider lanes remain valid first-task paths even when delegated routing is unavailable; delegated backends only become blocking when the selected lane explicitly depends on them.
- **D-04:** The shipped orchestration CLI should gain a first-task preview entrypoint so operators can reuse the same prefill outside Control UI.

</decisions>

---

*Phase: 195-guided-first-task-orchestration-and-fallbacks*
*Context gathered: 2026-04-08*
