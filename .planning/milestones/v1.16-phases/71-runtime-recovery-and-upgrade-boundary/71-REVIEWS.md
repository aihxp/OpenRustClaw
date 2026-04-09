---
phase: 71
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T18:55:13.391Z
plans_reviewed: [71-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 71

## Gemini Review

# Plan Review: 71-01 Extract the Runtime Reload Planning Lane

## Summary

The plan outlines a straightforward and necessary refactoring to move the runtime reload-planning logic out of the CLI crate (`crates/cli/src/commands/runtime.rs`) and into a dedicated application-layer service within the `openrustclaw-app` crate. This approach aligns perfectly with the phase goal of continuing the "greenfield transition" by separating pure domain logic (snapshot comparison and classification) from I/O and orchestration (snapshot capture and state persistence).

## Strengths

- **Clear Boundary Definition:** The plan correctly identifies the separation of concerns: the new service handles the logic, while the CLI is reduced to an adapter for I/O.
- **Scope Containment:** The plan remains tightly focused on the specific goal of Phase 71 without introducing unnecessary scope creep or attempting to rewrite the entire runtime module at once.
- **Test-Driven Verification:** The inclusion of specific, targeted `cargo test` commands ensures that both the newly extracted logic and the modified CLI adapter will be validated against existing contracts.

## Concerns

- **HIGH: Dependency and Model Location:** The plan does not specify where the domain models for the runtime snapshots and reload plans currently live. If these structs are currently defined in `openrustclaw-cli`, they will need to be moved to `openrustclaw-app` (or a shared `core` crate) to prevent circular dependencies. 
- **MEDIUM: Error Handling Continuity:** There is no explicit mention of how error handling will be structured. The new service must expose cleanly typed errors that the CLI adapter can map to the existing user-facing error messages to avoid degrading the operator experience.
- **MEDIUM: Serialization Compatibility:** The plan assumes the I/O layer remains unchanged, but moving the structures might affect how they are serialized/deserialized. Backward compatibility with existing persisted reload-state artifacts must be guaranteed.
- **LOW: Plan Detail:** The steps are quite terse. While acceptable for a high-level plan, the implementer will need to be careful about the exact surface area of the `runtime_reload_plan(...)` function being moved.

## Suggestions

- **Clarify Model Migration:** Add an explicit sub-step to Step 1 detailing the relocation of the necessary data structures (e.g., the runtime snapshot struct, the reload plan struct) to the `openrustclaw-app` crate to satisfy dependency rules.
- **Define Service Errors:** Add a requirement to define specific error types for the new `openrustclaw-app` service that can be seamlessly converted into the CLI's existing error outputs.
- **Add E2E CLI Test:** While unit tests are planned, consider adding an integration test (e.g., using `assert_cmd`) in the CLI crate to verify the actual end-to-end command execution still produces the exact same standard output and exit codes as before.
- **Verify State Compatibility:** Explicitly include a test case that loads a mock "legacy" snapshot file from disk to prove that the new adapter/service combo can still process existing state artifacts without breaking.

## Risk Assessment

**LOW**

**Justification:** This is a structural refactoring rather than a behavioral change. The primary risks revolve around Rust's strict dependency graph (moving structs between crates) and maintaining exact serialization compatibility. These are standard refactoring challenges that can be fully mitigated through the proposed regression tests and careful compiler feedback. The overall risk to system stability is low provided the existing tests are robust.

---

## Claude Review

The review is complete above. The plan is low-risk and directionally sound but would benefit from a defined service interface signature and explicit error-handling strategy at the new boundary.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
