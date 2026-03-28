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

### Current Phase 59 Outcome

That target shape is now real for the first proving slice:

1. durable setup state still lives in the CLI onboarding module
2. `openrustclaw-app` now owns setup handoff report composition
3. `inspect.rs` acts as the compatibility adapter into that service
4. `start.rs` and Control UI still consume the same report contract

### Current Phase 61 Outcome

The greenfield lane now also owns a second typed operator summary:

1. persisted self-hosted product-mode state still lives behind the existing CLI adapter module
2. `openrustclaw-app` now owns self-hosted product-mode report composition
3. `inspect.rs` adapts saved product-mode state, warnings, and transition receipts into that service
4. runtime and Control UI still consume the same self-hosted product-mode report contract

### Current Phase 62 Outcome

The greenfield lane now also owns the first bounded `start.rs` control-route family:

1. persisted self-hosted product-mode storage and transition receipts still live in the existing CLI adapter module
2. `openrustclaw-app` now owns the product-mode transition-and-report use case
3. `inspect.rs` remains the workspace adapter that bridges persisted state into that new service seam
4. `start.rs` now acts as the HTTP adapter for the `/control/self-hosted/product-mode` transition path instead of orchestrating the business logic directly

### Current Phase 63 Outcome

The greenfield lane now also owns a second operator-facing report surface:

1. persisted mobile node state, metrics, and activity receipts still live behind the existing CLI mobile data helpers
2. `openrustclaw-app` now owns mobile node operator report composition and attention-signal derivation
3. `mobile.rs` acts as the workspace adapter that loads mobile state into that service
4. the `/control/mobile/nodes/{id}/summary` route and Control UI continue to consume the same shipped report contract

### Current Phase 64 Outcome

The greenfield lane now also owns the first bounded compiled-skill overview seam:

1. compiled skill storage, compilation, and mutation-heavy lifecycle behavior still live in the existing skills crate and CLI adapters
2. `openrustclaw-app` now owns compiled manifest loading, artifact loading, executable-component derivation, and compiled reference reading for the shared overview lane
3. `skills.rs` acts as the CLI adapter for compiled-skill detail and preview flows
4. `start.rs` acts as the MCP/runtime adapter for the same compiled-skill overview contract

### Current Phase 65 Outcome

The greenfield lane now also owns the enterprise admin aggregation:

1. enterprise access, policy, autonomy, and orchestration state still load through existing CLI-owned modules
2. `openrustclaw-app` now owns enterprise admin status, detail, and supervision composition
3. `inspect.rs` acts as the workspace adapter that maps enterprise state into that aggregation service
4. the shipped `/control/enterprise/admin` route and Control UI continue to consume the same enterprise admin contract

### Current Phase 66 Outcome

The greenfield lane now also owns the first enterprise access write-route family:

1. enterprise access persistence and protected-scope enforcement still live behind the existing CLI enterprise access module
2. `openrustclaw-app` now owns the bootstrap, operator-upsert, governance-rule-upsert, and report-reload orchestration for that route family
3. `start.rs` acts as the HTTP adapter that maps payloads into the shared service and records operator tool results
4. the shipped `/control/enterprise/access/bootstrap`, `/control/enterprise/access/operators`, and `/control/enterprise/governance/rules` surface continues to return the same enterprise access summary contract

### Current Phase 67 Outcome

The greenfield lane now also owns the skill registry mutation lane:

1. skill files, DB persistence, ClawHub registry access, compile attempts, and plugin-event publication still live behind the existing `skills.rs` adapter helpers
2. `openrustclaw-app` now owns install, update, and uninstall orchestration plus mutation-result shaping for that lane
3. `skills.rs` acts as the async adapter that maps workspace and registry primitives into the shared service
4. the shipped control and runtime mutation contract for install, update, and uninstall continues to return the same skill mutation shape

## Review Defaults

When reviewing new work during `v1.13`:

- ask first: “can this live in the greenfield application lane?”
- reject new deep logic in legacy command hubs unless the change is explicitly compatibility-only
- prefer extracting one stable interface over adding one more helper to a hotspot file
- require every migration to preserve the current verification bundle before widening scope

## Default Contribution Checklist

Before adding new logic:

1. decide whether the change is domain, application, adapter, or compatibility-only work
2. if it is application work, default to `openrustclaw-app`
3. if it touches a legacy hotspot, note why that hotspot is still the right place
4. preserve the proving-slice verification bundle before widening the migration

## Next Migration Queue

Unless a future milestone reprioritizes it, the preferred migration order after the setup handoff proving slice is:

1. broader inspection summary composition
2. selected `start.rs` route families
3. mobile operator reporting
4. `skills.rs` decomposition

After `v1.14`, the next `skills.rs` migration queue should start from the remaining mutation-heavy lanes rather than reopening the compiled overview slice:

1. compiled-skill mutation and removal flows
2. install and registry workflows
3. auth-plugin and voice-plugin lifecycle helpers

After Phase 67, the next ranked greenfield queue is:

1. a bounded runtime command seam outside report composition
2. the remaining plugin-binding and voice-plugin mutation lanes in `skills.rs`
3. another bounded route family only if it materially improves the remaining migration path

## Verification Bundle

Use this bundle to confirm the chosen proving slice stays stable while the architecture changes around it:

```bash
cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture
cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture
mdbook build docs
```

---
*Update this file when migration targets, proving slices, or containment rules change.*
