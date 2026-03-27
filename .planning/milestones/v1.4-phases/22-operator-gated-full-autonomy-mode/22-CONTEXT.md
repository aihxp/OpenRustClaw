# Phase 22: Operator-Gated Full Autonomy Mode - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 22 should add the requested "god mode" as an explicit enterprise-controlled override lane, not as a silent weakening of the default runtime. The current runtime already has autonomy level, approval policy, and orchestration budgets, and Phase 18 already added supervised escalation and rollback. What is missing is the operator-visible contract around deliberately entering a stronger full-autonomy state.

This phase should close that gap without forking the orchestrator:

- keep full autonomy as a separate, reversible enterprise override layered on top of the existing Rust runtime
- preserve explicit budgets, shutdown semantics, and durable evidence for enablement and operator intervention
- make the stronger autonomy lane governable rather than opaque so it can feed the enterprise admin surface in the next phase

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing runtime autonomy machinery
Do not create a second executor. Full autonomy should reuse the existing autonomy policy and orchestration runtime, with a separate enterprise override contract that can be applied and revoked deliberately.

### Persist the override contract beside other enterprise control data
The enablement state, configured budgets, and intervention history should live under the enterprise control root so they can participate in the same audit and review story as access, governance, and policy.

### Treat kill switch and shutdown as first-class operator actions
The stronger autonomy lane needs explicit disable and kill-switch records, not just implicit runtime mutations or disappearing state.

### Stop this phase at the backend contract
This phase should ship the typed summary, routes, durable state, and enforcement semantics. The richer operator control loop belongs in Phase 23.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/control.rs` already owns the runtime autonomy policy and can persist changes to the control runtime artifact.
- `crates/cli/src/commands/orchestrate.rs` already enforces autonomy budgets and exposes supervised lifecycle state for active runs.
- `crates/cli/src/commands/enterprise_access.rs` already protects sensitive runtime and orchestration routes under enterprise scopes and governance rules.
- `crates/cli/src/commands/enterprise_policy.rs` already packages governance and supervision evidence into enterprise audit exports.
- `crates/cli/src/commands/inspect.rs` and `crates/cli/src/commands/start.rs` already expose typed enterprise admin summaries and runtime routes that can absorb a new full-autonomy report.

</code_context>

<specifics>
## Specific Ideas

- add a file-backed enterprise full-autonomy manifest with explicit enablement state, runtime budget overrides, and operator attribution
- append durable enable, disable, and kill-switch events to a structured ledger
- expose a typed full-autonomy summary and include it in enterprise admin inspection
- add runtime routes to inspect, enable, disable, and emergency-stop the full-autonomy lane
- keep route protection under the current enterprise runtime-control boundary and close with targeted tests plus truthful docs

</specifics>

<deferred>
## Deferred Ideas

- broader autonomous business-operation expansion across more domains
- autonomous execution with no operator kill-switch or evidence trail
- enterprise-wide policy engines, tenant-specific autonomy policy, or external approval systems

</deferred>
