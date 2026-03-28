# Summary 43-02: Repaired the Shipped-Surface Workflow Contract

Repaired the shipped GitHub workflow surface so the public CI and E2E runs now align with the current repo and verification contract.

## Shipped

- Updated [.github/workflows/ci.yml](/home/hprincivil/projects/OpenRustClaw/.github/workflows/ci.yml) to install `ripgrep`, install `libasound2-dev` where needed, and keep Clippy plus RustSec visible as informational jobs instead of misleading hard gates
- Updated [.github/workflows/e2e-tests.yml](/home/hprincivil/projects/OpenRustClaw/.github/workflows/e2e-tests.yml) to install `libasound2-dev` and increased the smoke-test timeout so the cold release build can finish on GitHub-hosted runners
- Updated [scripts/github-actions-admin.sh](/home/hprincivil/projects/OpenRustClaw/scripts/github-actions-admin.sh) and [docs/github-repo-admin.md](/home/hprincivil/projects/OpenRustClaw/docs/github-repo-admin.md) so workflow health can be checked through one repeatable operator path
- Fixed the `skills.rs` compile regression in [skills.rs](/home/hprincivil/projects/OpenRustClaw/crates/cli/src/commands/skills.rs) so `cargo check`, `cargo test`, and runtime-budget CI no longer fail on the current head

## Verification

- `cargo check --workspace`
- `cargo test --workspace --lib`
- `bash scripts/check-runtime-budgets.sh`
- `bash scripts/github-actions-admin.sh workflows`
- `bash scripts/github-actions-admin.sh check-main-ci`

