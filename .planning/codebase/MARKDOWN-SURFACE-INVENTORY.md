# Markdown Surface Inventory

**Created:** 2026-03-29
**Purpose:** Inventory of tracked Markdown in the OpenRustClaw repository before `v1.41` cleanup decisions were applied.

## Baseline Counts

- **Tracked Markdown files in Git:** `1285`
- **Internal planning/archive Markdown:** `995` under `.planning/`
- **Internal GSD/system Markdown:** `188` under `.codex/`
- **Live repo/product/package Markdown:** `102` outside `.planning/` and `.codex/`

## Live Surface Breakdown

| Bucket | Count | Notes |
| --- | ---: | --- |
| `docs/` | 63 | Canonical docs-site source plus repo-root planning docs |
| `crates/` | 27 | Package-facing `README.md` files |
| `slides/` | 6 | Workshop or presentation material |
| Repo-root singleton docs | 4 | `README.md`, `SECURITY.md`, `CLAUDE.md`, `LLM_SDK_SUMMARY.md` |
| `sidecar/` | 1 | Compatibility-lane README |
| `tests/` | 1 | E2E README |

## High-Signal Overlap Clusters

### Root planning docs vs mdBook mirrors

Canonical root planning docs:

- `docs/docs-audit.md`
- `docs/documentation-contract.md`
- `docs/feature-matrix.md`
- `docs/product-positioning.md`
- `docs/roadmap.md`
- `docs/surface-matrix.md`

mdBook mirrors:

- `docs/src/planning/docs-audit.md`
- `docs/src/planning/documentation-contract.md`
- `docs/src/planning/feature-matrix.md`
- `docs/src/planning/product-positioning.md`
- `docs/src/planning/roadmap.md`
- `docs/src/planning/surface-matrix.md`

### Internal or stale reference candidates

- `LLM_SDK_SUMMARY.md` — orphaned one-off snapshot; no inbound references found
- `CLAUDE.md` — active internal reference, but several facts were stale and needed refresh
- `docs/src/guides/cursor-integration.md` — embedded stale `CLAUDE.md` sample duplicated outdated guidance
- `docs/src/guides/providers.md` — presented a subset as if it were the full provider inventory
- `docs/src/planning/roadmap.md` — mirror page had drifted into its own content instead of staying a thin pointer

## Initial Disposition Candidates

| Surface | Initial status | Intended disposition |
| --- | --- | --- |
| `LLM_SDK_SUMMARY.md` | stale / orphaned | delete after merging useful provider-coverage context into canonical docs |
| `docs/src/planning/roadmap.md` | divergent mirror | normalize to thin mirror |
| `CLAUDE.md` | active but stale | refresh in place |
| `docs/src/guides/cursor-integration.md` | duplicated stale snippet | refresh in place |
| `docs/src/guides/providers.md` | scope wording drift | clarify that it covers common setups, not the full inventory |
| selected crate `README.md` files | minor terminology drift | keep canonical role, tighten wording |

## Out-of-Scope But Counted

- `.planning/milestones/**` and archived phase artifacts remain historical records, not cleanup targets for style-only rewrites.
- `.codex/**` remains internal workflow and skill documentation, not part of the public Markdown cleanup pass.
- `slides/**` remains presentation material unless a later phase finds a direct contradiction with canonical product docs.
