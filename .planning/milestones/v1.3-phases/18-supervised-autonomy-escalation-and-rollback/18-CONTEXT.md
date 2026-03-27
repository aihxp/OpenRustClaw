# Phase 18: Supervised Autonomy Escalation and Rollback - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 18 should deepen the existing orchestration supervision lane without turning it into an opaque autonomous controller. The current runtime already has active-run snapshots, recent events, pause/resume/kill controls, reflection candidates, and escalation recommendations. The missing contract is explicit supervised-autonomy lifecycle state plus durable evidence about intervention decisions.

This phase should stay centered on longer-running orchestrated runs:

- make escalation and rollback explicit states instead of implied notes
- preserve operator intervention evidence durably
- expose those states through the same shipped supervision surfaces operators already use

</domain>

<decisions>
## Implementation Decisions

### Reuse the active orchestration state model
Do not invent a second workflow engine. Extend `ActiveOrchestrationRun`, active-run events, and receipt/supervision reports instead.

### Make rollback a supervision action, not a hidden side effect
Rollback in this phase should mean an explicit operator-supervised lifecycle transition with durable rationale and optional rollback reference. It can stop or mark a run rather than silently mutating unrelated runtime state.

### Preserve evidence in structured records
Operator interventions should be written as durable structured records, not only embedded in freeform notes.

### Keep UI additive, not redesign-heavy
Phase 18 can extend the existing orchestration supervision UI with lifecycle/decision visibility and a small number of new controls. A broader admin/operator consolidation belongs to Phase 19.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/orchestrate.rs` already persists active-run snapshots and recent events under `.claw/control/orchestration-active/`.
- Active runs already support pause, resume, and kill via explicit routes and monitor logic.
- Receipt supervision and active supervision reports already exist and are rendered in `control_ui.html`.
- `SupervisionSummary` already includes `escalation_recommended`, so the phase can promote that into an explicit lifecycle/action contract.

</code_context>

<specifics>
## Specific Ideas

- add a lifecycle state field for active supervised runs
- add structured intervention decision records for escalate, resume, rollback, and kill-like operator actions
- expose decision history in active supervision payloads
- add explicit escalation and rollback control routes
- reflect lifecycle/decision state in Control UI and orchestration tests

</specifics>

<deferred>
## Deferred Ideas

- full multi-step rollback automation across other subsystems
- enterprise approval workflows or ticketing integration
- a broader enterprise admin console that spans access, policy, and autonomy in one place

</deferred>
