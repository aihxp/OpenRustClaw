# Phase 51: Crates.io Publish Path and Dry-Run Verification - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Establish and verify the crates.io publish workflow for the first public crate set, including dry-runs, publish ordering, and credential-sensitive operator steps.

## What We Know

- `openrustclaw-core` is the only crate in the first public publish boundary.
- The environment currently has no `CARGO_REGISTRY_TOKEN` and no `~/.cargo/credentials.toml`.
- That means actual crates.io publication is not yet possible from this workspace, but dry-run verification is still possible and necessary.
- The docs site currently has no page describing the Rust package publish path.

## Constraints

- The publish path must be explicit about credential requirements.
- The first operator runbook should not imply that more than one crate is in scope.
- Dry-run evidence must be preserved even if live publish is blocked.

## Implementation Direction

- add one rerunnable crates.io readiness script for `openrustclaw-core`
- document the exact preflight, publish, and post-publish checks in the docs site
- verify the dry-run publish path locally and preserve the credential blocker truthfully for the final phase
