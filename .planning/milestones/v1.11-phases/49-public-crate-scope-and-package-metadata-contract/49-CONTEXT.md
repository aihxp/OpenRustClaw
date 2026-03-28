# Phase 49: Public Crate Scope and Package Metadata Contract - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Define the first publishable OpenRustClaw crate set and normalize the metadata and package boundaries needed for a truthful crates.io surface.

## What We Know

- The workspace contains a large mixed set of internal crates, tests, benches, runtime binaries, and SDK-style crates.
- `openrustclaw-core` is the clearest first public target because it has no internal path-crate dependencies and already includes a crate-local `README.md`.
- `cargo package -p openrustclaw-core --allow-dirty --no-verify` already succeeds, but Cargo warns that the crate is missing `documentation`, `homepage`, and `repository` metadata.
- The workspace repository metadata still points at the stale `openrustclaw/openrustclaw` URL instead of the live `aihxp/OpenRustClaw` repo.

## Constraints

- The first crates.io surface must be truthful and supportable.
- We should not imply that every workspace member is ready for public publication.
- Metadata changes should align with the shipped self-hosted product story and current repo ownership.

## Implementation Direction

- lock the first public crate set to `openrustclaw-core`
- normalize workspace and crate metadata around the live GitHub repo and intended docs.rs surface
- capture the non-target workspace crates as internal-only for this first publication wave in phase evidence and milestone docs
