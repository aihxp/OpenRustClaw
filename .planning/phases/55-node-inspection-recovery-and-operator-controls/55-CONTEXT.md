# Phase 55: Node Inspection, Recovery, and Operator Controls - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Surface remote-node connectivity, failover, tunnel state, reverse-proxy fallback state, and recovery evidence through shipped inspection and operator surfaces.

## What We Know

- Phase 54 now persists the chosen remote-connectivity profile and records a matching bootstrap outcome in setup state.
- The setup handoff report already exposes that saved profile and the retained bootstrap outcomes.
- The existing Control UI setup handoff panel is the most direct shipped operator surface for this information.

## Constraints

- We still do not have a fully automated live node-health service for remote deployments, so this phase must present the saved connectivity contract and bootstrap evidence truthfully.
- The operator surface should make primary path and fallback order visible without hiding blockers or warning-level bootstrap outcomes.

## Implementation Direction

- render the saved remote-connectivity profile directly in the setup handoff summary card
- show the saved primary path and fallback order alongside the retained bootstrap outcomes
- keep recovery guidance tied to setup detail and outcome evidence instead of inventing synthetic live health
