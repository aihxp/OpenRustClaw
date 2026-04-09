---
phase: 114
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T20:01:30.497Z
plans_reviewed: [114-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 114

## Gemini Review

Here is the review of the implementation plan for Phase 114.

### Plan 114-01 Review

**Summary**
The plan correctly identifies the necessary architectural mappings to move the core assistant, chat, session, and inspect CLI commands to the new native delivery layer using application ports. However, it acts more as a high-level restatement of the phase goals rather than an actionable, step-by-step engineering plan. It severely lacks concrete implementation details, testing strategies, and error-handling mechanisms required for execution.

**Strengths**
*   **Architectural Alignment:** Correctly targets the "Ports and Adapters" (hexagonal) architecture established in the greenfield conversion by explicitly mapping CLI actions to specific domain ports (`AssistantConversationPort`, etc.).
*   **Clear Boundary Definition:** Step 3 explicitly highlights the most critical risk of this phase—preventing business logic (orchestration) from leaking back into the CLI parsing and rendering layer.

**Concerns**
*   **HIGH: Lack of Implementation Details.** The plan provides no concrete guidance on where files should be created (e.g., `crates/cli/src/commands/...`), how the CLI framework (presumably `clap`) will be structured, or how dependencies (the Ports) will be injected into the command handlers.
*   **HIGH: Missing Verification/Testing Strategy.** While Step 3 says to "Verify that parsing and rendering remain edge concerns," it does not specify *how*. There is no mention of writing unit tests with mocked ports to enforce this boundary.
*   **MEDIUM: Unaddressed Error Handling.** The plan does not explain how domain errors returned by the Ports will be caught, formatted, and translated into appropriate CLI exit codes and user-facing error messages.
*   **MEDIUM: Output Rendering.** There is no mention of how the CLI will handle standard output formatting (e.g., JSON vs. plain text, markdown rendering in the terminal) which is a significant part of the CLI delivery layer's responsibility.
*   **LOW: Integration with Main Entrypoint.** The plan doesn't mention how these new native delivery modules will be wired into the primary binary entrypoint.

**Suggestions**
*   **Flesh out file structure and wiring:** Specify the exact files to be created or modified (e.g., `crates/cli/src/commands/session.rs`). Detail how dependency injection will be handled to pass the required Ports to the CLI command runners.
*   **Define Error Mapping:** Add a specific step to implement an error translation layer that maps domain/port errors into semantic CLI errors and exit codes.
*   **Formalize Verification:** Rewrite Step 3 to explicitly require unit tests for the CLI handlers. The tests should pass mocked versions of the Ports to prove that the CLI layer only handles arguments and output, deferring all logic to the mock.
*   **Define Output Contracts:** Specify how the CLI should format the data returned from the Ports (e.g., using a dedicated presentation struct or existing display traits).

**Risk Assessment**
**HIGH**. The plan in its current state is far too abstract. Handing this plan to an autonomous agent or an engineer will result in significant guesswork, which heavily increases the risk of inconsistent implementation, poor error handling, or accidental violation of the very architectural boundaries this phase is trying to enforce. The plan needs to be broken down into concrete, actionable coding steps before execution.

---

## Claude Review

The review is complete above. The plan is sound but underspecified — the main actionable gap is naming concrete file paths and defining what "verification" actually means in testable terms.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.
