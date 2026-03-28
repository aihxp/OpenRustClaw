# Phase 50: Docs.rs Documentation Surface - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Make the selected public crate render intentionally on docs.rs with a clear docs build contract and a useful public landing page.

## What We Know

- Phase 49 selected `openrustclaw-core` as the first public crate target.
- `openrustclaw-core` currently has module docs, but its crate root rustdoc is only a one-line summary.
- docs.rs supports crate-specific build tuning through `[package.metadata.docs.rs]`.
- The first docs.rs phase should stay simple and truthful: one default target, one clear crate-level entry page, and one local verification path.

## Constraints

- The docs.rs contract should not rely on unpublished internal crates.
- The crate root docs should reflect the current public crate surface rather than the full product marketing story.
- Local verification should stay close to the docs.rs build path.

## Implementation Direction

- add an explicit docs.rs metadata table to `openrustclaw-core`
- turn the crate root docs into a useful landing page with module map and a working example
- verify both standard rustdoc generation and the docs.rs-style `--cfg docsrs` path
