# Phase 16: Enterprise Identity and Access Boundaries - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 16 turns the narrow v1.1 enterprise baseline into a real operator access boundary. The goal is not full enterprise IAM, SSO, or multi-tenancy. The goal is to stop treating every sensitive control-plane action as one shared operator and instead introduce an organization-oriented operator registry, explicit role or scope grants, and enforceable identity checks on a focused set of sensitive control actions.

</domain>

<decisions>
## Implementation Decisions

### Reuse the shipped control-plane boundary
- **D-01:** The current control API bearer or trusted-proxy auth remains the outer transport boundary. Enterprise identity should layer on top of it for sensitive actions rather than replace the existing control auth middleware.
- **D-02:** The enterprise access registry should live under the existing `.claw/control/` file-backed control surface so it is inspectable, testable, and consistent with the rest of the runtime.
- **D-03:** The first slice should use a compact organization plus operator registry with explicit role-to-scope grants instead of jumping straight to SSO, SCIM, or external identity providers.

### Scope the enforcement honestly
- **D-04:** Phase 16 should protect a narrow set of clearly sensitive actions first: control config writes, mobile command approval-sensitive actions, selected runtime mutations, and auth-plugin binding or authorization flows.
- **D-05:** Identity enforcement should be explicit and auditable through dedicated enterprise access routes and a typed summary, not buried in raw middleware behavior.
- **D-06:** Bootstrap needs to stay practical for an operator-managed deployment, so enterprise identity can be initialized through a dedicated bootstrap flow before stricter scoped operator actions are used.

### Operator-facing truth
- **D-07:** Control UI and docs should describe this phase as file-backed enterprise access foundations with scoped operator identities, not as full enterprise IAM.
- **D-08:** The summary should show organization identity, operator counts, role or scope coverage, and which sensitive routes require scoped operator identity.

### the agent's Discretion
- Prefer hashed operator tokens over plain-text secrets stored on disk.
- Keep role vocabulary small and obvious in this phase: owner, admin, operator, auditor.
- Use runtime summaries and route-scoped middleware rather than inventing a separate auth service.

</decisions>

<canonical_refs>
## Canonical References

### Existing control and enterprise surfaces
- `crates/cli/src/commands/start.rs` — control auth middleware, runtime control routes, sensitive control handlers
- `crates/cli/src/commands/inspect.rs` — enterprise foundations summary and other typed operator reports
- `crates/cli/src/commands/control.rs` — file-backed `.claw/control/` registry conventions
- `crates/cli/src/commands/control_ui.html` — shipped operator dashboard surface
- `crates/cli/src/commands/control_ui.rs` — Control UI rendering tests

### Prior milestone context
- `.planning/milestones/v1.1-phases/10-enterprise-policy-and-audit-foundations/10-CONTEXT.md`
- `.planning/milestones/v1.1-phases/10-enterprise-policy-and-audit-foundations/10-01-PLAN.md`

### Current milestone framing
- `.planning/PROJECT.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `.planning/STATE.md`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `control_auth_middleware` already establishes the transport-level control boundary through bearer or trusted-proxy auth.
- The `.claw/control/` registry already provides a file-backed pattern for operator-visible runtime contracts.
- The enterprise foundations summary already aggregates approval policy and audit evidence, so identity and access can extend that operator story instead of replacing it.
- Sensitive operator actions already cluster in `start.rs` around control config writes, mobile command decisions, runtime mutation routes, and auth-plugin setup flows.

### Gaps to Close
- Sensitive control routes still assume one shared operator identity once the transport auth token is accepted.
- There is no first-class organization or operator registry describing who is allowed to do which sensitive actions.
- Control UI can show the enterprise foundations summary, but it cannot yet explain identity, role, or scope boundaries for enterprise operators.

</code_context>

<specifics>
## Specific Ideas

- Add a file-backed enterprise access manifest under `.claw/control/enterprise/` with organization metadata, operator records, hashed tokens, roles, and explicit scopes.
- Expose `/control/enterprise/access` plus bootstrap and operator-management routes so the operator boundary is inspectable and usable.
- Add middleware that requires enterprise operator headers for a focused set of sensitive routes and enforces scope membership there.
- Surface the enterprise access boundary in Control UI with operator counts, role coverage, and protected-route scope mapping.

</specifics>

<deferred>
## Deferred Ideas

- SSO, SCIM, external IdP sync, and multi-tenant organization management remain out of scope.
- Full RBAC across every runtime and channel route remains future work after this first scoped boundary exists.
- Compliance exports and retention governance belong in later enterprise phases.

</deferred>

---
*Phase: 16-enterprise-identity-and-access-boundaries*
*Context gathered: 2026-03-27*
