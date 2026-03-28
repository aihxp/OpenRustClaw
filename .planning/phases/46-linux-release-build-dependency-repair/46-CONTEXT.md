# Phase 46: Linux Release Build Dependency Repair - Context

**Gathered:** 2026-03-28
**Status:** Ready for execution

## Goal

Repair the Linux dependency and cross-compilation setup that currently blocks tagged release builds.

## What We Know

- `x86_64-unknown-linux-gnu` only installs `protobuf-compiler` today and fails on missing ALSA development metadata.
- `aarch64-unknown-linux-gnu` installs a cross linker but not the ARM64 OpenSSL and ALSA development sysroot that Cargo build scripts expect.
- The existing workflow is already matrix-based and uploads `.tar.gz` plus `.sha256` artifacts on success.

## Constraints

- The build matrix should stay on GitHub-hosted runners.
- The repaired workflow must stay readable enough for operators to maintain.
- If the ARM64 Linux lane still fails after a truthful sysroot setup, the target must be re-scoped rather than silently kept as supported.

## Implementation Direction

- install `libasound2-dev` for the native Linux release lane
- install `libssl-dev:arm64` and `libasound2-dev:arm64` plus cross pkg-config env for the ARM64 Linux lane
- validate the repaired workflow through `workflow_dispatch` before relying on the final milestone tag
