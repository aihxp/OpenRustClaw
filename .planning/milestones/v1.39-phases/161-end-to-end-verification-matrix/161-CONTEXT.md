# Phase 161: End-to-End Verification Matrix - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Exercise the shipped product end to end across its real entrypoints and operator-facing paths instead of relying on compile-only confidence. Use the repo's existing E2E and integration harnesses first, then fill any obvious gaps with direct product checks only where needed.

</domain>

<decisions>
## Implementation Decisions

### existing harness first
Use `tests/e2e` and `tests/integration` as the primary verification matrix. Those crates already cover gateway, memory, scheduler, MCP, onboarding, runtime operator ops, browser workflow history, mobile operator reporting, voice outcomes, and security posture.

### evidence before repair
Do not jump to fixes from compile warnings. Run the matrix first, capture actual failures, and only then decide whether a greenfield repair is needed.

### bounded environment truth
If a path is blocked by environment rather than code, record it explicitly instead of counting it as a silent pass.

</decisions>

<code_context>
## Existing Code Insights

- `tests/e2e` provides smoke, horizontal, vertical, and regression suites plus a standalone `e2e` runner.
- `tests/integration` provides fixture-backed coverage for gateway, MCP, onboarding, runtime operator ops, voice, mobile, memory, scheduler, and browser history.
- The current workspace still has native crates (`openrustclaw-app`, `openrustclaw-gateway`, `openrustclaw-mcp`) plus surviving CLI delivery surfaces.

</code_context>

<specifics>
## Specific Ideas

- Run `openrustclaw-e2e-tests` first to establish a top-level product matrix.
- Run `openrustclaw-integration-tests` next to widen coverage across shipped operator and gateway surfaces.
- If the matrix passes, Phase 162 can triage the result as “no product failures found.”

</specifics>

<deferred>
## Deferred Ideas

- Broad new test harness design
- Non-verification feature work

</deferred>
