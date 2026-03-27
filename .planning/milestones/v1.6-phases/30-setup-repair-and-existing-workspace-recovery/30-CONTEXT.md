# Phase 30: Setup Repair and Existing Workspace Recovery - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 28 added durable setup state and Phase 29 made bootstrap outcomes truthful. Phase 30 has to turn that state into an explicit repair and re-entry flow so operators can recover partial or drifted workspaces without editing files by hand or guessing which setup step to rerun.

</domain>

<decisions>
## Implementation Decisions

### Reuse setup state and doctor diagnostics for recovery planning
Repair should be derived from the same durable setup contract and first-start health checks that onboarding already uses, not from a parallel recovery-only heuristic.

### Keep recovery step-oriented
Operators do not need a second wizard. They need explicit choices like resume, repair, and reset-with-backup, plus a visible repair plan that maps directly back to onboarding steps.

### Preserve trust before convenience
Reset and repair flows should say what they will touch and should keep backup behavior in place. Recovery should never hide state mutations.

</decisions>

<code_context>
## Existing Code Insights

- `OnboardingWizard::run()` already exposes resume, modify, health-check, and reset-with-backup choices.
- setup state now includes `selected_steps`, `completed_steps`, `blockers`, and `bootstrap_outcomes`.
- `doctor::collect_report(...)` and `doctor::first_start_readiness(...)` already identify the real blockers to first start.
- `run_selected_steps(...)` already knows how to execute setup steps while updating durable setup state.

</code_context>

<specifics>
## Specific Ideas

- add an explicit `Repair setup blockers or drift` path to onboarding when existing workspace state is detected
- derive a repair plan from resumable setup state, failed bootstrap outcomes, and doctor diagnostics
- reuse the durable setup-state contract to mark the targeted repair steps as active instead of bypassing setup state during repair

</specifics>

<deferred>
## Deferred Ideas

- final operator handoff and broader docs/control-surface alignment belong to Phase 31

</deferred>
