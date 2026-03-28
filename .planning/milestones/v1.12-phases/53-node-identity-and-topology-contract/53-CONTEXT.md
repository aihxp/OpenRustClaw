# Phase 53: Node Identity and Topology Contract - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Define one truthful contract for local runtime, distributed nodes, mobile nodes, and SSH-tunneled remote access so the product stops mixing transport, topology, and node terminology.

## What We Know

- OpenRustClaw already has a shipped mobile node surface and an advanced distributed crate, but the broader distributed lane is still described as gated in the shipped feature matrix.
- The current onboarding copy for remote gateway mode still tells operators to bring their own tunnel or reverse proxy, which leaves the remote-connectivity order implicit.
- The user wants the product direction to mimic OpenClaw's node-first approach, with SSH tunnel as the first fallback and reverse proxy as the third-tier last resort.
- The current production anchor is still one local runtime owner with protected `/control/...` surfaces.

## Constraints

- Phase 53 must stay truthful about what is already shipped versus what is still being hardened in later phases.
- We should clarify terminology before claiming an automated remote bootstrap path.
- The contract has to land in operator-facing docs and live onboarding copy, not only in planning artifacts.

## Implementation Direction

- add one canonical remote-connectivity guide that defines local runtime, mobile node, distributed node, SSH tunnel fallback, and reverse-proxy fallback
- update repo entry docs and production docs to point to the new contract
- align onboarding copy with the node-first, SSH-tunnel, reverse-proxy ordering without over-claiming that the bootstrap is fully automated yet
