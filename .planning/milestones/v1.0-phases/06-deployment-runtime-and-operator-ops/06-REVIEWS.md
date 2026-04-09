---
phase: 6
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T16:10:26.079Z
plans_reviewed: [06-01-PLAN.md, 06-02-PLAN.md, 06-03-PLAN.md]
---
# Cross-AI Plan Review — Phase 6

## Gemini Review

Here is a structured cross-AI review of the provided implementation plans for Phase 6.

# Phase 6 Plan Review: Deployment, Runtime, and Operator Ops

## General Observations
The plans provide a clear, cohesive sequence to improve operator visibility and documentation for runtime deployment and recovery. However, given the project's extensive "greenfield conversion" and "native delivery" architecture updates (as noted in the project history, specifically the transition to `openrustclaw-app` and the reduction of `start.rs` and `runtime.rs` to adapters), there is a significant architectural risk in the proposed implementation paths for Plan 01.

---

### 06-01-PLAN.md: Add a unified operator-ops runtime summary

**1. Summary**
This plan focuses on unifying the scattered runtime operations primitives into a single, cohesive operator-ops summary. It involves creating a typed summary in `runtime.rs`, exposing it via a new control endpoint in `start.rs`, and rendering it in `control_ui.html`.

**2. Strengths**
* **Operator-Centric:** Directly addresses the need for a single pane of glass for deployment, restart, and recovery decisions.
* **Reuses Primitives:** Builds on existing health, beacon, and lock features rather than reinventing the wheel.
* **Cross-Surface Consistency:** Ensures the CLI, control plane, and Control UI all report the same truthful state.

**3. Concerns**
* **Architectural Regression (HIGH):** The project history emphasizes that `start.rs` and `runtime.rs` have been systematically reduced to legacy adapters during the greenfield transition (e.g., v1.26 Phase 109, v1.17 Phase 75). Modifying `start.rs` and `runtime.rs` directly to compose new summaries risks violating the adapter-only exit enforcement and the native delivery topology.
* **UI Coupling (MEDIUM):** Modifying `control_ui.html` and `control_ui.rs` concurrently with backend logic requires strict adherence to the existing gateway/HTTP adapter contract to prevent tightly coupling frontend rendering with core backend state.

**4. Suggestions**
* **Align with Greenfield Architecture:** Implement the new operator summary composition logic within `openrustclaw-app` (the greenfield application shell) rather than `runtime.rs`. Use `runtime.rs` strictly as the adapter to fetch the underlying workspace primitives.
* **Respect Native Delivery Ports:** Expose the new endpoint through the `openrustclaw-gateway` or appropriate native delivery layer, using `start.rs` only as the HTTP transport adapter, ensuring no business logic creeps back into the legacy command tree.

**5. Risk Assessment**
**HIGH** — If implemented exactly as proposed, it risks reintroducing business logic into legacy hotspots that were explicitly isolated during the full-conversion roadmap.

---

### 06-02-PLAN.md: Align deployment and recovery docs with the shipped runtime

**1. Summary**
This plan aligns the project's documentation (README, installation, and production deployment guides) with the actual shipped Rust runtime commands, establishing a canonical, truthful runbook for operators.

**2. Strengths**
* **Truthfulness:** Explicitly ties documentation to shipped commands, reducing operator friction and tribal knowledge.
* **Holistic Updates:** Touches the entire user journey from the README entrypoint to deep production deployment guides.

**3. Concerns**
* **Missing Complex Topologies (MEDIUM):** The plan focuses on standard deployment but might overlook the "node-first remote connectivity" with "SSH tunnel fallback" and "reverse-proxy" configurations established in v1.12. If recovery paths for these specific networking topologies aren't documented, operators may still get stuck during an outage.
* **Product Modes (LOW):** Documentation must explicitly account for the different self-hosted product modes (`solo`, `team`, `company`, `enterprise`) introduced in v1.5, as recovery steps or service targets might differ.

**4. Suggestions**
* **Include Networking Fallbacks:** Explicitly weave the node/SSH-tunnel/reverse-proxy recovery states into the `production.md` runbook so network-level failures have a clear remediation path.
* **Reference Product Modes:** Ensure the installation and recovery docs mention how to verify and recover specific product modes if the operator is not using the default configuration.

**5. Risk Assessment**
**LOW** — Documentation changes carry low technical risk, though omitting specific architectural features could slightly degrade the operator experience.

---

### 06-03-PLAN.md: Lock runtime operator ops with cross-surface verification

**1. Summary**
This plan introduces a focused integration test to prove that the new deploy-run-recover summary and its associated data (health, locks, service states) remain correctly visible through the public control surfaces.

**2. Strengths**
* **Contract Enforcement:** Elevates operator guidance from "hope" to a verifiable contract via automated testing.
* **End-to-End Scope:** Tests the full vertical slice from workspace state to public API surface exposure.

**3. Concerns**
* **Test Flakiness (MEDIUM):** Tests dealing with runtime locks, managed-service states, and live beacons are notoriously prone to race conditions and flakiness, especially in CI environments.
* **Cleanup and Isolation (MEDIUM):** If the test fails midway, leftover locks or service states could poison subsequent integration tests or the local workspace environment.

**4. Suggestions**
* **Strict Teardown:** Ensure the integration test uses a robust `Drop` implementation or strict teardown mechanism to unconditionally clean up locks and mock service states, regardless of test pass/fail status.
* **Use Local Tmp:** Ensure the test harnesses utilize the existing temporary directory workflows (e.g., `scripts/use-local-tmp.sh`) to guarantee isolated execution contexts.

**5. Risk Assessment**
**LOW** — Adding tests is inherently low risk to the product, provided they are properly isolated so they don't break the CI pipeline.

---

## Claude Review

# Cross-AI Review: Phase 6 — Deployment, Runtime, and Operator Ops

## 06-01: Add a unified operator-ops runtime summary

### Summary
Solid plan that consolidates fragmented runtime primitives into a single typed summary exposed through CLI, control API, and browser UI. The scope is well-bounded and reuses existing infrastructure.

### Strengths
- Composes existing primitives rather than inventing new runtime machinery
- Covers all three operator surfaces (CLI, API, UI) in one plan
- Includes a rendering test for the Control UI panel
- Clear data flow: runtime.rs → start.rs endpoint → control_ui.html

### Concerns
- **MEDIUM** — No error/degraded-state handling specified. What does the summary show when health checks fail, the beacon is stale, or the service isn't installed? A summary that only works in the happy path undermines operator trust.
- **MEDIUM** — No schema or type definition sketched for the summary payload. Without at least field-level guidance, implementation could drift from what the UI and tests expect.
- **LOW** — The plan says "focused unit or integration tests" but doesn't specify whether these live in `runtime.rs` inline tests, `control_ui.rs`, or `tests/integration/`. Plan 03 covers integration, but plan 01's own verification step is vague.

### Suggestions
- Define the summary struct fields (health status, beacon age, service install state, lock holder, recovery action) even if just as a checklist.
- Specify degraded-state rendering: the summary should be *most* useful when something is wrong, not just when everything is green.
- Clarify that plan 01's verification is unit-level (struct construction, endpoint response shape) and plan 03 handles cross-surface integration.

### Risk Assessment
**LOW** — Well-scoped, reuses existing code, and the main risk (underspecified error states) is addressable during implementation without scope change.

---

## 06-02: Align deployment and recovery docs with the shipped runtime

### Summary
Documentation-only plan that rewrites operator-facing docs to match the actual shipped CLI commands. Necessary work that closes the gap between what the code does and what the docs claim.

### Strengths
- Targets three specific files at three different levels (README overview, install guide, production runbook)
- Explicitly scoped to shipped commands — no aspirational documentation
- Depends on 06-01, so docs can reference the new summary surface

### Concerns
- **MEDIUM** — Verification is "manual doc review plus command cross-checks." For a phase that claims production-ready operator ops, there's no mechanism to detect doc drift after this plan ships. A simple grep or link-check script would add durability.
- **LOW** — No mention of version-specific guidance. The `1.4.x` release line is live; docs should note which commands appeared in which version if operators might be on older binaries.
- **LOW** — The plan doesn't mention removing or deprecating the older broad deployment guidance it acknowledges exists. Leaving stale sections alongside new ones creates confusion.

### Suggestions
- Add a task to explicitly remove or collapse outdated deployment prose in `production.md` rather than just appending new content.
- Consider a lightweight doc-command cross-reference (even a comment block listing every `openrustclaw runtime ...` subcommand the doc references) so future edits can verify coverage.

### Risk Assessment
**LOW** — Documentation-only with clear scope. The main risk is incomplete cleanup of old content, which is cosmetic rather than functional.

---

## 06-03: Lock runtime operator ops with cross-surface verification

### Summary
Adds integration test coverage for the operator contract. Correctly positioned as the phase gate — if this test passes, the deploy-run-recover story is regression-protected.

### Strengths
- Explicitly tests the public surface, not internal helpers
- Depends on both prior plans, so it validates the full phase output
- "Realistic workspace-backed test" framing avoids mock-only coverage

### Concerns
- **HIGH** — The plan is extremely thin — one task, one file glob (`tests/integration/src`), no specifics on what "realistic workspace-backed" means. Does this require a running runtime process? A temp directory with `.claw/` state? Mocked control endpoints? Without answering this, the test could end up either trivial (just checking struct fields) or infeasible (requiring a full runtime boot in CI).
- **MEDIUM** — No mention of CI integration. If this test requires runtime process startup or port binding, it may not run in the existing GitHub Actions pipeline without setup changes — yet `user_setup: []` claims no setup is needed.
- **LOW** — Only one test is planned. A single integration test for the entire phase's operator contract is brittle; if it tests too much, failures become hard to diagnose.

### Suggestions
- Specify the test harness: does it call the summary function directly with synthetic state, or does it boot a control server and hit the HTTP endpoint? The former is achievable in CI; the latter needs explicit setup.
- Break into at least two test cases: one for the happy path (all systems healthy) and one for degraded state (e.g., stale beacon, missing service install).
- Confirm the test runs in the existing CI workflow without new infrastructure.

### Risk Assessment
**MEDIUM** — The verification plan is the weakest of the three. Its vagueness risks either producing a test that doesn't meaningfully protect the operator contract or requiring unexpected CI changes. This is the plan most likely to need revision during implementation.

---

## Overall Phase Assessment

**Overall Risk: LOW-MEDIUM**

The phase is well-scoped and achieves its stated goal: one coherent deploy-run-recover story across CLI, API, UI, docs, and tests. The dependency chain (01 → 02, 01 → 03) is correct. The main structural weakness is that Plan 03 (verification) is underspecified relative to the weight it carries as the phase gate. Tightening 03's test strategy before execution would meaningfully reduce phase risk.

The phase correctly defers cluster/HA/fleet concerns and stays within the single-node MVP boundary, which is the right call given the project's current state.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed the phase targets a coherent deploy-run-recover story, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for the concrete plan-specific risks.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed label: claude=LOW-MEDIUM. Gemini only provided plan-level risk assessments.
