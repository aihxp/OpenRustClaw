# Phase 34: Getting Started and Setup Guide Convergence - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 34 converges the getting-started path so installation, quickstart, and first-agent guidance all describe the same shipped setup lifecycle: choose a deployment mode, choose a setup depth, run onboarding, verify readiness, and continue with the persisted assistant and control surface.

</domain>

<decisions>
## Implementation Decisions

### Center onboarding as the setup spine
- Use `openrustclaw onboard` as the main path through installation and quickstart.
- Explain `Standard`, `Advanced`, and `Custom` setup consistently across the guides.

### Keep setup truth aligned with shipped repair and handoff surfaces
- Mention resume and repair paths instead of assuming a clean first run every time.
- Point readers to `openrustclaw doctor` and `Setup Handoff` in the Control UI.

### Replace speculative examples with shipped operator flows
- Avoid unsupported or unverifiable bespoke agent-build examples in the first-agent guide.
- Focus the first-agent guide on the first useful assistant workflow using the shipped runtime.

### the agent's Discretion
The exact structure of the three guides can be simplified as long as the onboarding, readiness, and handoff story stays consistent.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 28 through Phase 31 already established durable setup state, bootstrap outcomes, repair, and setup handoff.
- `README.md` and docs entry surfaces now point readers toward onboarding as the first step.

### Established Patterns
- The repo favors truthful setup claims and explicit operator visibility over “it should work” wording.
- Getting-started docs should link out to deeper guides instead of embedding entire advanced workflows.

### Integration Points
- `docs/src/getting-started/installation.md`
- `docs/src/getting-started/quickstart.md`
- `docs/src/getting-started/first-agent.md`

</code_context>

<specifics>
## Specific Ideas

- make installation end with onboarding and doctor rather than a vague “build and configure”
- make quickstart show the standard operator loop after setup
- reframe first-agent toward a first useful assistant workflow and link deeper extension docs

</specifics>

<deferred>
## Deferred Ideas

- deeper per-provider setup matrices
- advanced extension-authoring tutorials beyond the first-agent path

</deferred>
