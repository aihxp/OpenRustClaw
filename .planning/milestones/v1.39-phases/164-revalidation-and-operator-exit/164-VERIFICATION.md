# Verification 164: Revalidation and Operator Exit

## Commands

```bash
cargo test -p openrustclaw-e2e-tests --test e2e_tests
cargo test -p openrustclaw-integration-tests --lib
```

## Result

Passed.

- Revalidation E2E result: `89 passed, 0 failed, 3 ignored`.
- Revalidation integration result: `196 passed, 0 failed, 0 ignored`.
- No repair regression was possible because no repair patch landed, and the unchanged tree remained green under rerun.
