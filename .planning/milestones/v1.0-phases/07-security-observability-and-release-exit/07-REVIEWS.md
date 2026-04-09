---
phase: 7
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:12:19.234Z
plans_reviewed: [07-01-PLAN.md, 07-02-PLAN.md, 07-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 7

## Gemini Review

Here is the cross-AI review of the implementation plans for Phase 7 (Security, Observability, and Release Exit).

### Plan 07-01: Add a unified security posture summary

**1. Summary**
This plan extracts fragmented, CLI-only security signals into a unified, strongly-typed data structure exposed through both a control-plane endpoint and a new browser-based Control UI panel. It successfully operationalizes the requirement that operators should be able to inspect secure-default posture before release.

**2. Strengths**
*   **Pragmatic Reuse:** Adheres to the decision (D-01) to use existing primitives rather than inventing a new compliance framework.
*   **Surface Parity:** Brings the browser dashboard up to parity with the CLI, directly supporting the project's goal of "materially deeper operator parity."
*   **Testable Contract:** Includes a direct UI regression test for the new dashboard panel, ensuring the trust surface doesn't silently break in future milestones.

**3. Concerns**
*   **Error Handling for Partial State (MEDIUM):** The plan does not specify how the system should behave if certain security metrics (e.g., skill verification states or origin validation checks) temporarily fail to load or are unavailable.
*   **CLI Drift (LOW):** The plan mentions modifying `security.rs` to compose the typed data, but doesn't explicitly state that the legacy `openrustclaw security audit` command will be refactored to consume this exact same typed struct, risking future logic drift between the UI and CLI.

**4. Suggestions**
*   Ensure the legacy CLI command is explicitly refactored to consume the new typed data structure so that the CLI and the Control UI are guaranteed to report identical states.
*   Define a fallback or "degraded" state in the Control UI panel to clearly indicate to the operator when a specific security signal cannot be fetched, rather than failing the whole view.

**5. Risk Assessment**
**LOW**. The scope is tightly bounded to data aggregation and UI exposure without modifying the underlying enforcement logic, making it highly safe to execute.

---

### Plan 07-02: Write the MVP release-exit checklist

**1. Summary**
This plan formalizes the MVP release gate by creating an explicit, operator-facing checklist, aligning the existing observability documentation to serve as concrete release criteria, and updating the README to point directly to these trust surfaces. 

**2. Strengths**
*   **Truth Over Gloss:** Anchors abstract release readiness to concrete operations and commands, strictly adhering to the project's documentation principles.
*   **Avoids Doc Sprawl:** Updates existing files (`observability.md`, `README.md`) to integrate the release story natively rather than creating a disconnected set of external manuals.

**3. Concerns**
*   **Ambiguous Target Path (LOW):** Modifying `docs/src/deployment` is specified, but it's unclear if this implies modifying an existing file or creating a new one (e.g., `docs/src/deployment/release-exit.md`). 
*   **Manual Verification (MEDIUM):** The plan relies solely on a "Manual doc cross-check" for verification. Given the project's strict baseline for docs parity, manual checks risk falling out of sync as CLI commands evolve.

**4. Suggestions**
*   Explicitly name the target file for the checklist (e.g., `docs/src/deployment/release-checklist.md`) to avoid ambiguity during implementation.
*   Introduce a lightweight automated link-checker or command-validation script in CI to ensure that the commands referenced in the newly written checklist actually exist and match the current CLI help outputs.

**5. Risk Assessment**
**LOW**. This is entirely documentation-focused. The only risk is potential dead links or minor structural inconsistencies, which do not threaten runtime stability.

---

### Plan 07-03: Lock the release exit with verification

**1. Summary**
This plan closes the milestone loop by taking the docs and UI surfaces built in 07-01 and 07-02 and wrapping them in automated validation. It binds existing security E2E workflows and runtime budget scripts into a single, executable release gate.

**2. Strengths**
*   **High-Signal Reuse:** Heavily reuses existing integration and budget scripts instead of creating a brittle, net-new testing apparatus.
*   **Closes the Loop:** Directly fulfills requirement REL-01 by ensuring that the documented release checklist is backed by executable code.

**3. Concerns**
*   **Script Wrapper Ambiguity (MEDIUM):** The plan modifies `scripts` but does not define if this is a new orchestrator script (e.g., `run-release-gate.sh`) or an update to an existing script. A new wrapper could introduce brittle CI dependencies.
*   **Flakiness in E2E Security Workflows (HIGH):** E2E security tests (especially those involving proxies, origin validation, and node-first connectivity) are notoriously susceptible to environmental flakiness. Tying the final release gate strictly to these tests without addressing isolation/retries could paralyze the release pipeline.

**4. Suggestions**
*   Explicitly name the entrypoint for the verification bundle (e.g., creating `scripts/run-release-gate.sh`) and ensure its exit codes map cleanly to CI requirements.
*   Ensure the E2E security tests invoked by the release gate include appropriate state teardown, timeout bounds, and environmental isolation to prevent sporadic CI failures from blocking legitimate MVP releases.
*   Map test output explicitly to the checklist created in 07-02 so operators can immediately see which checklist item failed if the script exits with an error.

**5. Risk Assessment**
**MEDIUM**. While the goal is sound, aggregating multiple complex test suites and budget checks into a single gating script introduces a moderate risk of pipeline fragility if the underlying E2E tests are not perfectly stable.

---

## Claude Review

# Cross-AI Review: Phase 7 — Security, Observability, and Release Exit

## 07-01: Add a unified security posture summary

### Summary
Solid plan that consolidates scattered security signals into a typed summary exposed via control plane and Control UI. The scope is well-bounded and the approach of lifting existing audit data into a structured surface follows established milestone patterns.

### Strengths
- Reuses existing security audit logic rather than building a new framework
- Follows the pattern established in Phase 6: typed summary → control endpoint → UI panel
- Clear artifact list with specific files and purposes
- Addresses a real operator gap — security posture was CLI-only

### Concerns
- **MEDIUM**: No schema or field list for the typed security posture struct. What fields are "release-critical"? Plan 03 tests for them but neither plan defines them explicitly. This leaves the contract implicit.
- **MEDIUM**: No error/degraded-state handling described. What does the posture summary show when auth is misconfigured or skill verification keys are missing? A posture summary that only works in the happy path undermines trust.
- **LOW**: "Lock it with a UI test" for the Control UI panel is vague — is this a DOM assertion in Rust, a snapshot test, or a manual check?

### Suggestions
- Enumerate the specific posture fields (auth mode, origin validation status, trusted-proxy config, skill-signing state, sandbox caveats) so Plan 03 can test against a known contract
- Specify behavior when components are degraded or unconfigured — the summary should surface warnings, not hide them
- Clarify what "UI test" means concretely given the HTML-in-Rust-string pattern used in `control_ui.html`

### Risk Assessment
**LOW** — Well-scoped, follows proven patterns, and the concerns are about specification clarity rather than fundamental design flaws.

---

## 07-02: Write the MVP release-exit checklist

### Summary
Documentation-only plan that creates an operator-facing release checklist and aligns observability docs with release readiness. Reasonable scope, though the value depends entirely on whether the checklist references real, runnable commands rather than becoming a narrative artifact.

### Strengths
- Correctly identifies that release readiness is scattered and needs consolidation
- Ties observability docs to release checks rather than leaving them as a generic monitoring guide
- Adds the checklist to the docs index so it's discoverable

### Concerns
- **MEDIUM**: The plan says "docs/src/deployment" as a target but doesn't specify a filename. Is this a new file? Which existing file? This ambiguity could lead to file placement drift.
- **MEDIUM**: Verification is "manual doc cross-check" — for a release-exit checklist, there's no mechanism to detect when the checklist drifts from reality (e.g., a script is renamed or a command flag changes). Plan 03 partially addresses this but the coupling is loose.
- **LOW**: The README task ("add a concise top-level closeout loop") is vague about what exactly gets added and where in the README it lands.

### Suggestions
- Name the specific checklist file (e.g., `docs/src/deployment/release-checklist.md`)
- Include the actual commands/scripts the checklist will reference so reviewers can verify they exist
- Consider whether the checklist should be machine-parseable (e.g., a structured TOML or markdown with command annotations) so Plan 03's verification can actually validate it

### Risk Assessment
**LOW** — It's documentation work with bounded scope. The main risk is producing a checklist that looks good but drifts from the real verification commands over time.

---

## 07-03: Lock the release exit with verification

### Summary
Closes the loop by tying the release gate to automated checks. The intent is correct — an MVP cannot claim readiness without exercised verification. However, the plan is the vaguest of the three, with underspecified test targets and a "reuse or wrap" framing that could resolve to either meaningful automation or a thin shell script.

### Strengths
- Correctly refuses to accept docs-only release claims
- Reuses existing security E2E and budget tooling rather than creating new test sprawl
- Dependency ordering is right: posture summary (01) and checklist (02) must exist before verification (03) can reference them

### Concerns
- **HIGH**: "Reuse or wrap the existing security workflow tests and runtime budget script into a clear release-gate bundle" is too ambiguous. What does the bundle look like? A script? A test suite? A CI job? The deliverable is undefined. This is the plan most likely to produce something that doesn't actually gate anything.
- **MEDIUM**: No definition of what "release-critical fields" the security posture test should assert. Without the contract from Plan 01, this test could check trivial fields and miss the ones that matter.
- **MEDIUM**: The plan doesn't address how this gate integrates with the existing CI pipeline or release workflow. A gate that exists but isn't wired into the actual release path is theater.
- **LOW**: "Add focused verification for the typed security posture" in `tests/integration` — but the security posture is composed in `security.rs` (a CLI command module). Integration-testing a CLI command's internal struct composition may require awkward test wiring depending on how the posture function is factored.

### Suggestions
- Define the concrete deliverable: a single script (e.g., `scripts/release-gate.sh`) that runs the specific checks and exits non-zero on failure, or a test target like `cargo test --test release_gate`
- List the specific existing tests and scripts that will be included in the bundle (e.g., `test_security_workflow.rs`, `test_security_boundaries.rs`, `check-runtime-budgets.sh`)
- Specify whether this gate should run in CI and block releases, or is operator-invoked only
- Factor the security posture composition as a library function (not buried in a CLI handler) so integration tests can call it cleanly

### Risk Assessment
**MEDIUM** — The intent is right but the implementation is underspecified enough that it could resolve to either a meaningful automated gate or a cosmetic wrapper. This is the plan most likely to under-deliver relative to its claims.

---

## Overall Phase Assessment

**Overall Risk: LOW-MEDIUM**

The phase has sound structure and follows the milestone's established pattern (typed surface → docs → verification). The dependency chain is correct. The main systemic risk is that the three plans share an implicit contract (the security posture field list) that none of them explicitly defines, which means each plan's implementer is guessing at what the others expect.

**Key recommendation:** Plan 01 should define the posture struct fields explicitly, since both Plan 02 (checklist references) and Plan 03 (test assertions) depend on knowing what "release-critical security posture" actually contains. Without that, the phase hangs together narratively but not mechanically.

---

## Consensus Summary

### Agreed Strengths
- Reviewers generally agreed the plans are structured and traceable to the phase goal, but they emphasized different strengths.

### Agreed Concerns
- No clearly shared concern theme was detected across reviewers; reviewer-specific concerns are preserved above.

### Divergent Views
- Reviewers converged on an overall LOW risk posture.
