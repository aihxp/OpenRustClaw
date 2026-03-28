# Phase 45: Release Workflow Failure Audit and Target Contract - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Turn the live `Release Binaries` GitHub Actions failures into a truthful target contract and concrete repair plan.

## What We Know

- The latest tagged `Release Binaries` run is `23673584206` for tag `v1.9`.
- `Build x86_64-unknown-linux-gnu` fails in `Build packaged artifacts` because `alsa-sys` cannot find `alsa.pc`.
- `Build aarch64-unknown-linux-gnu` fails earlier in `Build packaged artifacts` because `openssl-sys` cannot find a cross-compilation OpenSSL install or configured pkg-config path.
- `Build aarch64-apple-darwin` succeeds.
- `Publish GitHub Release Assets` is skipped because the Linux matrix is failing.

## Constraints

- The release workflow must stay truthful about supported targets.
- The fix should preserve the current packaged artifact contract of `.tar.gz` plus `.sha256`.
- If Linux ARM cannot be made repeatable on GitHub-hosted runners, it must be explicitly re-scoped instead of silently left broken.

## Implementation Direction

- capture the failing run IDs and job logs in phase evidence
- repair the missing Linux system dependencies for the supported targets
- verify the repaired build matrix through `workflow_dispatch` before relying on the final milestone tag run
