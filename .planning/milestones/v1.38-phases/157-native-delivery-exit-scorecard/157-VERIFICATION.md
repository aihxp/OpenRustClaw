# Verification 157: Native Delivery Exit Scorecard

## Commands

```bash
cargo metadata --no-deps --format-version 1
wc -l crates/cli/src/main.rs crates/cli/src/commands/mod.rs crates/gateway/src/lib.rs crates/mcp/src/lib.rs crates/app/src/*.rs
rg -n "mod commands|pub mod|fn main|subcommand|Commands|clap::|Parser|Subcommand" crates/cli/src/main.rs crates/cli/src/commands/mod.rs crates/gateway/src/lib.rs crates/mcp/src/lib.rs
```

## Result

Passed. The scorecard is grounded in the live source tree and keeps the final claim truthful.
