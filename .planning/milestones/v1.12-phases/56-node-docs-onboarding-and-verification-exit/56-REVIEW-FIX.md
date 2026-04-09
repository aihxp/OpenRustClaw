---
status: fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 56 Code Review Fix

Updated the installation guide so the saved remote-connectivity profile uses the same fallback order
as the canonical deployment guide.

## Outcome

- `WR-01` fixed by inserting the Tailscale tailnet fallback before SSH tunnel in the onboarding
  guidance.

## Verification

- `mdbook build docs`
