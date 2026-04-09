---
phase: 13
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:58:38.620Z
plans_reviewed: [13-01-PLAN.md, 13-02-PLAN.md, 13-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 13

## Gemini Review

# Phase 13: Mobile Runtime Parity – Plan Review

## 1. Summary
The plans for Phase 13 present a well-structured, incremental approach to closing the mobile runtime parity gap. They correctly identify the core issue—operators currently have to stitch together fragmented mobile state (receipts, conflicts, sync state)—and propose a sensible, Rust-backend-driven aggregation strategy. The sequencing (backend aggregation -> API/UI wiring -> documentation/verification) is logical, and the explicit scoping to defer larger dashboard rewrites to Phase 14 demonstrates excellent discipline.

## 2. Strengths
* **Strong Context Alignment:** The plans adhere strictly to the design constraints (D-02, D-07), ensuring that approval-sensitive actions and trust boundaries remain intact rather than being bypassed by new UI logic.
* **Appropriate Scoping:** Explicitly deferring the broader Control UI dashboard restructuring to Phase 14 prevents scope creep while still delivering immediate value through the targeted `nodes/{id}/summary` endpoint.
* **Backend-Driven Aggregation:** By building the typed operator report in Rust (`13-01`) rather than trying to stitch raw JSON payloads in the frontend, the platform maintains a single source of truth and simplifies the UI logic (`13-02`).
* **Test-Driven:** `13-01` explicitly requires focused integration coverage to ensure the richer report captures realistic flows (e.g., approval and conflict states) rather than just unit-testing the struct assembly.
* **Clear Dependency Chain:** The execution waves and dependencies (13-01 -> 13-02 -> 13-03) are correctly defined to prevent integration bottlenecks.

## 3. Concerns
* **[MEDIUM] Unbounded Aggregation Risk (13-01):** The plan mentions aggregating "recent activity" but does not explicitly enforce bounds. If a node has a massive history of receipts, sync conflicts, or media artifacts, aggregating all of them could lead to unbounded memory usage, high CPU load, or massive payload sizes.
* **[MEDIUM] Error Handling & Missing Data (13-02):** The API route (`/control/mobile/nodes/{id}/summary`) and UI rendering must handle edge cases where a node ID does not exist, a node is entirely offline with no history, or underlying receipt files are corrupted/missing. 
* **[LOW] UI Degradation/Transition (13-02):** When rendering the new typed report in `control_ui.html`, the plan doesn't specify how to gracefully degrade if older nodes lack the complete data structures expected by the new typed summary.

## 4. Suggestions
* **Enforce Activity Limits:** Update `13-01` to explicitly state that the "recent activity" aggregation must be bounded (e.g., limit to the last 50 events/receipts) to guarantee consistent performance and payload sizes.
* **Specify API Error Contracts:** Update `13-02` to define the expected HTTP response codes for the new route (e.g., returning 404 when an invalid node ID is requested) and ensure the Control UI renders a clear "Node Not Found" or "No Activity" empty state instead of breaking.
* **Define Default/Empty States:** Ensure the typed operator struct in `13-01` provides sensible defaults (`Default::default()`) so that nodes with partial pairing history or missing receipts still render successfully in the UI.

## 5. Risk Assessment
**Overall Risk Level: LOW**

**Justification:** The architectural approach is fundamentally sound and strictly follows established patterns in the codebase (backend aggregation -> UI rendering). The scope is tightly controlled, deliberately avoiding a full frontend rewrite. The main risks are performance regressions related to unbounded data aggregation, which are easily mitigated by adding simple limits to the queries in `13-01`.

---

## Claude Review

# Cross-AI Review: Phase 13 — Mobile Runtime Parity

## 13-01: Add typed mobile operator reports

### Summary
Solid foundational plan that aggregates scattered mobile runtime data into a single typed report. The approach of composing existing durable receipts rather than inventing new data sources is correct.

### Strengths
- Builds on existing `mobile_node_summary_data()`, `mobile_metrics_data()`, and related helpers
- Preserves approval-sensitive and conflict-sensitive signals explicitly
- Integration tests validate the report contract against realistic fixture flows

### Concerns
- **MEDIUM** — The plan doesn't specify the report struct schema. "At the agent's discretion" is fine per CONTEXT.md, but the plan should at least name the key fields so Plan 02 (which renders them) can be designed coherently.
- **LOW** — No mention of how stale or disconnected nodes are represented in the report. A node that hasn't heartbeated in hours should surface differently than a healthy one.
- **LOW** — Test file placement in `tests/integration/` adds a new module but the plan doesn't clarify whether existing mobile test coverage overlaps.

### Suggestions
- List the top-level fields of the operator report struct (even tentatively) so the route and UI plans can reference them.
- Include an explicit "attention needed" or "requires operator action" flag derived from approval backlog + unresolved conflicts.

### Risk Assessment
**LOW** — Straightforward aggregation over existing data. Main risk is schema ambiguity propagating to Plan 02.

---

## 13-02: Surface mobile operator reports in runtime APIs and Control UI

### Summary
Reasonable wiring plan that exposes the typed report via a runtime route and renders it in the dashboard. Correctly scopes UI work to the primary node view and defers broader dashboard cleanup.

### Strengths
- Clean separation: one route, one render target, no sprawl
- Explicitly preserves existing control actions and approval-aware contracts
- Dashboard contract tests in `control_ui.rs` lock the rendering

### Concerns
- **MEDIUM** — `start.rs` is already identified as a compatibility-heavy hotspot (per CLAUDE.md). Adding another route there increases file size and merge risk. The plan should acknowledge this and keep the handler minimal (delegate to `mobile.rs` helpers).
- **MEDIUM** — No error handling specified for the route. What happens when the node ID doesn't exist? The plan should specify a 404 or typed error response consistent with other `/control/mobile/...` routes.
- **LOW** — The route path `{id}/summary` may collide with or confuse the existing `mobile_node_summary_data()` CLI helper. Naming should distinguish the richer operator report from the existing summary.

### Suggestions
- Keep the `start.rs` handler to a thin delegation wrapper; all logic should live in `mobile.rs`.
- Match the error response pattern used by existing `/control/mobile/nodes/{id}/...` routes.
- Consider naming the route `/control/mobile/nodes/{id}/report` to distinguish from the existing summary concept.

### Risk Assessment
**LOW-MEDIUM** — The `start.rs` hotspot concern is real but manageable with disciplined delegation.

---

## 13-03: Align mobile parity docs and preserve verification

### Summary
Standard docs-and-verification closeout plan. Appropriately scoped.

### Strengths
- Verification artifact tied to concrete evidence, not vague claims
- Updates both operator-facing docs (README, feature-matrix) and planning artifacts
- Wave 2 dependency on 01+02 is correct

### Concerns
- **LOW** — `README.md` changes for a single typed report route risk bloating the root README. A one-line pointer to the feature matrix or mobile docs may be sufficient.
- **LOW** — No mention of updating the CONTEXT.md "Gaps to Close" section to reflect closed gaps.

### Suggestions
- Keep README changes minimal; detailed mobile report docs belong in `docs/feature-matrix.md` or a dedicated mobile section.

### Risk Assessment
**LOW** — Docs-only plan with clear scope.

---

## Overall Phase Assessment

**Overall Risk: LOW**

The phase is well-scoped: aggregate existing data → expose via route → render in UI → document. The dependency chain is clean and the scope boundary (defer broad dashboard cleanup to Phase 14) is explicitly maintained.

**Top recommendation:** Pin down the operator report struct fields in Plan 01 before execution so Plans 02 and 03 can reference concrete field names rather than designing against an unknown schema.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers converged on an overall LOW risk posture.
