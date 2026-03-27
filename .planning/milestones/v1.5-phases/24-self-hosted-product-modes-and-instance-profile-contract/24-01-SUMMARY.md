# Summary 24-01: Add A Durable Self-Hosted Product-Mode Contract

- Added [self_hosted.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/self_hosted.rs) as a file-backed contract for the self-hosted open-source product modes.
- Defined four explicit deployment modes: `solo`, `team`, `company`, and `enterprise`.
- Persisted the selected product mode under `.claw/control/self-hosted/product-mode.json`, separate from runtime execution topology.
- Added unit coverage for manifest persistence, supported-mode validation, and default mode behavior.

Requirements completed: `MODE-01`, `MODE-02`
