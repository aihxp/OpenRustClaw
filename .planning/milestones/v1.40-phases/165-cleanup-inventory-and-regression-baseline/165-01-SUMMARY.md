# Summary 165-01: Cleanup Inventory and Regression Baseline

The cleanup inventory is now explicit. Public-facing documentation still exposes internal migration terminology in the architecture and contributor docs, `crates/app/Cargo.toml` still uses that language in its crate description, and the CI workflow currently hard-fails on warnings even though the workspace still emits dead-code warnings in `openrustclaw-cli`.

The regression baseline is also established: `cargo check --workspace` passes, `mdbook build docs` passes, `scripts/check-repo-hygiene.sh` passes, and `cargo test -p openrustclaw-e2e-tests --test e2e_tests` passes. That is enough to bound the first cleanup and documentation changes without guessing.
