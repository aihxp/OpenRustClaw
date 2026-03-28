# Phase 70: Runtime Vault and Secret Mutation Boundary - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move one real runtime vault mutation seam behind the greenfield lane so `runtime.rs` stops owning more high-risk secret mutation logic directly.

## What We Know

- `crates/cli/src/commands/runtime.rs` still owns the vault mutation path inline through `set_vault_secret(...)` and `delete_vault_secret(...)`.
- Those helpers power the shipped CLI and the `/control/runtime/vault/{key}` control API, so this is a real runtime mutation surface instead of an internal-only helper.
- The greenfield win here is to move vault mutation rules and report shape into `openrustclaw-app` while keeping `runtime.rs` as the adapter around vault file loading and persistence.

## Constraints

- This phase must extract a real runtime vault mutation seam, not just rename helpers inside `runtime.rs`.
- The shipped runtime vault mutation contract must stay stable for both CLI and control API callers.
- The migration should leave a truthful follow-on path for the route-family extraction that will consume this seam later in the milestone.

## Implementation Direction

- add a runtime vault service to `openrustclaw-app`
- move vault mutation state transitions behind that service
- keep `runtime.rs` as the adapter that loads and saves the workspace vault file
