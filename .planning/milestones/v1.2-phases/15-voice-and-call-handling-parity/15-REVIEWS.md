---
phase: 15
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:04:06.916Z
plans_reviewed: [15-01-PLAN.md, 15-02-PLAN.md, 15-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 15

## Gemini Review

Here is the structured review of the implementation plans for Phase 15: Voice and Call Handling Parity.

### Plan 15-01: Add a typed voice operator report

**1. Summary**
This plan focuses on the backend aggregation of existing voice, talk, and bounded call receipts into a unified, typed operator report exposed via a new (or updated) control route. It directly addresses the phase's core mandate to eliminate operator fragmentation by creating a single source of truth for runtime voice state without introducing new telephony paradigms or state layers.

**2. Strengths**
*   **Adherence to Constraints:** Strictly follows decision D-02 and D-07 by aggregating existing persisted evidence rather than inventing new state tracking.
*   **Test-Driven:** Explicitly requires focused integration coverage for "attention-signal behavior" (stale sessions, errors), ensuring the report actually provides operator value and isn't just a passthrough.
*   **Clear Boundaries:** Scoped cleanly to the `inspect.rs` and `start.rs` adapter layers, avoiding deep rewrites of the underlying runtime mechanisms.

**3. Concerns**
*   **Performance / I/O Blocking (MEDIUM):** Aggregating data from `voice_runtime`, `talk`, and `skills` likely involves multiple file reads or SQLite queries. If executed synchronously and sequentially on the main thread or within a blocking HTTP handler, this could introduce latency to the control plane.
*   **Partial Failure Handling (MEDIUM):** The plan does not specify how the report should behave if one subsystem (e.g., talk metrics) fails to load due to a corrupt receipt file while the others succeed. A brittle aggregation could break the entire voice dashboard.
*   **Route Clarity (LOW):** The plan modifies `start.rs` but does not explicitly name the intended HTTP route (e.g., `/control/voice/summary`), which could lead to naming inconsistencies during implementation.

**4. Suggestions**
*   **Specify Concurrency:** Update the implementation task to explicitly require parallel or asynchronous data fetching across the three subsystems to prevent control-plane blocking.
*   **Define Degradation:** Explicitly state that the typed report must support `Option<T>` or similar partial-failure states so that a corrupt `talk` receipt doesn't hide a healthy `voice_call` summary.
*   **Name the Route:** Define the exact route path in the plan to ensure frontend alignment in Plan 15-02.

**5. Risk Assessment**
**MEDIUM.** The architectural approach is sound, but the risk of introducing control-plane latency or brittle error handling through data aggregation requires careful implementation of the I/O boundaries.

---

### Plan 15-02: Surface the voice operator report in Control UI

**1. Summary**
This plan wires the newly created typed operator report into the Control UI dashboard, upgrading the raw `pre` panes into structured tables and summary cards. It ensures that the backend parity gains from 15-01 are actually visible and usable by the operator while explicitly preserving existing actionable voice controls.

**2. Strengths**
*   **Scoped UI Changes:** Explicitly targets the *top-level* voice panes and avoids the trap of a broader dashboard redesign (which was explicitly deferred in the phase context).
*   **Contract Testing:** Modifies `control_ui.rs` alongside the HTML, ensuring that the new UI renderers are backed by dashboard contract tests rather than just visual inspection.
*   **Action Preservation:** Explicitly mandates keeping existing voice, talk, and bounded call actions intact, reducing the risk of functional regressions.

**3. Concerns**
*   **Scope Creep (LOW):** "Richer voice and call operator story" is slightly subjective. There is a small risk the implementer might over-engineer the UI components if not strictly bounded by the existing dashboard CSS/patterns.
*   **State Sync (LOW):** If the new typed report is expensive to generate (see 15-01), the UI might need loading states or polling adjustments that aren't mentioned in the plan.

**4. Suggestions**
*   **Visual Constraints:** Explicitly state that the UI additions must use existing CSS classes and component patterns (e.g., standard summary cards, typed tables) to enforce the "no visual redesign" constraint.
*   **Empty States:** Add a requirement to handle and test the "empty state" (e.g., when voice is entirely unconfigured or zero receipts exist) gracefully.

**5. Risk Assessment**
**LOW.** The plan correctly limits itself to consuming the new endpoint and updating the contract tests. Assuming 15-01 is implemented cleanly, this is a straightforward UI binding task.

---

### Plan 15-03: Align voice docs and preserve verification

**1. Summary**
This plan handles the lifecycle closeout for the phase, requiring documentation updates in the `README.md` and `feature-matrix.md`, alongside the generation of the `15-VERIFICATION.md` artifact. It ensures the repository's public claims match the newly shipped reality.

**2. Strengths**
*   **Truthful Claims:** Focuses on describing the *shipped* dashboard surface rather than making generic marketing claims about voice parity.
*   **Lifecycle Compliance:** Properly sequences the verification artifact generation *after* tests pass, adhering to the repository's strict milestone integrity rules (Phase 9 legacy).
*   **Dependency Ordering:** Correctly depends on both 15-01 and 15-02 being complete.

**3. Concerns**
*   **None significant (LOW):** This is a standard, well-structured documentation and closeout plan.

**4. Suggestions**
*   **Screenshot/CLI Output:** Suggest including a snippet of the new CLI inspect output or a note about the UI layout in the `15-VERIFICATION.md` to provide concrete historical evidence of the UX change.

**5. Risk Assessment**
**LOW.** The plan perfectly matches the documentation and verification contracts established in earlier milestones. Zero technical risk.

---

## Claude Review

# Cross-AI Review: Phase 15 — Voice and Call Handling Parity

## Plan 15-01: Add a typed voice operator report

### Summary
Solid aggregation plan that consolidates three existing data sources (voice runtime, talk runtime, bounded voice-calls) into one typed operator report. The approach correctly builds on existing data rather than inventing new runtime capabilities.

### Strengths
- Reuses existing `voice_status`, `talk::runtime_status`, and `skills::voice_call_health_data` rather than duplicating logic
- Follows the established report→route→dashboard pattern from Phase 13
- Attention signals derived from durable receipts, not ephemeral state
- Integration test explicitly targets attention-signal behavior (stale sessions, talk errors, stale calls)

### Concerns
- **MEDIUM**: The plan doesn't specify the report struct shape at all. "At the agent's discretion" is fine for context docs, but the plan should at least name the key fields (readiness, attention items, recent activity) as typed categories to prevent scope drift during implementation.
- **LOW**: No mention of what "stale" means quantitatively — thresholds for stale sessions/calls should be explicit or configurable, otherwise tests are brittle.
- **LOW**: Only two files listed for code changes (`inspect.rs`, `start.rs`), but the aggregation logic may warrant a dedicated module or at least a clearly bounded function to avoid further bloating these already-large files.

### Suggestions
- Define the report struct fields in the plan (even as pseudocode) so Plan 02 can reference concrete field names for rendering
- Specify staleness thresholds or note they come from existing voice runtime constants
- Consider whether the aggregation function belongs in `inspect.rs` or a shared voice module that both `inspect` and `start` can call

### Risk Assessment
**LOW** — The plan is well-scoped and grounded in existing patterns. The concerns are about specificity, not direction.

---

## Plan 15-02: Surface the voice operator report in Control UI

### Summary
Straightforward dashboard wiring plan that consumes the typed report from 15-01. Correctly scoped to rendering rather than logic duplication.

### Strengths
- Explicitly preserves existing voice actions while upgrading the summary surface
- Stays within the existing dashboard rendering paradigm (summary cards, typed tables)
- Dashboard contract tests in `control_ui.rs` lock the renderer behavior
- Clear dependency on 15-01

### Concerns
- **MEDIUM**: No specifics on what "attention tables" or "recent-activity surfaces" look like in HTML. Given `control_ui.html` is likely already large, the plan should indicate where in the page hierarchy the new panes land (replacing raw `pre` blocks? added above them? new tab?).
- **LOW**: No mention of error/loading states — what does the dashboard show if the voice report route returns empty or errors?
- **LOW**: The plan mentions "upgrade remaining top-level voice panes" which is vague and could invite scope creep beyond what the report provides.

### Suggestions
- Specify which existing `pre` panes are being replaced or augmented
- Add a brief note on graceful degradation (voice not configured → clear "not configured" state)
- Tighten "upgrade remaining top-level voice panes" to "render the 15-01 report as the primary voice summary card"

### Risk Assessment
**LOW** — Standard dashboard wiring with established patterns. The vagueness around layout is a minor implementation risk, not a design risk.

---

## Plan 15-03: Align voice docs and preserve verification

### Summary
Standard docs-and-verification closeout plan. Necessary for lifecycle compliance but carries minimal technical risk.

### Strengths
- Correctly depends on both 15-01 and 15-02 (wave 2)
- Verification artifact captures concrete report and dashboard behavior, not abstract claims
- Updates both operator-facing docs (`README.md`, `feature-matrix.md`) and planning artifacts

### Concerns
- **LOW**: `README.md` changes for a single operator report feel heavyweight — confirm the README voice section actually needs updating vs. just `feature-matrix.md`.
- **LOW**: No mention of verifying the route is documented in any API reference or operator guide beyond README.

### Suggestions
- Keep README changes minimal (one line if any); focus detail in `feature-matrix.md`

### Risk Assessment
**LOW** — Boilerplate closeout with no technical risk.

---

## Overall Phase Assessment

**Overall Risk: LOW**

The phase is well-bounded: aggregate existing data, expose it, render it, document it. The three-plan sequence follows an established pattern (Phase 13 mobile report is the direct precedent), dependency ordering is correct, and scope is realistic for the stated goals.

The main cross-cutting concern is **specificity** — Plans 01 and 02 defer too many structural decisions to implementation time. This isn't dangerous given the precedent patterns, but naming the report fields and the target dashboard location in the plans would reduce implementation ambiguity and make the wave-2 verification more concrete.

No security, performance, or scope-creep risks detected. The phase achieves its goals if executed as written.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: claude=LOW.
