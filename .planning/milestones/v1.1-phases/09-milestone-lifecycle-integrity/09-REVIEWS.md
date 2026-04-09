---
phase: 9
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:24:29.068Z
plans_reviewed: [09-01-PLAN.md, 09-02-PLAN.md, 09-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 9

## Gemini Review

# Cross-AI Plan Review: Phase 9 Milestone Lifecycle Integrity

## 1. Summary
The phase 9 plans provide a clear, logical, and tightly scoped approach to solving the milestone lifecycle integrity gap. By decoupling the preservation of verification evidence from the physical retention of phase directories, the plans ensure that historical milestone data remains continuously auditable. The sequencing is highly practical—starting with the core mechanism to generate the archive, followed by aligning workflow documentation, and concluding with cleanup semantics and self-verification.

## 2. Strengths
- **Logical Sequencing**: The rollout builds predictably. Foundational code changes (Plan 01) precede documentation updates (Plan 02), which precede final alignment and self-verification (Plan 03).
- **Minimal Intrusion**: The approach leverages existing logic (e.g., `cmdMilestoneComplete()` and Phase 8's verification inspection) instead of re-architecting the system or building heavy new standalone CLI commands.
- **Self-Enforcing Integrity**: Plan 03 explicitly includes the creation of Phase 9's own `VERIFICATION.md` file, proving the contract immediately upon completion.
- **Clear Separation of Concerns**: The distinction between raw phase directory archiving (which can happen later) and immediate milestone verification snapshotting is cleanly delineated.

## 3. Concerns
- **Error Handling During Evidence Collection (MEDIUM)**: Plan 09-01 states that the collector summarizes per-phase verification status, but it doesn't specify the behavior if a phase's verification artifact is malformed, unparseable, or unexpectedly missing during the run. Will it crash the milestone completion process, or gracefully mark it as debt?
- **Index Linkage (LOW)**: While Plan 09-01 mentions writing `vX.Y-VERIFICATIONS.md`, the plan doesn't explicitly mention linking this new artifact in the master `.planning/MILESTONES.md` index, though the context implies milestone completion updates it. 
- **Backward Compatibility (LOW)**: The deferred context states retrofitting v1.0 is out of scope. However, there's a minor risk that running new audit/cleanup commands on older milestones could surface unhandled exceptions if the new tooling strictly expects Phase 8/9 artifact formats.

## 4. Suggestions
- **Enhance Plan 09-01, Task 1**: Add explicit instructions to implement graceful fallbacks when parsing fails. If a phase lacks proper verification data, the tool should record it as "missing_verification" debt rather than throwing a fatal error.
- **Enhance Plan 09-01, Task 2**: Explicitly state that the newly created `vX.Y-VERIFICATIONS.md` file should be referenced/linked within the updated `MILESTONES.md` entry for easy operator discovery.
- **Enhance Plan 09-03, Task 1**: In the `cleanup.md` update, explicitly instruct the operator on *how* to review the verification debt after cleanup (e.g., pointing them directly to the `vX.Y-VERIFICATIONS.md` file path conventions).

## 5. Risk Assessment
**LOW**

**Justification**: The proposed work is restricted entirely to internal orchestration tools (Node/CJS scripts) and documentation (`.md` files). It poses zero risk to the shipped Rust product, application runtime, or core operator capabilities. The additions are additive (creating new summary files) rather than destructively modifying existing archived data. The overall risk is low and the return on maintainability and auditability is high.

---

## Claude Review

# Cross-AI Review: Phase 9 — Milestone Lifecycle Integrity

## Overall Assessment

Phase 9 is a focused, low-risk documentation and tooling phase that closes a real gap: milestone archives currently lose verification evidence. The three plans are well-scoped, correctly sequenced, and avoid overbuilding. The main risk is that the phase is so narrowly scoped to the GSD tooling layer (`.codex/` JavaScript files and workflow Markdown) that it has no direct impact on the Rust product itself — it's purely planning-infrastructure work.

---

## Plan 09-01: Archive milestone verification evidence durably

### Summary
Extends `milestone.cjs` to emit a `vX.Y-VERIFICATIONS.md` during milestone completion, using the Phase 8 verification-artifacts collector. Straightforward two-task plan with clear output.

### Strengths
- Reuses existing `verification-artifacts.cjs` rather than building new inspection logic
- Output format is plain Markdown — inspectable, diffable, no new tooling needed
- Clear success criteria tied to observable artifacts

### Concerns
- **MEDIUM** — No error handling specified. What happens if verification state is partially unavailable (e.g., a phase directory was already moved before milestone completion)? The plan should specify graceful degradation — emit what's available and flag gaps.
- **LOW** — The verification spec says "temp workspace" testing but doesn't describe how to set up that temp workspace or whether CI covers it. Manual-only verification for tooling changes is fragile.
- **LOW** — No schema or format contract for `vX.Y-VERIFICATIONS.md` is specified in the plan. The "Specific Ideas" section in CONTEXT.md sketches columns (phase, status, score, artifact path, debt notes) but the plan doesn't commit to a format.

### Suggestions
- Define the Markdown table schema explicitly in the plan so Plan 02's doc alignment has a concrete target
- Add a task or note for the degraded-state case (missing phases, partial verification data)

### Risk Assessment
**LOW** — Small, well-contained change to an internal tool with no production impact.

---

## Plan 09-02: Align audit and archive workflow guidance

### Summary
Pure documentation update across three workflow files to reference the new verification archive artifact consistently. Depends correctly on 09-01.

### Strengths
- Addresses a real coherence gap — docs currently describe different mental models of where evidence lives
- Correctly scoped to the three files that matter
- Each task has a focused verify/done gate

### Concerns
- **LOW** — The plan updates `help.md` but doesn't mention whether `help.md` is auto-generated or hand-maintained. If it's generated, the edit may be overwritten.
- **LOW** — No mention of whether existing operators need a migration note or changelog entry for the new archive artifact path.

### Suggestions
- Confirm `help.md` maintenance model before editing
- Consider a one-line note in the completion workflow about what changes for operators who already shipped milestones without the verification archive

### Risk Assessment
**LOW** — Documentation-only changes with no execution risk.

---

## Plan 09-03: Align cleanup semantics and close the phase with evidence

### Summary
Closes the loop by updating cleanup docs to stop implying destructive semantics, updating PROJECT.md, and writing Phase 9's own verification artifact. Depends correctly on both prior plans.

### Strengths
- Self-referentially consistent — the phase about verification evidence ships its own verification evidence
- PROJECT.md update ensures the baseline is recorded for future milestone planning
- Cleanup doc fix addresses a real semantic gap (cleanup ≠ evidence destruction)

### Concerns
- **MEDIUM** — The PROJECT.md update says "reflect that v1.1 now has a milestone-level verification archive baseline" but this is Phase 9 of v1.1. If the milestone version numbering has shifted (the project is now at v1.47), this wording may be stale or confusing. The plan should reference the correct milestone version.
- **LOW** — The verification task says "capture archive artifact evidence and lifecycle verification commands" but doesn't specify what commands to run or what output to capture. This makes the verification artifact potentially subjective.

### Suggestions
- Fix the milestone version reference to match the actual milestone this phase ships under
- Specify concrete verification commands (e.g., `gsd milestone complete --dry-run` output, file existence checks) in the verification artifact template

### Risk Assessment
**LOW** — Documentation and verification artifact only, no code risk.

---

## Cross-Cutting Observations

| Concern | Severity | Scope |
|---|---|---|
| No degradation behavior for partial/missing phase data during collection | MEDIUM | 09-01 |
| Stale milestone version reference in 09-03 PROJECT.md task | MEDIUM | 09-03 |
| No explicit Markdown schema for the verification archive | LOW | 09-01, 09-02 |
| Verification is manual-only with no CI coverage | LOW | 09-01 |
| No operator migration/changelog note | LOW | 09-02 |

## Final Verdict

**Overall Risk: LOW.** This is a clean, well-sequenced phase that delivers exactly what it promises. The two medium concerns (error handling for partial state, stale version reference) are worth addressing before execution but neither is blocking. The phase correctly avoids scope creep — it doesn't try to retrofit old milestones or build a new audit CLI, both of which were explicitly deferred.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
