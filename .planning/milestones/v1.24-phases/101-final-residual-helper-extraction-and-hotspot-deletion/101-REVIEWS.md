---
phase: 101
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:37:52.921Z
plans_reviewed: [101-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 101

## Gemini Review

Here is the review of the implementation plan for Phase 101.

### 1. Summary

The plan correctly captures the high-level intent of Phase 101 but lacks any actionable technical depth or specificity. It essentially restates the phase's contextual goals as execution steps. It reads more like an objective statement than a technical implementation plan, leaving critical architectural decisions, file targets, and verification methodologies completely unresolved for the engineer executing it. 

### 2. Strengths

* **Goal Alignment:** Accurately reflects the core mandate of the phase context (moving logic from legacy commands to `openrustclaw-app`).
* **Scope Completeness:** Explicitly lists all the functional areas mentioned in the context (continuity, tool-audit, voice-call reporting, and compiled-skill MCP).
* **Behavioral Contract:** Correctly identifies that the migration must happen without behavioral drift for the operator and runtime.

### 3. Concerns

* **HIGH - Lack of Implementation Detail:** The plan fails to explicitly name the target files (`inspect.rs`, `skills.rs`, and `start.rs`) in the execution steps. It does not define *what* specific helpers are being extracted, nor what the target structs, traits, or service interfaces in `openrustclaw-app` will look like.
* **HIGH - Missing Verification Strategy:** Step 3 instructs to "Verify that the migrated operator... contracts still pass", but provides zero details on *how*. There are no specific test suites, E2E scripts, or manual CLI validation steps outlined to prove behavior remains unchanged.
* **MEDIUM - Missing Dependency Sequencing:** The plan groups the extraction of four distinct, complex domains (continuity, tools, voice, skills) into a single monolithic step. It does not sequence them, increasing the risk of massive, difficult-to-review pull requests and merge conflicts.
* **MEDIUM - Unclear Compatibility Strategy:** The context explicitly requires "compatibility-preserving adapter changes," but the plan does not explain how the old CLI modules will map to the new services to preserve the existing CLI API surface during the transition.

### 4. Suggestions

* **Granular Task Breakdown:** Split Step 1 into sequential sub-tasks per domain to allow for incremental PRs (e.g., 1a. Extract continuity logic from `start.rs`, 1b. Extract tool-audit logic from `inspect.rs`, 1c. Extract MCP/skills logic from `skills.rs`).
* **Define Architectural Boundaries:** Explicitly name the target helper functions being removed and map them to the new service abstractions being created in `openrustclaw-app` (e.g., "Create a `ToolAuditService` in `openrustclaw-app` to replace `print_tool_history` in `inspect.rs`").
* **Define the Adapter Pattern:** Specify how the legacy CLI commands will instantiate and call the new app services to maintain backward compatibility.
* **Explicit Testing Steps:** Replace Step 3 with specific verification instructions. List the exact `cargo test` commands, integration tests, or manual user flows (e.g., verifying `openrustclaw inspect tools` output matches the previous version perfectly) required to prove success.

### 5. Risk Assessment

**HIGH**

**Justification:** While the conceptual goal is correct, executing a plan this vague on legacy hotspots is highly risky. Because the plan is devoid of technical specifics, the implementer will be forced to make major structural and boundary decisions on the fly. Ripping business logic out of CLI commands and into a core application layer requires a precise map of what is moving where; without it, the risk of introducing regressions, breaking CLI contracts, or creating messy abstractions is severe.

---

## Claude Review

The review is complete above. The core finding: **the plan's strategy is correct but far too abstract to execute safely** — it needs function-level extraction targets, a defined landing zone in `openrustclaw-app`, and explicit verification coverage before implementation begins.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=HIGH.
