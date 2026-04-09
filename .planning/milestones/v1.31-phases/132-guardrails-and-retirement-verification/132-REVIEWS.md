---
phase: 132
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:18:41.787Z
plans_reviewed: [132-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 132

## Gemini Review

Here is the review of the proposed implementation plan for Phase 132.

### Summary
Plan 132-01 accurately reflects the strategic goals outlined in the phase context but is entirely too abstract to be an executable engineering plan. It describes the *outcomes* of the planning process ("Identify how...", "Define what...") rather than the actual technical steps required to implement the guardrails in the OpenRustClaw codebase. Without specifying the mechanisms for enforcement (e.g., CI scripts, Rust visibility rules, or linting configurations), this plan risks resulting in unenforced conventions rather than durable guardrails.

### Strengths
*   **Strategic Alignment:** The plan perfectly mirrors the stated user decisions, ensuring that the focus remains strictly on retirement verification and preventing regressions.
*   **Clear Boundaries:** It correctly identifies that legacy files must be stripped of ownership and isolated from the new native paths.
*   **Goal-Oriented:** The steps directly map to the requirements needed to advance the roadmap to the `7/8` state.

### Concerns
*   **[HIGH] Lack of Technical Specificity:** The steps are directives to plan, not directives to execute. Telling an engineer or an AI to "Identify how retired-file dependencies... should be verified" does not explain *how* to build the verification mechanism (e.g., using `cargo tree`, custom bash scripts like `check-repo-hygiene.sh`, or Rust module `pub(crate)` restrictions).
*   **[HIGH] Missing Automated Enforcement:** The plan does not explicitly require the creation of automated CI checks. If guardrails rely on manual review or documentation alone, the legacy code will inevitably leak back into the native execution path.
*   **[MEDIUM] Undefined Shim Contracts:** Step 1 mentions "shim behavior," but the plan doesn't define how a shim is allowed to interact with the legacy code versus the native code, nor how that boundary is technically enforced.
*   **[LOW] Missing File Targets:** The plan does not specify which directories or files represent the "retired delivery files" versus the new native paths, making it hard to scope the actual work.

### Suggestions
*   **Specify Enforcement Mechanisms:** Update Step 1 to explicitly define the tooling. For example: "Implement a CI script (e.g., augmenting `check-repo-hygiene.sh`) that fails the build if any module in `crates/*/src/native/` imports from `crates/*/src/legacy/`."
*   **Define Rust Module Visibility:** Add a step to use Rust's module system to enforce boundaries at compile time (e.g., ensuring legacy modules are not `pub` or are restricted using `pub(in crate::legacy)`).
*   **Automate the Regression Check:** Replace Step 3 with a concrete action: "Create a failing test case that attempts to violate the guardrail, verify the CI tooling catches it, and document the enforcement mechanism in `VERIFICATION.md`."
*   **Concrete Inventory:** Explicitly list the target directories that constitute the "retired files" so the scope of the guardrails is unambiguous.

### Risk Assessment
**HIGH**
The risk is high because the plan lacks actionable technical steps. If this plan is handed to an execution agent or developer, they will have to invent the implementation details from scratch. Without explicitly mandating automated, CI-enforced guardrails, the project risks silently regressing into a state where new native code depends on legacy modules, undermining the entire greenfield conversion effort.

---

## Claude Review

# Review: Phase 132-01 — Guardrails and Retirement Verification

## Summary

This is a thin governance/verification phase that defines rules for keeping retired legacy delivery files out of the native product path. The plan is intentionally scoped to policy definition rather than implementation, which is appropriate for a guardrails phase. However, the plan is extremely vague — it reads more like a goal statement than an actionable plan.

## Strengths

- Correctly scoped to verification rules rather than code changes
- Ties advancement (`7/8`) to explicit proof, not assumption
- Aligns with the project's pattern of requiring truthful verification artifacts

## Concerns

- **HIGH**: The three steps are abstract goal statements, not concrete actions. "Identify how retired-file dependencies should be verified" doesn't say *what* the verification mechanism is (CI check? grep-based lint? cargo dependency audit? manual checklist?). A plan should propose a mechanism, not just say "define one."
- **MEDIUM**: No list of which specific files are considered "retired." Without an explicit inventory, the guardrails have nothing concrete to guard. The plan should reference or produce a canonical retired-file manifest.
- **MEDIUM**: "Shim behavior" is mentioned but never defined. What shims exist? What are their allowed behaviors? What constitutes a violation? The plan assumes shared context that should be made explicit.
- **LOW**: Step 3 is a gate condition, not an implementation step. It belongs in a verification section, not in the plan body.

## Suggestions

- Replace step 1 with a concrete mechanism: e.g., "Add a CI deny-list that fails if any crate under `crates/app/` imports from retired modules" or "Create `RETIRED.md` manifest and a `scripts/check-retirement.sh` that greps for violations"
- Add an explicit list of retired files or reference the artifact that contains them (likely from Phase 131 or the v1.37 legacy retirement milestone)
- Define "shim" concretely — which files are shims, what they're allowed to re-export, and when they should be removed
- Move step 3 into a "Verification" section with a clear pass/fail criterion

## Risk Assessment

**MEDIUM** — The phase goal is sound and well-bounded, but the plan as written is too abstract to execute without significant interpretation. The risk isn't that something goes wrong — it's that the output will be another abstract policy document rather than enforceable guardrails. Tightening the plan to name specific files, specific mechanisms, and specific pass/fail criteria would drop this to LOW.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=HIGH, claude=MEDIUM.
