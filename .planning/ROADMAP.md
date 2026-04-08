# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 through v1.45** - shipped. Full milestone history and archives: `.planning/MILESTONES.md`
- 🚧 **v1.46 Milestone Audit, Deck Convergence, and Release Alignment** - active

## Overview

`v1.46` is a truthfulness and release-integrity milestone. It does not reopen the shipped delegated-agent fabric work from `v1.45`; it audits that shipped stack, closes document-deck drift, and repairs the public release lane so milestone tags, GitHub Releases, and the public crates.io package surface no longer contradict one another.

## Phases

**Phase Numbering:**
- Integer phases continue the live sequence from the prior milestone.
- This milestone starts at Phase 196 because `v1.45` ended at Phase 195.

- [ ] **Phase 196: Shipped Milestone Audit and Residual Blocker Inventory** - Review shipped milestone archives, preserve real residual gaps, and distinguish resolved work from archival markers.
- [ ] **Phase 197: Canonical Deck Convergence and Product Truthfulness** - Update the live planning and docs deck so delegated-agent fabric, Cursor support, routing-console UX, and release semantics read consistently everywhere.
- [ ] **Phase 198: Public Release Lane Repair and GitHub Release Alignment** - Repair the public semver release lane, the missing `v1.45` GitHub milestone release entry, and the latest-release metadata.
- [ ] **Phase 199: Release Evidence Sync and Closeout** - Preserve release evidence, verification commands, and the final milestone outcome in the planning archive.

## Phase Details

### Phase 196: Shipped Milestone Audit and Residual Blocker Inventory
**Goal**: Preserve one truthful blocker and gap inventory after auditing the shipped milestone stack.
**Depends on**: Nothing (first phase)
**Requirements**: AUD-01, AUD-02
**Success Criteria** (what must be TRUE):
  1. The shipped milestone stack is reviewed against its archives instead of assumed from memory.
  2. Any residual blocker or gap is recorded once in canonical planning docs instead of reappearing as drift.
  3. Archived requirements and roadmap snapshots exist for `v1.45` before the live deck is overwritten.

### Phase 197: Canonical Deck Convergence and Product Truthfulness
**Goal**: The canonical docs deck and live planning files describe the current shipped product and release semantics consistently.
**Depends on**: Phase 196
**Requirements**: DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
  1. Delegated-agent fabric, Cursor support, routing-console UX, and guided first-task orchestration are represented truthfully across canonical docs.
  2. Public semver releases and milestone/archive tags are described as different surfaces with different jobs.
  3. The docs deck no longer contains stale “gated” or “not shipped” claims for shipped `v1.45` behavior.

### Phase 198: Public Release Lane Repair and GitHub Release Alignment
**Goal**: GitHub Releases, milestone tags, and the public crate lane reflect one truthful public release contract.
**Depends on**: Phase 197
**Requirements**: REL-01, REL-02, REL-03
**Success Criteria** (what must be TRUE):
  1. The missing `v1.45` milestone tag/release is created as an archive marker with truthful notes.
  2. The public semver lane for `openrustclaw-core` is either published or preserved at a truthful blocker checkpoint with evidence.
  3. GitHub’s `Latest` release metadata points at the public semver line instead of the planning-milestone line.

### Phase 199: Release Evidence Sync and Closeout
**Goal**: Preserve the release evidence and milestone closeout in one place.
**Depends on**: Phase 198
**Requirements**: REL-04
**Success Criteria** (what must be TRUE):
  1. Verification commands, release URLs, and publication evidence are recorded in the planning archive.
  2. The live milestone deck reflects the repaired release contract and its evidence.
  3. The repo is ready for the next planning cycle without rediscovering this release story.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 196. Shipped Milestone Audit and Residual Blocker Inventory | 0/0 | Not started | - |
| 197. Canonical Deck Convergence and Product Truthfulness | 0/0 | Not started | - |
| 198. Public Release Lane Repair and GitHub Release Alignment | 0/0 | Not started | - |
| 199. Release Evidence Sync and Closeout | 0/0 | Not started | - |

## Current Status

- Active milestone: `v1.46 Milestone Audit, Deck Convergence, and Release Alignment`
- Roadmap progress: 0/4 phases complete
- Current work: audit the shipped milestone stack, converge the canonical docs deck, and repair public release metadata
- Next step: `$gsd-plan-phase 196`
