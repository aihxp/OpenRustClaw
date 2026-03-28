# Summary 55-01: Surfaced Remote Connectivity in the Operator Dashboard

## What Changed

- The setup handoff panel now shows the saved primary remote path and fallback order.
- The same panel also renders the saved remote-connectivity detail text alongside blockers and pending-step guidance.
- The Control UI test contract now locks that remote-connectivity rendering path in place.

## Result

Phase 55 makes the saved node-first plus fallback contract visible in the shipped operator surface. Operators no longer need to infer the chosen path only from raw setup JSON or bootstrap outcome rows.

## Follow-on

- Phase 56 can align the remaining setup docs and milestone verification around the same setup handoff surface.
