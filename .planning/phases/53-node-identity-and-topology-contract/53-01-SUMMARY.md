# Summary 53-01: Defined the Node and Topology Contract

## What Changed

- Added a canonical `Remote Connectivity` guide that defines the current contract for local runtime, mobile nodes, distributed nodes, node-first remote access, SSH tunnel fallback, and reverse-proxy last-resort fallback.
- Updated the README, mdBook introduction and navigation, production runbook, and distributed crate README to point at the same topology language.
- Replaced the vague remote gateway onboarding wording with the explicit fallback order the milestone now uses.

## Result

Phase 53 closes the terminology gap before bootstrap work begins. OpenRustClaw now states clearly that the preferred remote direction is node-first, that SSH tunnel is the first fallback, and that reverse proxy is a bounded last-resort path rather than the default topology.

## Follow-on

- Phase 54 will turn the node-first plus fallback contract into a real remote bootstrap path.
- Phase 55 will surface node and fallback health through operator inspection.
