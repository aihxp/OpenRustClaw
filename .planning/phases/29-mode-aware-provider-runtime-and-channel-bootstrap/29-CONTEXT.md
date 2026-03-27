# Phase 29: Mode-Aware Provider, Runtime, and Channel Bootstrap - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Phase 28 established durable setup state and resumable standard, advanced, and custom paths. Phase 29 has to make those paths truthful by connecting onboarding to the real provider, runtime, and channel readiness surfaces that OpenRustClaw already ships.

The goal is not to create another onboarding-only definition of "configured". The goal is to have setup write configuration, immediately validate what it can, and persist whether the affected runtime surface is actually ready, degraded, or blocked.

</domain>

<decisions>
## Implementation Decisions

### Use shipped health and probe surfaces instead of onboarding-specific heuristics
`runtime::runtime_health_status(...)`, `services::channel_probes_status(...)`, and existing runtime/control helpers already define the believable readiness story. Phase 29 should call those rather than inventing a second bootstrap contract.

### Persist bootstrap outcomes inside setup state
Phase 28 already made setup state durable. Phase 29 should extend that state with concrete bootstrap outcomes so later repair and handoff phases can reuse the same evidence.

### Treat unsupported bootstrap paths honestly
If onboarding can save configuration but cannot validate or fully bootstrap a selected surface, the outcome should be recorded as warning or blocked instead of being reported as silently complete.

</decisions>

<code_context>
## Existing Code Insights

- `crates/cli/src/commands/onboard.rs` currently saves provider and channel config but mostly treats a write as success.
- `crates/cli/src/commands/runtime.rs` already exposes `runtime_health_status(...)` and `runtime_service_install_status(...)`.
- `crates/cli/src/commands/services.rs` already exposes `channel_probes_status(...)` and persists probe reports under `.claw/control/`.
- `crates/cli/src/commands/self_hosted.rs` already defines mode descriptors and recommended runtime defaults for `solo`, `team`, `company`, and `enterprise`.
- `crates/cli/src/commands/doctor.rs` already defines the first-start readiness gate, so onboarding should converge on that contract rather than bypass it.

</code_context>

<specifics>
## Specific Ideas

- Add a typed bootstrap outcome list to setup state so onboarding can persist provider, runtime, and channel validation results.
- Validate the selected provider immediately after setup using runtime health instead of assuming the selected API key or Ollama choice is usable.
- Validate the configured channel immediately after setup using the shipped channel probe report and persist the result.
- Record runtime/control-plane alignment against the selected deployment mode so standard and advanced/custom paths converge into the same setup-state truth.

</specifics>

<deferred>
## Deferred Ideas

- Deep repair or reset flows belong to Phase 30.
- Final operator handoff messaging and broader docs/control-surface alignment belong to Phase 31.

</deferred>
