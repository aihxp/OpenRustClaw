---
phase: 37
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:46:33.142Z
plans_reviewed: [37-01-PLAN.md, 37-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 37

## Gemini Review

Here is a review of the implementation plans for Phase 37.

### 1. Summary
The plans effectively divide the phase into two logical steps: defining the cleanup inventory and no-touch boundaries (37-01), and then scheduling the concrete follow-up work into subsequent phases (37-02). By explicitly deferring code movement and focusing solely on mapping the brownfield state, the plans align perfectly with the phase goal of creating a safe, prioritized refactoring contract. However, the plans are quite brief and could benefit from more rigorous methodology and verification steps to ensure the resulting inventory is comprehensive and actionable.

### 2. Strengths
* **Strict Scope Adherence:** The plans resist the temptation to start refactoring immediately, strictly adhering to the "inventory first" mandate.
* **Clear Separation of Concerns:** Plan 37-01 focuses on *what* needs to be cleaned (the artifact), while Plan 37-02 focuses on *when* it will happen (the roadmap).
* **Concrete Verification:** The inclusion of specific shell commands (`test`, `rg`) for verification makes the completion criteria objective and scriptable.
* **Alignment with Context:** The plans directly address the specific targets mentioned in the decisions context (e.g., `start.rs`, `ci.yml`, `sidecar`).

### 3. Concerns
* **[MEDIUM] Lack of Discovery Methodology:** Plan 37-01 says "Review current codebase concerns and drift signals," but doesn't specify *how*. Relying only on known issues (like `start.rs`) might miss other oversized or deprecated surfaces. There is no mention of using code analysis tools (e.g., `tokei`, line-counting) or explicitly reading the existing `.planning/codebase/CONCERNS.md`.
* **[MEDIUM] Incomplete Verification in 37-02:** Plan 37-02 Step 3 explicitly states "Update roadmap and requirement status", but the verification steps only check for the existence of the context file and grep `CLEANUP.md`. There is no verification that `ROADMAP.md` or the milestone tracker was actually updated.
* **[LOW] Undefined "No-Touch" Criteria:** The plans don't specify how a "no-touch" boundary is determined. Without a clear heuristic (e.g., enterprise auth paths, core cryptography), the inventory might be subjective.

### 4. Suggestions
* **Enhance 37-01 with Empirical Discovery:** Update Plan 37-01 to explicitly include reading `.planning/codebase/CONCERNS.md` and running basic static analysis (e.g., finding files over a certain line count) to ensure the inventory is data-driven, not just based on top-of-mind memory.
* **Define an Inventory Schema:** In Plan 37-01, define a required structure for `CLEANUP.md` to ensure it hits all Success Criteria. For example, mandate sections for: `[Canonical Surfaces]`, `[Oversized Hubs]`, `[Deprecated/Generated]`, `[Cleanup Targets]`, and `[No-Touch Boundaries]`.
* **Update 37-02 Verification:** Add a verification step in Plan 37-02 to explicitly check the `ROADMAP.md` (or `.planning/MILESTONES.md`) to ensure Phases 38, 39, and 40 have been formally scheduled and described. For example: `rg -n "Phase 38" .planning/ROADMAP.md`.
* **Establish Boundary Rules:** Briefly define the criteria for "no-touch boundaries" in the `CLEANUP.md` file (e.g., "Do not modify enterprise auth flows unless explicitly scoped in a dedicated security phase").

### 5. Risk Assessment
**LOW**
Because this phase consists entirely of documentation and roadmap planning, the technical risk to the codebase is virtually zero. The only risk is organizational: producing an incomplete or overly vague `CLEANUP.md` that fails to adequately protect high-risk boundaries during actual code movement in Phases 38-40. Implementing the suggestions above will mitigate this minor risk.

---

## Claude Review

The review is complete above. Both plans are low-risk documentation deliverables that appropriately set up the cleanup contract before any code movement begins. The one actionable suggestion across both: add a workspace-member enumeration step so the inventory doesn't silently miss smaller crates.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
