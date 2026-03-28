# Phase 68: Runtime Command Boundary Cleanup - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move one bounded runtime command seam behind a cleaner application or adapter boundary so the greenfield transition broadens beyond reports and route wrappers.

## What We Know

- `crates/cli/src/commands/runtime.rs` still owns the provider and model switch mutation lane inline.
- That lane mutates real operator config, maintains control-plane defaults, validates the selected provider, writes a timestamped config backup, and is reused by the shipped control API in `start.rs`.
- The provider or model switch path is a better bounded target than the larger backup, health, or upgrade planning surfaces because it is narrower and already exposes a stable mutation contract.
- The greenfield win here is to move the provider or model switch orchestration plus control-plane default logic into `openrustclaw-app` while keeping `runtime.rs` as the adapter around config loading, provider validation, and config persistence.

## Constraints

- This phase must extract one real runtime command seam, not just rename helpers inside `runtime.rs`.
- The existing runtime switch-provider and switch-model contract must stay stable for both CLI and control API callers.
- The migration should preserve a truthful follow-up queue for larger runtime seams like vault mutation, reload, backup, or upgrade planning.

## Implementation Direction

- add a runtime provider-switch service to `openrustclaw-app`
- move provider or model switch orchestration and control-plane default handling behind that service
- keep `runtime.rs` as the adapter that loads the effective config, validates provider support, and writes the updated config with backup
