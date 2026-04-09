---
phase: 86
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:20:25.822Z
plans_reviewed: [86-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 86

## Gemini Review

Here is the review of Plan 86-01:

### Summary
The plan correctly identifies the scope and goals of Phase 86 by targeting the extraction of non-voice-call skill-control routes (compile, background-service, invoke, execute) into `openrustclaw-app`. It adheres strictly to the boundaries established in the context decisions. However, the plan is exceedingly brief and lacks the technical depth required to guide a safe, verifiable refactoring of this scale. It reads more like a restatement of the requirements than an actionable implementation plan.

### Strengths
*   **Strict Scope Adherence:** Accurately targets the specific routes mentioned in the context (compile, compiled-detail, background-service, invoke, execute) while implicitly leaving out voice/channel routes.
*   **Clear Architectural Target:** Correctly identifies the destination (`openrustclaw-app`) and the role of the remaining code (`start.rs` as purely an HTTP adapter).
*   **Targeted Verification:** The verification commands are specific and align perfectly with the requirement to use a deterministic workspace skill fixture through the runtime control router.

### Concerns
*   **[HIGH] Lack of Interface Definition:** The plan states it will "Finalize the request and service boundary" but provides zero details on what the `SkillControlService` trait/struct, its input models, or its return types will look like.
*   **[HIGH] Missing Error Handling Strategy:** Moving orchestration out of `start.rs` means application-level errors must now be bubbled up and mapped to HTTP status codes. The plan completely ignores how error translation will be handled across this new boundary.
*   **[MEDIUM] Vague Fixture Setup:** While the verification commands are good, the plan doesn't explain how the "workspace-local skill fixture" will be implemented, provisioned, or what specific edge cases (e.g., compile failures, execution timeouts) it needs to cover.
*   **[MEDIUM] Implicit Cleanup:** The plan does not explicitly instruct the developer to delete the legacy orchestration code in `start.rs`, which is required to fulfill the success criteria of stopping reliance on route-local orchestration.

### Suggestions
*   **Define the Boundary:** Detail the specific API of the `SkillControlService`. List the method signatures (e.g., `async fn execute_skill(...) -> Result<SkillOutput, SkillError>`) that will be added to `openrustclaw-app`.
*   **Detail Error Mapping:** Add a sub-step explaining how `SkillError` variants from the app layer will be pattern-matched in `start.rs` to return appropriate HTTP 4xx/5xx status codes and operator-result records.
*   **Flesh out the Test Fixture:** Describe what the workspace skill fixture actually entails (e.g., a dummy script that echoes input, a script that intentionally fails to compile) to ensure the tests validate both happy and sad paths.
*   **Explicit Deletion Step:** Add a step explicitly stating that once the routes are re-wired to the service lane, the legacy command helper invocations previously living in `start.rs` must be removed.

### Risk Assessment
**MEDIUM**

The risk is medium because while the boundaries are safely constrained (ignoring Phase 87 scope), the plan itself leaves too much open to interpretation. The lack of defined data models, error handling strategies, and concrete testing scenarios means the implementer will have to make significant architectural decisions on the fly, increasing the likelihood of bugs, inconsistent HTTP responses, or an incomplete refactoring. Adding technical depth to the steps will mitigate this risk.

---

## Claude Review

The review is complete above. The plan is solid — low risk, well-scoped, follows established patterns. The main actionable feedback is to explicitly enumerate the exact route handlers in scope and clarify the request model shape.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
