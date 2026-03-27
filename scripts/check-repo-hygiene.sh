#!/usr/bin/env bash
set -euo pipefail

test -f docs/feature-matrix.md
test -f docs/surface-matrix.md
test -f docs/product-positioning.md
test -f crates/cli/src/commands/start/auth.rs
test -f .planning/codebase/CLEANUP.md

rg -n 'surface-matrix|product-positioning|feature-matrix' .github/workflows/ci.yml
rg -n '## Core Runtime|## Experience Layers' docs/feature-matrix.md
rg -n '## Scope Split|## Phase 1 Completion Note' docs/surface-matrix.md
rg -n '## What It Ships Well|## Where It Is Intentionally Stronger|## Current Boundaries' docs/product-positioning.md
rg -n 'mod auth;|use self::auth::' crates/cli/src/commands/start.rs
rg -n 'Completed In v1.8 So Far|Deferred Cleanup Debt' .planning/codebase/CLEANUP.md

git check-ignore -q sidecar/.venv
git check-ignore -q sidecar/.pytest_cache
git check-ignore -q sidecar/src/__pycache__
