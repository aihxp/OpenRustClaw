---
phase: 91
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:26:54.335Z
plans_reviewed: [91-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 91

## Gemini Review

# Review of Plan 91-01: Voice Provider Resolution and Session Lifecycle Services

## Summary
The plan outlines a focused structural refactoring to decouple voice session lifecycle management and provider resolution from the concrete voice adapter implementation (`voice_runtime.rs`). By introducing an app-level service to manage the state machine while retaining I/O operations (synthesis, file persistence) in the adapter layer, it correctly addresses the phase goal of removing command-local orchestration dependencies. The approach is sound, though it requires careful handling of state consistency across the new boundary.

## Strengths
*   **Clear Architectural Boundary:** Correctly separates the business logic (lifecycle state machine, provider resolution) from the infrastructure concerns (I/O, file persistence, synthesis).
*   **Comprehensive Lifecycle Scope:** Explicitly enumerates the necessary state transitions (start, append, respond, reconnect, pause, resume, interrupt, health, reap, prewarm shaping) to be handled by the new service.
*   **Contract Preservation:** Adheres to the decision to preserve existing request and response contracts, which will minimize cascading changes in consuming code.
*   **Targeted Verification:** Includes a specific step for app tests and compilation checks to validate the extraction.

## Concerns
*   **[MEDIUM] State Consistency and Atomicity:** Splitting the lifecycle state mutations (app layer) from session persistence (adapter layer) creates a risk of split-brain state. If the app service transitions a session to "paused" but the adapter fails to persist this state or halt I/O, the system will become inconsistent.
*   **[MEDIUM] Concurrency and Race Conditions:** Voice operations (like `interrupt` or `pause` arriving while an `append` or `respond` is processing) are highly asynchronous. Moving the state machine to a higher-level service requires explicit concurrency control (e.g., locks, channels) to ensure safe mutations.
*   **[LOW] Error Propagation:** The plan does not detail how I/O or synthesis errors originating in the adapter will be surfaced to and handled by the new app service to trigger appropriate lifecycle state changes (e.g., moving to an error or disconnected state).
*   **[LOW] Testing Depth:** While app tests and CLI compilation are mentioned, the plan lacks explicit integration testing to ensure the new app service and the modified adapter communicate correctly under simulated load or failure conditions.

## Suggestions
*   **Define Concurrency Strategy:** Explicitly state how the new app-side lifecycle service will handle concurrent mutation requests (e.g., using `tokio::sync::RwLock`, an actor model, or message passing) to prevent race conditions during interrupts or disconnects.
*   **Establish Consistency Guarantees:** Ensure the interface between the app service and the adapter allows for atomic updates or provides clear rollback mechanisms if an adapter persistence operation fails after a state transition is approved.
*   **Explicit Error Handling:** Add a sub-step to map adapter-level I/O and synthesis errors into domain-level lifecycle events, ensuring the state machine can react gracefully to hardware or network failures.
*   **Expand Verification Scope:** Update the verification step to include integration tests that specifically target the interaction seam between the new `lifecycle_service` and the `voice_runtime` adapter, simulating scenarios like mid-stream interruptions.

## Risk Assessment
**MEDIUM**

The architectural intent is correct and aligns with the project's greenfield conversion goals. However, decoupling a highly asynchronous state machine (voice lifecycle) from its underlying persistence and I/O mechanics introduces non-trivial risks around concurrency and state synchronization. If the interface between the app layer and the adapter isn't carefully designed to handle failures and concurrent interrupts, it could lead to hanging sessions or inconsistent voice behavior. Addressing the concurrency and consistency concerns in the implementation steps will mitigate this risk.

---

## Claude Review

# Cross-AI Review: Phase 91-01 — Voice Provider Resolution and Session Lifecycle Mutation Services

## Summary

A straightforward extraction plan that moves provider-resolution logic and session lifecycle state mutations from the CLI adapter (`voice_runtime.rs`) into `openrustclaw-app`, following the same greenfield conversion pattern used across dozens of prior phases. The plan is concise and correctly scoped, but light on detail regarding the state machine's edge cases and the contract boundary between the new service and the adapter.

## Strengths

- **Clear separation of concerns**: Provider resolution and session state transitions are genuine business logic that belongs in `openrustclaw-app`, not in a CLI command adapter.
- **Preserves existing contracts**: Explicitly commits to keeping synthesis, metadata probing, and session persistence in the adapter — avoids scope creep into I/O-heavy code.
- **Consistent with project patterns**: Follows the same extract-to-app, delegate-from-adapter pattern validated in ~30 prior greenfield phases.
- **Verification included**: Plans for both app-level unit tests and a clean CLI compile check.

## Concerns

- **LOW — State machine transitions undefined**: The plan lists 11 lifecycle operations (start, append, respond, reconnect, pause, resume, interrupt, health, reap, prewarm) but doesn't specify valid state transitions or what happens on invalid transitions. If the adapter currently enforces ordering implicitly, that logic could be lost or duplicated.
- **LOW — Provider resolution rules unspecified**: "Provider-resolution rules" are mentioned but not described. If resolution involves fallback chains, feature-gated providers, or credential lookup, the plan should note which parts move and which stay.
- **LOW — Error propagation boundary**: No mention of how errors from the new service propagate back through the adapter to the caller. The adapter presumably maps app errors to voice-runtime response types — this should be explicit.
- **LOW — No rollback consideration**: If the extraction breaks voice runtime in a way that's hard to detect (e.g., subtle ordering change), there's no mention of how to verify behavioral equivalence beyond "clean compile."

## Suggestions

- Document the valid state transitions for the session lifecycle, even as a comment in the new service. This prevents the state machine from becoming implicit again.
- Specify which provider-resolution rules move: is it just "pick provider by config key," or does it include fallback logic, credential validation, or capability matching?
- Add at least one test that exercises an invalid state transition (e.g., `resume` before `pause`) to confirm the service rejects it.
- Consider a brief note on whether the adapter retains any state of its own or becomes fully stateless after extraction.

## Risk Assessment

**LOW**. This is a well-bounded extraction following a proven pattern in this codebase. The operations being moved are logic-only (no I/O, no persistence), which minimizes the blast radius. The main risk is subtle behavioral drift in state transition ordering, but given that the voice runtime surface is likely not the most exercised path, this is manageable with the suggested transition tests.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=MEDIUM, claude=LOW.
