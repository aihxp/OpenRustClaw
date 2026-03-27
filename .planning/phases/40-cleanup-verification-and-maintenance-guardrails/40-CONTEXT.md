# Phase 40: Cleanup Verification and Maintenance Guardrails - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning and execution
**Mode:** Autonomous cleanup closeout

<domain>
## Phase Boundary

This phase makes the cleanup durable. The goal is to leave behind a rerunnable verification bundle and one maintained debt record so later milestones do not have to rediscover the cleanup contract from scratch.

</domain>

<decisions>
## Implementation Decisions

### add a dedicated repo-hygiene script
The cleanup guardrail should be reusable outside this milestone summary, so it belongs in a small script rather than only in phase notes.

### wire the hygiene script into CI
If the repo drifts back to deleted docs or local-artifact ambiguity, CI should catch that immediately.

### keep remaining debt in the cleanup contract
The existing cleanup inventory already owns the hotspot list, so remaining debt should stay there instead of creating another parallel file.

</decisions>

<code_context>
## Existing Code Insights

- Phase 38 already fixed the current CI/docs drift and sidecar hygiene boundaries.
- Phase 39 already created a measurable `start.rs` extraction and a focused verification path.
- The missing piece is one explicit rerun bundle and CI enforcement for the cleanup contract.

</code_context>

<specifics>
## Specific Ideas

- add `scripts/check-repo-hygiene.sh`
- run it from CI
- document the verification bundle in `.planning/codebase/CLEANUP.md`

</specifics>

<deferred>
## Deferred Ideas

- broader cargo or clippy guardrails over future cleanup slices
- additional runtime/sidecar contract tests beyond this bounded milestone

</deferred>
