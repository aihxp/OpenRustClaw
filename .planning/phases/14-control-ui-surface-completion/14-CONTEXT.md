# Phase 14: Control UI Surface Completion - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Bring the shipped Control UI materially closer to OpenClaw parity across the deeper runtime surfaces that already exist, with emphasis on replacing remaining raw JSON inspection panes with typed renderers over the current browser, supervision, mobile, voice, and skill/runtime detail contracts.

</domain>

<decisions>
## Implementation Decisions

### UI completion contract
- **D-01:** Treat raw JSON reliance as the primary Control UI parity gap. The dashboard already exposes many real actions and typed APIs, but several high-value detail panes still call `setText(...)` directly on rich payloads.
- **D-02:** Preserve typed-runtime-first rendering. Phase 14 should improve the dashboard by consuming existing typed routes and reports rather than inventing frontend-only state or hidden derivations.
- **D-03:** Prefer a small number of high-leverage renderer improvements over a scattered styling pass. The point is operator comprehension, not cosmetic churn.

### Operator value
- **D-04:** The highest-value surfaces are the ones an operator actually inspects during trust-sensitive work: voice session detail, mobile command/conflict/message detail, skill detail, and other remaining raw panes inside `/control/ui`.
- **D-05:** Browser and orchestration already have richer typed tables from earlier phases; Phase 14 should align the rest of the dashboard with that same standard instead of reworking completed areas.
- **D-06:** This phase should make common flows clearer without changing the action model or introducing a separate frontend architecture.

### Existing boundaries to preserve
- **D-07:** Runtime state, trace, transcript, and receipt files remain the system of record. The dashboard is a renderer over shipped typed APIs, not a second workflow engine.
- **D-08:** Voice/call parity itself remains Phase 15 work; Phase 14 should improve Control UI rendering over existing voice surfaces, not expand voice backend capability.

### the agent's Discretion
- The exact set of panes upgraded first is at the agent's discretion as long as the work clearly reduces raw-JSON dependence on parity-critical surfaces.
- Small layout or table additions are acceptable when they directly support typed rendering and operator clarity.

</decisions>

<canonical_refs>
## Canonical References

**Downstream implementation MUST read these before editing.**

### Dashboard and routes
- `crates/cli/src/commands/control_ui.html` — current dashboard markup and client-side render logic
- `crates/cli/src/commands/control_ui.rs` — dashboard contract tests
- `crates/cli/src/commands/start.rs` — runtime routes the dashboard consumes

### Existing typed surface examples
- `.planning/phases/12-multi-agent-supervision-parity/12-CONTEXT.md` and summaries — typed report -> dashboard tables pattern
- `.planning/phases/13-mobile-runtime-parity/13-CONTEXT.md` and summaries — mobile report -> dashboard render pattern

### Supporting runtime detail sources
- `crates/cli/src/commands/orchestrate.rs`
- `crates/cli/src/commands/mobile.rs`
- `crates/cli/src/commands/inspect.rs`
- voice/talk/skills runtime helpers referenced by the existing dashboard routes

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Browser workflow history, orchestration supervision, session continuity, runtime operator ops, security posture, and enterprise foundations already demonstrate the desired typed-renderer pattern.
- The mobile main node view now also uses a typed per-node operator report rather than raw JSON.
- Many remaining panes still use `setText(...)` over rich payloads, especially in voice session detail, mobile sub-detail panes, talk session detail, and skill detail flows.

### Established Patterns
- The dashboard already has reusable `renderTableRows(...)`, `setText(...)`, and fetch helpers, so new typed renderers can slot into the current structure without rewriting the dashboard shell.
- Prior parity phases favored compact summary blocks plus tables for timelines, signals, or recent history rather than giant nested dumps.

### Gaps to Close
- High-value operator detail panes still surface raw objects rather than structured views.
- The dashboard experience is uneven: some panels feel production-ready, while others still read like developer introspection tools.
- Control UI parity is currently limited more by renderer depth than by missing runtime routes.

</code_context>

<specifics>
## Specific Ideas

- Inventory the remaining `setText(...)` raw-detail panes and group them by operator value.
- Upgrade the highest-value raw panes into typed summary/timeline/table renderers backed by existing APIs.
- Add dashboard contract tests for each upgraded surface to keep the renderer set from regressing.

</specifics>

<deferred>
## Deferred Ideas

- Broad visual redesign or layout overhaul is out of scope.
- New backend feature lanes for voice/call or other domains remain in their own phases.
- Live chat/browser-native assistant UI remains intentionally outside this Control UI completion phase.

</deferred>

---
*Phase: 14-control-ui-surface-completion*
*Context gathered: 2026-03-27*
