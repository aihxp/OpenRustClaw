# Application Service Boundaries

OpenRustClaw ships one product through several delivery surfaces: CLI, HTTP control routes, Control UI, MCP, and runtime workers. The codebase stays maintainable by keeping business rules in reusable application services and keeping transport layers focused on input parsing, output shaping, and workspace or runtime integration.

## Ownership Model

Use this split when adding or changing shipped behavior:

| Layer | Owns |
|------|------|
| shared domain crates | types, traits, config, persistence contracts |
| application services | use-case orchestration, report composition, policy enforcement, service interfaces |
| delivery adapters | CLI commands, `/control/...` handlers, Control UI bindings, MCP handlers, worker startup paths |
| compatibility surfaces | older command or bootstrap modules that still expose shipped entry points but should stay thin |

The key rule is simple: **delivery layers should call application services, not reimplement business rules themselves**.

`crates/app` provides the main shared application-service crate for this product surface.

## Important Delivery Surfaces

The largest adapter-heavy delivery surfaces remain:

- `crates/cli/src/commands/start.rs`
- `crates/cli/src/commands/mobile.rs`
- `crates/cli/src/commands/skills.rs`
- `crates/cli/src/commands/runtime.rs`
- `crates/cli/src/commands/inspect.rs`

These files still expose real shipped behavior. They are valid places for delivery-specific wiring, but they should not become the default home for new cross-cutting rules.

## Shipped Boundary Examples

The current product already applies this boundary model across operator-visible slices such as:

- setup handoff reporting
- self-hosted product-mode reporting and transitions
- mobile node operator summaries
- compiled-skill overview and runtime bindings
- enterprise admin aggregation
- enterprise access and governance route families
- skill install, update, uninstall, and background-service control paths
- runtime switch-provider and switch-model flows

Across those slices, application services compose the shared reports or decisions, while delivery layers adapt persisted state, requests, receipts, and response contracts.

## Contributor Defaults

When adding or changing shipped behavior:

1. start by asking whether the logic belongs in a reusable application service
2. keep delivery modules focused on transport, rendering, wiring, or persistence adaptation
3. avoid adding fresh cross-module helpers that deepen adapter coupling
4. preserve the current verification bundle before widening a shared service surface

## Verification Baseline

Use this baseline when touching the shared application-service boundary:

```bash
cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture
cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture
mdbook build docs
```
