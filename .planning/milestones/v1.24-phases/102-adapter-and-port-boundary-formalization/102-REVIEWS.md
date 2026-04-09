---
phase: 102
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:38:42.087Z
plans_reviewed: [102-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 102

## Gemini Review

# Review of Plan 102-01: Adapter and Port Boundary Formalization

## Summary
The plan correctly identifies the architectural goal of separating business logic from infrastructure concerns by formalizing adapter boundaries for file system and compiled-skill MCP integrations. However, the plan is overly abstract and reads more like a set of architectural guidelines than an executable implementation plan. It fails to identify the specific files, modules, or structs that need to be refactored, making it difficult to execute or verify predictably.

## Strengths
* **Architectural Clarity:** Clearly articulates the strict boundary between application services (validation, reporting, orchestration) and adapters (workspace I/O, transport wiring).
* **Alignment with Goals:** Directly addresses the phase's core objective of replacing anonymous helper clusters with explicit named adapters.
* **Safety Focus:** Explicitly highlights the requirement to preserve existing shipped command and runtime contracts during the refactoring process.

## Concerns
* **HIGH: Complete Lack of Specificity.** The plan refers to "targeted hotspots" and "explicit adapter seams" without identifying *which* specific crates, modules, files, or structs are actually being modified or created.
* **HIGH: Missing Implementation Details.** There are no concrete steps on how to extract these boundaries. It doesn't define what the new traits (Ports) will look like, how the structs (Adapters) will implement them, or how the dependencies will be wired together.
* **MEDIUM: Vague Verification Strategy.** While step 3 mentions verifying that boundaries preserve contracts, it provides no testing strategy. It is unclear if this relies on existing E2E tests, requires new unit tests for the adapters, or involves manual CLI validation.
* **LOW: Scope Ambiguity.** It's unclear if "targeted hotspots" covers all remaining legacy code or a specific, bounded subset intended for this specific phase.

## Suggestions
* **Enumerate Targets:** Explicitly list the target files and modules containing the "surviving file-system and compiled-skill MCP boundaries" mentioned in the context.
* **Define Interfaces:** Draft the proposed Rust traits (Ports) that the application services will depend on, and the corresponding struct implementations (Adapters).
* **Detail the Wiring:** Add explicit steps explaining how the new adapters will be injected into the application services (e.g., passing trait objects or using generic type parameters).
* **Define the Test Plan:** Outline a concrete verification strategy, specifying which existing integration tests must pass, and detailing any new unit tests needed to cover the newly isolated adapter logic.

## Risk Assessment
**HIGH**

While the architectural intent is sound and aligns well with the project's greenfield conversion goals, the plan itself is too vague to be executed autonomously or predictably. Without identifying the specific code to be changed and outlining concrete implementation and testing steps, there is a high risk of scope creep, incomplete refactoring, or unintended regressions in the system's integration points. The plan needs to be revised into a concrete, step-by-step technical implementation guide.

---

## Claude Review

# Review: Phase 102-01 — Adapter and Port Boundary Formalization

## Summary

This plan is extremely thin — three high-level steps with no concrete file paths, no named hotspots, no specific adapter structs, and no verification criteria. It reads as a restated goal rather than an actionable implementation plan. Without knowing which files are being touched, what the adapter interfaces look like, or how success is measured, this plan cannot be executed or reviewed meaningfully.

## Strengths

- Correctly separates adapter (workspace/transport wiring) from application service (validation/reporting/orchestration) responsibilities
- Acknowledges that the goal is readability and ownership clarity, not line-count migration
- Preserves the constraint that `openrustclaw-app` owns business rules

## Concerns

- **HIGH** — No specific files or hotspots are identified. The phase context mentions "final hotspots" and "surviving file-system and compiled-skill MCP boundaries" but the plan never names them. An implementer would have to rediscover the scope from scratch.
- **HIGH** — No adapter interfaces or struct names are proposed. "Introduce explicit adapter seams" is a goal, not a step.
- **MEDIUM** — No verification approach. Step 3 says "verify that the extracted boundaries preserve the shipped command and runtime contracts" but doesn't say how — no tests, no compilation check, no specific contracts to validate against.
- **MEDIUM** — No dependency ordering. If multiple hotspots are being touched, the plan doesn't sequence them or identify which extractions are independent.
- **LOW** — No rollback or incremental strategy. If an extraction breaks a contract mid-flight, there's no guidance on how to recover.

## Suggestions

- List the specific files and functions that constitute the "remaining legacy hotspots" (likely surfaces in `crates/cli/src/commands/` based on project context)
- Name the adapter structs to be introduced and the trait boundaries they'll implement
- Define a concrete verification step: at minimum `cargo test --workspace` passing, ideally with a before/after comparison of public API surface
- Sequence the extractions so each one can be committed and verified independently

## Risk Assessment

**MEDIUM-HIGH** — The plan is directionally correct but too vague to execute without significant discovery work during implementation. The risk isn't that it will break things — it's that the implementer will either under-deliver (cosmetic renames) or over-deliver (large refactor beyond scope) because the boundaries aren't specified. The phase context doc is actually more specific than the plan itself, which is a red flag.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Reviewers disagreed on overall risk level: gemini=HIGH, claude=MEDIUM-HIGH.
