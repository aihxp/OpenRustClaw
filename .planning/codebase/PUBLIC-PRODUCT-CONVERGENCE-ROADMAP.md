# Public Product Convergence Roadmap

**Created:** 2026-03-29
**Purpose:** Canonical follow-on roadmap for cleaning the repo safely, converging public documentation and metadata, repairing CI and release automation, and shipping the next public package release.
**Status:** Active at `0/1` shipped milestones, or `0%`
**Baselines preserved:** historical seam ledger closed at `18/18`; adapter-only full-conversion roadmap closed at `6/6`; native-delivery planning roadmap closed at `8/8`; native-delivery implementation roadmap closed at `6/6`; native-product E2E roadmap closed at `1/1`

## What This Queue Means

The architecture and verification queues are closed. The next truthful denominator is public product convergence: make the shipped repo, docs, metadata, CI, and release surfaces line up with what the product actually is, while cleaning code and files only where that can be proven safe.

This roadmap measures:

- safe repo cleanup with regression protection
- public documentation and package-metadata convergence
- repair or retirement of failing GitHub Actions workflows
- release and crates publication readiness backed by real verification

## Milestone Sequence

### v1.40 Public Product Cleanup, Documentation Convergence, CI Repair, and Release

Primary target: clean and simplify the repo without regressions, remove internal migration language from public surfaces, fix GitHub Actions failures, and ship the next public release.

- cleanup inventory and deletion or merge plan backed by tests
- public docs and metadata convergence
- codebase cleanup and sync across source, docs, workflows, and packaging
- CI and release-automation repair plus release publication

## Exit Criteria

OpenRustClaw should only claim this queue complete when all of the following are true:

- the repo has a verified cleanup inventory and any deletions or merges are bounded by tests or equivalent verification
- public-facing docs and metadata no longer rely on internal migration terminology
- relevant GitHub Actions workflows are green or intentionally retired with rationale
- the next public package and release artifacts are shipped and the docs match the shipped result

## Companion Documents

- `.planning/codebase/NATIVE-PRODUCT-E2E-ROADMAP.md` — completed product-verification roadmap
- `.planning/ROADMAP.md` — active milestone phases
- `.planning/PROJECT.md` — project-level milestone context and decisions
