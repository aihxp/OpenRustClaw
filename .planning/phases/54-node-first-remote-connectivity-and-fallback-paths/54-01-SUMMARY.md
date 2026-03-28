# Summary 54-01: Persisted the Remote Connectivity Profile

## What Changed

- Added a serializable remote-connectivity profile to onboarding state and durable setup state.
- Gateway setup now captures a remote-connectivity profile when operators choose remote gateway guidance, with node-first as the recommended path and SSH tunnel or reverse proxy available as bounded fallback choices.
- The setup bootstrap ledger now records the remote-connectivity outcome, and the setup handoff report carries the saved profile forward for later inspection surfaces.

## Result

Phase 54 turns the node-first plus fallback contract into a real onboarding artifact. OpenRustClaw still does not pretend that remote node bootstrap is fully automated, but it now preserves the operator's chosen connectivity path and fallback order as part of the durable setup story.

## Follow-on

- Phase 55 can now render node and fallback health from the saved setup contract and operator evidence.
- Phase 56 can align the remaining docs and verification around the persisted remote-connectivity shape.
