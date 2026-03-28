# Phase 96: Browser Session Persistence, Workflow Execution, Inspection, and Sequence-Orchestration Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move browser session persistence, workflow execution, inspection, and sequence orchestration behind `openrustclaw-app` so the browser command surface approaches adapter-only ownership for its remaining execution lanes.

</domain>

<decisions>
## Implementation Decisions

- Keep browser automation runtime calls, session file reads or writes, and artifact creation in `crates/cli/src/commands/browser.rs`.
- Move session-record shaping, workflow-record composition, and workflow-history filtering into an app-side browser workflow service.
- Preserve the existing browser inspection and sequence payloads while reducing command-local bookkeeping logic.

</decisions>
