# Phase 31: Setup Handoff and Operator Surface Alignment - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 31 closes the milestone by making the setup result legible after the wizard exits. The same durable setup state should drive the CLI handoff, the shipped operator surface, and the onboarding/setup docs so operators do not see three different versions of setup truth.

</domain>

<decisions>
## Implementation Decisions

### Reuse durable setup state as the handoff contract
Do not create a separate dashboard-only or docs-only summary model. The handoff status should be derived from setup state and its bootstrap outcomes.

### Close the loop in both CLI and Control UI
The end of onboarding is still a shipped operator surface, so the CLI should print an explicit handoff. Control UI should expose the same setup state for later review and repair follow-up.

### Fix stale wording while aligning docs
The milestone should remove stale wording like `QuickStart vs Advanced` and replace it with the current standard/advanced/custom plus repair story.

</decisions>

<code_context>
## Existing Code Insights

- `print_completion()` still prints a generic summary that does not reflect durable setup handoff state.
- `inspect.rs` already exposes typed summaries for other operator surfaces and is the right place for a setup handoff report.
- `start.rs` and `control_ui.html` already follow a simple summary-route plus dashboard-panel pattern.
- setup docs still describe the broader milestone partially, but at least one README bullet is stale relative to Phase 28+30.

</code_context>

<specifics>
## Specific Ideas

- add shared setup handoff helpers in onboarding so CLI and inspect can derive the same status
- expose `/control/setup/handoff` and render it in Control UI
- update quickstart, installation, and README wording to match the shipped setup contract

</specifics>

<deferred>
## Deferred Ideas

- any deeper setup analytics or onboarding telemetry remains out of scope for this milestone

</deferred>
