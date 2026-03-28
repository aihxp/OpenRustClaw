# Plan 64-01 Summary: Extract the First Stable Service Seam from `skills.rs`

## Result

Passed. The read-only compiled-skill overview lane now runs through `openrustclaw-app` instead of being split between `skills.rs` and `start.rs`.

## What Changed

- added a compiled-skill overview service to `openrustclaw-app`
- moved compiled manifest loading, artifact loading, executable-component derivation, and compiled reference reading behind that shared service
- reduced `crates/cli/src/commands/skills.rs` to the CLI adapter for compiled-skill detail and preview flows
- reduced the compiled-skill MCP/runtime path in `crates/cli/src/commands/start.rs` to the adapter that exposes the same shipped tool and reference surface
- updated contributor guidance so future compiled-skill overview work lands behind the new seam instead of deepening `skills.rs`

## Evidence

- `crates/app/src/compiled_skill_overview.rs`
- `crates/cli/src/commands/skills.rs`
- `crates/cli/src/commands/start.rs`
- `cargo test -p openrustclaw-app compiled_skill_overview -- --nocapture`
- `cargo test -p openrustclaw-cli test_invoke_compiled_skill_includes_reference_preview -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_exposes_compiled_skill_tools -- --nocapture`
