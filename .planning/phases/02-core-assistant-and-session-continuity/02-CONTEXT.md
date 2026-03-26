# Phase 2: Core Assistant and Session Continuity - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the primary assistant conversation path feel durable and coherent across normal operator use. This phase hardens the existing persisted CLI, gateway, and control-session surfaces so operators can trust that a resumed assistant session is the same conversation, not a hidden reset with stale metadata.

</domain>

<decisions>
## Implementation Decisions

### Primary assistant surface
- **D-01:** Treat the persisted assistant path as the canonical MVP conversation model across CLI and assistant-tagged runtime surfaces.
- **D-02:** Reuse existing route-key-based restore behavior before adding any new session identity mechanism.
- **D-03:** Prefer improving continuity semantics and visibility in shipped surfaces over inventing a new frontend.
- **D-04:** CLI assistant and gateway-managed assistant sessions should present the same continuity story: stable identity, persisted state, and inspectable history.

### Continuity trust
- **D-05:** Persisted state alone is not enough; operators need explicit signals about whether a session was resumed, how it was matched, and what history was restored.
- **D-06:** Continuity should be surfaced through typed reports and operator-facing UX, not only hidden inside session metadata blobs.
- **D-07:** Restored sessions must preserve assistant identity metadata consistently across CLI, webchat, and control-inspected surfaces.
- **D-08:** Restart and reconnect behavior should prefer reuse of active route-bound sessions over spawning silent duplicates.

### Operator visibility
- **D-09:** The control API and Control UI should show assistant-first continuity details in a way operators can scan quickly.
- **D-10:** Session inspection should summarize continuity state, not force operators to reverse-engineer raw JSON metadata and history arrays.
- **D-11:** CLI session inspection should also expose continuity signals because CLI remains the primary operator surface in MVP.
- **D-12:** Webchat and other assistant-tagged surfaces should inherit the same continuity contract via shared session metadata and inspectable reports.

### Verification posture
- **D-13:** Phase 2 needs regression coverage around restore-or-create behavior, continuity report shaping, and operator-facing inspection outputs.
- **D-14:** Documentation for quickstart and control surfaces should describe resumed-session behavior the same way the code implements it.
- **D-15:** This phase can defer broader memory-write policy to Phase 3, but it should make resumed session context understandable enough that future policy work has a trustworthy surface to attach to.

### the agent's Discretion
- The exact naming of continuity fields, report formatting, and UI copy is at the agent's discretion as long as it is concise and operator-usable.
- The split between CLI, control API, and control UI work can move slightly if the resulting continuity story is more coherent.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Assistant runtime and persisted CLI flow
- `crates/cli/src/commands/assistant.rs` — shared assistant identity, surface metadata, and CLI route-key helpers
- `crates/cli/src/commands/chat.rs` — persisted CLI assistant flow and route-key-based resume behavior
- `crates/cli/src/commands/session.rs` — operator session inspection and send flows

### Persisted session and gateway continuity
- `crates/db/src/session_store.rs` — durable session and conversation persistence, route-key lookup, and history loading
- `crates/gateway/src/sessions.rs` — restore-or-create session behavior for gateway or webchat surfaces
- `tests/integration/src/gateway_test.rs` — current session manager coverage for assistant metadata across surfaces

### Operator inspection and UX surfaces
- `crates/cli/src/commands/inspect.rs` — control-session inspection payloads
- `crates/cli/src/commands/start.rs` — `/control/sessions` and `/control/sessions/{id}` handlers
- `crates/cli/src/commands/control_ui.html` — current operator UI for browsing sessions

### Existing operator-facing docs
- `docs/src/getting-started/quickstart.md` — current promises about persisted assistant resume behavior
- `README.md` — top-level claims about assistant and operator surfaces

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `chat.rs` already reuses a stable CLI route key and reloads persisted history for the same workspace and user.
- `gateway::SessionManager` already has `restore_or_create_session` and `restore_or_create_session_with_context`, so route-bound webchat continuity exists at the persistence layer.
- `assistant.rs` already centralizes assistant identity and session metadata for CLI or assistant-tagged surfaces.
- `inspect.rs` and `/control/sessions` already provide a natural typed control surface for richer continuity summaries.

### Established Patterns
- Assistant surfaces encode continuity through `assistant_identity`, `assistant_surface`, `assistant_session_model`, and `route_key` metadata.
- Operator APIs generally expose typed JSON reports consumed directly by Control UI.
- The repo treats control surfaces as a stable operator contract, so continuity improvements should land in the API model first and UI second.

### Gaps to Close
- Session inspection currently returns raw session plus history without an assistant-focused continuity summary.
- Control UI lists sessions generically and does not call out whether a session is an assistant surface, what continuity model it uses, or whether the session is likely resumed.
- CLI and gateway continuity behavior have targeted tests, but not a unified operator-facing contract around resumed-session trust.

</code_context>

<specifics>
## Specific Ideas

- A resumed assistant session should answer three operator questions immediately: "what surface is this?", "why was this session reused?", and "how much history came back?"
- Control UI should prefer a small continuity summary over a wall of raw JSON for the first glance.
- The initial Phase 2 slice should improve trust and inspection first, then move toward deeper webchat or reconnect UX.

</specifics>

<deferred>
## Deferred Ideas

- Explicit assistant memory-write policy remains Phase 3 work.
- Richer standalone webchat product UI can wait if Control UI and typed reports provide a clear continuity contract first.
- Enterprise session governance, approval flows, and multi-operator collaboration remain outside this phase boundary.

</deferred>

---
*Phase: 02-core-assistant-and-session-continuity*
*Context gathered: 2026-03-26*
