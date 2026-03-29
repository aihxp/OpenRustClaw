# Verification 165: Cleanup Inventory and Regression Baseline

## Commands

```bash
rg -n "greenfield|brownfield|native-delivery|adapter-only|legacy command tree|migration" README.md docs crates -g '!docs/book/**' -g '!target/**'
scripts/check-repo-hygiene.sh
cargo check --workspace
mdbook build docs
cargo test -p openrustclaw-e2e-tests --test e2e_tests
cargo search openrustclaw-core --limit 5
cargo search openrustclaw-cli --limit 5
git tag --sort=-v:refname | head -n 20
```

## Result

Passed.

- The cleanup target inventory is concrete and file-backed.
- The docs build and E2E baseline are green.
- The compile baseline is green but exposes warning debt that can explain current CI failures under `-D warnings`.
- The current public package state and tag state are now explicit inputs to the release phases.
