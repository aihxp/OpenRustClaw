# Phase 15: Voice and Call Handling Parity - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Deepen the shipped voice and bounded call-handling lane so operators can understand real-time voice readiness, attention pressure, and recent activity from one coherent runtime story instead of jumping between separate voice, talk, and voice-plugin panels.

</domain>

<decisions>
## Implementation Decisions

### Voice parity contract
- **D-01:** Treat operator coherence as the main parity gap. The repo already has real voice session receipts, talk receipts, voice metrics, voice outcomes, bounded voice-call receipts, artifacts, and event timelines.
- **D-02:** Preserve the trust-first boundary. Phase 15 should aggregate and render the existing Rust-owned voice, talk, and bounded call receipts rather than inventing live telephony claims or a new frontend-only runtime.
- **D-03:** Prefer typed operator summaries over scattered raw panes. The runtime should answer practical voice-operator questions before the dashboard renders them.

### Operator value
- **D-04:** Operators should be able to answer: is voice configured, which sessions or calls need attention, is talk healthy, are any stale calls or sessions accumulating, and what happened most recently.
- **D-05:** Voice parity should stay grounded in shipped bounded behavior: voice sessions, talk receipts, prewarm and reap controls, voice-note transcription, and compiled skill voice-call receipts.
- **D-06:** The first concrete improvement should unify voice, talk, and bounded voice-call evidence into one typed operator report rather than adding more isolated controls.

### Existing boundaries to preserve
- **D-07:** The existing voice runtime, talk receipts, and skill voice-call registry remain the system of record.
- **D-08:** Control UI improvements should consume typed runtime summaries and the existing action routes, not duplicate decision logic in JavaScript.

### the agent's Discretion
- The exact operator report schema is at the agent's discretion as long as it surfaces readiness, attention, recent activity, and coverage across voice, talk, and bounded call receipts.
- Small dashboard layout additions are acceptable if they directly improve operator clarity for the shipped voice surface.

</decisions>

<canonical_refs>
## Canonical References

**Downstream implementation MUST read these before editing.**

### Voice and call runtime sources
- `crates/cli/src/commands/voice_runtime.rs` — voice session receipts, metrics, health, outcomes, artifacts, events, and runtime status
- `crates/cli/src/commands/talk.rs` — talk runtime status, metrics, receipts, and event timelines
- `crates/cli/src/commands/skills.rs` — bounded voice-plugin and voice-call receipts, health, metrics, events, and artifacts

### Runtime and operator surfaces
- `crates/cli/src/commands/inspect.rs` — existing typed operator-summary patterns
- `crates/cli/src/commands/start.rs` — shipped `/control/...` routes
- `crates/cli/src/commands/control_ui.html` — current voice, talk, and bounded voice-call dashboard panes
- `crates/cli/src/commands/control_ui.rs` — dashboard contract tests

### Validation patterns
- `tests/integration/src/voice_outcomes_test.rs` — recent voice evidence pattern
- `tests/integration/src/mobile_operator_report_test.rs` — typed operator-report integration coverage pattern
- `.planning/phases/13-mobile-runtime-parity/13-CONTEXT.md` — report -> route -> dashboard pattern

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `voice_runtime::voice_status`, `voice_provider_catalog`, `voice_metrics`, `voice_session_health`, and `voice_session_outcomes` already expose the primary voice runtime truth.
- `talk::runtime_status` and `talk::runtime_metrics_data` already summarize talk receipts without requiring raw receipt inspection.
- `skills::voice_call_health_data`, `voice_call_metrics_data`, and `voice_calls_data` already summarize bounded voice-call receipts and derive health.
- Control UI already has typed renderers for voice session detail, talk receipt detail, and bounded voice-call events, artifacts, and metrics.

### Established Patterns
- Recent parity phases improved operator trust by aggregating existing persisted evidence into typed runtime summaries before wiring dashboard rendering.
- The dashboard already supports summary cards and typed tables, so Phase 15 should extend those patterns instead of adding new rendering paradigms.

### Gaps to Close
- Operators still have to read several raw `pre` panes and mentally combine voice, talk, and bounded voice-call state.
- There is no one typed summary that answers whether the voice surface is healthy, configured, and attention-worthy.
- The dashboard exposes real voice actions but still lacks a top-level parity surface that feels operationally coherent.

</code_context>

<specifics>
## Specific Ideas

- Add a typed voice operator summary that aggregates voice readiness, provider coverage, session health, talk coverage, bounded voice-call coverage, attention signals, and recent activity.
- Expose that report through a shipped runtime route and use it to upgrade the top-level voice/talk dashboard panes.
- Preserve focused verification proving stale sessions, talk errors, and stale bounded voice calls surface as operator-visible attention signals.

</specifics>

<deferred>
## Deferred Ideas

- Full live telephony, browser-native real-time calling UX, or mass-market call-center features remain out of scope.
- Broad visual redesign of the dashboard remains outside this phase.
- New voice providers or deeper media processing beyond the current shipped lanes remain separate work.

</deferred>

---
*Phase: 15-voice-and-call-handling-parity*
*Context gathered: 2026-03-27*
