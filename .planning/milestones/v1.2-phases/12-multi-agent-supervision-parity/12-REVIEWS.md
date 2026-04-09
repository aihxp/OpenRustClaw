---
phase: 12
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:57:04.823Z
plans_reviewed: [12-01-PLAN.md, 12-02-PLAN.md, 12-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 12

## Gemini Review

Here is the cross-AI review of the implementation plans for Phase 12: Multi-Agent Supervision Parity.

### 12-01-PLAN.md

**Summary**
This plan focuses on backend data aggregation, creating typed helpers in `orchestrate.rs` to synthesize existing raw JSON orchestration artifacts into operator-friendly summaries for both completed receipts and active runs. It appropriately prioritizes structured, typed data over frontend-only reconstruction.

**Strengths**
- Strictly builds on existing persisted state rather than introducing new parallel data stores or reinventing the orchestration loop.
- Separates data aggregation from presentation, providing a clean typed API for downstream consumption.
- Aligns well with the architectural constraint to preserve existing approval boundaries and trust-first models.

**Concerns**
- **HIGH: Security/Data Exposure** — Richer summaries of traces, worker outputs, and transcripts might inadvertently leak secrets, API keys, or sensitive prompt contexts if they are aggregated and shipped to the UI without explicit redaction or sanitization.
- **MEDIUM: Performance / I/O Blocking** — Aggregating multiple file-backed JSON artifacts on the fly (receipts, traces, active states) could introduce blocking I/O or memory bloat, especially for very long-running or deeply nested multi-agent loops.
- **MEDIUM: Error Handling** — The plan does not specify how to handle corrupted, partially written, or locked state files. This is a common edge case for live active-run tracking where the engine is actively writing to the JSON file while the UI attempts to read it.

**Suggestions**
- Explicitly add redaction and sanitization requirements for the typed summaries to prevent secret leakage.
- Ensure the summary helpers handle missing files, partial reads, or deserialization errors gracefully (e.g., returning partial summaries) without panicking the orchestration runtime.
- Consider truncating massive traces or bounding the number of events loaded into memory to protect the runtime.

---

### 12-02-PLAN.md

**Summary**
This plan wires the new typed backend summaries into the HTTP control endpoints (`start.rs`) and the Control UI dashboard (`control_ui.html`). It effectively closes the operator visibility gap by swapping raw JSON dumps for structured, human-readable rendering.

**Strengths**
- Correctly sequenced to depend on `12-01`, ensuring the typed runtime contracts are in place before wiring the UI.
- Specifically targets `control_ui.rs` to add dashboard contract tests, guaranteeing that the new supervision panels will be caught by CI if they drift from the runtime state.

**Concerns**
- **MEDIUM: UI Performance / Payload Size** — Rendering massive tables of worker outcomes, full delegation graphs, or deep active-run event tails in a single HTML payload might bloat the endpoint response and degrade browser performance.
- **LOW: UI Error States** — The plan doesn't mention how the UI should degrade if the backend endpoint returns a partial summary or an error due to concurrent file writes.

**Suggestions**
- Implement explicit limits, pagination, or truncation for the active-run event lists and worker outcome tables directly in the UI or endpoint.
- Define explicit UI empty/error states for when an orchestration run has no clear active state or if the summary fails to generate cleanly.

---

### 12-03-PLAN.md

**Summary**
This plan ensures the new supervision capabilities are truthfully documented in the repository and closes the phase out with the required `VERIFICATION.md` artifact. It aligns perfectly with the project's strict documentation and lifecycle completion contracts.

**Strengths**
- Fully adheres to the OpenRustClaw "truth over gloss" mandate by insisting the docs describe actual shipped parity.
- Properly gated behind the successful execution of both backend and frontend plans (Wave 2).

**Concerns**
- **LOW: Missing Operational Limits** — If performance mitigations (like truncation or event limits) are added to 12-01/12-02, the documentation might omit them, leading to operator confusion when they don't see their entire massive trace.

**Suggestions**
- Ensure the updated `README.md` and `feature-matrix.md` explicitly mention any operational boundaries or limits on the supervision summaries (e.g., "displays the last 100 active events").

---

### Overall Risk Assessment

**Overall Risk Level: MEDIUM**

**Justification:** 
The architectural approach is very sound. Reusing existing orchestration ledgers and simply aggregating them into a typed representation avoids scope creep and honors the "brownfield pragmatism" constraint. However, combining multiple I/O operations to build richer summaries introduces moderate risks around performance and data exposure. If the runtime blocks while parsing massive traces, if it crashes on a partially written live file, or if sensitive context is piped directly to the dashboard without filtering, it could violate the product's trust and reliability mandates. 

Adding explicit constraints for payload bounding, error handling on partial file reads, and sensitive data redaction will easily lower this to a **LOW** risk delivery.

---

## Claude Review

# Cross-AI Review: Phase 12 — Multi-Agent Supervision Parity

## 12-01: Add richer typed supervision summaries to orchestration runtime

### Summary
Solid foundational plan that extends existing orchestration types with operator-readable supervision summaries. The scope is well-contained to a single file and builds on established patterns.

### Strengths
- Builds on existing `OrchestrationRunRecord` and `ActiveOrchestrationRun` types rather than inventing new infrastructure
- Clear separation between receipt (completed) and active-run (live) supervision
- Preserves existing approval/resource visibility rather than replacing it

### Concerns
- **MEDIUM** — No schema definition for the typed supervision reports. "At the agent's discretion" from CONTEXT.md is flexible but risks producing summaries that don't compose well with Plan 02's UI rendering. A sketch of the key fields would reduce integration friction.
- **LOW** — No mention of backward compatibility. If existing consumers of `read_run_supervision()` depend on the current shape, adding richer types could break them unless the new summaries are additive.
- **LOW** — Verification says "run focused orchestration unit tests" but doesn't specify whether new tests are written or existing ones are extended.

### Suggestions
- Include a minimal field list for both summary types so Plan 02 can design UI in parallel
- Clarify whether existing supervision helpers are extended or new ones are added alongside them

### Risk Assessment
**LOW** — Single-file change extending established patterns with typed helpers. The main risk is schema mismatch with Plan 02, mitigated by the sequential dependency.

---

## 12-02: Surface richer supervision in runtime APIs and Control UI

### Summary
Reasonable wiring plan that connects Plan 01's typed summaries to runtime endpoints and dashboard rendering. The dependency on 12-01 is correct and the file scope is appropriate.

### Strengths
- Explicit requirement that UI is backed by typed runtime data, not frontend-only derivation
- Dashboard contract tests in `control_ui.rs` lock the new panels against regression
- Touches exactly the three files that need updating — no scope creep

### Concerns
- **MEDIUM** — `start.rs` is called out in CLAUDE.md as a "compatibility-heavy surface" and a "very large subsystem file." The plan doesn't acknowledge the complexity of editing this file or mention strategies for keeping the change bounded within it.
- **MEDIUM** — No mention of error states in the UI. What does the dashboard show when supervision data is missing, partially available, or from a run that predates the new summary types? Older receipts won't have the richer fields.
- **LOW** — No mention of whether new endpoints are added or existing ones are extended. New endpoints would need route registration; extended ones need response-shape compatibility.

### Suggestions
- Add a task or note about graceful degradation for receipts/runs that lack the new supervision fields
- Mention the specific route paths being added or modified so the scope in `start.rs` is predictable

### Risk Assessment
**MEDIUM** — The `start.rs` edit carries inherent risk due to file size and complexity. Dashboard contract tests mitigate UI regression but the plan would benefit from acknowledging the backward-compatibility surface for older orchestration data.

---

## 12-03: Align supervision docs and preserve verification

### Summary
Standard documentation and verification closeout plan. Appropriate as a Wave 2 dependency on both prior plans.

### Strengths
- Correctly depends on both 12-01 and 12-02 completing first
- Verification artifact preserves evidence per the established lifecycle contract
- Targets the right doc files (README, feature-matrix) without over-documenting

### Concerns
- **LOW** — `README.md` changes for a supervision detail upgrade may be too granular for the repo entrypoint. A one-line mention pointing to feature-matrix or docs may be sufficient.
- **LOW** — No mention of updating the ADR or architecture docs if the supervision model is a meaningful new abstraction.

### Suggestions
- Keep README changes minimal — a pointer rather than detailed supervision description
- Consider whether `docs/src/` (the docs book) also needs a supervision section, not just `feature-matrix.md`

### Risk Assessment
**LOW** — Documentation-only plan with no code risk. The main concern is doc scope calibration.

---

## Overall Phase Assessment

**Overall Risk: LOW-MEDIUM**

The phase is well-scoped and builds incrementally on existing infrastructure. The three-plan sequence (types → wiring → docs) follows established project patterns. The primary risks are:

1. Schema coordination between Plans 01 and 02 (mitigated by sequential dependency)
2. Editing `start.rs` safely given its size and compatibility surface
3. Graceful handling of pre-existing orchestration data that lacks the new supervision fields

The phase goals — making delegated-run supervision operator-readable without inventing new policy or execution infrastructure — are achievable with the proposed plans.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW-MEDIUM.
