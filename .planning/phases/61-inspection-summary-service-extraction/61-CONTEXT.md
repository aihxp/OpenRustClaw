# Phase 61: Inspection Summary Service Extraction - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move broader inspection-summary composition into `openrustclaw-app` so `inspect.rs` stops acting as the long-term business-logic owner for typed operator summaries.

## What We Know

- `v1.13` already migrated setup handoff reporting into `openrustclaw-app`, so the next truthful step is another bounded summary family rather than a broad inspection rewrite.
- `self_hosted_product_mode_summary` is already operator-visible through runtime routes and Control UI, and it has focused tests in `inspect.rs`.
- The current summary mixes state loading, transition-target calculation, warning collection, and operator-facing detail composition inside `inspect.rs`.
- The underlying persisted state and transition events still live in `self_hosted.rs`, which can remain the adapter-facing source for this phase.

## Constraints

- The migrated summary must preserve the existing runtime and Control UI contract.
- This phase should reduce business-logic ownership inside `inspect.rs` without broadening scope into control-route extraction yet.
- Persisted product-mode storage remains in the CLI command module for now; this phase is about report composition ownership.

## Implementation Direction

- add a self-hosted product-mode service module to `openrustclaw-app`
- move report composition and transition-target derivation into the new service
- keep `inspect.rs` as a thin adapter that maps saved product-mode state into the application service
