# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- ✅ **v1.6 Proper Onboarding and Setup** — shipped 2026-03-28. Archive: `.planning/milestones/v1.6-ROADMAP.md`
- ✅ **v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite** — shipped 2026-03-27. Archive: `.planning/milestones/v1.7-ROADMAP.md`
- ✅ **v1.8 Clean Codebase** — shipped 2026-03-27. Archive: `.planning/milestones/v1.8-ROADMAP.md`
- ✅ **v1.9 GitHub Repository Presence and Actions Recovery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.9-ROADMAP.md`
- 🚧 **v1.10 Release Binaries Workflow Recovery** — active. Goal: restore truthful tagged release artifact generation in `release-binaries.yml`

## Current Status

- Active milestone: **v1.10 Release Binaries Workflow Recovery**
- Progress: 0 of 4 phases complete
- Most recent shipment: **v1.9 GitHub Repository Presence and Actions Recovery**
- Next phase: **45**
- Next step: `$gsd-plan-phase 45` or `$gsd-autonomous`

## Live Planning

Open milestone goal: repair the remaining public GitHub automation gap so version tags produce truthful downloadable release artifacts again.

### Phase 45: Release Workflow Failure Audit and Target Contract

**Goal:** Turn the current live `Release Binaries` failures into a truthful target contract and concrete repair plan.

**Why this phase exists**
- The current tagged workflow fails in public, so the repo needs evidence-driven planning before changing the release contract.

**Exit criteria**
- Latest failing tagged workflow evidence is captured in phase artifacts.
- Supported release targets and broken targets are stated explicitly.
- The repair plan reflects current workflow reality, not stale assumptions.

### Phase 46: Linux Release Build Dependency Repair

**Goal:** Repair the Linux dependency and cross-compile setup that currently blocks tagged release builds.

**Why this phase exists**
- The current workflow fails on missing Linux packaging dependencies before a truthful release can be published.

**Exit criteria**
- Supported Linux release jobs complete on GitHub-hosted runners or are truthfully re-scoped.
- Workflow setup installs or configures the required Linux build dependencies.
- Verification captures the repaired Linux release path.

### Phase 47: Release Publish Path and Tag Contract Hardening

**Goal:** Make successful supported target builds flow cleanly into GitHub release asset publication.

**Why this phase exists**
- Build success is not enough if the publish job still cannot assemble or upload the tagged artifacts correctly.

**Exit criteria**
- Artifact naming and handoff between build and publish jobs are consistent.
- The release workflow publishes expected archives and checksums for supported targets.
- Tagged release verification proves the publish path end-to-end.

### Phase 48: Release Verification and Operator Exit

**Goal:** Leave one repeatable operator runbook and verification bundle for future tagged releases.

**Why this phase exists**
- Future milestone tags should not require rediscovering how to validate or recover the release workflow.

**Exit criteria**
- The repo contains a repeatable release verification and recovery path.
- Docs and admin helpers describe the supported release process truthfully.
- Milestone closeout preserves the live release workflow evidence.
