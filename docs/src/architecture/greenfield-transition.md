# Greenfield Transition

OpenRustClaw is not doing a rewrite-from-scratch reset. The repo already ships real operator and product surfaces, so the safer strategy is a **greenfield lane inside the current brownfield codebase**.

That means:

- keep the shipped product working
- stop deepening the largest legacy command hubs by default
- add a cleaner application layer for new behavior
- migrate one real slice at a time until the old seams shrink

## What Counts As Greenfield In This Repo

For `v1.13`, “greenfield” means new work should aim for this shape:

| Layer | Owns |
|------|------|
| shared domain crates | types, traits, config, persistence contracts |
| application lane | use-case orchestration, report composition, service interfaces |
| adapters | CLI commands, `/control/...` handlers, control UI rendering, transport-specific bindings |
| legacy compatibility | oversized command hubs that still exist but should stop growing by default |

The key rule is simple: **presentation and transport layers should call application services, not own the business rules themselves**.

The first concrete application shell now exists in `crates/app` as `openrustclaw-app`.

## Current Brownfield Containment Surfaces

The main legacy containment surfaces are:

- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/mobile.rs`
- `crates/cli/src/commands/skills.rs`
- `crates/cli/src/commands/runtime.rs`
- `crates/cli/src/commands/inspect.rs`

These files still hold shipped behavior, so they are not “bad” or forbidden. They are just no longer the preferred place for new cross-cutting logic.

## First Migration Target

The first proving slice is **setup handoff reporting**.

That slice currently spans:

- setup-state persistence in `onboard.rs`
- report composition in `inspect.rs`
- route exposure in `start.rs`
- UI rendering in `control_ui.html`

It is the right first slice because it is operator-visible, already tested, and small enough to migrate without destabilizing the whole control plane.

That migration is now in place: `inspect.rs` loads the durable setup state, maps it into `openrustclaw-app`, and returns the same setup handoff report contract to the runtime and Control UI surfaces.

## Contributor Defaults

When adding or changing behavior during this transition:

1. start by asking whether the logic belongs in a reusable application service
2. treat large command modules as adapters unless the task is explicitly a compatibility fix
3. avoid adding fresh cross-module helpers that deepen legacy coupling
4. keep the existing verification bundle green before expanding the migration

## Verification Baseline

The current proving-slice bundle is:

```bash
cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture
cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture
mdbook build docs
```
