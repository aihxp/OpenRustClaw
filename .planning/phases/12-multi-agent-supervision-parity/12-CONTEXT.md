# Phase 12: Multi-Agent Supervision Parity - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Expand the shipped orchestration and delegated-run supervision surfaces so operators can inspect and control more of the delegated-run lifecycle than final receipts alone. This phase should deepen the existing Rust-owned orchestration runtime, receipt files, active-run snapshots, and Control UI rather than introducing a separate multi-agent system.

</domain>

<decisions>
## Implementation Decisions

### Supervision parity contract
- **D-01:** Treat operator-readable supervision as the main parity gap. The repo already records delegations, worker results, checkpoints, relationships, traces, transcripts, active-run snapshots, and active-run events, but much of that remains buried in raw JSON or shallow tables.
- **D-02:** Preserve the current trust-first orchestration model. Phase 12 should expose more of the delegated-run lifecycle, approval context, and live run state without weakening explicit pause, resume, kill, or approval boundaries.
- **D-03:** Prefer typed supervision summaries over frontend-only reconstruction. The runtime should aggregate worker, delegation, checkpoint, and live-run state before Control UI renders it.

### Operator visibility
- **D-04:** Receipt supervision should answer practical operator questions: which workers were delegated, what each worker returned, which ones failed or need input, what confidence they reported, and whether escalation is recommended.
- **D-05:** Active supervision should expose the live run state in operator terms: current actor, pause or kill flags, recent event activity, resource totals, and live attention signals.
- **D-06:** Control UI should render supervision as tables and compact summaries instead of leaving operators to parse raw JSON blobs for routine inspection.

### Existing boundaries to preserve
- **D-07:** Approval policy, autonomy level, delegation limits, and resource signals already flow through routing and active-run data; Phase 12 should surface those, not invent new policy semantics.
- **D-08:** Reflection candidates and decision lessons are already part of the orchestration loop. The new supervision surfaces should remain compatible with that pattern and make it easier to inspect why those candidates exist.

### the agent's Discretion
- The exact supervision-summary schema is at the agent's discretion as long as it stays typed, stable, and clearly derived from existing orchestration runtime artifacts.
- The split between receipt supervision and live active-run supervision can move slightly as long as the phase ends with one coherent operator story.

</decisions>

<canonical_refs>
## Canonical References

**Downstream implementation MUST read these before editing.**

### Orchestration runtime and artifacts
- `crates/cli/src/commands/orchestrate.rs` — orchestration run records, supervision summary, trace/resources helpers, active-run snapshots, and live event files
- `crates/cli/src/commands/start.rs` — shipped `/control/orchestration/...` routes

### Operator surfaces
- `crates/cli/src/commands/control_ui.html` — current orchestration tables and raw detail panes
- `crates/cli/src/commands/control_ui.rs` — dashboard contract tests

### Existing phase patterns
- `.planning/phases/11-browser-automation-depth/11-CONTEXT.md` and summaries — recent pattern for ledger -> typed runtime -> Control UI -> verification
- `tests/integration/src` — focused ledger and summary tests used by recent trust-surface phases

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `OrchestrationRunRecord` already persists `delegations`, `worker_results`, `checkpoints`, `trace`, `relationships`, `transcript`, `reflection_candidates`, and `supervision`.
- `ActiveOrchestrationRun` already tracks `current_stage`, current actor, pause and kill flags, worker counts, relationship counts, resource totals, and receipt linkage.
- `read_run_supervision()`, `read_run_trace()`, `read_run_transcript()`, and `read_run_resources()` already expose most of the orchestration receipt material through shipped runtime endpoints.
- Control UI already has orchestration receipt and active-run sections with inspect, pause, resume, kill, and reflection-candidate promotion controls.

### Established Patterns
- Prior parity phases improved trust by turning raw stored data into typed summaries and dashboard panels instead of asking operators to decode underlying files manually.
- The runtime already uses stable JSON file-backed state under `.claw/control/orchestration-runs` and `.claw/control/orchestration-active`, so supervision work should extend those types instead of replacing them.
- Reflection candidates, resource totals, and active operator controls are already shipped and should remain first-class parts of the supervision story.

### Gaps to Close
- Receipt supervision currently omits operator-readable views of delegated tasks and worker outcomes from the primary runtime summary.
- Active-run inspection exposes raw state and events, but the dashboard does not yet present a clearer live supervision summary for current actor, flags, event tail, or attention signals.
- The run list and receipt detail surfaces still make operators infer too much from low-level JSON rather than exposing the delegated-run lifecycle directly.

</code_context>

<specifics>
## Specific Ideas

- Add a typed supervision detail report that rolls up delegations, worker outcomes, checkpoint summary, resource state, approval policy, and escalation signals in one operator-facing shape.
- Add a typed active-run supervision report that combines active snapshot data with recent events and explicit operator control or attention indicators.
- Upgrade Control UI to render worker outcomes, delegated tasks, live operator state, and recent events as tables instead of raw JSON.

</specifics>

<deferred>
## Deferred Ideas

- Full autonomous multi-agent planning, cross-run reflection memory, or enterprise approval chains remain outside this phase.
- New worker execution backends or distributed orchestration infrastructure are outside scope.
- Broader mobile, browser, or voice parity work remains in later phases even if this phase establishes reusable supervision patterns.

</deferred>

---
*Phase: 12-multi-agent-supervision-parity*
*Context gathered: 2026-03-26*
