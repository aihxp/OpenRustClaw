---
phase: 115
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:02:42.354Z
plans_reviewed: [115-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 115

## Gemini Review

# Plan Review: 115-01-PLAN.md

## 1. Summary

The proposed plan is exceptionally brief and operates at a very high level of abstraction. While it correctly identifies the goal of migrating control and runtime CLI entrypoints to native delivery modules via `ControlPlanePort` and `RuntimeOperationsPort`, it lacks the necessary implementation details to be actionable. It serves more as a restatement of the phase objectives rather than a concrete technical plan, omitting critical information such as which specific commands are affected, file paths, testing strategies, and error-handling mechanisms.

## 2. Strengths

*   **Strategic Alignment:** The plan accurately reflects the domain context and implementation decisions, specifically targeting the transition away from legacy command hubs.
*   **Clear Boundaries:** It explicitly references the correct architectural boundaries (`ControlPlanePort` and `RuntimeOperationsPort`) for the migration.
*   **Compatibility Awareness:** Acknowledges the need for compatibility shims and explicitly states the constraint that they must remain bounded and not leak orchestration logic.

## 3. Concerns

*   **[HIGH] Lack of Actionable Detail:** The plan does not list which specific CLI commands (e.g., `start`, `stop`, `status`, `config`) are being migrated. A developer would need to perform full discovery to execute this plan.
*   **[HIGH] Missing File Targets:** There are no source (legacy) or destination (native) file paths or module names specified, making it impossible to review the structural impact of the changes.
*   **[MEDIUM] Undefined Verification Strategy:** "Verify that compatibility shims... stay bounded" is vague. There is no concrete definition of how this will be verified (e.g., automated architectural tests, manual review guidelines, specific module visibility rules).
*   **[MEDIUM] Missing Error Handling & I/O:** The plan does not address how CLI output formatting (e.g., standard out, standard error, JSON output vs. human-readable text) or error handling will be bridged between the legacy CLI interface and the new native ports.
*   **[LOW] No Testing Plan:** There is no mention of migrating existing tests or writing new tests for the new native delivery modules.

## 4. Suggestions

*   **Enumerate Commands:** Explicitly list every CLI command that falls under "control CLI entrypoints" and "runtime CLI entrypoints" to define the exact scope of the migration.
*   **Specify File Paths:** Provide the expected directory structure and file names for the new native delivery modules (e.g., `crates/cli/src/delivery/control.rs`).
*   **Define Shim Mechanics:** Detail exactly how the compatibility shims will be structured to guarantee they do not "regain orchestration ownership." Consider using Rust's module visibility (`pub(crate)`) to enforce these boundaries.
*   **Include Testing and I/O:** Add explicit steps for migrating or creating tests (unit and E2E) and define how the native delivery modules will handle formatting results for the terminal.
*   **Define Rollback/Failure State:** Briefly outline what happens if a command fails to route through the new port during the transition phase.

## 5. Risk Assessment

**HIGH**

**Justification:** While the architectural intent is safe and correct, the plan itself poses a high execution risk due to its extreme ambiguity. It pushes all design and discovery work onto the executor during the implementation phase. Without specifying the exact commands, files, and testing strategies, there is a high likelihood of scope creep, missed edge cases, or inconsistent implementations across different CLI commands. The plan needs to be significantly expanded before it can be safely executed.

---

## Claude Review

The review is complete above. The key takeaway: the plan's direction is correct but it's too abstract — it needs file paths, subcommand inventories, and testable verification criteria before it's executable.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=HIGH.
