---
phase: 189
requested_reviewers: [claude]
reviewers: [claude]
reviewed_at: 2026-04-09T21:05:34.662Z
plans_reviewed: [189-01-PLAN.md, 189-02-PLAN.md]
---
# Cross-AI Plan Review — Phase 189

## Claude Review

# Cross-AI Review: Phase 189 — Runtime Integration and Agent Journey Delivery

## 189-01: Bounded Delegated CLI Provider + Runtime Factory Wiring

### Summary
This plan adds a delegated CLI provider that plugs into the existing runtime provider factory, letting eligible local agent backends (Claude Code, Codex, Gemini CLI) execute through the same orchestration path as direct providers. The scope is well-bounded: policy evaluation before invocation, non-interactive default modes, and audit receipts through existing surfaces.

### Strengths
- Reuses `AgentBackendControlService.evaluate_execution` rather than inventing a parallel policy path
- Audit receipts flow through the existing `ExternalBackendAuditEntry` surface — no new inspection pane needed
- Defaults to strictest non-interactive mode (D-02), which is the right security posture for a first iteration
- Dependency on the already-shipped discovery and contract layer (Phase 188) is clean

### Concerns
- **HIGH**: The plan says "wire into the runtime provider factory" but doesn't specify how a delegated CLI backend maps to the `LlmProvider` trait. Local CLI wrappers have fundamentally different I/O (subprocess stdio vs HTTP streaming). If the trait contract assumes streaming token responses, the adapter will either block or fake streaming. The plan should name the trait boundary and describe the adaptation strategy.
- **MEDIUM**: No mention of timeout or resource limits on subprocess execution. A hung CLI process could block the runtime indefinitely. Even a simple `tokio::time::timeout` wrapper should be specified.
- **MEDIUM**: The `evaluate_execution` path currently returns no audit entry on success (line 256-260 of `agent_backend_control.rs`). If delegated runtime calls must "leave durable audit receipts," the success path also needs to write an entry. The plan's must-have truth contradicts the current implementation.
- **LOW**: `command_env_allowlist` exists in `AgentBackendPolicy` but the plan doesn't mention environment sanitization before spawning the subprocess. This is a security surface.

### Suggestions
- Specify whether the delegated provider implements `LlmProvider` directly or a new `DelegatedExecutionProvider` trait that the orchestration layer dispatches to
- Add an explicit subprocess timeout default (e.g., 300s) with operator override via config
- Fix the success-path audit gap: `evaluate_execution` should return an audit entry for allowed executions too, or the caller must write one post-execution
- Name the environment sanitization step: filter `std::env::vars()` through `command_env_allowlist` before `Command::new()`

### Risk Assessment
**MEDIUM** — The core approach is sound and well-scoped, but the trait-boundary ambiguity and missing success-path audit are real implementation gaps that could cause rework in wave 2.

---

## 189-02: Control-Registry Delegated Backend Profiles

### Summary
Extends `control init` to seed vendor-managed model-profile templates for eligible delegated backends, making delegated execution a visible operator-configurable lane. Depends on 189-01 for the runtime execution path.

### Strengths
- D-05 decision to use `vendor-managed` placeholders instead of guessing model names is honest and avoids stale catalog drift
- Builds on existing `control init` seeding logic rather than adding a new setup surface
- Keeps delegated backends inspectable through already-shipped audit and tool-execution surfaces (D-03)

### Concerns
- **MEDIUM**: The plan says delegated backends become "selectable" during control init, but doesn't describe what happens when an operator selects a vendor-managed profile and the backend isn't execution-eligible (e.g., `readiness != Ready`). The profile would exist but fail silently at runtime. There should be a validation step or warning during init.
- **MEDIUM**: No mention of how fallback chains work when a delegated backend is the primary in a model profile but becomes unavailable. Does the control registry fall back to a direct provider? The plan should at least acknowledge this or defer it explicitly.
- **LOW**: "Delegated runtime receipts remain inspectable" is listed as a must-have truth but is really a continuation of 189-01's audit surface. If 189-01's success-path audit gap isn't fixed, this truth is also false.

### Suggestions
- Add a readiness check during `control init` that annotates vendor-managed profiles with current backend readiness status
- Explicitly defer or describe fallback behavior when a delegated profile's backend goes offline
- Ensure the verification step includes a test where a non-ready delegated backend is referenced in a profile and the system produces a clear operator-visible error

### Risk Assessment
**LOW-MEDIUM** — Narrower scope than 189-01, and the main risk is inherited from the first plan's audit gap. The control-init extension itself is straightforward.

---

## Phase-Level Assessment

**Overall Risk: MEDIUM**

The phase goals are achievable and the dependency ordering is correct. The main structural risk is the under-specified trait boundary in 189-01 — how subprocess-based CLI execution maps to the existing provider abstraction will determine whether this is a clean integration or a forced adapter. The success-path audit gap should be fixed before either plan ships, since both plans' must-have truths depend on it.

The security posture is mostly sound (non-interactive defaults, policy-first evaluation), but environment sanitization and subprocess timeouts need explicit treatment given that this is spawning external processes with operator credentials.

---

## Consensus Summary

### Agreed Strengths
- Single-reviewer artifact: see the completed reviewer section above for the usable strengths signal.

### Agreed Concerns
- No cross-review consensus is available because only one reviewer completed successfully.

### Divergent Views
- No multi-reviewer comparison is available; parsed overall risk label: claude=MEDIUM.
