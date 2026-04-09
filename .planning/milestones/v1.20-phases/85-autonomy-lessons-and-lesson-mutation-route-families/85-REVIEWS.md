---
phase: 85
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:19:02.161Z
plans_reviewed: [85-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 85

## Gemini Review

Here is the cross-AI review for Plan 85-01 based on the provided phase context and project goals.

### 1. Summary

The plan accurately captures the primary architectural intent of Phase 85: extracting the autonomy lesson-control logic from the HTTP routing layer (`start.rs`) into a dedicated, reusable application service within `openrustclaw-app`. It correctly bounds the scope to lesson listing, creation, and deactivation while leaving broader orchestration untouched. However, the plan is extremely high-level and lacks the necessary technical depth regarding data contracts, error boundaries, dependency injection, and specific test assertions required to ensure a regression-free refactor.

### 2. Strengths

*   **Architectural Alignment:** Perfectly aligns with the phase goal of separating HTTP adapter concerns (`start.rs`) from business logic and response shaping (`openrustclaw-app`).
*   **Bounded Scope:** Correctly limits the extraction to the targeted lesson-oriented autonomy surfaces, preventing scope creep into unrelated routing families.
*   **Focus on DRY:** Explicitly targets the consolidation of the duplicated registry-description and summary shaping logic into a single, stable service boundary.

### 3. Concerns

*   **[HIGH] Undefined Error Handling Boundary:** The plan does not specify how domain errors originating from the new `openrustclaw-app` service (e.g., "Lesson Not Found", "Validation Failed", "Persistence Error") will be safely mapped to appropriate HTTP status codes (404, 400, 500) within the `start.rs` adapter.
*   **[HIGH] Missing Contract Definitions:** The plan fails to outline the expected data structures, traits, or request/response DTOs for the new service. Leaving this ambiguous risks inconsistent API shapes or leaky database models being passed directly to the HTTP layer.
*   **[MEDIUM] State and Dependency Injection Unclear:** It is not specified how the new service will access the underlying persistence layer (the `control` module). The plan needs to address how state/database pools will be injected into the new service.
*   **[MEDIUM] Shallow Verification Strategy:** The verification section simply lists two `cargo test` commands. It does not define *what* behavior needs to be asserted to prove the "shipped lesson-control contract remains truthful."

### 4. Suggestions

*   **Define the Interface:** Expand Step 1 to explicitly define the proposed struct/trait for the service (e.g., `LessonControlService`) and the primary methods it will expose (e.g., `list_lessons`, `create_lesson`, `deactivate_lesson`).
*   **Explicit Error Mapping:** Add a step detailing the creation of a specific domain error enum for the service, and explicitly state that `start.rs` will pattern-match against this enum to return strict, documented HTTP status codes.
*   **Clarify Dependency Wiring:** Briefly describe how the `control` module's persistence components will be provided to the new `openrustclaw-app` service (e.g., via constructor injection or application state extractors in the router).
*   **Detailed Test Assertions:** Enhance the Verification section with specific test cases that must pass, such as:
    *   Asserting that a successful lesson creation returns the exact previous JSON shape and a `201 Created` (or `200 OK` depending on the current contract).
    *   Asserting that deactivating a non-existent lesson properly maps to a `404 Not Found`.
    *   Asserting that the duplicated summary shaping is now uniformly applied across all endpoints.

### 5. Risk Assessment

**Risk Level: MEDIUM**

**Justification:** The conceptual architectural direction is completely correct and poses a low risk to the system. However, the plan's extreme brevity introduces a *medium execution risk*. Because the plan glosses over the critical seams of the refactor—specifically error translation between the domain and HTTP layers, and the exact data payloads being passed—there is a significant risk of accidentally breaking the public API contract or introducing subtle bugs during implementation. Fleshing out the structural and testing details will easily mitigate this and reduce the risk to LOW.

---

## Claude Review

# Cross-AI Review: Phase 85 — Autonomy Lessons and Lesson Mutation Route Families

## Plan 85-01: Extract the Autonomy Lesson-Control Route Family

### Summary

A straightforward extraction plan that moves autonomy lesson listing and mutation logic from the monolithic `start.rs` control-plane hub into a dedicated application-layer service in `openrustclaw-app`. The plan follows the established greenfield conversion pattern used across dozens of prior phases. It is concise and well-scoped but light on implementation detail.

### Strengths

- Tight scope: only lesson list, create, and deactivate — no scope creep into broader autonomy or orchestration surfaces
- Preserves the adapter pattern: `start.rs` stays as the HTTP boundary, new service owns business logic
- Consistent with the project's established extraction methodology (v1.19–v1.24 greenfield conversions set this pattern)
- Verification includes both unit-level (`openrustclaw-app`) and integration-level (`openrustclaw-cli`) test targets
- Clear separation of concerns: summary shaping and post-mutation report composition move to the service layer

### Concerns

- **MEDIUM** — No detail on the service interface. The plan doesn't specify the function signatures, input/output types, or error contract for the new lesson-control service. Prior phases in this repo defined at least a trait or struct sketch. Without this, the implementer must reverse-engineer the contract from `start.rs` inline handlers during execution.
- **LOW** — No mention of how the "registry-description shaping" duplication gets consolidated. The CONTEXT.md notes that handlers duplicate the same shaping after every mutation, but the plan doesn't explicitly call out deduplication as a step — it's implied but not stated.
- **LOW** — Verification commands reference specific test names (`autonomy_lessons_control`, `autonomy_lessons_route_family_uses_service_lane`) that don't exist yet. This is fine as intent, but there's no fallback verification (e.g., `cargo test --workspace` or manual curl checks) if the named tests prove insufficient.
- **LOW** — No mention of whether the lesson persistence layer (`control` module) needs any interface changes or if the new service consumes it as-is. The CONTEXT.md says it "only needs one stable application-owned service boundary above it," but the plan should confirm the persistence API is sufficient.

### Suggestions

- Add a brief interface sketch for the lesson-control service (2–3 function signatures) so the extraction boundary is unambiguous before implementation starts.
- Explicitly list the deduplication of post-mutation report shaping as a sub-step of Step 1, since the CONTEXT.md identifies it as a key motivator.
- Add `cargo test --workspace` as a final catch-all verification step to confirm no regressions beyond the targeted tests.
- Confirm in Step 2 that the `control` module's persistence API is consumed without modification, or note any required adapter glue.

### Risk Assessment

**LOW** — This is a well-bounded mechanical extraction following a pattern the project has executed over 30 times. The scope is narrow, the seam is clean (lesson routes are a discrete subset of the autonomy surface), and the verification targets both layers. The concerns are about plan documentation quality rather than technical risk. The main execution risk is an implementer misreading the implicit deduplication intent and producing a 1:1 lift without consolidating the duplicated shaping logic.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: claude=LOW.
