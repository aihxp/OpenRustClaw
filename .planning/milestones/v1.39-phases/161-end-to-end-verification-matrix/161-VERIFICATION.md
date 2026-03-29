# Verification 161: End-to-End Verification Matrix

## Commands

```bash
cargo run -p openrustclaw-e2e-tests --bin e2e
cargo test -p openrustclaw-e2e-tests --test e2e_tests -- --nocapture
cargo test -p openrustclaw-integration-tests --lib -- --nocapture
```

## Result

Passed.

- The standalone E2E runner completed successfully in mock mode.
- `openrustclaw-e2e-tests` completed at `89 passed, 0 failed, 3 ignored`.
- `openrustclaw-integration-tests` completed at `196 passed, 0 failed, 0 ignored`.

No product failures were observed in the milestone's primary shipped verification matrix.
