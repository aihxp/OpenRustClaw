---
phase: 88
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:22:57.683Z
plans_reviewed: [88-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 88

## Gemini Review

# Plan Review: 88-01-PLAN.md

## Summary
The plan provides a high-level, conceptually correct sketch for refactoring the route registration logic in `start.rs`. It correctly targets the extraction of route-builder helpers for `control_plane_router` and `runtime_control_router` to reduce sprawl, aligning perfectly with the phase goals. However, the plan is extremely sparse, lacking specific technical details, module boundaries, and a step-by-step approach to safely untangle the oversized inline route chain without causing regressions.

## Strengths
- **Goal Alignment:** Directly addresses the phase objective of reducing shared route registration and state wiring sprawl.
- **Safety Boundary:** Explicitly adheres to the decision to keep the cleanup compatibility-preserving (restructuring only, no handler contract changes).
- **Automated Verification:** Includes a specific integration test command to verify the changes structurally and behaviorally.

## Concerns
- **[HIGH] Lack of Technical Specificity:** The plan does not identify which specific route families are being extracted, what the new helper functions/modules will be named, or where they will reside in the directory structure (e.g., `crates/cli/src/routes/...`). 
- **[HIGH] Monolithic Execution Risk:** Refactoring a massive central router in a single step is highly prone to errors. The plan lacks an iterative approach (e.g., extracting one route family at a time).
- **[MEDIUM] Incomplete Testing Scope:** Relying solely on a single new test (`cleaned_up_route_registration_preserves_migrated_route_families`) is insufficient. A central router refactor demands that the *entire* existing test suite passes to guarantee no unintended un-wiring occurred.
- **[LOW] Missing Shared State Details:** The plan mentions reducing "shared-state sprawl" in the objective but the steps don't detail how the state dependency injection will be handled or simplified in the new helpers.

## Suggestions
- **Detail the Extraction Targets:** Explicitly list the route families that will be moved (e.g., control-plane routes, non-voice skill-control routes, voice/channel runtime skill routes) and define their new target files/modules.
- **Iterative Steps:** Break Step 1 into sub-steps, extracting and wiring one route family at a time to isolate failures and make reviews easier.
- **Broaden Verification:** Add a mandatory `cargo test --workspace` step to the verification block to ensure no existing routing tests or handler integrations regress.
- **Define State Handling:** Briefly specify how the extracted route builders will accept shared state (e.g., passing a unified `AppState` struct vs. individual dependencies).

## Risk Assessment
**MEDIUM**
While the conceptual risk is low because this is a pure structural refactoring (no business logic changes), the execution risk is moderate to high due to the lack of detail in the plan. Modifying the central application router without a meticulous, step-by-step migration strategy increases the likelihood of dropped routes, incorrect state wiring, and regressions. Fleshing out the specific module targets and enforcing full-suite test verification will lower this risk significantly.

---

## Claude Review

The review is complete above. The plan is **low risk** and directionally sound but would benefit from naming concrete files/functions and broadening the verification gate beyond a single test.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
