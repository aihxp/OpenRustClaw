---
phase: 25
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:25:54.939Z
plans_reviewed: [25-01-PLAN.md, 25-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 25

## Gemini Review

Here is the structured review of the implementation plans for Phase 25.

### Summary
The plans provide a pragmatic, well-scoped approach to introducing tiered onboarding by dividing the work into two logical steps: capturing and persisting the deployment choice (25-01) and surfacing that state in diagnostics (25-02). The approach correctly leverages the existing `self_hosted.rs` contract and adheres to the constraint of deferring complex lifecycle transitions. However, Plan 25-01 currently omits a critical requirement regarding operator education and trust boundaries.

### Strengths
* **Logical Separation of Concerns:** Splitting the persistence logic (25-01) from the diagnostic reporting (25-02) allows for targeted, independent verification.
* **Adherence to Constraints:** The plans strictly follow the "Deferred Ideas" by avoiding complex upgrade/downgrade transitions and keeping the focus on first-run setup.
* **Non-Disruptive Diagnostics:** Plan 25-02 correctly targets a warning-only approach for missing modes in `doctor.rs`, ensuring that existing installations or offline instances are not blocked from starting.
* **Single Source of Truth:** Reusing the `self_hosted.rs` manifest prevents state drift and avoids creating redundant configuration files.

### Concerns
* **[HIGH] Missing Educational UX:** Plan 25-01 fails to explicitly address Success Criterion 2: *"Each path explains its trust boundary, setup steps, and recommended defaults honestly."* The plan only states that defaults will change, not that the CLI will explain the implications (e.g., who holds the keys, network exposure) to the operator.
* **[MEDIUM] Incomplete Error Handling Definition:** Neither plan specifies what should happen if writing the deployment path to the self-hosted manifest fails (e.g., due to file permission errors during setup). It is unclear if onboarding should abort or proceed with a warning.
* **[LOW] Re-onboarding Behavior Unclear:** The plans do not specify how the wizard should behave if a user re-runs `$ openrustclaw onboard` when a deployment path is already set.

### Suggestions
* **Update Plan 25-01** to explicitly require adding CLI output that explains the trust boundary, security posture, and setup implications for the selected mode *before* the operator confirms their choice.
* **Define Failure Modes:** Add a requirement in Plan 25-01 to handle manifest write failures gracefully (e.g., halt onboarding with a clear error message instructing the user to fix permissions).
* **Handle Existing State:** Specify in Plan 25-01 that if a deployment path is already detected, the wizard should display the current mode and ask if the user wishes to retain it or exit, avoiding accidental overwrites.
* **Actionable Diagnostics:** In Plan 25-02, ensure the `doctor.rs` warning provides an actionable command (e.g., `openrustclaw onboard --mode`) so the operator knows exactly how to resolve the warning.

### Risk Assessment
**LOW**
The overall technical risk is low. The plans modify localized, interactive CLI paths (`onboard.rs`, `doctor.rs`) and rely on an established configuration contract without altering the core agent runtime or orchestration engines. Addressing the missing UX requirements for explaining trust boundaries will fully align the plans with the phase goals.

---

## Claude Review

# Cross-AI Review: Phase 25 — Tiered Onboarding and First-Run Paths

## Plan 25-01: Branch Onboarding By Deployment Path

### Summary
Straightforward plan to add a deployment-path selector early in the onboarding wizard and persist it via the existing self-hosted manifest. Scope is well-contained and builds on existing code.

### Strengths
- Reuses existing onboarding skeleton and self-hosted contract — no new abstractions
- Only two files touched, minimizing blast radius
- Persistence goes through the canonical manifest, avoiding state duplication

### Concerns
- **MEDIUM** — No mention of non-interactive / headless onboarding (CI, Docker, scripted installs). If the selector is interactive-only, automated deployments have no way to set the mode.
- **LOW** — No mention of what happens if the manifest file already exists with a mode set (re-onboarding scenario). Should it skip, confirm, or overwrite?
- **LOW** — "Onboarding defaults reflect the chosen path" is vague. The plan doesn't specify which defaults change per mode (e.g., does enterprise default to stricter auth? Does solo skip multi-user setup steps?).

### Suggestions
- Add a `--mode solo|team|company|enterprise` CLI flag for non-interactive use
- Specify at least the key defaults that differ per mode, even as a brief table
- Document behavior when re-running onboarding with an existing mode selection

### Risk Assessment
**LOW** — Small scope, clear ownership, builds on shipped contracts. The headless gap is the main practical risk but is addressable without scope change.

---

## Plan 25-02: Reflect Deployment Path In First-Start Diagnostics

### Summary
Carries the mode selection into workspace-status and doctor output. Correctly scoped as warning-only rather than blocking, which matches the context decisions.

### Strengths
- Warning-only posture avoids breaking existing installs that predate mode selection
- Touches only the two expected surfaces (doctor, onboarding status)
- Explicit about not making diagnostics stricter than intended

### Concerns
- **MEDIUM** — No specification of what the doctor warning message says or how it guides the operator to fix it. A warning without a remediation hint ("run `openrustclaw onboard --mode ...`") is noise.
- **LOW** — No mention of whether workspace-status output format is human-only or also machine-readable (JSON). If other tooling consumes status, the mode field needs a stable key.

### Suggestions
- Include the remediation command in the doctor warning text
- Confirm whether workspace-status has a JSON output path and add the mode field there too

### Risk Assessment
**LOW** — Purely additive diagnostic output with no blocking behavior. Minimal risk of regression.

---

## Overall Phase Assessment

**Risk: LOW.** Both plans are well-scoped, build on shipped code, and avoid scope creep. The main gap across both plans is the non-interactive/headless path — worth addressing in 25-01 since automated deployments are a real use case for a self-hosted product.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=LOW.
