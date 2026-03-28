# Phase 54: Node-First Remote Connectivity and Fallback Paths - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Make remote connectivity node-first while turning SSH tunnel usage into the primary fallback path and reverse proxy into a bounded last-resort fallback for self-hosted deployments.

## What We Know

- Phase 53 defined the current vocabulary and fallback order in shipped docs and onboarding copy.
- Onboarding already persists deployment mode, setup path, and bootstrap outcomes, so Phase 54 can extend that setup contract without inventing a separate state store.
- The full remote node bootstrap is not automated yet, so this phase should capture and expose the chosen remote-connectivity path truthfully instead of faking a fully provisioned node workflow.

## Constraints

- The implementation must stay truthful that advanced remote connectivity is still operator-managed.
- SSH tunnel and reverse proxy should be recorded as fallback paths, not silently promoted to the default topology.
- The setup contract should be inspectable later from the setup handoff surface.

## Implementation Direction

- persist a remote-connectivity profile in setup state
- let onboarding capture the preferred node-first path and fallback order when remote gateway guidance is selected
- record the resulting remote-connectivity bootstrap outcome in the same durable outcome ledger used by other setup steps
- expose the saved remote-connectivity profile through the setup handoff report for later operator surfaces
