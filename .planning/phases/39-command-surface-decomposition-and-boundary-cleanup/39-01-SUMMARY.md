# Summary 39-01

Extracted the control-auth and enterprise-access middleware cluster from `crates/cli/src/commands/start.rs` into `crates/cli/src/commands/start/auth.rs` without changing the public router wiring in `run(...)`.
