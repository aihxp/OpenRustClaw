# Summary 45-01: Live Release Failure Contract Captured

## What Changed

- Audited the live `Release Binaries` run for tag `v1.9`.
- Captured the failing Linux jobs and their exact dependency failures.
- Locked the repair direction around runner dependency setup and target support truthfulness.

## Evidence

- Run `23673584206` on `https://github.com/aihxp/OpenRustClaw/actions/runs/23673584206`
- Job `68972150932`: `alsa-sys` failed because `alsa.pc` was missing on `x86_64-unknown-linux-gnu`
- Job `68972150928`: `openssl-sys` failed because cross OpenSSL discovery was not configured for `aarch64-unknown-linux-gnu`

## Outcome

Phase 45 now has enough evidence to implement the Linux release-lane repair without guessing.
