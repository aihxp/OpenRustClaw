---
phase: 27
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:30:16.515Z
plans_reviewed: [27-01-PLAN.md, 27-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 27

## Gemini Review

# Phase 27 Plan Review: Self-Hosted Product Surface Alignment

## Summary
The plans for Phase 27 are well-scoped, pragmatic, and directly address the phase goal of aligning documentation and UI control surfaces with the newly implemented self-hosted product modes. By strictly limiting the scope to textual updates and UI copy alignment, the plans successfully adhere to the project's constraint of not inventing new lifecycle features during this phase. The separation of documentation updates (27-01) and UI copy updates (27-02) provides a clear execution path.

## Strengths
* **Strict Scope Management:** The plans explicitly avoid scope creep (such as introducing pricing or commercial packaging) and focus entirely on reflecting the already-shipped capabilities.
* **Clear Tracing to Requirements:** The plans directly address the `SURF-01` requirement and the success criteria for truthful representation of the product modes.
* **Targeted Files:** The identified files (`README.md`, `installation.md`, `control_ui.html`, etc.) are exactly the right places to make these changes based on the codebase context.
* **Test Verification:** Plan 27-02 includes a specific, executable `cargo test` command to verify that the UI changes do not break existing dashboard assertions.

## Concerns
* **[MEDIUM] Manual Verification for Documentation:** Plan 27-01 relies entirely on "manual review" for verification. Given the project's maturity and the presence of `docs/book.toml`, relying solely on manual review risks introducing broken internal links or formatting errors that could break the documentation build.
* **[LOW] Hardcoded Terminology Risk:** If the UI updates in `control_ui.html` (Plan 27-02) hardcode the textual descriptions of the modes (solo, team, company, enterprise) rather than relying on the backend state/enums, it could lead to future drift between the backend definitions and the frontend display.
* **[LOW] Verification Artifact Contract:** While Plan 27-02 mentions "explicit verification evidence", it does not explicitly state the creation of a `VERIFICATION.md` file, which has been a strict requirement since milestone v1.1.

## Suggestions
* **Automate Doc Checks (27-01):** Add a verification step to Plan 27-01 to run the documentation build (e.g., `mdbook build docs/`) and a markdown link checker to ensure no broken links are introduced during the rewrite.
* **Source of Truth for Copy (27-02):** Ensure that `control_ui.rs` passes the deployment path names to the HTML template using the existing typed runtime summaries, rather than hardcoding the text directly into `control_ui.html`.
* **Explicit Artifact Generation (27-02):** Explicitly add "Generate `VERIFICATION.md` documenting the alignment between the updated docs and the UI" to the verification steps of Plan 27-02 to adhere to project standards.

## Risk Assessment
**LOW**

The overall risk is low because the modifications are limited to static documentation files and HTML/UI template copy. No core orchestration, routing, or state mutation logic is being altered. As long as the existing dashboard tests pass and the documentation builds successfully, these plans are safe to execute.

---

## Claude Review

The review is complete above. Both plans are low-risk, well-scoped documentation and copy alignment tasks that directly serve the phase goals.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
