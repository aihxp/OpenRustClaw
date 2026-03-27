# Phase 41: GitHub About and Public Positioning Contract - Context

**Gathered:** 2026-03-27
**Status:** Ready for execution

## Goal

Replace stale GitHub repo framing with a public entry surface that matches the shipped self-hosted Rust-first product.

## What We Know

- The public repo page still shows an outdated hybrid Rust and Python framework description instead of the current self-hosted assistant product story.
- The README already reflects the newer product framing and is the best canonical base for public positioning.
- Live GitHub repo About and topics are external admin surfaces, so this phase needs both in-repo source-of-truth files and a repeatable sync path.

## Constraints

- GitHub About metadata and topics cannot be changed truthfully from git alone.
- Any live sync path must fail clearly when GitHub auth is missing or invalid.
- Public repo entry links and badges must point at current docs and release surfaces.

## Implementation Direction

- Add a canonical repo metadata file under `.github/`
- Add a repo-admin helper script that can validate local state and optionally check or apply live metadata
- Align README public entry links and badges with the current repo surface
- Attempt live GitHub sync only after the local contract is in place
