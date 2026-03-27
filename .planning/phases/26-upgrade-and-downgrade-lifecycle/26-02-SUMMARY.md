# Summary 26-02: Ship The Upgrade And Downgrade Operator Loop

- [start.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/start.rs) now accepts `POST /control/self-hosted/product-mode` to apply a product-mode transition.
- [control_ui.html](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/control_ui.html) now shows recent product-mode transitions and includes a shipped transition form for upgrade or downgrade actions.
- [control_ui.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/control_ui.rs) now locks the panel and transition control wiring in the static dashboard test surface.

Requirements completed: `LIFE-01`, `LIFE-02`
