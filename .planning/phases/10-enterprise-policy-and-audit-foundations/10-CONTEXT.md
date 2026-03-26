# Phase 10: Enterprise Policy and Audit Foundations - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 10 does not attempt full enterprise readiness. It establishes a narrow, truthful baseline around approval-sensitive assistant actions and the durable audit evidence that operators need before broader enterprise policy, RBAC, or compliance work can be layered on top.

</domain>

<decisions>
## Implementation Decisions

### Reuse shipped policy boundaries
- **D-01:** The enterprise baseline should reuse existing approval and policy contracts instead of inventing a parallel enterprise subsystem.
- **D-02:** Mobile command approval, browser backend policy, and runtime autonomy approval policy are already the most concrete approval-sensitive surfaces in the repo, so they should anchor the summary.
- **D-03:** Durable audit evidence should come from shipped records that already persist across runtime restarts, especially mobile command history, browser backend audit entries, and tool execution history.

### Operator-facing baseline
- **D-04:** The baseline should be exposed through a single typed Control UI/runtime surface so operators do not need to manually correlate three or four lower-level endpoints.
- **D-05:** The summary should state both the policy contract and the recent audit evidence in plain operator language, not just dump raw JSON.
- **D-06:** Documentation should describe this phase as a foundation for later enterprise work, not as a claim that full enterprise governance is finished.

### the agent's Discretion
- A compact summary endpoint in `start.rs` is preferable to spreading enterprise status across more existing panels.
- It is acceptable to define “enterprise-sensitive actions” narrowly for v1.1 around operator approvals, external browser backend policy, and auditable side-effect execution.

</decisions>

<canonical_refs>
## Canonical References

### Existing approval and audit surfaces
- `crates/cli/src/commands/mobile.rs` — mobile command approval, rejection, metrics, and event timelines
- `crates/cli/src/commands/browser.rs` — external backend policy and backend audit log
- `crates/cli/src/commands/control.rs` — runtime autonomy approval policy defaults and validation
- `crates/cli/src/commands/orchestrate.rs` — request-level approval policy overrides and rendered descriptions
- `crates/cli/src/commands/inspect.rs` — durable tool execution history
- `crates/cli/src/commands/start.rs` — shipped control endpoints that can expose the new summary
- `crates/cli/src/commands/control_ui.html` — shipped operator surface for new enterprise summary visibility

### Current milestone framing
- `.planning/PROJECT.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `.planning/STATE.md`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Runtime control already exposes `/control/browser/backend-policy`, `/control/browser/backend-audit`, `/control/tool-executions`, and mobile approval endpoints.
- Mobile command records already distinguish `pending_approval`, `approved`, `executed`, and `rejected` states and can emit a timeline for each command.
- Control runtime config already validates explicit autonomy approval policies (`none`, `side_effects`, `always`).
- Tool execution history already records operator actions like `mobile.command.approve`, `mobile.command.reject`, and `mobile.capability.execute`.

### Gaps to Close
- Operators do not yet have one place that explains the current approval boundary for enterprise-sensitive actions.
- Durable audit evidence exists, but operators have to know which raw endpoints to inspect manually.
- The docs do not yet describe the narrow enterprise baseline clearly enough to support a later enterprise milestone.

</code_context>

<specifics>
## Specific Ideas

- Add a typed `/control/enterprise/foundations` summary that reports runtime approval policy, browser backend policy, mobile approval metrics, and recent auditable approval-sensitive events.
- Surface that summary in Control UI as an “Enterprise Foundations” panel with explicit approval boundary text and recent evidence.
- Update operator-facing docs so the baseline is described as explicit approval contracts plus durable audit review, not generic “enterprise-ready” wording.

</specifics>

<deferred>
## Deferred Ideas

- RBAC, SSO, multi-tenancy, and compliance exports remain out of scope.
- Broader supervised autonomy and richer approval workflows beyond the current core surfaces remain future milestone work.
- Full parity across every sensitive surface is not required if the baseline is truthful and extensible.

</deferred>

---
*Phase: 10-enterprise-policy-and-audit-foundations*
*Context gathered: 2026-03-26*
