# Phase 42: GitHub Topics and Discovery Surface - Context

**Gathered:** 2026-03-27
**Status:** Ready for execution

## Goal

Make repo discovery metadata intentional and repeatable instead of ad hoc or stale.

## What We Know

- `.github/repository-metadata.json` already contains the topic set used for the live repo sync.
- The public repo now reflects those topics, but the docs do not yet spell out the canonical set clearly.
- The repo-admin helper already validates and reapplies topics, so the main remaining work is to make the discovery contract explicit and verifiable.

## Constraints

- Topics live on GitHub, not in git by default, so the in-repo contract must stay primary.
- Discovery tags should stay tied to shipped product claims, not growth or marketing keywords.

## Implementation Direction

- document the canonical topic set directly in `docs/github-repo-admin.md`
- keep `.github/repository-metadata.json` as the source of truth
- verify the live repo topics match the local contract
