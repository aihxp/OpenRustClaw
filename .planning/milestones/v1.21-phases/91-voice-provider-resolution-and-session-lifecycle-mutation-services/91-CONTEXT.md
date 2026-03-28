# Phase 91: Voice Provider Resolution and Session Lifecycle Mutation Services - Context

**Gathered:** 2026-03-28
**Status:** Completed
**Mode:** Autonomous

<domain>
## Phase Boundary

Move the voice provider resolution, session lifecycle, and state mutation lanes out of `voice_runtime.rs` so those voice runtime mutation flows stop depending on command-local orchestration.

</domain>

<decisions>
## Implementation Decisions

- Keep artifact synthesis, file persistence, and transcriber I/O in `voice_runtime.rs`.
- Move provider-resolution rules and session lifecycle mutations into `openrustclaw-app`.
- Preserve the shipped voice runtime request and response contracts.

</decisions>
