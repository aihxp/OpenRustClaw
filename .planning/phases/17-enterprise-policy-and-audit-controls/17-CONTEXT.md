# Phase 17: Enterprise Policy and Audit Controls - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 17 turns the new enterprise access boundary into a real policy and audit surface. The scope is still intentionally narrow: unify the most important enterprise-sensitive policy knobs into one coherent runtime surface, and make the resulting policy plus audit evidence exportable. This is not full compliance packaging or every possible enterprise control.

</domain>

<decisions>
## Implementation Decisions

### Build on shipped enterprise foundations
- **D-01:** Reuse the Phase 16 enterprise access registry and the Phase 10 enterprise foundations summary instead of inventing a second enterprise subsystem.
- **D-02:** The new enterprise policy surface should aggregate the controls operators already care about: scoped enterprise access, autonomy approval policy, browser backend policy, mobile approval defaults, and audit export settings.
- **D-03:** Policy writes should land through one enterprise route even if the underlying durable state spans `.claw/control/enterprise/`, control runtime YAML, and runtime config TOML.

### Keep the audit story durable
- **D-04:** Audit export should serialize both current policy state and recent evidence into a durable file-backed bundle operators can keep or review later.
- **D-05:** Exported evidence should reuse existing durable sources: enterprise access registry, enterprise foundations recent events, browser backend audit, and operator tool execution history.
- **D-06:** Export routes should describe what was exported and where it was written rather than returning raw file paths with no context.

### the agent's Discretion
- A compact `enterprise_policy.rs` module is preferable to overloading the access registry module with broader policy logic.
- Phase 17 can stay summary- and API-first; a bigger operator surface can land in Phase 19.

</decisions>

<canonical_refs>
## Canonical References

- `.planning/PROJECT.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `.planning/STATE.md`
- `.planning/phases/16-enterprise-identity-and-access-boundaries/16-CONTEXT.md`
- `.planning/phases/16-enterprise-identity-and-access-boundaries/16-VERIFICATION.md`
- `crates/cli/src/commands/enterprise_access.rs`
- `crates/cli/src/commands/inspect.rs`
- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/control.rs`
- `crates/cli/src/commands/browser.rs`
- `crates/mobile/src/protocol.rs`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `enterprise_access.rs` already establishes file-backed operator identity and protected-route scope metadata.
- `enterprise_foundations_summary(...)` already aggregates runtime approval policy, browser backend policy, mobile metrics, and recent audit evidence.
- `control.rs` already owns the runtime autonomy policy YAML, including `approval_policy`.
- `browser.rs` already derives external backend policy from runtime config TOML.
- Tool execution history and browser backend audit entries already persist durable evidence that can be reused in exports.

### Gaps to Close
- Enterprise-sensitive policy is still scattered across several runtime files and summary routes.
- There is no single writable enterprise policy surface for access, autonomy approval, browser backend controls, and mobile approval defaults.
- Operators cannot export a durable enterprise audit bundle that freezes both current policy state and recent evidence together.

</code_context>

<specifics>
## Specific Ideas

- Add a dedicated enterprise policy manifest under `.claw/control/enterprise/` for mobile approval defaults and audit export settings.
- Add `/control/enterprise/policy` GET and PUT to aggregate and update the unified enterprise policy contract.
- Add `/control/enterprise/audit/export` to produce a durable JSON export bundle with policy snapshot, access snapshot, and recent audit evidence.

</specifics>

<deferred>
## Deferred Ideas

- Compliance-targeted CSV or PDF exports are out of scope.
- Deep policy editing UI belongs in Phase 19.
- Tenant-aware policy separation remains future enterprise work.

</deferred>

---
*Phase: 17-enterprise-policy-and-audit-controls*
*Context gathered: 2026-03-27*
