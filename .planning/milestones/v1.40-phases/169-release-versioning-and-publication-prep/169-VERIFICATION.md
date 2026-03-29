# Verification 169: Release Versioning and Publication Prep

## Commands

```bash
cargo search openrustclaw-core --limit 1
bash scripts/check-crates-io-readiness.sh openrustclaw-core
rg -n "0\\.1\\.1|v0\\.1\\.1" README.md docs crates -g '!docs/book/**' -g '!target/**'
```

## Result

Passed.

- `openrustclaw-core` readiness passed end to end for version `1.4.0`.
- Public version references were synchronized to the new release lane.
