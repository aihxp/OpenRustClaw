# Greenfield Transition Contract

**Created:** 2026-03-28
**Purpose:** Canonical boundary and migration contract for `v1.13 Brownfield-to-Greenfield Transition`

## Why This Exists

OpenRustClaw already ships a broad product surface. The next architecture problem is not missing features; it is that too much new behavior still wants to land in oversized command and control-plane modules.

This contract defines how the repo moves toward a greenfield-style architecture without pretending the current brownfield product can be rewritten all at once.

## Target Architecture Lane

### Layer Model

| Layer | Responsibility | Allowed Dependencies | Must Not Depend On |
|------|----------------|----------------------|--------------------|
| `openrustclaw-core` and shared domain crates | domain types, traits, config, persistence primitives | workspace foundations and external libraries | CLI rendering, HTTP handlers, control UI details |
| `greenfield application lane` | use-case orchestration, report composition, service interfaces, migration-safe business logic | domain crates and selected capability crates | direct clap command parsing, HTML rendering, route registration |
| adapters and presentation | CLI commands, `/control/...` handlers, control UI renderers, transport-specific bindings | domain crates plus greenfield application services | deep cross-calls into unrelated legacy command modules |
| legacy containment zone | oversized mixed-surface modules kept for compatibility while migration proceeds | existing behavior only, adapter calls into new services allowed | new default feature logic that could live in the greenfield lane |

### Current New Entry Point

The greenfield application shell now lives in `crates/app` as `openrustclaw-app`.

From this milestone forward, new behavior should be added by:

1. defining or extending a domain contract in the existing shared crates if needed
2. implementing the use case in the new application shell
3. wiring legacy CLI, route, or UI adapters to that new service

The default should not be “put more logic into `start.rs`, `mobile.rs`, `skills.rs`, or `inspect.rs`.”

## Brownfield Containment Rules

### Legacy Hotspots

| Surface | Current Signal | Containment Rule |
|---------|----------------|------------------|
| `crates/cli/src/commands/start.rs` | control-plane and runtime route concentration | treat as route-registration and adapter surface; new business logic must move outward first |
| `crates/cli/src/commands/mobile.rs` | mixed node control, reporting, and command logic | allow compatibility fixes only unless migration work is active |
| `crates/cli/src/commands/skills.rs` | mixed skill management and control-plane behavior | no new cross-cutting feature logic without extracting a service seam |
| `crates/cli/src/commands/runtime.rs` | mixed runtime mutation and reporting logic | prefer new runtime-facing services instead of extending command-local helpers |
| `crates/cli/src/commands/inspect.rs` | report composition and inspection aggregation hub | treat as the primary candidate adapter into new report services |

### No-Touch Without Explicit Migration Scope

- auth and origin enforcement
- enterprise approval and governance behavior
- setup and onboarding persistence semantics
- archived milestone artifacts and verification evidence

## Ranked Migration Inventory

| Priority | Slice | Current Files | Why It Matters | Target Phase |
|---------|-------|---------------|----------------|--------------|
| 1 | setup handoff reporting | `onboard.rs`, `inspect.rs`, `start.rs`, `control_ui.html`, `control_ui.rs` | already spans durable setup state, one control route, and one shipped operator panel with focused tests | Phase 59 |
| 2 | inspection summary composition | `inspect.rs` plus route handlers in `start.rs` | repeated typed reports can move behind common service boundaries | follow-up after Phase 59 |
| 3 | control route families | `start.rs` route groups and handlers | largest control-plane coupling hotspot in the repo | follow-up |
| 4 | mobile operator reporting | `mobile.rs`, `inspect.rs`, `control_ui.html` | high-value operator surface with heavy mixed responsibilities | follow-up |
| 5 | skills command surface | `skills.rs` | large command hub with future contributor risk | follow-up |

## First Proving Slice

### Selected Slice

`Setup handoff reporting` is the first proving slice for the greenfield lane.

### Why This Slice

- it is real shipped behavior, not a fake demo surface
- it crosses multiple seams already: setup state, report composition, route exposure, and UI rendering
- it has existing targeted regression coverage:
  - `setup_handoff_summary`
  - `dashboard_includes_setup_handoff_panel`
- it is small enough to migrate without reopening the entire control-plane or mobile surface

### Desired Target Shape

The migrated slice should look like:

1. setup-state domain data remains in onboarding or shared setup modules
2. a greenfield application service builds the handoff report
3. route handlers and CLI surfaces call that service
4. Control UI renders the returned report without owning the business rules

## Review Defaults

When reviewing new work during `v1.13`:

- ask first: “can this live in the greenfield application lane?”
- reject new deep logic in legacy command hubs unless the change is explicitly compatibility-only
- prefer extracting one stable interface over adding one more helper to a hotspot file
- require every migration to preserve the current verification bundle before widening scope

## Verification Bundle

Use this bundle to confirm the chosen proving slice stays stable while the architecture changes around it:

```bash
cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture
cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture
mdbook build docs
```

---
*Update this file when migration targets, proving slices, or containment rules change.*
