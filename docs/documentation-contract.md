# OpenRustClaw Documentation Contract

This file defines which documentation surfaces are canonical, which are mirrors, and which should be merged or deleted instead of drifting forever.

## Why This Exists

OpenRustClaw now has enough shipped surface area that documentation drift is product debt. Users should not need to compare `README.md`, repo-root planning pages, and mdBook pages to discover which one tells the truth.

## Canonical Sources

| Surface | Canonical location | Purpose |
| --- | --- | --- |
| Repo entrypoint | `README.md` | First impression, product story, setup entry, docs map |
| Guided product docs | `docs/src/` | mdBook for getting started, guides, deployment, operations, and API reference |
| Planning-facing product docs | `docs/feature-matrix.md`, `docs/surface-matrix.md`, `docs/roadmap.md`, `docs/product-positioning.md` | Current shipped-surface, planning, and positioning references |
| Docs audit | `docs/docs-audit.md` | Documentation and test coverage audit for shipped feature families |
| Security policy | `SECURITY.md` | Repo security reporting and policy surface |
| Milestone execution state | `.planning/` | Internal GSD planning and archive artifacts |

## Mirror Rules

The mdBook planning pages under `docs/src/planning/` are mirrors or navigation entry points for the root planning docs. They should summarize and link back to the canonical root file rather than duplicating the full planning content.

Use mirrors only when they improve navigation:

- mdBook entry points that point to canonical root planning docs
- short “source of truth lives here” pages
- redirects or moved notices during cleanup

Do not use mirrors as a second full-content copy of the same guide.

## Merge and Delete Rules

When two docs cover the same topic:

1. Keep the stronger canonical page.
2. Merge any unique truth from the weaker page into the canonical page.
3. Replace the weaker page with a short redirect, stub, or delete it entirely if nothing depends on it.

Delete a page when all of these are true:

- it has no unique operator or user truth
- another page already owns the topic
- keeping it would create ongoing drift

## Audience Entry Points

| Audience | Start here |
| --- | --- |
| New self-hosted operator | `README.md` → `docs/src/getting-started/installation.md` |
| Returning operator | `docs/src/getting-started/quickstart.md` |
| Deployment owner | `docs/src/deployment/production.md` |
| Security or compliance reviewer | `SECURITY.md`, `docs/src/guides/security.md` |
| Planning or scope reviewer | `docs/roadmap.md`, `docs/feature-matrix.md`, `docs/surface-matrix.md`, `docs/product-positioning.md` |

## Sync Rules for Future Milestones

Every milestone that changes shipped behavior should update:

- `README.md` if the product story, setup path, or first-stop guidance changed
- the relevant mdBook page under `docs/src/`
- any affected canonical root planning docs under `docs/`
- `docs/docs-audit.md` if a shipped feature family gained or changed docs coverage expectations

If a milestone cannot name the canonical doc it changed, the documentation work is incomplete.

## Maintenance Workflow

Use this lightweight workflow for future docs work:

1. change the canonical page first
2. update any mdBook mirror or stub that points at it
3. update the relevant planning doc if the shipped-surface claim changed
4. update `docs/docs-audit.md` if the feature-family coverage changed
5. delete stale duplicates instead of leaving them behind as “temporary” copies

The acceptable end state is one canonical page plus clearly marked mirrors, not two partially correct pages.

## Current Cleanup Targets

This milestone should specifically converge:

- the repo entry story in `README.md`
- mdBook landing and summary navigation in `docs/src/introduction.md` and `docs/src/SUMMARY.md`
- setup guidance in `docs/src/getting-started/`
- operator guidance in deployment, observability, and security docs
- planning-facing product docs so they match the shipped runtime and operator story

## Non-Goals

- turning planning docs into marketing copy
- keeping duplicate pages alive for convenience
- documenting unshipped behavior as if it is already supported
