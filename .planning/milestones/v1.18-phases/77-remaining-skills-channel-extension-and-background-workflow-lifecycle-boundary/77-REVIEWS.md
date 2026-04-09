---
phase: 77
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T19:08:54.722Z
plans_reviewed: [77-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 77

## Gemini Review

# Plan Review: 77-01 Extract the Remaining Skills Lifecycle Seam

## 1. Summary
The plan correctly targets the core objective of Phase 77 by outlining the extraction of background workflow scheduling and channel-extension binding from the `skills.rs` hotspot into the `openrustclaw-app` service layer. It successfully establishes a clear boundary where the app layer handles business logic (validation, shaping) and the CLI acts as an adapter. However, the plan is highly abstract and omits critical technical implementation details regarding data contracts, error handling, and database interaction boundaries. 

## 2. Strengths
* **Perfect Alignment with Phase Goals:** Directly addresses the required architectural shift, moving business logic to `openrustclaw-app` and treating `skills.rs` strictly as an adapter.
* **Focused Scope:** Avoids scope creep by specifically targeting the remaining scheduling and binding seams without reopening read-only extension manifests.
* **Clear Verification Targets:** Provides explicit, runnable test commands targeting both the app layer isolation and the CLI adapter integration.
* **Logical Ordering:** The step-by-step approach (build service -> wire adapter -> verify) follows a safe, standard refactoring pattern.

## 3. Concerns
* **HIGH - Missing Database and Transaction Context:** The plan does not specify how the new service in `openrustclaw-app` will interact with the scheduler database or persistence layers. It is unclear if `skills.rs` injects a database connection/transaction, or if the application layer manages its own pool. This is critical for preventing partial failures.
* **MEDIUM - Undefined Data Contracts (DTOs):** The plan mentions "job-shaping" and "binding metadata shaping" but fails to define the internal data structures that will pass between the CLI adapter and the application service.
* **MEDIUM - Omitted Error Handling Strategy:** There is no mention of how validation failures (e.g., invalid trigger arguments, missing component presence) will be structured and propagated back to the CLI for operator-facing composition.
* **LOW - Backward Compatibility Risks:** The plan does not explicitly state whether the newly shaped durable scheduler job metadata will remain compatible with existing persisted jobs/bindings in the database.

## 4. Suggestions
* **Define Service Signatures:** Update the plan to include the proposed Rust function signatures for the new `openrustclaw-app` service (e.g., `pub async fn schedule_background_service(...) -> Result<...>`, `pub async fn bind_channel_extension(...) -> Result<...>`).
* **Specify Error Boundaries:** Detail a specific application-level `Error` enum that the new service will return, ensuring it captures validation constraints so `skills.rs` can map them to CLI operator output.
* **Clarify Persistence Injection:** Add a sub-step detailing how the database connection or transaction pool is passed to the new application service to ensure atomic operations.
* **Add Negative Test Cases:** Expand the Verification section to include at least one test command for error paths (e.g., `cargo test ... invalid_trigger_rejected`).
* **Ensure State Compatibility:** Add a brief check to confirm that the generated metadata structures match the existing database schema to prevent breaking current operator schedules.

## 5. Risk Assessment
**MEDIUM**

**Justification:** The architectural intent is solid, and the target is well-isolated. However, moving critical persistence and scheduling logic without defining the data contracts, error boundaries, or database transaction scopes in the plan introduces a moderate risk of implementation friction. Without these details mapped out, the engineer implementing the plan may accidentally introduce regressions in error reporting or database connection handling. Expanding the plan with concrete Rust signatures and error handling strategies will lower this risk to LOW.

---

## Claude Review

The review is complete above. The plan is **low risk** — a clean mechanical extraction following established project patterns, with minor suggestions around expanding the verification step and adding an API sketch.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
