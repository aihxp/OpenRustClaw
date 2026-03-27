#!/usr/bin/env bash
set -euo pipefail

search() {
  if command -v rg >/dev/null 2>&1; then
    rg -n "$1" "$2"
  else
    grep -En "$1" "$2"
  fi
}

test -f docs/feature-matrix.md
test -f docs/surface-matrix.md
test -f docs/product-positioning.md
test -f crates/cli/src/commands/start/auth.rs
test -f .planning/codebase/CLEANUP.md

search 'surface-matrix|product-positioning|feature-matrix' .github/workflows/ci.yml
search '## Core Runtime|## Experience Layers' docs/feature-matrix.md
search '## Scope Split|## Phase 1 Completion Note' docs/surface-matrix.md
search '## What It Ships Well|## Where It Is Intentionally Stronger|## Current Boundaries' docs/product-positioning.md
search 'mod auth;|use self::auth::' crates/cli/src/commands/start.rs
search 'Completed In v1.8 So Far|Deferred Cleanup Debt' .planning/codebase/CLEANUP.md

git check-ignore -q sidecar/.venv
git check-ignore -q sidecar/.pytest_cache
git check-ignore -q sidecar/src/__pycache__
