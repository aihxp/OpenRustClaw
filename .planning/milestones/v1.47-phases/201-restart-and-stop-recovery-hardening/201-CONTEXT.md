# Phase 201 Context

## Title

Restart and Stop Recovery Hardening

## Goal

Make stop and restart reconcile runtime locks, recoverable stale processes, and listener reuse safely.

## Why This Phase Exists

Even with better conflict diagnostics, the runtime still has to recover from stale workspace state and avoid launching child `start` processes that immediately fail. `restart` should not turn a listener conflict into a vague early-exit error, and `stop` needs to clear stale runtime presence metadata cleanly.

## Expected Outputs

- `openrustclaw restart` probes the configured listener before relaunching and reports classified conflicts directly
- `openrustclaw stop` clears stale beacons alongside stale runtime locks
- Listener ownership recovery can stop a recoverable OpenRustClaw runtime even when the lock is gone but the beacon still proves workspace ownership
