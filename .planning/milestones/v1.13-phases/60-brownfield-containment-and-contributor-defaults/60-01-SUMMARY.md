# Plan 60-01 Summary: Lock Contributor Defaults and Brownfield Containment Rules

## Result

Passed. The repo now treats `openrustclaw-app` as the default application lane for new business logic, while the major CLI command hubs are explicitly documented as compatibility adapters unless a migration phase is active.

## What Changed

- contributor docs now tell humans to put new use-case logic in `openrustclaw-app`
- `CLAUDE.md` now carries the same default for agent-driven edits
- planning-facing cleanup and greenfield contracts now preserve the next migration queue and compatibility-only exception rules

## Evidence

- `docs/src/contributing/development.md`
- `CLAUDE.md`
- `.planning/codebase/GREENFIELD.md`
- `.planning/codebase/CLEANUP.md`
- `mdbook build docs`
