# Verification 170: Release Execution and Post-Release Sync

## Commands

```bash
cargo login
cargo publish -p openrustclaw-core --allow-dirty
cargo search openrustclaw-core --limit 1
bash scripts/build-release-artifacts.sh --target x86_64-unknown-linux-gnu --output dist
git tag -a v1.4.0 -m "OpenRustClaw v1.4.0"
git push origin main
git push origin v1.4.0
git status --short --branch
```

## Result

Passed.

- `openrustclaw-core 1.4.0` is published on crates.io.
- The repo now has the pushed public tag `v1.4.0`.
- Local `main` and `origin/main` are synchronized after the release-alignment commit.
