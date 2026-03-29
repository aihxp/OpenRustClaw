# Verification 170: Release Execution and Post-Release Sync

## Commands

```bash
cargo login
cargo publish -p openrustclaw-core --allow-dirty
cargo search openrustclaw-core --limit 1
bash scripts/build-release-artifacts.sh --target x86_64-unknown-linux-gnu --output dist
git tag -a v0.1.1 -m "OpenRustClaw v0.1.1"
git push origin main
git push origin v0.1.1
git status --short --branch
```

## Result

Passed.

- `openrustclaw-core 0.1.1` is published on crates.io.
- The repo now has the pushed public tag `v0.1.1`.
- Local `main` and `origin/main` are synchronized after the release-alignment commit.
