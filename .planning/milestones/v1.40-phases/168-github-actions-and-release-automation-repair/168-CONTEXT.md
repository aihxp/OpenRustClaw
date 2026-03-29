# Phase 168: GitHub Actions and Release Automation Repair - Context

**Gathered:** 2026-03-29
**Status:** Completed

<domain>
## Phase Boundary

Repair or intentionally retire failing GitHub Actions and release-automation lanes until the shipped CI and release surfaces are understandable and locally reproducible.

</domain>

<decisions>
## Implementation Decisions

### repo-owned verification
Prefer checked-in scripts over opaque action defaults when the project needs a standing exception policy or reproducible local verification path.

### keep useful lanes
Retire nothing if the lane is still meaningful and can be repaired truthfully.

### local parity first
Fix the workflow by making it reproducible locally before claiming GitHub Actions is repaired.

</decisions>

<code_context>
## Existing Code Insights

- The E2E workflow referenced `workflow_dispatch` conditions without actually declaring the trigger.
- The security-audit lane depended on the RustSec action without a repo-owned advisory policy, which diverged from the project’s actual accepted exceptions.
- Clippy and warning-sensitive lanes needed the underlying code cleanup from Phase 167 before the workflows could turn green.

</code_context>

<specifics>
## Specific Ideas

- Add `workflow_dispatch` to the E2E workflow.
- Replace the security audit action with a repo-owned script that encodes the accepted advisory policy.
- Re-run the repaired CI-equivalent checks locally.

</specifics>

<deferred>
## Deferred Ideas

- Full GitHub-side release creation, which remains blocked on invalid local `gh` authentication rather than repository source changes

</deferred>
