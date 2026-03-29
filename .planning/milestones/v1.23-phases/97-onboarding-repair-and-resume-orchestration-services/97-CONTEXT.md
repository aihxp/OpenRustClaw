# Phase 97: Onboarding, Repair, and Resume Orchestration Services - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Move onboarding, repair, and resume orchestration business logic out of `onboard.rs` so setup lifecycle flows compose through `openrustclaw-app` instead of command-local orchestration.

</domain>

<decisions>
## Implementation Decisions

- Keep setup-state persistence, workspace I/O, and bounded runtime probing in `crates/cli/src/commands/onboard.rs`.
- Move setup-step selection, repair-plan derivation, bootstrap-outcome shaping, and handoff reporting into a dedicated `openrustclaw-app` service.
- Preserve the current CLI and control-surface setup contract while reducing setup-transition business rules in the legacy command module.

</decisions>
