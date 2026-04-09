---
phase: 14
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:02:09.158Z
plans_reviewed: [14-01-PLAN.md, 14-02-PLAN.md, 14-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 14

## Gemini Review

Here is the structured review of the Phase 14 implementation plans based on the provided project context and decisions.

### 1. Summary
The plans correctly identify the goal of Phase 14: upgrading the Control UI by replacing raw JSON dumps with typed renderers backed by existing APIs. The phase is well-structured into sequential execution batches followed by a dedicated documentation and verification plan. The plans successfully adhere to the project's architectural constraints by strictly targeting the frontend renderer (`control_ui.html`) and its contract tests (`control_ui.rs`) without attempting to invent new backend routes or state management. However, the division of work between Plan 01 and Plan 02 is overly vague, lacking specific target assignments for the high-value panes identified in the context.

### 2. Strengths
- **Architectural Discipline:** The plans strictly adhere to D-02 and D-07, targeting only `control_ui.html` and `control_ui.rs` to consume existing typed routes, completely avoiding backend scope creep.
- **Lifecycle Adherence:** Plan 14-03 perfectly aligns with the project's strict documentation and verification lifecycle, ensuring that parity claims in the `README.md` and `docs/feature-matrix.md` are backed by the `14-VERIFICATION.md` artifact.
- **Dependency Ordering:** The sequential dependency (14-01 -> 14-02 -> 14-03) is correct and prevents race conditions when modifying the exact same target files across multiple batches.

### 3. Concerns
- **HIGH - Ambiguous Scoping:** Plans 14-01 and 14-02 rely on vague definitions ("first batch" and "second batch"). The context explicitly identifies the highest-value targets: *voice session detail, mobile command/conflict/message detail, talk session detail, and skill detail*. Leaving the exact targets unspecified risks an autonomous agent missing critical surfaces or creating overlapping work.
- **MEDIUM - Test Verification Vagueness:** The verification steps mention running "focused dashboard contract tests" but fail to define what constitutes a passing test for this specific phase (e.g., asserting the removal of raw JSON `setText(...)` calls for the targeted payloads).

### 4. Suggestions
- **Explicit Target Allocation:** Update Plan 14-01 to explicitly name its targets in the Task and Objective sections (e.g., "Target 1: Voice session detail and Talk session detail panes").
- **Explicit Target Allocation:** Update Plan 14-02 to explicitly name the remaining targets (e.g., "Target 2: Mobile command/conflict/message detail and Skill detail panes").
- **Sharpen Verification Criteria:** In Plans 14-01 and 14-02, update the Verification sections to explicitly require asserting that `setText(...)` is no longer used to dump raw JSON for the upgraded payloads, and that the new typed renderers (tables/summaries) are correctly populated.
- **Concrete Documentation Rules:** In Plan 14-03, specify that the `README.md` and `feature-matrix.md` updates must explicitly list the newly upgraded typed surfaces rather than just claiming "improved parity."

### 5. Risk Assessment
**Risk Level: MEDIUM**
**Justification:** The architectural approach is highly safe and respects the project's boundaries (LOW risk of breaking the backend or violating system constraints). However, the execution risk is MEDIUM because an autonomous agent executing these vague "batch" instructions is likely to guess the boundaries incorrectly, potentially missing the specific operator-critical surfaces called out in the phase context. Adding explicit lists to the tasks will lower this risk to LOW.

---

## Claude Review

# Cross-AI Review: Phase 14 — Control UI Surface Completion

## 14-01: Upgrade the first batch of high-value raw detail panes

### Summary
A reasonable starting plan that targets the most impactful raw-JSON panes for typed rendering. However, it is notably vague about *which* panes will be upgraded, leaving scope entirely to implementer discretion with no prioritized inventory.

### Strengths
- Correctly identifies raw JSON as the primary parity gap
- Constrains work to existing typed payloads rather than inventing new backend routes
- Includes contract test coverage requirement

### Concerns
- **HIGH**: No explicit list of which panes are in "batch 1." The plan says "highest-value" but never enumerates them. This makes scope unverifiable — you can't confirm success criteria if you don't know what was targeted.
- **MEDIUM**: Single task block covers what could be 5-10 distinct renderer upgrades. If one pane is complex (e.g., voice session detail with nested timeline data), it could block the entire plan.
- **LOW**: No fallback for panes where existing typed payloads are insufficient — the plan assumes all target panes already have adequate backend data, but doesn't verify this.

### Suggestions
- Add a concrete inventory task that lists the `setText(...)` call sites, groups them by operator value, and commits the ranked list before coding starts
- Break the single task into per-pane or per-group subtasks so progress is incremental
- Add a pre-check: confirm the existing API response shape is rich enough for each target pane before building the renderer

### Risk Assessment
**MEDIUM** — The plan will produce real improvements but the lack of a concrete scope inventory makes it hard to verify completion or detect scope drift.

---

## 14-02: Extend typed rendering to remaining priority panes

### Summary
A continuation plan that mirrors 14-01's structure for a second batch. It inherits all of 14-01's vagueness problems and adds no new specificity about what distinguishes "batch 2" from "batch 1."

### Strengths
- Correctly depends on 14-01, preserving ordering
- Reuses established helper patterns (`renderTableRows`, etc.) rather than introducing new abstractions
- Goal of cross-surface consistency is the right framing

### Concerns
- **HIGH**: Same inventory gap as 14-01 — no enumeration of which panes are in this batch. The boundary between 14-01 and 14-02 is entirely implicit.
- **MEDIUM**: "Feels more consistent" is a subjective success criterion. There's no measurable definition of when the dashboard crosses the consistency threshold.
- **LOW**: If 14-01 runs long, 14-02's scope may shrink to near-nothing or balloon to compensate, since neither plan has a fixed target list.

### Suggestions
- Define the batch split upfront in 14-01's inventory task — batch 1 = these panes, batch 2 = those panes
- Replace "feels more consistent" with a concrete metric: e.g., "≤N remaining `setText(...)` calls on parity-critical panes" or "all panes listed in the inventory are upgraded"
- Consider merging 14-01 and 14-02 into a single plan with two waves, since they touch the same two files and the artificial split adds planning overhead without adding clarity

### Risk Assessment
**MEDIUM** — Functional but redundant with 14-01. The two-plan split is justified only if the batch boundary is explicit, which it currently isn't.

---

## 14-03: Align Control UI docs and preserve verification

### Summary
A clean docs-and-verification closeout plan. Straightforward and well-scoped relative to the implementation plans.

### Strengths
- Correctly gates on both 14-01 and 14-02
- Requires truthful documentation tied to actual shipped renderers, not aspirational claims
- Preserves the project's verification artifact lifecycle contract

### Concerns
- **MEDIUM**: `README.md` and `docs/feature-matrix.md` changes are mentioned but the plan gives no guidance on *what* to say — if the implementation plans lack a concrete pane inventory, the docs plan can't reference specific improvements either.
- **LOW**: No mention of updating `control_ui.html` inline comments or the dashboard's own help/about content, which operators may also read.

### Suggestions
- Make the verification artifact template explicit: list the fields it must contain (panes upgraded, test results, before/after evidence)
- Tie documentation content to the inventory from 14-01 so the docs commit references specific named surfaces

### Risk Assessment
**LOW** — This is the least risky plan. Its quality depends entirely on the implementation plans delivering a clear, enumerable set of changes.

---

## Overall Phase Assessment

**Overall Risk: MEDIUM**

The phase goal is sound and the constraint of working within existing typed APIs is the right call. The primary structural weakness is that **no plan contains a concrete inventory of target panes**. All three plans defer scope to implementer discretion, which means:

1. Success criteria are unverifiable without post-hoc interpretation
2. The 14-01 / 14-02 split has no defined boundary
3. Documentation and verification can't reference specific deliverables

**Top recommendation**: Add a preliminary inventory step (or a dedicated short plan) that audits all remaining `setText(...)` raw-detail call sites in `control_ui.html`, ranks them, and commits the ranked list. Everything else flows from that list.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: claude=MEDIUM.
