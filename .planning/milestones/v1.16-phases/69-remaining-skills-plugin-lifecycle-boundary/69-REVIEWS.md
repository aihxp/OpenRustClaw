---
phase: 69
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T18:51:32.813Z
plans_reviewed: [69-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 69

## Gemini Review

Here is the cross-AI review for Plan 69-01 based on the provided project context and requirements.

### 1. Summary

The plan is concise and effectively addresses the phase goal of extracting a plugin lifecycle boundary from the `skills.rs` CLI command into the application layer (`openrustclaw-app`). By specifically targeting the voice-plugin binding lane, the plan establishes a clear, tightly scoped refactoring effort that correctly aims to separate core business logic (validation, record shaping) from the delivery/adapter mechanism. 

### 2. Strengths

*   **Highly Focused Scope:** The plan avoids boiling the ocean by directly targeting a single, identifiable mutation lane (voice-plugin binding) rather than attempting to rewrite all of `skills.rs` at once.
*   **Clear Boundary Definition:** It explicitly defines the architectural split: business rules, validation, and record composition move to the application layer, while the CLI remains an adapter.
*   **Test-Driven Verification:** Explicitly lists targeted regression commands for both the new application service and the refactored CLI adapter to ensure the contract remains stable before and after the extraction.

### 3. Concerns

*   **Partial Orchestration Extraction (MEDIUM):** The context states that `skills.rs` will "resolve compiled-skill details, persist the registry entry, and publish the plugin lifecycle event." While leaving persistence and event publication in the delivery layer might be a necessary intermediate step, it violates clean architecture principles by leaving side-effect orchestration in the adapter rather than moving it behind an application use-case boundary. 
*   **Error Handling and Contract Mapping (MEDIUM):** The plan does not explicitly mention how domain errors originating from the new `openrustclaw-app` service will be mapped back to existing CLI exit codes or API HTTP responses. A change in error structure could unintentionally break the "shipped voice-plugin bind contract".
*   **Control API Blindspot (LOW):** The context notes this lane feeds the shipped control API through `/control/skills/voice-plugins/bind`. The plan's steps and verification only explicitly mention updating and testing `skills.rs` (the CLI). It is ambiguous whether the API route handler also needs to be updated to consume the new service directly, or if it currently proxies through the CLI module.

### 4. Suggestions

*   **Introduce Dependency Inversion for Side Effects:** Instead of having `skills.rs` orchestrate the save and publish steps after calling the service, define traits (ports) in `openrustclaw-app` for the plugin registry and event publisher. Have the new service orchestrate the validation, persistence, and event publication. `skills.rs` should simply wire up the implementations (adapters) and execute the service command.
*   **Explicitly Define Error Types:** Add a step to define a specific `enum VoicePluginBindError` in `openrustclaw-app`. Ensure the CLI (and API) explicitly map these new domain errors to the legacy output formats to guarantee the external contract remains completely unchanged.
*   **Add API Verification:** If the control API (`/control/skills/voice-plugins/bind`) is affected, explicitly add a step to update its route handler to use the new service, and add a corresponding `cargo test` command in the Verification section to ensure the HTTP contract is unbroken.

### 5. Risk Assessment

**LOW**

**Justification:** The refactoring is well-scoped, targets a specific vertical slice, and prioritizes preserving the existing contract through regression tests. The identified concerns are architectural refinements (moving side-effect orchestration fully into the app layer) and implementation details (error mapping) rather than fundamental flaws that threaten project stability. The incremental approach minimizes the risk of breaking the broader `skills.rs` surface area.

---

## Claude Review

The review is complete above. The plan is sound but would benefit from an explicit service interface definition and control-API verification coverage. Overall risk: **LOW-MEDIUM**.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
