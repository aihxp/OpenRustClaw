---
phase: 26
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:29:06.875Z
plans_reviewed: [26-01-PLAN.md, 26-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 26

## Gemini Review

Here is a review of the implementation plans for Phase 26: Upgrade and Downgrade Lifecycle.

### 1. Summary

The implementation plans for Phase 26 (26-01 and 26-02) establish a solid, auditable foundation for transitioning OpenRustClaw deployment modes. By separating the core transition logic and event ledger (26-01) from the API and Control UI exposure (26-02), the plans adhere well to the architectural decisions of keeping transitions additive and utilizing the existing product-mode manifest. However, the plans currently lack critical details regarding authorization for the new API endpoint and fail to define a "preview" or "dry-run" mechanism, which is essential for ensuring operators can review retained-data warnings *before* committing to a downgrade.

### 2. Strengths

- **Auditable Design:** Plan 26-01 correctly prioritizes capturing the full context of a transition (from-mode, to-mode, actor, direction, warnings), which aligns perfectly with the enterprise foundations established in earlier milestones.
- **Additive Data Handling:** Explicitly choosing to surface retained enterprise state as warnings rather than performing silent data deletion prevents unexpected data loss and respects the operator's control over the environment.
- **Clear Separation of Concerns:** Plan 26-01 handles the backend contract and ledger, while Plan 26-02 focuses strictly on surfacing these capabilities to the operator via the API and Control UI.
- **Reuses Existing Concepts:** Integrating the transition history directly into the existing product-mode manifest and summary output (via `inspect.rs`) avoids introducing unnecessary new state management systems.

### 3. Concerns

- **HIGH:** **Missing Authorization Constraints:** Modifying the product mode is a highly privileged, globally impactful action. Plan 26-02 introduces a `POST /control/self-hosted/product-mode` endpoint but does not explicitly state the authorization checks required to invoke it (e.g., must be authenticated as an admin-level operator).
- **MEDIUM:** **Lack of Pre-flight/Dry-Run Validation:** The goal is to "surface retained-data or governance warnings when moving down from stronger modes." If the operator only sees these warnings *after* the transition is applied via the Control UI, the purpose of the warning is diminished. There is no mechanism described to preview the impact of a downgrade.
- **MEDIUM:** **Atomicity of the Transition:** Updating the product-mode manifest and appending to the JSONL event ledger are two distinct file operations. The plans do not specify how the system will handle a failure midway through the process (e.g., out of disk space, permissions issue), which could leave the product-mode state out of sync with its ledger.
- **LOW:** **Event Ledger Growth:** While likely a slow-growing file, the plans do not address if or how the JSONL event ledger will be rotated or capped over the lifetime of a long-running instance.

### 4. Suggestions

- **Introduce a Pre-flight API Endpoint:** Add a preview or dry-run mechanism (e.g., `POST /control/self-hosted/product-mode/preview`) to Plan 26-02. This allows the Control UI to calculate and display the exact warnings (retained state, governance implications) to the operator for confirmation *before* the actual transition is executed.
- **Explicit Authorization:** Explicitly update Plan 26-02 to require strict admin-level authorization on the new transition API endpoint, ensuring that only highly privileged operators can alter the deployment mode.
- **Ensure Atomic Writes:** Update Plan 26-01 to mandate atomic file operations (e.g., writing to a temporary file and renaming) when updating the product-mode manifest and ledger, ensuring the system never enters an inconsistent state during a transition failure.
- **Define Warning Scenarios:** Briefly document the specific conditions that will trigger a warning during a downgrade (e.g., "Active SSO configurations exist," "Full-autonomy data retained on disk") so the implementation is unambiguous.

### 5. Risk Assessment

**Overall Risk Level: MEDIUM**

**Justification:** The core architecture is sound and aligns with the project's historical emphasis on auditability and operator control. However, the risk is elevated to MEDIUM because the plans omit explicit security authorization for the transition API and lack a preview mechanism for downgrades. Without a preview, an operator might accidentally orphan complex enterprise configurations or data, discovering the warnings only after the downgrade has taken effect. Implementing the suggested dry-run API and explicit authorization checks will reduce this risk to LOW.

---

## Claude Review

The review is complete above. The two plans are well-structured with one actionable gap: **add auth gating to the transition endpoint in 26-02** before implementation.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
