# Phase 7: Security, Observability, and Release Exit - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the MVP with explicit secure-default posture, measurable runtime behavior, and an operator-facing release gate. This phase hardens the shipped auth/origin/skill-verification surfaces, the existing observability and metrics path, and the release budget/test tooling into one truthful release-exit contract.

</domain>

<decisions>
## Implementation Decisions

### MVP release-exit boundary
- **D-01:** Phase 7 should use the security, observability, and release primitives already present in the Rust runtime instead of inventing a new compliance framework.
- **D-02:** The MVP security story centers on gateway auth, control-plane auth, origin validation, trusted-proxy posture, skill verification, vault usage, and bounded execution policies.
- **D-03:** The MVP observability story centers on the shipped metrics, health, logs, runtime events, and operator diagnostics surfaces, not a full external monitoring product.
- **D-04:** Release exit means operators can answer three questions explicitly: are secure defaults enabled, are the key runtime signals observable, and which checks must pass before shipping.

### Operator trust and release gating
- **D-05:** Security posture should be inspectable from the same operator control surfaces used elsewhere in the milestone, not only from colored CLI output.
- **D-06:** The release gate should be written as a concrete checklist or report tied to existing commands, scripts, tests, and docs, not a vague "ready to ship" statement.
- **D-07:** Verification should prefer high-signal existing test suites and release-budget scripts instead of broad new test sprawl.

### the agent's Discretion
- The exact balance between security posture reporting, release documentation, and verification automation can move between plans as long as the phase ends with one credible release-exit story.
- It is acceptable to summarize current sandbox limitations truthfully if the release gate makes those limits explicit rather than pretending they are fully solved.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Security posture and controls
- `crates/cli/src/commands/security.rs` — current security audit CLI and skill-signing key generation
- `crates/cli/src/commands/start.rs` — runtime auth, origin validation, trusted proxy handling, metrics, and control-plane routes
- `crates/security/src/origin_check.rs` — origin validation logic
- `crates/security/src/skill_verifier.rs` — skill verification posture and signing support
- `crates/security/src/audit.rs` — audit event structure and logging

### Observability and release tooling
- `docs/src/operations/observability.md` — existing observability guide
- `scripts/check-runtime-budgets.sh` — release-time binary, startup, and memory budget gate
- `scripts/build-release-artifacts.sh` — packaged release artifact path

### Verification anchors
- `tests/integration/src/security_test.rs` — integration coverage for core security helpers
- `tests/e2e/src/test_security_workflow.rs` — end-to-end security workflow coverage
- `tests/e2e/tests/regression/test_security_boundaries.rs` — security boundary regression tests
- `README.md` and deployment docs — final operator-facing release claims

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `security.rs` already performs a security audit, but the result is CLI-only and not shaped as a typed control-plane or browser-visible posture summary.
- `start.rs` already enforces and records gateway auth, origin validation, trusted-proxy handling, and security metrics during real runtime execution.
- Observability surfaces are already shipped through `/metrics`, health endpoints, logs, runtime events, and the Control UI shell.
- Release tooling already includes bounded budget checks and release-artifact packaging scripts.
- E2E and integration coverage already include focused security boundary and workflow tests that can anchor the final release exit.

### Established Patterns
- Earlier phases improved trust by turning existing internal state into typed summaries first, then aligning docs and verification around that summary.
- The milestone has repeatedly used one concise operator loop per phase instead of broad feature essays.
- Phase 6 established the right closing pattern for runtime-oriented work: summary surface, docs alignment, then cross-surface verification.

### Gaps to Close
- Security posture is still fragmented between CLI output, config fields, and runtime behavior; there is no single shipped summary an operator can inspect in Control UI.
- Release exit is implied by scattered scripts and tests, but there is no explicit operator checklist that says what must pass before shipping MVP.
- Observability docs describe broad capabilities, but the release story still needs to call out the exact metrics, logs, health, and budget checks that define MVP readiness.

</code_context>

<specifics>
## Specific Ideas

- A strong first slice is a typed security posture summary that surfaces auth, origin validation, trusted-proxy config, skill verification state, and notable warnings through `/control/...` and Control UI.
- The release-exit docs should probably live as one operator-facing checklist tied directly to `security audit`, runtime budget checks, key test commands, and the shipped observability surfaces.
- Verification can stay lean if it reuses the existing security E2E coverage and release-budget tooling, then adds just enough glue to make the release gate explicit.

</specifics>

<deferred>
## Deferred Ideas

- Enterprise compliance packaging, RBAC, SSO rollout policy, and audit export remain outside this MVP closeout.
- Full security policy automation and centralized observability backends beyond the shipped metrics/logs path are deferred.
- Formal certification-style release processes are out of scope for the MVP milestone.

</deferred>

---
*Phase: 07-security-observability-and-release-exit*
*Context gathered: 2026-03-26*
