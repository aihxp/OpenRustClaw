---
phase: 30
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:36:26.180Z
plans_reviewed: [30-01-PLAN.md, 30-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 30

## Gemini Review

Here is a review of the implementation plans for Phase 30, focusing on their alignment with the phase goals, completeness, and potential risks.

### 1. Summary

The proposed plans for Phase 30 successfully target the goal of providing operators with explicit, trustworthy ways to repair or resume partial setups. Plan 30-01 appropriately focuses on the user-facing entry points and visibility (the "trust" aspect of the success criteria), ensuring operators know exactly what the system intends to do. Plan 30-02 tackles the underlying logic, cleanly connecting the existing `doctor` diagnostics and durable setup state to generate actionable repair plans without inventing a parallel system. Together, they represent a pragmatic, well-scoped approach to improving the resilience of the onboarding lifecycle. However, both plans are currently lacking in technical specificity, particularly around edge-case handling and precise testing strategies.

### 2. Strengths

*   **Reuses Existing Primitives:** Leveraging the existing `doctor` module and the durable setup state contract (Phase 28/29) is a highly efficient and safe approach. It avoids creating a secondary "source of truth" for workspace health.
*   **Prioritizes Operator Trust:** Plan 30-01 explicitly calls out printing a concrete repair plan *before* execution and keeping destructive options (reset-with-backup) cleanly separated. This aligns perfectly with the project's core value of maintaining visibility.
*   **Logical Separation of Concerns:** Splitting the UX/entry point (30-01) from the underlying mapping/diagnostic logic (30-02) allows for easier implementation and isolated testing.
*   **Avoids "Magic":** By making the repair path an explicit choice rather than a hidden heuristic, the plans respect the operator's control over the workspace.

### 3. Concerns

*   **[MEDIUM] Unrepairable States (Plan 30-02):** The plan assumes `doctor` diagnostics can be cleanly mapped back to onboarding steps. However, `doctor` might surface environmental blockers (e.g., required ports already in use by other system processes, lack of filesystem permissions) that the onboarding wizard *cannot* fix. If the repair helper doesn't distinguish between "missing config" (fixable) and "environmental blocker" (unfixable by the tool), it may lead the operator into a frustrating loop.
*   **[MEDIUM] Corrupted State Handling:** If the durable setup state file itself is corrupted or manually malformed by the user, how does the repair entry point behave? It needs a graceful fallback (likely suggesting a reset-with-backup) rather than crashing the wizard.
*   **[LOW] Vague Verification Steps:** Both plans use `cargo test -p openrustclaw-cli onboard -- --nocapture` as their verification step. This is too generic. It implies manual verification or running a broad integration test suite rather than executing targeted, newly written tests.
*   **[LOW] Missing Module Specifics:** The plans do not specify *where* these new helpers will live (e.g., `crates/cli/src/setup/repair.rs`) or which exact structs are being modified (e.g., `OnboardingWizard`).

### 4. Suggestions

*   **Refine the Repair Helper Logic (30-02):** Explicitly design the `repair-plan helper` to categorize issues into "Repairable via Setup" and "Requires Manual Operator Action" (e.g., freeing a port). Only proceed with the automated repair if the blockers are actionable by the wizard; otherwise, present the manual blockers to the user clearly.
*   **Define State Corruption Fallbacks (30-01):** Ensure the `OnboardingWizard` catches deserialization or read errors when loading the durable setup state and immediately offers the "reset-with-backup" path as the safest recovery route.
*   **Specify Testing Targets:** Update the verification steps to name specific test functions or modules. For example:
    *   *Plan 30-01:* `cargo test -p openrustclaw-cli test_onboarding_menu_repair_visibility`
    *   *Plan 30-02:* `cargo test -p openrustclaw-cli repair::tests::test_derive_plan_from_doctor_output`
*   **Detail the UX Preview (30-01):** Briefly specify *how* the repair plan is printed (e.g., a bulleted list of the exact setup steps that will be re-run) so the PR reviewer knows what standard of "visibility" to expect.

### 5. Risk Assessment

**Risk Level: LOW**

**Justification:** The architectural risk is minimal because the plans rely entirely on existing, proven systems (`doctor` and the durable setup state). There is no major refactoring proposed. The primary risks are UX-related (poor handling of edge cases like unfixable environment issues or vague test coverage), which can be easily mitigated during the implementation phase by following the suggestions above. The plans represent a safe, incremental improvement to the platform's reliability.

---

## Claude Review

# Cross-AI Review: Phase 30 — Setup Repair and Existing Workspace Recovery

## Plan 30-01: Add Explicit Repair Entry to Onboarding

### Summary
Straightforward UI-level change adding a dedicated "repair" menu option to the existing workspace onboarding flow. Scope is well-contained.

### Strengths
- Reuses existing onboarding menu infrastructure rather than building a parallel entry point
- Keeps reset-with-backup visually separate from repair (different risk profiles, different options)
- Minimal surface area — one new menu choice, one repair plan display

### Concerns
- **LOW** — No mention of what happens if the operator selects "repair" but there's nothing to repair (all steps passing). Should degrade gracefully with a "nothing to fix" message.
- **LOW** — Plan doesn't specify the exact menu label or ordering relative to existing choices. Minor, but ordering affects discoverability.

### Suggestions
- Add a no-op path: if repair-plan derivation yields zero steps, tell the operator and return to the menu instead of running an empty step list.

### Risk Assessment
**LOW** — This is a menu addition on an existing surface with existing infrastructure.

---

## Plan 30-02: Derive Repair Steps from Durable Setup State and Doctor

### Summary
The substantive plan of the pair. Builds a repair-plan helper that combines three existing signals (incomplete setup steps, failed bootstrap outcomes, doctor diagnostics) into a targeted list of onboarding steps to re-run. Reuses the durable setup-state contract to track those steps.

### Strengths
- Derives repair from existing state rather than introducing a parallel diagnostic path — good convergence
- Three-signal approach (setup state + bootstrap outcomes + doctor) covers both "never finished" and "finished but drifted" scenarios
- Unit tests for the derivation logic are called out explicitly

### Concerns
- **MEDIUM** — No discussion of signal conflicts. What if doctor says a step is broken but setup state says it completed successfully? The plan should specify that doctor/bootstrap failures override setup-state completion markers, since the goal is to fix the real state of the world.
- **LOW** — No mention of step dependency ordering during repair. If step B depends on step A and both need repair, they must run in order. The existing `run_selected_steps` likely handles this, but the plan should confirm.
- **LOW** — Plan doesn't mention whether the repair plan is displayed to the operator for confirmation before execution. Plan 30-01 says "print a concrete repair plan" but 30-02 doesn't reference this confirmation gate.

### Suggestions
- Explicitly state that doctor/bootstrap failures take precedence over setup-state "completed" markers when building the repair list.
- Confirm that `run_selected_steps` respects dependency ordering, or sort the repair list before handing it off.
- Wire the repair-plan output from 30-02 into the display step from 30-01 (the plans reference each other implicitly but should be explicit about the interface).

### Risk Assessment
**LOW** — Well-scoped, builds on proven infrastructure, and the signals being combined are already available. The medium concern about conflict resolution is a logic question, not an architectural risk.

---

## Overall Phase Assessment

**Risk: LOW.** Both plans are tight, additive, and converge on existing contracts. The phase goals (explicit re-entry, trust-preserving repair, reuse of setup state) are directly addressed. No scope creep. The main gap is specifying how conflicting signals are resolved during repair-plan derivation — worth one sentence in 30-02 before implementation.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Risk posture could not be parsed reliably from the reviewer output; inspect the sections above.
