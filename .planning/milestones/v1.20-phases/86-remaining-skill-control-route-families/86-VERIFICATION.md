---
phase: 86
verified: 2026-03-28
status: passed
score: "3/3 must-haves verified"
---

# Phase 86 Verification

## Must-Haves

1. The remaining skill-control route family is composed through `openrustclaw-app`.
2. `start.rs` is the HTTP adapter for those skill-control surfaces instead of owning orchestration directly.
3. Verification proves the shipped skill-control contract remains truthful.

## Evidence

- `crates/app/src/skill_control.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app skill_control -- --nocapture`
- `cargo test -p openrustclaw-cli remaining_skill_control_route_family_uses_service_lane -- --nocapture`

## Result

Passed. OpenRustClaw now routes the remaining non-voice-call skill-control family through `openrustclaw-app`, while `start.rs` remains the HTTP adapter for the shipped runtime skill-control surfaces.
