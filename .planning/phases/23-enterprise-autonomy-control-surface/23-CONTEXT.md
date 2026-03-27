# Phase 23: Enterprise Autonomy Control Surface - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 22 finished the backend contract for operator-gated full autonomy: there is now a durable manifest, event log, dedicated governance scope, protected enable or disable or kill-switch routes, and typed audit or admin summaries. What is still missing is the shipped operator loop over that contract.

This phase should close the milestone by making that stronger autonomy lane usable from Control UI:

- let operators inspect the current full-autonomy state and recent events without leaving the shipped dashboard
- let the same enterprise-admin surface enable, disable, and kill-switch the lane through the typed runtime routes
- keep the UI grounded in the backend contracts added in Phase 22 rather than inventing frontend-only state

</domain>

<decisions>
## Implementation Decisions

### Reuse the existing enterprise admin surface
The user already has one shipped place for enterprise access, governance, policy, and audit actions. Add full-autonomy inspection and controls there instead of fragmenting the operator workflow.

### Drive the UI from the typed autonomy report
Do not rebuild full-autonomy state from raw files or ad hoc assumptions. The UI should fetch the new `/control/enterprise/autonomy` summary and render that contract directly.

### Reuse saved operator and approver headers
The higher-risk autonomy writes should plug into the same requester and approver header persistence already used by governance and policy actions.

### Stop at a truthful enterprise operator baseline
This phase should not invent wizard flows, approval inboxes, or mobile approvals. A clear summary plus explicit buttons for enable, disable, and kill switch is enough to close the milestone honestly.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/control_ui.html` already has enterprise summary cards, admin forms, header persistence, and async action helpers.
- `crates/cli/src/commands/control_ui.rs` already locks the shipped dashboard against key panel and control regressions through HTML-presence tests.
- `crates/cli/src/commands/start.rs` now exposes `GET /control/enterprise/autonomy` plus protected enable or disable or kill-switch routes.
- `crates/cli/src/commands/inspect.rs` and `crates/cli/src/commands/enterprise_policy.rs` already carry the full-autonomy report into enterprise admin and audit review summaries.

</code_context>

<specifics>
## Specific Ideas

- add one dedicated full-autonomy summary card or panel in `/control/ui`
- show current status, budgets, baseline policy, recent events, and recent execution evidence from the typed autonomy report
- add form inputs for override budgets and note or reason plus buttons for enable, disable, and kill switch
- route those actions through the existing saved operator and approver headers
- close with static dashboard coverage, docs alignment, and a final phase verification artifact

</specifics>

<deferred>
## Deferred Ideas

- approval inboxes or richer workflow state machines
- separate autonomy mobile surfaces
- tenant-specific autonomy UIs or external approval integrations

</deferred>
