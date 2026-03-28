# Phase 56: Node Docs, Onboarding, and Verification Exit - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Close the milestone by aligning onboarding, docs, and verification around the supported local and remote node connectivity contract.

## What We Know

- The core contract, setup-state persistence, and setup handoff rendering are already in place from Phases 53-55.
- The remaining risk is doc drift: installation and quickstart still need to call out the saved remote-connectivity profile and the setup handoff surface explicitly.
- The verification bundle for this closeout should keep the docs build plus the setup-handoff surface checks together.

## Constraints

- The closeout must stay truthful that advanced remote connectivity remains operator-managed.
- We should not introduce another parallel docs story; the existing getting-started and deployment guides should converge on the same contract.

## Implementation Direction

- align installation and quickstart with the remote-connectivity guide and setup handoff
- keep the canonical remote-connectivity guide in sync with what onboarding now saves
- close the phase with docs build and setup-handoff surface verification
