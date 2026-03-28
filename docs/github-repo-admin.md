# GitHub Repo Admin

This document is the canonical in-repo contract for OpenRustClaw's public GitHub surface.

It covers:

- the desired repo About description and homepage
- the canonical GitHub topic set
- the repeatable admin flow for checking or applying that metadata
- the verification path for repo-facing GitHub Actions and badges
- the repeatable workflow-health checks for the public GitHub Actions surface

## Canonical Metadata

The desired public repo metadata lives in:

- `.github/repository-metadata.json`

That file is the source of truth for:

- repo description
- homepage
- GitHub topics

## Canonical Topic Set

OpenRustClaw currently publishes this canonical GitHub topic set:

- `rust`
- `self-hosted`
- `assistant`
- `ai-assistant`
- `open-source`
- `llm`
- `mcp`
- `browser-automation`
- `voice-assistant`
- `enterprise`

Keep the topic set in `.github/repository-metadata.json` aligned with the current shipped product story.
If the product claim changes, update the metadata file first, then re-run the repo-admin sync.

## Admin Commands

Use the repo-admin helper:

```bash
bash scripts/github-repo-admin.sh show-desired
bash scripts/github-repo-admin.sh validate-local
bash scripts/github-repo-admin.sh show-live
bash scripts/github-repo-admin.sh check-live
bash scripts/github-repo-admin.sh apply-live
```

`show-desired` and `validate-local` do not require GitHub auth.

`show-live`, `check-live`, and `apply-live` require GitHub auth for `aihxp/OpenRustClaw`.

The helper resolves auth in this order:

1. `GH_TOKEN`
2. `gh auth token` from the local `gh` keyring login
3. `GITHUB_TOKEN`

## Verification

Minimum local verification:

```bash
bash scripts/github-repo-admin.sh validate-local
```

Minimum live verification when GitHub auth is available:

```bash
bash scripts/github-repo-admin.sh check-live
bash scripts/github-actions-admin.sh recent-runs
bash scripts/github-actions-admin.sh check-main-ci
bash scripts/github-actions-admin.sh check-release-binaries
```

## Notes

- If GitHub auth is unavailable, keep `.github/repository-metadata.json` current and treat live repo-surface sync as blocked, not silently complete.
- GitHub topics and the About panel are not stored in git by default; this doc plus `scripts/github-repo-admin.sh` make that admin surface repeatable.

## Workflow Health

Use the Actions helper for the public automation surface:

```bash
bash scripts/github-actions-admin.sh workflows
bash scripts/github-actions-admin.sh recent-runs
bash scripts/github-actions-admin.sh check-main-ci
bash scripts/github-actions-admin.sh check-release-binaries
```

`check-main-ci` currently expects the latest `main` runs for:

- `Shipped Surface CI`
- `Shipped Surface E2E Tests`

to be either `success` or `skipped`.

Within `Shipped Surface CI`, the current hard gate is the shipped-surface verification bundle:

- parity inventory
- cargo check
- cargo test
- rustfmt
- runtime budget checks

`Shipped Surface Clippy (Informational)` and `Shipped Surface Security Audit` stay visible in the run, but they are currently advisory signals rather than workflow-failing gates because they still reflect broader workspace lint and upstream dependency debt outside this GitHub-recovery milestone.

`check-release-binaries` inspects the latest `Release Binaries` run, or the latest run for a specific branch or tag when you pass a ref:

```bash
bash scripts/github-actions-admin.sh check-release-binaries
bash scripts/github-actions-admin.sh check-release-binaries v1.10
bash scripts/github-actions-admin.sh check-release-binaries main
```

For non-tag `workflow_dispatch` runs, it requires all build jobs to pass and allows the publish job to stay `skipped`. For tag runs, it also requires `Publish GitHub Release Assets` to succeed.
