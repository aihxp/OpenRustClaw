# Phase 13: Mobile Runtime Parity - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Broaden the shipped mobile runtime lane so operators can understand mobile node health, approval pressure, pending work, and recent device activity from one coherent runtime story. This phase should deepen the existing Rust-owned mobile receipts, metrics, app-session records, command receipts, notification/inbox/outbox records, and runtime state instead of inventing a new mobile subsystem.

</domain>

<decisions>
## Implementation Decisions

### Mobile parity contract
- **D-01:** Treat operator readability as the main mobile parity gap. The repo already persists node manifests, pairing history, runtime state, app sessions, sync conflicts, notifications, inbox/outbox receipts, command records, capability executions, media artifacts, and per-node activity entries.
- **D-02:** Preserve the current trust-first mobile contract. Phase 13 should expose a richer mobile operator view without weakening approval-gated commands, explicit wake/rehydrate controls, or durable receipt trails.
- **D-03:** Prefer typed mobile operator reports over frontend-only stitching. The runtime should aggregate mobile health, counts, recent activity, and approval-sensitive signals before the dashboard renders them.

### Operator visibility
- **D-04:** Mobile node inspection should answer practical operator questions quickly: is the node reachable, is push configured, is sync healthy, are approvals or conflicts piling up, what recent actions happened, and what should the operator look at next.
- **D-05:** Global mobile visibility should stay grounded in the existing fleet metrics and receipt counts instead of introducing speculative device scoring.
- **D-06:** The first shipped UI improvement in this phase should be limited to exposing the new typed mobile report cleanly; broader Control UI completion still belongs to Phase 14.

### Existing boundaries to preserve
- **D-07:** Command approvals, execution receipts, sync-conflict resolution, notification acknowledgement, and message acknowledgement already form the trust boundary. The richer mobile report should surface those signals, not bypass them.
- **D-08:** Existing mobile activity, session metrics, and command metrics helpers should remain the source of truth for derived counts and timelines.

### the agent's Discretion
- The exact mobile operator report schema is at the agent's discretion as long as it is typed, stable, and clearly derived from existing mobile runtime artifacts.
- A small amount of mobile-specific dashboard wiring is acceptable if it directly renders the typed report, but large multi-surface dashboard cleanup remains deferred to Phase 14.

</decisions>

<canonical_refs>
## Canonical References

**Downstream implementation MUST read these before editing.**

### Mobile runtime and artifacts
- `crates/cli/src/commands/mobile.rs` — mobile manifests, runtime state, pairing/app-session/conflict/message/command/capability/media receipts, metrics, summaries, and CLI helpers
- `crates/cli/src/commands/start.rs` — shipped `/control/mobile/...` routes

### Operator surfaces
- `crates/cli/src/commands/control_ui.html` — current mobile dashboard sections
- `crates/cli/src/commands/control_ui.rs` — dashboard contract tests

### Existing validation and fixture patterns
- `tests/integration/src/documented_scenario_test.rs` — mobile operator scenario coverage
- `tests/integration/src/fixture_suite_test.rs` — pairing and approval workflow fixtures
- `.planning/phases/12-multi-agent-supervision-parity/12-CONTEXT.md` and summaries — recent pattern for typed operator report -> runtime route -> selective dashboard rendering -> verification

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `mobile_node_summary_data()` already aggregates counts across pairings, sessions, conflicts, notifications, inbox/outbox messages, commands, capability executions, and media artifacts, but it is CLI-only today.
- `mobile_metrics_data()` already exposes fleet-wide mobile counts, while `node_runtime_data()`, `node_push_state_data()`, `node_sync_state_data()`, `app_session_metrics_data()`, `command_metrics_data()`, and `node_activity_data()` expose the underlying typed slices.
- The runtime already exposes many `/control/mobile/...` routes, and Control UI already has a substantial mobile section with actions for pairing, heartbeat, push registration, sync, wake, rehydrate, command dispatch, notifications, inbox/outbox, sync conflicts, capability execution, and media artifacts.

### Established Patterns
- Recent parity phases improved operator trust by aggregating existing stored data into typed summaries before exposing them through runtime endpoints and selective dashboard rendering.
- The mobile lane already preserves durable receipts under `.claw/mobile/...`; parity work should compose those receipts rather than replacing them.
- Approval-sensitive behavior is already explicit in the mobile command lane, and new summaries should make that approval pressure clearer rather than more implicit.

### Gaps to Close
- Mobile operators still need to bounce across raw runtime, push, sync, activity, pairing, session, and command views to understand one node’s state.
- The main mobile dashboard detail still relies heavily on raw JSON panes instead of a typed mobile operator report.
- The current runtime lacks one cohesive per-node report that carries health, approval, conflict, and recent-activity signals together.

</code_context>

<specifics>
## Specific Ideas

- Add a typed mobile operator report that combines per-node summary counts, runtime status, push/sync state, session metrics, command metrics, recent activity, and explicit attention signals.
- Expose that report through a shipped runtime route and use it to improve the main mobile inspection surface in Control UI without attempting the full mobile dashboard cleanup yet.
- Add focused integration coverage proving the richer mobile report preserves approval-sensitive and conflict-sensitive signals.

</specifics>

<deferred>
## Deferred Ideas

- Broad mobile dashboard restructuring across every detail pane remains Phase 14 work.
- Native mobile app packaging, background transport reliability, or real device distribution remain outside scope.
- Voice/call parity and broader cross-surface dashboard completion remain later phases.

</deferred>

---
*Phase: 13-mobile-runtime-parity*
*Context gathered: 2026-03-27*
