# Phase 200 Context

## Title

Runtime Ownership and Conflict Classification

## Goal

Make lifecycle commands classify listener ownership and stale-runtime state before failing on a busy port.

## Why This Phase Exists

The current operator failure is opaque: `openrustclaw restart` can fail early, and a follow-on `openrustclaw start` only reports that `127.0.0.1:18789` is busy. The runtime needs to tell operators whether that listener belongs to a healthy OpenRustClaw runtime, stale OpenRustClaw state, or another process entirely.

## Expected Outputs

- A listener-conflict diagnosis layer that distinguishes active OpenRustClaw ownership, stale OpenRustClaw state, and foreign-process conflicts
- `openrustclaw start` conflict output that uses the new classification instead of a generic busy-port message
- Runtime startup ordering that does not record a new lock owner before listener reservation succeeds
