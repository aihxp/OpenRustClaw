---
phase: 93
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:28:46.165Z
plans_reviewed: [93-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 93

## Gemini Review

# Review of Plan 93-01: Orchestration Request Routing, Override Validation, and Lifecycle-State Transition Services

## Summary
Plan 93-01 outlines a conceptually sound but highly abstract approach to decoupling orchestration business logic from the CLI adapter layer (`orchestrate.rs`). It correctly identifies the boundary between core domain concerns (routing, autonomy overrides, state transitions) and infrastructure concerns (I/O, persistence). However, the plan lacks technical depth regarding error handling, concurrency, and exactly how the new `openrustclaw-app` service will manage complex state transitions without leaking persistence details.

## Strengths
*   **Clear Architectural Boundaries:** Correctly isolates domain logic (routing, validation) from infrastructure concerns (file I/O, config loading, background workers), adhering to clean architecture principles.
*   **Targeted Scope:** Focuses strictly on moving existing logic rather than rewriting the orchestration engine, minimizing the risk of scope creep.
*   **Focused Verification:** Explicitly plans for both app-unit coverage for the new service and targeted command tests to ensure the CLI adapter integrates correctly.

## Concerns
*   **Missing Error Handling Strategy (HIGH):** The plan does not specify how orchestration routing failures, invalid autonomy overrides, or illegal state transitions will be surfaced from the app service back to the CLI layer.
*   **Concurrency and Race Conditions (MEDIUM):** Moving lifecycle-state transitions to the app service while leaving "active-run persistence" in the adapter layer could create race conditions if state changes occur rapidly or asynchronously. The plan doesn't address how the app service will coordinate with the persistence layer safely.
*   **Opaque Validation Rules (MEDIUM):** "Autonomy override validation" is mentioned, but there is no detail on how the service will access the necessary context (e.g., operator permissions, system limits) to perform this validation without coupling to the config layer.
*   **Lack of Rollback/Failure Recovery (LOW):** There is no mention of how the system handles a failed state transition or an interrupted routing request.

## Suggestions
*   **Define the Error Contract:** Explicitly design the domain error types that the new orchestration service will return (e.g., `InvalidTransition`, `OverrideDenied`, `RouteNotFound`) and how the CLI will map these to user-facing output.
*   **Clarify Persistence Coordination:** Specify a clear interface or callback mechanism for the app service to trigger persistence updates in the adapter layer, ensuring state transitions and saves are atomic or safely coordinated.
*   **Detail the Validation Context:** Define the input structure for the `override validation` helper. Ensure it accepts pure data structs (context, permissions) rather than requiring the service to fetch configuration itself.
*   **Elaborate on Testing:** Expand Step 3 to explicitly include testing of negative paths: invalid overrides, illegal state transitions (e.g., jumping from `pending` directly to `completed` without `running`), and missing routes.

## Risk Assessment
**Risk Level: MEDIUM**

**Justification:** While the architectural intent is solid and aligns with the goal of moving business logic into `openrustclaw-app`, the lack of implementation detail introduces moderate risk. Separating state transition logic from the persistence mechanism (which remains in the adapter) is a common source of subtle bugs and race conditions. Without a defined strategy for error propagation and state synchronization, the execution of this plan could lead to an unreliable orchestration lifecycle. Fleshing out the interface contracts before implementation will mitigate this risk to LOW.

---

## Claude Review

The review is complete above. The plan is serviceable but needs more specificity around the extraction surface, state machine definition, and security invariant preservation before implementation.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.
