---
phase: 97
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:32:59.312Z
plans_reviewed: [97-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 97

## Gemini Review

Here is the review for the proposed plan for Phase 97.

### 1. Summary

Plan 97-01 correctly identifies the architectural boundaries required by the phase context, accurately delegating pure business logic (step selection, repair derivation) to `openrustclaw-app` while retaining side-effect-heavy operations (I/O, probing) in the CLI adapter (`onboard.rs`). However, the plan is extremely high-level. Because onboarding and repair workflows inherently require a back-and-forth dance between probing the environment and making decisions, the lack of detail regarding the data flow and interface design between the core service and the adapter poses an execution risk.

### 2. Strengths

*   **Architectural Alignment:** Perfectly aligns with the stated domain decisions by keeping I/O, state persistence, and runtime probing strictly within the CLI adapter layer.
*   **Clear Separation of Concerns:** Clearly enumerates the specific responsibilities that belong in the new `openrustclaw-app` service (defaults, step selection, repair-plan helpers, handoff shaping).
*   **Safety First:** Explicitly aims to preserve the existing CLI and control-surface setup contracts, minimizing the blast radius of the refactor.

### 3. Concerns

*   **HIGH: Unclear Data Flow and Interface Design:** Onboarding and repair often require sequential probing and decision-making (e.g., probe environment -> decide next step -> execute step -> probe again). The plan does not specify how `onboard.rs` will communicate probe results to the pure `openrustclaw-app` service or how the service will instruct the CLI on what to persist.
*   **MEDIUM: Missing Error Handling Strategy:** Setup and repair are fragile processes susceptible to environmental failures. The plan does not address how the extracted service will model partial successes, failure states, or generate user-facing error guidance.
*   **MEDIUM: Vague Testing Approach:** "Verify with app coverage" is too generic. Because the app service is being divorced from the actual I/O, it needs a strictly defined data model or trait interface to allow for deterministic unit testing of the state machine logic.
*   **LOW: Granularity of Steps:** Step 1 and Step 2 are massive undertakings bundled into single sentences. Splitting the extraction into smaller, verifiable chunks (e.g., extracting "repair derivation" first, then "onboarding") would be safer.

### 4. Suggestions

*   **Define the Interface:** Update the plan to explicitly define the data structures (e.g., `EnvironmentProbeResult`, `SetupAction`, `RepairPlan`) that will cross the boundary between `onboard.rs` and the new `openrustclaw-app` service.
*   **Model the State Machine:** Suggest implementing the orchestration service as a state machine where the CLI feeds it environmental state, and the service returns the next required `SetupAction` (which the CLI then executes).
*   **Specify Error Handling:** Explicitly add a requirement to define an error enum in `openrustclaw-app` that covers all possible setup/repair failure modes so the CLI can render helpful diagnostics.
*   **Clarify Testing:** Specify that the new `openrustclaw-app` orchestration service must be exhaustively unit-tested using mocked/synthesized environment states to ensure all repair and setup branches are covered without touching a real filesystem.

### 5. Risk Assessment

**MEDIUM**

**Justification:** The architectural goal is sound and the intended boundary lines are correct. However, the plan's brevity obscures the complexity of decoupling highly stateful, I/O-dependent onboarding logic from pure business rules. Without a clearer design for how the CLI and the App service will exchange state and instructions, the developer risks getting bogged down in messy interface iterations or accidentally leaking I/O concerns back into the app layer. Detail is needed to make this safely executable.

---

## Claude Review

# Review: Phase 97-01 — Onboarding/Repair/Resume Orchestration Extraction

## Summary

This is a straightforward extract-and-delegate refactor moving setup orchestration logic from `onboard.rs` into `openrustclaw-app`. The plan is concise but underspecified — it describes the right architectural direction but lacks the detail needed to execute confidently, particularly around which specific functions move, what the service interface looks like, and how the adapter boundary is drawn.

## Strengths

- Correct separation: CLI keeps I/O and persistence, app layer owns business rules
- Consistent with the project's established greenfield migration pattern
- Scope is appropriately bounded — no feature additions, just extraction
- Preserves existing CLI and control-surface contracts

## Concerns

- **HIGH** — No interface sketch. "Setup lifecycle service" is named but not defined. What methods? What types flow across the boundary? Without this, implementation will rediscover the design during coding, risking inconsistency.
- **MEDIUM** — No inventory of what moves. `onboard.rs` likely contains interleaved I/O and logic. The plan doesn't identify which functions or code blocks are orchestration vs. adapter concerns, making the split ambiguous.
- **MEDIUM** — "Repair-plan helpers" and "handoff shaping" are vague. These could mean anything from a single enum match to a complex state machine. The plan should name the concrete repair/resume flows being extracted.
- **LOW** — "CLI library verification" is unclear. Does this mean `cargo test` on the CLI crate, an integration test that runs the onboard command, or something else?
- **LOW** — No mention of how existing tests (if any) on `onboard.rs` are handled during the split.

## Suggestions

- Read `onboard.rs` and list the specific functions/blocks that contain setup-step selection, repair-plan derivation, and handoff logic — include them in the plan as an extraction inventory.
- Define the service trait or struct interface (even 3-4 method signatures) so the boundary is concrete before coding starts.
- Specify whether the new service lives in an existing `openrustclaw-app` module or gets a new submodule, and name it.
- Clarify the verification step: unit tests on the extracted service, plus `cargo test -p openrustclaw-cli` to confirm no regression.

## Risk Assessment

**MEDIUM** — The direction is sound and the scope is contained, but the plan is too thin to execute without significant design-during-implementation. The main risk isn't doing the wrong thing — it's that the vague boundary will lead to an incomplete extraction or an awkward interface that needs rework. Adding an extraction inventory and interface sketch would drop this to LOW.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers converged on an overall MEDIUM risk posture.
