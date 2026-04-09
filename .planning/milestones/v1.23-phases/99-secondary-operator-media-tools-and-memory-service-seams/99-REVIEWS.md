---
phase: 99
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:35:10.191Z
plans_reviewed: [99-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 99

## Gemini Review

# Review of Plan 99-01

## Summary
The plan correctly identifies the target of Phase 99: extracting business logic from legacy CLI command modules (`tools.rs`, `media.rs`, `memory.rs`) and moving it into dedicated services within the `openrustclaw-app` layer. It adheres to the architectural constraint of keeping I/O, workspace reads, and transport wiring in the CLI layer. However, the plan is exceptionally brief and lacks the technical depth necessary to guide a safe and verifiable implementation.

## Strengths
*   **Architectural Alignment:** Correctly identifies the division of responsibilities, keeping I/O and transport in the adapter/CLI layer while moving logic (summaries, shaping, parsing) to the application layer.
*   **Targeted Scope:** Focuses specifically on the domains identified in the phase context (`tools`, `media`, `memory`).

## Concerns
*   **[HIGH] Lack of Implementation Detail:** The plan lacks specifics on the new service interfaces, data structures, and how the existing CLI code will be migrated. It does not define what "tool-host summaries", "media support helpers", or "memory view shaping" actually entail at the code level.
*   **[MEDIUM] Missing Error Handling Strategy:** Refactoring logic from the CLI layer to an application service often requires adapting error types (e.g., mapping internal application errors to user-facing CLI errors). The plan does not address how errors will be handled across this new boundary.
*   **[MEDIUM] Vague Verification Steps:** "Verify the extracted lane with app coverage and CLI library verification" is too generic. It does not specify what existing tests need to pass, what new tests need to be written for the `openrustclaw-app` services, or how the integration will be validated.
*   **[LOW] Unknown Dependencies:** The plan doesn't mention if these new services have dependencies on other internal crates or external libraries that need to be managed during the extraction.

## Suggestions
*   **Define Service Interfaces:** Explicitly define the Rust `struct`s and `trait`s (or functions) that will be created in `openrustclaw-app` (e.g., `ToolHostService`, `MediaShapingService`, `MemoryRenderService`).
*   **Detail the Migration Path:** Outline the specific functions or blocks of code in `tools.rs`, `media.rs`, and `memory.rs` that will be moved or modified.
*   **Specify Error Handling:** Define the error types that the new `openrustclaw-app` services will return and how the CLI will translate them into operator-friendly output.
*   **Concrete Testing Plan:** Specify that unit tests must be added to `openrustclaw-app` for the newly extracted logic, and that existing CLI integration tests must be run to ensure no behavioral regressions.

## Risk Assessment
**MEDIUM**

While the architectural goal is sound and the conceptual risk is low, the execution risk is elevated due to the lack of detail in the plan. The vagueness increases the likelihood of introducing regressions, handling errors poorly across the new boundary, or failing to cleanly separate I/O from business logic during implementation. A more detailed plan is required to ensure a smooth transition.

---

## Claude Review

The review is complete above. The key takeaway: **the plan direction is right but it's too thin to execute safely** — it needs a concrete seam inventory, service API sketches, and specific verification steps before implementation begins.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
