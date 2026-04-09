---
phase: 82
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:15:31.112Z
plans_reviewed: [82-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 82

## Gemini Review

# Plan Review: 82-01 Extract the Diagnostics and Live Event Route Family

## Summary
The plan accurately captures the core objective of the phase by extracting the diagnostics orchestration and websocket payload shaping from the CLI's `start.rs` into the `openrustclaw-app` service layer. It correctly maintains the boundary constraint of leaving the actual websocket transport loop in `start.rs` and avoids scope creep by reusing the existing `doctor::collect_report` logic. While the high-level steps are sound, the plan is brief and omits specific details about the interface design, error propagation, and type definitions for the event payloads.

## Strengths
*   **Precise Scope:** Directly addresses the phase goal of slimming down the `start.rs` route hub without modifying the underlying `doctor` command logic.
*   **Clear Boundaries:** Correctly identifies what should move (report collection, interval normalization, payload shaping) versus what should stay (the websocket transport loop itself).
*   **Focused Verification:** Includes targeted `cargo test` commands to verify the new application service and the wiring in the CLI route family.

## Concerns
*   **MEDIUM: Missing Interface Definitions:** The plan does not specify the trait, struct, or method signatures for the new `control_diagnostics` service. This ambiguity could lead to suboptimal boundary design during implementation.
*   **LOW: Undefined Error Handling:** It does not explain how errors from `doctor::collect_report` will be bubbled up through the new service layer and transformed into the expected `"error"` websocket payload.
*   **LOW: Payload Typing:** The plan mentions `"diagnostics"` versus `"error"` payload shaping but doesn't mention extracting or defining strict Serde-compatible struct types in `openrustclaw-app` to guarantee the JSON contract remains unchanged.
*   **LOW: Verification Gap:** The proposed tests appear to be unit or wiring tests. There is no explicit callout for an integration test that connects to the websocket endpoint to guarantee the end-to-end JSON contract behavior.

## Suggestions
*   **Define the Service Interface:** Add a step to explicitly define the public interface (e.g., `pub struct DiagnosticsService`) and its methods (e.g., `fn get_report()`, `fn format_ws_payload()`) before doing the extraction.
*   **Formalize Event Payloads:** Introduce specific Data Transfer Objects (DTOs) or Enums (e.g., `DiagnosticsWsEvent::Report(...)`, `DiagnosticsWsEvent::Error(...)`) in `openrustclaw-app` with `#[derive(Serialize)]` to strictly enforce the payload shapes.
*   **Detail Error Mapping:** Explicitly state how internal errors from the `doctor` checks will be caught and mapped to the application service's error type, ensuring the final JSON output matches the existing contract.
*   **Add an E2E Integration Test:** Supplement the unit tests with a brief integration test that starts the HTTP server, connects a mock websocket client, and asserts the shape of the first emitted diagnostics payload.

## Risk Assessment
**LOW**
The risk is minimal because this is purely a structural refactoring task. The complex logic (the actual system diagnostics inside `doctor::collect_report`) remains untouched, and the network transport mechanism (the websocket upgrade loop) stays in place. The primary risks are minor serialization regressions or subtle changes to the websocket polling interval, both of which are easily mitigated by strict type definitions and the proposed testing strategy.

---

## Claude Review

The review is complete above. The plan is **low-risk** and well-scoped — the main recommendation is to add clarity around the websocket session lifecycle boundary and strengthen verification assertions on the payload contract.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
