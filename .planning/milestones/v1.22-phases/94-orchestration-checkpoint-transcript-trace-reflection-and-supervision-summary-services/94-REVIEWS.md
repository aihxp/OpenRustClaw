---
phase: 94
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:29:44.933Z
plans_reviewed: [94-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 94

## Gemini Review

Here is the cross-AI plan review for Phase 94, Plan 94-01.

### 1. Summary

Plan 94-01 correctly identifies the necessary architectural shift to move business logic (composition, aggregation, reflection) out of the CLI adapter (`orchestrate.rs`) and into the core application layer (`openrustclaw-app`). It accurately respects the architectural boundary by keeping raw file I/O operations within the CLI while delegating the processing to the new service. However, the plan is highly abstract and lacks critical technical details regarding the data contracts between the CLI and the app service, error handling strategies, and integration verification.

### 2. Strengths

*   **Architectural Alignment:** The plan strictly adheres to the project's goal of shrinking the CLI surface into a thin adapter by moving complex composition logic into `openrustclaw-app`.
*   **Clear I/O Boundary:** Correctly identifies that file reads (receipt and active-run persistence) should remain in the CLI layer, ensuring the core app service remains pure and agnostic of the underlying file system structure.
*   **Test-Driven Focus:** Explicitly includes app-unit coverage as a verification step, which is crucial when refactoring complex composition logic.

### 3. Concerns

*   **HIGH: Undefined Data Contracts:** The plan does not specify the structures (structs/enums) that will be passed from the CLI adapter to the new reporting service, nor does it define the output payloads the service will return. Without clear contracts, the CLI might still end up doing unnecessary data transformation.
*   **MEDIUM: Missing Error Handling Strategy:** There is no mention of how the new app service will handle malformed traces, missing resources, or parsing failures. It needs a clear strategy for propagating domain-specific errors back to the CLI for user-friendly display.
*   **MEDIUM: Potential Monolith Service:** Grouping "trace-resource aggregation, reflection-candidate generation, and supervision payload builders" into a single "reporting service" risks creating a monolithic module. These are distinct responsibilities that might warrant separate modules or traits within the app layer.
*   **LOW: Incomplete Verification Scope:** While unit tests are mentioned, there is no plan to verify the end-to-end output. Since this is a refactoring effort, the plan should explicitly state that the final CLI output must perfectly match the legacy output.

### 4. Suggestions

*   **Define Boundary Models:** Explicitly define the input structs (e.g., `RawTraceData`, `RawCheckpoint`) that `orchestrate.rs` will pass to the service, and the output structs (e.g., `SupervisionSummaryPayload`) it expects back.
*   **Specify Error Variants:** Introduce a specific error enum for the reporting service (e.g., `ReportingError`) that the CLI can map to appropriate exit codes or error messages.
*   **Modularize the App Service:** Instead of one large service, consider planning distinct modules under an `orchestration_reports` namespace:
    *   `TraceAggregator`
    *   `ReflectionGenerator`
    *   `SupervisionComposer`
*   **Add Output Parity Testing:** Include a verification step to run the legacy `orchestrate.rs` command against a known dataset, run the newly refactored command against the same dataset, and assert that the output payloads are identical.

### 5. Risk Assessment

**Risk Level: MEDIUM**

**Justification:** The architectural direction is sound and aligns with the project's greenfield conversion goals. However, the plan's extreme brevity leaves too much implementation ambiguity. The lack of defined data boundaries and error handling mechanisms introduces a moderate risk of scope creep, regressions, or a messy implementation phase where the developer has to invent the contracts on the fly. Fleshing out the data models before implementation would lower this risk to LOW.

---

## Claude Review

# Cross-AI Review: Phase 94 Plan 94-01

## Summary

A straightforward extraction plan that moves orchestration reporting composition out of the legacy `orchestrate.rs` CLI command module into `openrustclaw-app`. The plan is minimal and well-scoped — three steps covering service creation, caller update, and verification. However, it lacks specificity on the service's API surface, error handling strategy, and what "trace-resource aggregation" and "reflection-candidate generation" concretely entail at the code level.

## Strengths

- **Tight scope**: Stays within the established greenfield extraction pattern — move business logic to app, leave adapter concerns in CLI
- **Clear ownership boundary**: Receipt/active-run file reads stay in `orchestrate.rs`; composition logic moves to app
- **Consistent with project direction**: Continues the `orchestrate.rs` shrinkage mandate from earlier greenfield phases
- **Verification included**: Mentions both app-unit coverage and clean CLI compile as exit criteria

## Concerns

- **LOW — Vague service contract**: "trace-resource aggregation, reflection-candidate generation, and supervision payload builders" are listed but no function signatures, struct shapes, or trait boundaries are specified. For a plan, even rough type sketches would reduce ambiguity during execution.
- **MEDIUM — No error handling strategy**: The plan doesn't mention how reporting failures should propagate. Should a failed trace aggregation silently degrade, return a partial result, or bubble up? This matters for supervision summaries where incomplete data could mislead operators.
- **LOW — No mention of existing code inventory**: The plan doesn't reference which specific functions or blocks in `orchestrate.rs` are being extracted. Without that, the executor must re-discover the extraction surface.
- **LOW — "Focused app-unit coverage" is underspecified**: No indication of what scenarios the tests should cover — happy path only? Malformed trace data? Missing checkpoint files? Empty transcripts?
- **LOW — No integration test consideration**: Only app-unit tests and CLI compile are mentioned. No check that the end-to-end orchestration inspection payloads remain identical after extraction.

## Suggestions

- **Name the functions being extracted**: Even a bullet list of the current functions/blocks in `orchestrate.rs` that will move would sharpen execution.
- **Define the service's public API shape**: At minimum, list the 3–4 public methods the reporting service will expose and their input/output types.
- **Add a payload equivalence check**: Before/after comparison of orchestration inspection output to confirm no regression — this can be a simple snapshot test.
- **Specify error propagation**: State whether reporting failures are fatal or degraded, and whether partial results are acceptable.

## Risk Assessment

**LOW** — This is a well-understood mechanical extraction following an established pattern that the project has executed dozens of times across v1.13–v1.38. The concerns are about plan specificity rather than architectural risk. The main execution risk is accidentally changing inspection payload shapes without noticing, which the suggested snapshot test would catch.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: claude=LOW.
