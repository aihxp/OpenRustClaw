---
phase: 11
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:27:52.127Z
plans_reviewed: [117-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 11

## Gemini Review

Here is the review of Plan 117-01 based on the provided project context and requirements.

### Summary
Plan 117-01 outlines a high-level strategic approach to transition the remaining large operator command families (such as browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted flows) from the legacy CLI tree to native delivery modules backed by app ports. It correctly identifies the risk of a "vague rewrite" and emphasizes an incremental, explicitly roadmapped implementation path. However, the plan is overly abstract and lacks the concrete technical specifics, file paths, and explicit mapping criteria required for a developer to actually execute the transition without ambiguity.

### Strengths
- **Architectural Alignment:** Correctly targets "app-port-backed native delivery modules," which aligns perfectly with the project's greenfield conversion and native delivery layer goals.
- **Risk Awareness:** Explicitly acknowledges the danger of "generic future cleanup" and "vague rewrites," opting for an incremental approach.
- **Documentation First:** Prioritizes updating the roadmap to make the command families explicit before starting implementation, ensuring better tracking and visibility.

### Concerns
- **[HIGH] Lack of Technical Specificity:** The plan does not explicitly list the command families in scope within the steps, nor does it provide a framework for *how* the mapping to app ports will be structured. It reads more like a goal statement than an actionable implementation plan.
- **[HIGH] Missing Verification Criteria:** Step 3 mentions "Verify that the milestone keeps the implementation path incremental," but provides no concrete success criteria. There is no mention of how to test or validate that a mapped command family is functioning correctly in the new native delivery path.
- **[MEDIUM] Unclear Legacy Deprecation Strategy:** The plan mentions leaving the legacy command tree but does not outline the sequence of events (e.g., run in parallel, feature flag, immediate deprecation) for retiring the old code once the new native path is established.
- **[LOW] Dependency Ordering Not Addressed:** Some command families (like onboarding) may have dependencies on others (like skills or orchestration). The plan does not specify an order of operations for migrating these families.

### Suggestions
- **List the Target Families Explicitly:** Update the plan to explicitly enumerate the target command families (e.g., `browser`, `orchestration`, `mobile`, `voice`, `onboarding`, `skills`) to strictly bound the scope.
- **Provide a Draft Mapping Template:** Include an example or template of how a legacy command maps to a new app port (e.g., `crates/cli/src/legacy/browser.rs` -> `crates/app/src/ports/browser/`).
- **Define Concrete Verification Steps:** Add a step to define E2E or unit tests for the new native delivery modules to ensure feature parity with the legacy commands before switching traffic.
- **Phased Implementation Rollout:** Structure the plan to enforce the incremental approach by migrating one specific, low-risk command family first as a proof-of-concept (e.g., `skills`), before moving on to larger families like `orchestration`.
- **Include a Deprecation Step:** Add a specific step detailing how legacy command files will be marked as deprecated and subsequently removed.

### Risk Assessment
**MEDIUM**
While the strategic intent of the plan is sound and well-aligned with the project's goals, its current form is too abstract. The lack of concrete technical details, specific mapping instructions, and clear verification criteria introduces a significant risk that developers will have to "rediscover delivery boundaries" during implementation—the exact problem the plan aims to solve. Fleshing out the technical specifics will reduce this risk to LOW.

---

## Claude Review

The review is complete above. The plan is **low risk** but would benefit from naming its concrete output artifact and cross-referencing which families were already handled in the v1.25–v1.32 native delivery milestones.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
