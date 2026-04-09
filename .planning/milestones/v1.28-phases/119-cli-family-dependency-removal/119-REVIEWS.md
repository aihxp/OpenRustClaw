---
phase: 119
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:07:22.085Z
plans_reviewed: [119-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 119

## Gemini Review

Here is the review of the provided implementation plan.

### Summary

Plan 119-01 correctly identifies the core architectural goal of Phase 119—eliminating command-to-command orchestration in favor of native delivery modules communicating through app ports. However, the plan is currently too abstract to be actionable. It reads more like a restatement of the phase's context and requirements rather than a concrete technical roadmap. It lacks specific file paths, code examples, architectural patterns, and a clear execution strategy for how these dependency removals will actually be implemented in the codebase.

### Strengths

*   **Goal Alignment:** Accurately captures the primary directive to treat command-to-command routing as an architectural smell.
*   **Risk Awareness:** Explicitly calls out the need to ensure compatibility shims do not accidentally become a new hidden orchestration layer.
*   **Architectural Clarity:** Correctly identifies that shared behavior should be routed through app ports or explicit delivery helpers.

### Concerns

*   **HIGH:** **Lack of Actionability.** The plan provides no concrete steps. Telling a developer to "State which command-to-command dependencies must be removed" is a planning step, not an implementation step. The plan itself needs to identify these targets.
*   **HIGH:** **Missing Scope definition.** The plan does not specify *which* CLI families are in scope for this specific phase (e.g., `crates/cli/src/commands/...`).
*   **MEDIUM:** **No Migration Strategy.** There is no defined pattern for how to safely untangle an existing command-to-command dependency without breaking the legacy compatibility layer during the transition.
*   **MEDIUM:** **Vague Verification.** "Verify that compatibility remains possible" is not a testable criteria. There are no steps defining how to prove the refactor was successful (e.g., unit tests, integration tests matching legacy output).

### Suggestions

*   **Identify Target Files:** Update Step 1 to explicitly list the legacy command files (e.g., within `crates/cli/`) that currently exhibit command-to-command orchestration and are targeted for this phase.
*   **Define the Target Architecture:** Provide a concrete example in the plan of how a coupled command will be refactored. Show how shared logic will be extracted into `crates/app/` (or similar) and how the native delivery module will call that port directly.
*   **Draft a Step-by-Step Migration Path:** Break down the work into safe, verifiable chunks. For example: 1. Extract shared logic to app port. 2. Update Command A to use the port. 3. Update Command B to use the port. 4. Remove the routing link between Command A and B.
*   **Specify Testing Requirements:** Add explicit steps to run existing CLI integration tests to guarantee that the external behavior of the CLI remains identical despite the internal structural changes.

### Risk Assessment

**HIGH**

**Justification:** The plan in its current state cannot be handed off to an engineer or an execution agent because it lacks technical specifics. Proceeding with this plan guarantees scope creep, inconsistent implementation, and a high likelihood of regressions, as the "how" is left entirely up to the implementer at execution time. The plan must be revised to include concrete codebase targets and refactoring patterns.

---

## Claude Review

Here is my review:

---

# Cross-AI Review: Phase 119 — CLI Family Dependency Removal

## Review of Plan 119-01

### Summary

Plan 119-01 is a lightweight architectural governance plan that establishes rules for decoupling CLI command families from internal command-to-command orchestration. The direction is sound — native delivery modules should own their flows via app ports rather than routing through legacy command files. However, the plan is extremely thin: three high-level steps with no concrete inventory of which dependencies exist, no migration strategy, and no verification criteria beyond "verify compatibility remains possible." For a phase touching a core architectural boundary, this lacks the specificity needed for confident execution.

### Strengths

- Correctly identifies command-to-command orchestration as an architectural smell blocking clean native delivery
- Aligns with the project's greenfield conversion trajectory and existing app-port abstraction strategy
- Keeps compatibility shims bounded to forwarding/translation, preventing them from becoming a new hidden orchestration layer
- Scope is appropriately narrow — defines rules rather than attempting a full rewrite

### Concerns

- **HIGH** — No inventory of existing command-to-command dependencies. Without knowing which commands call which, the "removal rules" are abstract policy with no actionable targets. The plan should enumerate the actual dependency graph (e.g., `start.rs` calling into `runtime.rs` helpers, `skills.rs` invoking setup flows).
- **HIGH** — No verification criteria. Step 3 says "verify that compatibility remains possible" but defines no test, grep pattern, CI check, or architectural fitness function to enforce the rule going forward.
- **MEDIUM** — No definition of "app ports" or "delivery helpers" in context. The plan assumes the reader knows what these abstractions are and where they live. Cross-references to the relevant crate paths and trait definitions would make execution unambiguous.
- **MEDIUM** — No ordering or phasing. If multiple command families have cross-dependencies, removal order matters. Breaking a dependency that another family still relies on mid-migration could cause compilation failures.
- **LOW** — No rollback or incremental migration strategy. If a dependency removal breaks a compatibility shim, there's no described path to recover without reverting.

### Suggestions

- Add a **dependency inventory step** before defining removal rules: grep for cross-command `use` imports and function calls within `crates/cli/src/commands/` to produce a concrete list of what needs severing.
- Define **explicit verification**: at minimum, a `cargo check` pass confirming no command module imports another command module (enforceable via a CI lint or grep-based check).
- Reference the specific app-port traits and delivery-helper modules that command families should route through instead, with file paths.
- Add a **migration order** based on the dependency graph — leaf commands (no inbound callers) first, hub commands (many callers) last.
- Include a **VERIFICATION.md template** consistent with v1.1+ milestone requirements.

### Risk Assessment

**MEDIUM** — The architectural direction is correct and the scope is bounded, but the plan's lack of concrete inventory and verification criteria means execution will require significant interpretation. The risk is not that the wrong thing gets built, but that the rules get defined abstractly without enforcement, leaving the same orchestration patterns free to re-emerge in future work.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=HIGH, claude=MEDIUM.
