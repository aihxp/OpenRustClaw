# Phase 66: Additional Route Family Extraction - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Reduce direct command-local orchestration inside another bounded runtime or `/control/...` route family by moving it behind a cleaner application-facing seam.

## What We Know

- `crates/cli/src/commands/start.rs` still owns several route families that both mutate durable state and reload typed summaries inline.
- The enterprise access write family is the next best bounded candidate: three shipped routes bootstrap the registry, upsert operators, and upsert governance rules, then immediately reload the same enterprise access summary.
- Those routes already expose one stable typed report contract through `inspect::enterprise_access_summary(...)`, so the migration can move the mutation-and-report orchestration without changing the HTTP payloads or returned summary shape.
- The real greenfield win in this phase is to move the use-case boundary out of `start.rs`, not to rewrite enterprise access storage or the summary composition that already lives elsewhere.

## Constraints

- This phase must extract one real route family, not broadly refactor unrelated enterprise handlers in `start.rs`.
- The shipped `/control/enterprise/access/bootstrap`, `/control/enterprise/access/operators`, and `/control/enterprise/governance/rules` behavior must remain stable from the runtime API perspective.
- The migration should preserve a truthful follow-up queue for the remaining `start.rs` route families still owning mutation-heavy orchestration.

## Implementation Direction

- add an enterprise-access control service to `openrustclaw-app`
- move the bootstrap, operator-upsert, and governance-rule-upsert orchestration behind that service
- keep `start.rs` as the HTTP adapter that accepts payloads, records operator tool results, and returns the same enterprise access summary contract
