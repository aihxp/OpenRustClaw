# Phase 20: Enterprise Governance and Approval Chains - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 20 should deepen the existing enterprise access boundary into a real governance contract. v1.3 established scoped operator identity, enterprise policy, and an admin surface, but the current write path is still effectively flat: if one operator proves the required scope, the write goes through.

This phase should strengthen that without inventing a second control plane:

- keep enterprise governance inside the existing Rust-owned access, inspect, and Control UI surfaces
- add explicit requester-role and approver-role rules for higher-risk scopes
- preserve separation of duties for dual-approval actions instead of treating one scoped operator as sufficient everywhere

</domain>

<decisions>
## Implementation Decisions

### Extend enterprise access instead of forking policy
Governance belongs beside the operator registry and protected-route contract. Do not create a second manifest just for approval chains.

### Make governance scope-driven and typed
The stricter boundary should attach to existing protected enterprise scopes so the control plane can inspect and render one coherent governance report.

### Keep dual approval explicit in the request contract
If a route requires dual approval, the second approver should be visible as an explicit operator identity rather than an invisible server-side shortcut.

### Preserve the shipped admin loop
Control UI already has a usable enterprise admin surface. Expand it with governance visibility and secondary approver headers rather than replacing it.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/enterprise_access.rs` owns the operator registry, scoped route classifier, and request authentication.
- `crates/cli/src/commands/inspect.rs` already exposes typed enterprise access and enterprise admin summaries that the UI consumes.
- `crates/cli/src/commands/start.rs` already wires the enterprise access, policy, audit export, and admin routes plus the enterprise middleware.
- `crates/cli/src/commands/control_ui.html` already persists enterprise operator headers and provides enterprise admin actions, but it does not yet support governance rules or a second approver identity.

</code_context>

<specifics>
## Specific Ideas

- add a governance policy to the enterprise access manifest with per-scope role and approval rules
- enforce requester-role limits and dual-approval headers for selected higher-risk scopes
- expose a typed governance summary through enterprise access and enterprise admin reports
- add Control UI inputs for secondary approver headers plus one governance rule editor and governance table
- close with docs and verification that explain the new dual-operator contract truthfully

</specifics>

<deferred>
## Deferred Ideas

- full enterprise IAM, SSO, SCIM, or external approval systems
- out-of-band approval workflows across email, chat, or mobile
- autonomy-mode governance and kill-switch packaging beyond the enterprise governance baseline

</deferred>
