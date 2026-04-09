---
phase: 67
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T18:48:30.098Z
plans_reviewed: [67-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 67

## Gemini Review

# Plan Review: 67-01-PLAN.md

## 1. Summary
The plan outlines a direct, concise approach to migrating the skill registry mutation logic (install, update, uninstall) from the CLI layer (`skills.rs`) into a dedicated application service within `openrustclaw-app`. It correctly identifies the need for an adapter pattern in the CLI to interact with this new service, aligning perfectly with the phase's goal of establishing a stable greenfield boundary for mutation operations. However, the plan is extremely high-level and lacks architectural specifics regarding how side-effects and external dependencies will be managed across the new boundary.

## 2. Strengths
- **Clear Objective Alignment**: Directly addresses the phase goal of extracting mutation orchestration into `openrustclaw-app` while maintaining `skills.rs` as the adapter.
- **Focused Scope**: Limits the scope strictly to install, update, and uninstall operations, avoiding the temptation to rewrite the entire `skills.rs` file at once.
- **Test-Driven Verification**: Explicitly includes verification steps with specific test targets for both the core service and the CLI data flow to ensure the mutation contract is preserved.

## 3. Concerns
- **Missing Interface Definition [HIGH]**: The context notes that this lane involves "registry calls, policy checks, DB writes, event publication, compile attempts." The plan does not define how these heavy side-effects will be provided to the `openrustclaw-app` service. If they are not abstracted via traits or dependency injection, the app layer will become tightly coupled to CLI or infrastructure concerns.
- **Error Handling Ambiguity [MEDIUM]**: There is no mention of how complex failures (e.g., registry network timeouts, compilation errors, DB transaction aborts) will be modeled, mapped, and bubbled up through the new service seam to preserve the existing CLI and Control API error contracts.
- **Policy Checks Overlooked [MEDIUM]**: The context explicitly mentions "policy checks," but the steps do not clarify whether these rules will be evaluated inside the new app service (recommended for domain logic) or left in the adapter layer.
- **Rollback/Failure State [LOW]**: Mutation operations (like an install failing halfway through compilation) require cleanup. The plan doesn't specify how partial state is handled in the new service.

## 4. Suggestions
- **Define Dependencies**: Expand Step 1 to explicitly design the traits/interfaces required by the new service to perform side effects (e.g., `SkillRegistry`, `SkillStore`, `EventPublisher`, `SkillCompiler`).
- **Clarify Adapter Role**: Expand Step 2 to explicitly state that `skills.rs` will implement these dependency traits and inject them into the `openrustclaw-app` service.
- **Explicit Error Modeling**: Add a sub-step detailing the creation of domain-specific error types in `openrustclaw-app` and how they will map back to the existing user-facing CLI errors.
- **Incorporate Policy Checks**: explicitly document that policy enforcement will be moved into the new service to ensure both CLI and control API callers are subjected to the same rules.

## 5. Risk Assessment
**MEDIUM**

**Justification**: While the conceptual direction is absolutely correct and aligns with previous greenfield extractions, the extraction of a mutation-heavy lane involving multiple complex side-effects (DB, external registry, system compiler, async events) is structurally risky. The lack of architectural detail on how these dependencies will be inverted could lead to a leaky abstraction, a circular dependency nightmare, or a stalled migration. Defining the dependency injection strategy and error mapping upfront would reduce this risk to **LOW**.

---

## Claude Review

The review is complete above. The plan is **directionally correct** but needs more implementation detail — specifically service API signatures, side-effect inventory, and error-path coverage — to execute cleanly at MEDIUM risk.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
