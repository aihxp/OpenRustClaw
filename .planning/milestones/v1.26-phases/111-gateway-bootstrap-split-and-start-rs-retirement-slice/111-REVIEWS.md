---
phase: 111
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:58:23.510Z
plans_reviewed: [111-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 111

## Gemini Review

Here is the review of the proposed implementation plan for Phase 111.

### Summary
Plan 111-01 is extremely high-level and reads more like a project management brief than an executable technical plan. While it correctly identifies the required components (control, websocket, webhook, MCP) and aligns with the strategic goal of turning `start.rs` into a temporary compatibility shell, it completely lacks concrete implementation steps. The verbs used ("Map", "Name", "Verify") imply an analysis phase rather than the actual code extraction and refactoring required to achieve the split. 

### Strengths
* **Strategic Alignment:** Directly addresses the phase goal of decoupling startup ownership from route/tool logic before attempting deletions.
* **Comprehensive Scope Coverage:** Correctly identifies the specific startup responsibilities that need to be relocated (control, websocket, webhook, MCP).
* **Safe Transition Path:** Adheres to the decision to keep `start.rs` as a bounded forwarding role, reducing the risk of a "big bang" migration failure.

### Concerns
* **[HIGH] Lack of Technical Detail:** The plan contains no concrete coding steps. It does not specify which new Rust modules will be created (e.g., `crate::gateway::bootstrap`), what structs or functions will take over the logic, or how dependencies will be injected into the new targets.
* **[HIGH] Missing Implementation of the Shell:** Step 3 talks about verifying that the roadmap leaves `start.rs` with a bounded forwarding role, but there are no steps to actually *build* that forwarding shell or wire it to the new native targets.
* **[MEDIUM] No Verification Strategy:** There are no steps defining how to test the new startup sequences. We need to know how to verify that the application still boots correctly and that the legacy `start.rs` shell correctly delegates to the new implementations.
* **[MEDIUM] State Management Risks:** Startup sequences often involve initializing global or shared state (e.g., database pools, telemetry, configuration). The plan does not address how this state initialization will be safely moved or shared during the transition period.

### Suggestions
* **Define Target Modules:** Explicitly name the new modules that will be created to house the extracted logic (e.g., `src/delivery/gateway/startup.rs`, `src/delivery/mcp/startup.rs`).
* **Make Steps Executable:** Change the analysis steps into concrete actions. For example, instead of "Map control... away", use "Extract websocket server initialization from `start.rs` into `gateway::startup::boot_websocket_server()`".
* **Implement the Forwarding Shell:** Add a specific step to refactor the remaining code in `start.rs` to call the newly created bootstrap functions in the native delivery layer.
* **Add Testing Steps:** Include concrete verification steps, such as running the existing E2E or integration tests to ensure the server still boots and accepts connections on all expected ports (control, webhook, websocket) after the refactor.

### Risk Assessment
**HIGH** 
The risk is high because the plan is currently unactionable as an engineering implementation guide. Executing this plan as written would likely result in documentation of a future architecture rather than the actual codebase refactoring required by the phase decisions. The plan needs to be rewritten to focus on the concrete Rust refactoring steps necessary to achieve the stated goals.

---

## Claude Review

The review is complete above. The plan is low-risk and directionally sound but would benefit from an explicit `start.rs` inventory step and concrete target module names to make the scoping exercise actionable rather than abstract.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=HIGH.
