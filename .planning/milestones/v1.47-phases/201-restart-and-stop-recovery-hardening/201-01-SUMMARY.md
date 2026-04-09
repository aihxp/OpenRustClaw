---
phase: 201-restart-and-stop-recovery-hardening
plan: "01"
completed: 2026-04-09
one-liner: Hardened stop/restart recovery so stale beacons are cleared, beacon-backed OpenRustClaw ownership can be stopped, and restart reports listener conflicts before launching a doomed child runtime.
requirements-completed: [RUN-01, RUN-03]
---

# Phase 201 Plan 01 Summary

`restart_runtime_process` now probes the configured listener before relaunching, which turns the observed `restart launch exited early` symptom into the same classified conflict surface used by `start`. `stop_runtime_process` now clears stale runtime beacons, clears them alongside stale locks, and can stop a recoverable OpenRustClaw runtime from beacon-backed ownership when the lock record is gone.

## Verification

- `cargo test -p openrustclaw-cli stop_runtime_process_clears_stale_runtime_beacon_without_lock -- --nocapture`
- `cargo test -p openrustclaw-cli ensure_configured_listener_available_reports_foreign_process_conflict -- --nocapture`
- `cargo test -p openrustclaw-cli restart_runtime_process_reports_listener_conflict_before_launch -- --nocapture`

---

*Phase: 201-restart-and-stop-recovery-hardening*
*Completed: 2026-04-09*
