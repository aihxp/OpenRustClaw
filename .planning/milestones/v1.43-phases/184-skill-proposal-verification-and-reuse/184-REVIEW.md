---
status: findings
depth: standard
files_reviewed: 7
files_reviewed_list:
  - crates/core/src/types.rs
  - crates/db/src/models.rs
  - crates/db/src/migrate.rs
  - crates/db/src/lib.rs
  - crates/db/src/skill_proposal_store.rs
  - crates/app/src/skill_proposals.rs
  - crates/app/src/lib.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 184 Code Review

Standard review of the Phase 184 skill-proposal queue, verification, and install surfaces.

### WR-01: Install can leak an active skill before durable install state is recorded

**File:** `crates/app/src/skill_proposals.rs:220-229`

**Issue:** `SkillProposalService::install()` installs the skill artifact first and only then calls `mark_skill_proposal_installed()`. If the durable proposal-state update fails after installation succeeds, the live skill remains installed without the proposal history or installed-state record that Phase 184 promised as the source of truth.

**Fix:** On installed-state write failure, immediately roll back the just-installed skill before returning the error, and add a regression test so failed installs cannot bleed into the active skill set.
