---
phase: 18
requested_reviewers: [gemini, claude]
reviewers: [gemini]
reviewed_at: 2026-04-09T17:18:43.977Z
plans_reviewed: [18-01-PLAN.md, 18-02-PLAN.md, 18-03-PLAN.md]
partial_review: true
reviewer_errors: {"claude":"spawnSync claude ETIMEDOUT"}
---
# Cross-AI Plan Review — Phase 18

## Gemini Review

Here is a structured review of the plans for Phase 18: Supervised Autonomy Escalation and Rollback.

### 1. Summary

The plans for Phase 18 outline a clear, focused, and additive approach to introducing explicit supervised-autonomy lifecycle states. By breaking the work into three logical steps—data model/persistence (18-01), API/UI exposure (18-02), and documentation/closure (18-03)—the plans ensure a safe, layered integration. The approach correctly aligns with the architectural constraint to avoid inventing a new workflow engine, instead opting to enhance the existing `ActiveOrchestrationRun` and supervision reporting mechanics. The scope is well-bounded, explicitly deferring multi-step automated rollbacks in favor of capturing operator intent and durable evidence.

### 2. Strengths

*   **Layered Delivery:** Separating the underlying state machinery (18-01) from the control routes and UI rendering (18-02) reduces integration risk and makes testing more straightforward.
*   **Architectural Alignment:** The plans reuse the existing `.claw/control/orchestration-active/` persistence layer and `SupervisionSummary` models, preventing parallel systems and reducing complexity.
*   **Scope Discipline:** The plans adhere strictly to the CONTEXT.md decisions by keeping "rollback" as an explicit lifecycle marker/state rather than attempting complex, multi-system state reversions.
*   **Auditability Focus:** Explicitly calling out the need for "structured intervention decision records" ensures that operator actions (escalate, resume, rollback, kill) are preserved as durable evidence, satisfying requirement AUTO-04.

### 3. Concerns

*   **Undefined Rollback Semantics (HIGH):** While the context notes that rollback means "an explicit operator-supervised lifecycle transition with durable rationale... rather than silently mutating unrelated runtime state," the implementation plan (18-01) does not explicitly define what the orchestration engine should actually *do* when a rollback is triggered. If it acts similarly to "kill" but with a different status, this needs to be explicitly stated to prevent developers from over-engineering a functional reversion system.
*   **Concurrency and Race Conditions (MEDIUM):** Active orchestration runs are dynamic. Introducing new operator-driven state transitions (like escalation or rollback) requires robust handling of concurrent file writes to the snapshot in `.claw/control/orchestration-active/`. The plans do not mention file locking or atomic update strategies for these new state changes.
*   **Audit Log Mutability (MEDIUM):** If intervention evidence is simply stored as a field on a single, overwritten JSON snapshot, there is a risk of losing history if multiple interventions occur. The evidence should ideally be append-only or stored in an array of historical decisions within the snapshot.
*   **State Machine Validation (LOW):** The plans do not explicitly mention validating state transitions (e.g., ensuring an operator cannot "resume" a run that has already been "rolled back" or "killed").

### 4. Suggestions

*   **Clarify Rollback Mechanics in 18-01:** Update Plan 18-01 to explicitly define the operational behavior of a rollback (e.g., "A rollback transitions the run to a terminal `RolledBack` state, halts execution similar to `kill`, and requires an attached operator rationale string. It does NOT attempt to revert file system or external state.").
*   **Specify Evidence Structure in 18-01:** explicitly mandate that intervention evidence is stored as a chronologically appended list (e.g., `Vec<InterventionRecord>`) within the active run snapshot, rather than a single mutable field, to ensure complete auditability.
*   **Enforce Transition Rules in 18-01/18-02:** Add a requirement to implement a strict state transition function (e.g., checking that valid transitions are `Running -> Paused -> Escalated -> RolledBack`) to prevent illegal state mutations via the new control routes.
*   **Security Check in 18-02:** Explicitly note in Plan 18-02 that the new escalation and rollback control routes must be registered behind the existing control-plane authentication/authorization middleware.

### 5. Risk Assessment

**Risk Level: LOW**

**Justification:** The overall risk is low because the phase is purely additive and builds upon an already stable orchestration supervision foundation. The decision to treat "rollback" as a lifecycle state marker rather than a complex functional reversion eliminates the most significant technical risk associated with this feature. As long as concurrent state updates are handled cleanly and the state machine transitions are validated, these plans are highly likely to succeed and fulfill the AUTO-03 and AUTO-04 requirements without destabilizing the core runtime.

---

## Consensus Summary

### Review Coverage
- Partial review run: completed reviewers `gemini`; failed reviewers `claude`.

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available, and no explicit overall risk label was parsed from the completed review.
