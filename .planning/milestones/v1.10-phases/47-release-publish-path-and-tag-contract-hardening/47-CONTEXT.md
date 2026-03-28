# Phase 47: Release Publish Path and Tag Contract Hardening - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Make successful supported target builds flow cleanly into GitHub release asset publication.

## What We Know

- The build job artifact naming already matches the packaging script output: `dist/*.tar.gz` and `dist/*.sha256`.
- The publish job is intentionally tag-only and simply downloads, flattens, and uploads those artifacts to the GitHub release.
- The missing confidence today is not the publish step shape but whether the build matrix can finish and hand off artifacts cleanly on live runners.

## Constraints

- The published release should only claim targets that the live workflow can actually build on GitHub-hosted runners.
- The final validation must come from a real tag-triggered run, not only `workflow_dispatch`.

## Implementation Direction

- use the repaired workflow_dispatch run as the build-lane proof
- keep the publish job artifact handoff simple and aligned with `scripts/build-release-artifacts.sh`
- use the final milestone tag as the end-to-end publish validation point
