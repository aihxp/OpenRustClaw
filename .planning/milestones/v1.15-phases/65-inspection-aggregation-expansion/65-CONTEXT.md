# Phase 65: Inspection Aggregation Expansion - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Move another real inspection or aggregation family into `openrustclaw-app` so `inspect.rs` keeps shrinking as a business-logic owner instead of staying the default report composition hub.

## What We Know

- `crates/cli/src/commands/inspect.rs` still owns several large operator-facing aggregation families after the `v1.14` migrations.
- The enterprise admin surface is the next best bounded candidate: it aggregates enterprise access, policy, autonomy, and active orchestration supervision into one shipped summary.
- That summary already feeds the `/control/enterprise/admin` route and Control UI, so it is a real shipped aggregation instead of an internal helper.
- Access, policy, and autonomy data still come from existing CLI-owned modules today; the greenfield win in this phase is to move the aggregation and supervision composition, not to rewrite every enterprise dependency at once.

## Constraints

- This phase must extract one real aggregation family, not attempt a broad rewrite of all enterprise summaries in `inspect.rs`.
- The shipped `/control/enterprise/admin` and Control UI contract must remain stable.
- The migration should preserve a truthful follow-up queue for the remaining inspection aggregates still owned by `inspect.rs`.

## Implementation Direction

- add an enterprise-admin aggregation service to `openrustclaw-app`
- move status, detail, and supervision composition for the enterprise admin summary behind that service
- keep `inspect.rs` as the adapter that loads enterprise access, policy, autonomy, and active orchestration state into the new service
