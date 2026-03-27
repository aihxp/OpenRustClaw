# Phase 28: Setup State and Resumable Onboarding Contract - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

<domain>
## Phase Boundary

The current onboarding wizard already asks for deployment mode and a setup depth, but those choices only live in process memory. Re-running onboarding does not have a durable setup-state contract to resume from, inspect, or reason about.

This phase should:

- persist setup progress as first-class state under the existing control-plane workspace
- record deployment mode, deployment path, setup depth, selected steps, completed steps, blockers, and next action
- support resuming onboarding from durable setup state instead of treating every run as a clean start
- make the setup-depth choice explicit as a standard path plus an advanced or custom path

</domain>

<decisions>
## Implementation Decisions

### Reuse the workspace control root
The setup-state contract should live under `.claw/control/` alongside other operator-owned runtime state rather than inventing a second settings root.

### Preserve one setup story
Standard, advanced, and custom setup paths should all converge back into the same durable state contract. Custom should not become a separate onboarding system.

### Keep repair deeper work for later phases
Phase 28 should define and persist state plus enable resumability. Richer repair, reset, and operator surfaces can build on that contract in later phases.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` already has deployment-mode selection, QuickStart/Advanced branching, and existing-workspace detection, but the state is transient.
- `crates/cli/src/commands/doctor.rs` already treats onboarding-managed workspace state as part of first-start readiness.
- `crates/cli/src/commands/self_hosted.rs` already persists the deployment mode and onboarding path, so setup-state should complement rather than replace that contract.

</code_context>

<specifics>
## Specific Ideas

- add a durable `setup-state.json` manifest under `.claw/control/`
- persist status such as `in_progress`, `blocked`, or `ready`
- persist selected setup path as `standard`, `advanced`, or `custom`
- when onboarding reruns, offer resume if setup-state shows unfinished work
- make custom setup choose explicit steps and persist that step list

</specifics>

<deferred>
## Deferred Ideas

- dashboard rendering for setup-state
- richer repair actions beyond resume and reset-with-backup
- post-setup docs alignment beyond whatever minimal copy is needed to keep tests truthful

</deferred>
