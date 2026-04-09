---
phase: 8
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:14:39.942Z
plans_reviewed: [08-01-PLAN.md, 08-02-PLAN.md, 08-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 8

## Gemini Review

Here is the review of the implementation plans for Phase 08.

# Cross-AI Plan Review: Phase 08 - Verification Artifact Contract

## 1. Summary

The proposed plans for Phase 08 are well-structured, tightly scoped, and directly address the goal of enforcing the verification artifact contract. The work is logically sequenced: it begins by establishing shared verification inspection logic and hard lifecycle gates (Plan 01), proceeds to expose verification debt in audit and reporting surfaces (Plan 02), and concludes by aligning the scaffolding tools, updating documentation, and fulfilling the contract for Phase 08 itself (Plan 03). This approach ensures consistency between enforcement and reporting without introducing scope creep.

## 2. Strengths

*   **Excellent Separation of Concerns:** The division into core enforcement (Plan 01), reporting/audit (Plan 02), and scaffolding/closure (Plan 03) prevents massive, tangled commits and makes the work easily verifiable.
*   **Strong Reuse Strategy:** Introducing `verification-artifacts.cjs` as a single source of truth for readiness inspection directly addresses the historical drift between `init`, `phase complete`, and `audit-uat`.
*   **Self-Referential Proof:** Plan 03 explicitly requires writing Phase 8's own `08-VERIFICATION.md` artifact, forcing the new contract to immediately validate itself before the phase can close.
*   **Clear Documentation Alignment:** Every code change is paired with a corresponding update to the workflow or help documentation, ensuring the operator experience matches the new constraints.

## 3. Concerns

*   **MEDIUM: Ambiguity in "Staleness" Definition.** Decision D-05 states stale means the verification artifact is "older than the latest execution evidence in the phase directory, especially summaries and UAT artifacts." The plans do not specify exactly which files constitute "execution evidence." Relying on arbitrary file modification times in a directory can be brittle (e.g., touching a file or modifying a `.plan` file might falsely flag the verification as stale).
*   **LOW: Inspector Error Handling.** The plans do not specify how the shared inspector (`verification-artifacts.cjs`) should handle malformed `VERIFICATION.md` files (e.g., missing frontmatter, unparseable YAML) or completely absent phase directories (which is a valid state for roadmap-only milestones).
*   **LOW: Bootstrapping Phase 8's Completion.** In Plan 03, the `VERIFICATION.md` file must be physically written to disk *before* any command invoking the new `phase complete` gate is run. If the implementation attempts to complete the phase before the file is fully flushed, the newly implemented hard gate will block the completion of its own phase.

## 4. Suggestions

*   **Define "Execution Evidence" Explicitly:** In Plan 01, explicitly define the glob patterns or specific filenames that trigger the staleness check (e.g., `*PLAN.md`, `*SUMMARY.md`, `*UAT.md`). Avoid checking the modification time of the directory itself or unrelated files.
*   **Robust Inspector Fallbacks:** Ensure the shared inspector in Plan 01 gracefully handles unparseable verification files by returning a specific `malformed` or `gaps_found` state rather than throwing an unhandled exception. It should also cleanly return `missing` if the phase directory doesn't exist yet.
*   **Explicit Execution Order in Plan 03:** Update the task description in Plan 03 to explicitly state that the generation of `08-VERIFICATION.md` must be the absolute final step, and it must be completed before triggering any milestone/phase completion hooks.
*   **Add Schema Validation to Inspector:** Since Plan 03 aligns scaffolding to the active schema, Plan 01's inspector should ideally perform a lightweight schema validation check to ensure operators don't bypass the gate with an empty or incorrectly formatted `VERIFICATION.md`.

## 5. Risk Assessment

**LOW**

**Justification:** The proposed changes are entirely isolated to the `.codex/get-shit-done/` lifecycle tooling and do not impact the OpenRustClaw application runtime, build process, or public distribution. The phased rollout minimizes the risk of breaking existing operator workflows, and the scope is strictly limited to enforcing a contract that was already intended but loosely applied. As long as the "staleness" timestamp comparison is implemented deterministically, the risk of regressions is minimal.

---

## Claude Review

# Cross-AI Review: Phase 8 — Verification Artifact Contract

## 08-01: Harden the core verification lifecycle contract

### Summary
Solid foundational plan that introduces a shared verification inspector and wires it into the two most critical lifecycle gates (bootstrap and phase-complete). The scope is tight and the dependency chain is clean.

### Strengths
- Single shared inspector avoids rule duplication across commands
- Fixes a real correctness bug: milestone bootstrap undercounting phases
- Temp-workspace CLI probes are the right verification strategy for tooling changes
- Clear separation between gating (hard block) and reporting (deferred to 08-02)

### Concerns
- **MEDIUM** — No definition of "stale" is given in the plan itself. The CONTEXT.md mentions "older than latest execution evidence" but the plan should pin this down before implementation, since the inspector's freshness logic is load-bearing for all downstream consumers.
- **LOW** — The plan doesn't mention backward compatibility for existing milestones whose verification artifacts predate the new schema. If the inspector runs against archived phases during audit, it could produce false negatives.
- **LOW** — No explicit error message contract. When `phase complete` blocks, the operator-facing output should be specific enough to act on (which file, what status, what to do next). Worth a one-liner in the tasks.

### Suggestions
- Add a task or sub-bullet defining the freshness heuristic concretely (mtime comparison? frontmatter timestamp vs. summary timestamp?)
- Specify that the inspector returns structured data (not just a boolean) so 08-02 can consume categories without re-parsing

### Risk Assessment
**LOW** — Well-scoped, addresses a real gap, and the verification approach (temp-workspace probes) is appropriate for CLI tooling.

---

## 08-02: Surface verification readiness debt before lifecycle steps

### Summary
Natural follow-on that extends the shared inspector into audit output. The scope is appropriate — reporting only, no new gates. The docs tasks are proportional.

### Strengths
- Reuses the inspector from 08-01 rather than building parallel logic
- Distinguishes debt categories (missing, stale, pending, gaps_found) instead of a binary pass/fail
- Updates both operator-facing docs and help text, not just code

### Concerns
- **MEDIUM** — The plan modifies `uat.cjs` but doesn't mention whether existing audit consumers (scripts, CI, or operator muscle memory) expect a stable output format. Adding new debt categories could break downstream parsing if anyone consumes audit output programmatically.
- **LOW** — `progress.md` and `help.md` are listed but the plan doesn't scope how much rewriting is needed. If these files are large, the docs tasks could balloon. Worth confirming they're small targeted edits.
- **LOW** — No mention of how `human_needed` verification status renders in audit output. CONTEXT.md calls it "a routed state, not an automatic pass" — the audit should surface it distinctly from blocking states.

### Suggestions
- Confirm audit output format stability or note that it's internal-only
- Explicitly include `human_needed` as a reported category in audit output, even if it's non-blocking

### Risk Assessment
**LOW** — Reporting-only changes with clear dependency on 08-01. Minimal blast radius.

---

## 08-03: Align verification scaffolds, workflow guidance, and phase evidence

### Summary
Closes the loop by fixing the scaffold command's output format and eating its own dogfood (Phase 8 produces its own verification artifact under the new contract). Good design choice to make the phase self-proving.

### Strengths
- Self-referential verification: the phase proves its own contract works by using it
- Fixes a real drift bug where `scaffold verification` emits outdated frontmatter
- Dependency on both 08-01 and 08-02 is correct — scaffolds should match what the inspector validates

### Concerns
- **MEDIUM** — The plan says "parse against the declared verification schema" but doesn't specify where the schema is canonically defined. If `template.cjs` and `frontmatter.cjs` are both sources of truth, the scaffold fix needs to pick one. This should be explicit.
- **LOW** — Writing the Phase 8 verification artifact is listed as a docs task, but it's really a lifecycle task that must happen last and must pass the inspector from 08-01. The ordering within the plan is implicit rather than stated.

### Suggestions
- Name the canonical schema source (template.cjs? frontmatter.cjs? both?) so the scaffold alignment has a single target
- Note that the Phase 8 verification artifact must itself pass the `phase complete` gate introduced in 08-01 — this is the real acceptance test

### Risk Assessment
**LOW** — Clean closure plan. The self-proving aspect is the strongest design choice in the entire phase.

---

## Overall Phase Assessment

**Overall Risk: LOW**

The three plans form a clean dependency chain (inspector → reporting → scaffold alignment + self-proof) with no circular dependencies or scope creep. The phase addresses a genuine lifecycle integrity gap rather than speculative tooling.

**One cross-cutting note:** The freshness definition ("stale") is the single most important design decision across all three plans, and it's currently underspecified. Pinning it down in 08-01 before implementation will prevent rework in 08-02 and 08-03.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed the phase is tightly scoped around a shared verification inspector and a self-proving closure path, but they emphasized different strengths.

### Agreed Concerns
- Both reviewers highlighted the need to define verification freshness or staleness more explicitly before implementation.

### Divergent Views
- Reviewers converged on an overall LOW risk posture.
