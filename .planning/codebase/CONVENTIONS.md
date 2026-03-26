# Coding Conventions

**Analysis Date:** 2026-03-26

## Naming Patterns

**Files and modules:**
- Rust modules use snake_case file names such as `origin_check.rs`, `memory_tools.rs`, `tool_factory.rs`
- Command surfaces are grouped by domain in `crates/cli/src/commands/*.rs`
- Provider/channel/platform modules are usually one file per integration
- Test-heavy modules often keep `mod tests` in the same file rather than splitting unit tests into separate files

**Functions and types:**
- Functions, variables, and module names are snake_case in Rust
- Public structs, enums, and traits use PascalCase
- Constants are UPPER_SNAKE_CASE, for example `DEFAULT_VAULT_PATH` in `crates/cli/src/commands/runtime.rs`
- Traits are used heavily for stable subsystem boundaries, e.g. `LlmProvider`, `Tool`, `MemoryStore`, `Channel` in `crates/core/src/traits.rs`

## Code Style

**Formatting:**
- `cargo fmt` is enforced in CI via `.github/workflows/ci.yml`
- `cargo clippy -- -D warnings` is also enforced in CI
- Rustdoc-style module comments are common at file tops using `//!`
- Imports are usually grouped with std imports first, then third-party crates, then local crate imports

**Lint / compile posture:**
- CI sets `RUSTFLAGS: "-D warnings"`
- Workspace code tends to compile under a strict warning-free standard

## Import and Module Organization

**Order:**
1. `std` imports
2. external crate imports
3. internal crate/module imports

**Patterns:**
- `pub mod ...` and `pub use ...` are used to build small public facades, for example in:
  - `crates/gateway/src/lib.rs`
  - `crates/agent/src/lib.rs`
  - `crates/cli/src/lib.rs`

## Error Handling

**Primary pattern:**
- Use `Result`-returning functions, often with `anyhow::Result` or crate-local result aliases
- Convert errors at CLI or HTTP boundaries instead of panicking in runtime code
- Use serde/typed structs for request and response boundaries

**Observed exceptions:**
- Test code uses many `unwrap()` / `expect()` calls
- Some source files also contain `expect()` / `unwrap()` in places that deserve care during edits, especially large command modules and SDK wrappers

## Logging and Observability

**Framework:**
- `tracing` is the shared logging/tracing path
- OpenTelemetry and Prometheus are first-class observability dependencies

**Patterns:**
- Operational/runtime surfaces tend to favor structured state over ad hoc stdout
- CLI commands still mix human-readable output with typed JSON-style reporting in some areas

## Comments and Documentation

**Patterns:**
- File-level `//!` comments are common and usually explain subsystem purpose
- README and docs coverage is broad: root `README.md`, crate READMEs, `docs/src/`, ADRs
- This repo treats docs as part of the maintained surface contract, not just ancillary notes

## Function and Module Design

**Observed design norms:**
- Small crates expose narrow public surfaces through `lib.rs`
- Large command surfaces collect many related handlers in one file rather than splitting deeply
- Traits and crate boundaries are the main abstraction strategy; implementation details sit behind them
- Async functions do not use naming prefixes like `async_`; async behavior is inferred from signature

## Testing Conventions

- Unit tests usually live inline under `mod tests`
- Broader behavior is covered by `tests/integration/` and `tests/e2e/`
- `#[tokio::test]` is the dominant async test pattern
- Several crates also maintain their own `tests/integration_tests.rs`

## Practical Guidance for Edits

- Match the existing crate/module split before creating new top-level abstractions
- Prefer extending trait-backed layers over bypassing them directly
- Keep operator/config/runtime paths typed and serializable
- Be cautious when touching very large subsystem files because local conventions may be file-specific

## High-Signal Paths

- `crates/core/src/traits.rs`
- `crates/gateway/src/lib.rs`
- `crates/agent/src/lib.rs`
- `crates/cli/src/main.rs`
- `.github/workflows/ci.yml`

---
*Convention analysis: 2026-03-26*
*Update when formatting, CI enforcement, or abstraction patterns materially change*
