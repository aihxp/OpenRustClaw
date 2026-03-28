# Phase 89: Mobile Notification and Dispatch Lifecycle Services - Context

**Gathered:** 2026-03-28
**Status:** Completed
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the mobile notification, outbound message, dispatch, approval, wake, and rehydrate lifecycle lanes out of `mobile.rs` so those operator mutation flows stop depending on command-local orchestration.

</domain>

<decisions>
## Implementation Decisions

- Keep workspace file I/O and runtime execution in `crates/cli/src/commands/mobile.rs`.
- Move lifecycle state transitions and command-timeline composition into `openrustclaw-app`.
- Preserve the existing CLI and control-plane contracts while reducing command-local orchestration.

</decisions>
