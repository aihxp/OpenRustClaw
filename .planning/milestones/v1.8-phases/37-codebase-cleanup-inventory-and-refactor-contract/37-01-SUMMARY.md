# Summary 37-01

Created the canonical cleanup inventory at `.planning/codebase/CLEANUP.md`.

The inventory now captures:

- priority structural hotspots such as `crates/cli/src/commands/start.rs`
- repo drift targets such as stale CI parity-doc checks
- local/generated sidecar artifacts as non-canonical repo noise
- explicit no-touch and extra-care boundaries for later refactors
