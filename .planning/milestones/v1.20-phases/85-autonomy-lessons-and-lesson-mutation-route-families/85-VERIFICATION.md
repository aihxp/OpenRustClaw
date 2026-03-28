---
phase: 85
verified: 2026-03-28
status: passed
score: "3/3 must-haves verified"
---

# Phase 85 Verification

## Must-Haves

1. The autonomy lesson route family is composed through `openrustclaw-app`.
2. `start.rs` becomes the HTTP adapter for lesson summary and lesson mutation flows instead of owning the business rules directly.
3. Verification proves the shipped lesson-control contract remains truthful.

## Evidence

- `crates/app/src/autonomy_lessons_control.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app autonomy_lessons_control -- --nocapture`
- `cargo test -p openrustclaw-cli autonomy_lessons_route_family_uses_service_lane -- --nocapture`

## Result

Passed. OpenRustClaw now routes the autonomy lesson-control route family through `openrustclaw-app`, while `start.rs` remains the HTTP adapter for the shipped lesson-control surfaces.
