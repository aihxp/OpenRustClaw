---
status: findings
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/app/src/onboarding_lane_catalog.rs
  - crates/app/src/lib.rs
  - crates/cli/src/commands/onboard.rs
  - crates/cli/src/commands/models.rs
  - crates/cli/src/commands/inspect.rs
  - crates/app/src/setup_handoff.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 188 Code Review

Standard review of the Phase 188 onboarding lane and model-selection cohesion flow.

### WR-01: Provider choice builder drops additional delegated lanes for the same provider

**File:** `crates/cli/src/commands/onboard.rs:349-390`

**Issue:** `build_provider_choices()` stores delegated onboarding lanes in a single `delegated: Option<OnboardingLaneDescriptor>` slot per provider. When multiple delegated backends map to the same provider family, each later lane overwrites the previous one. In practice this collapses distinct delegated local-agent choices such as OpenAI-backed `codex` and `cursor` into a single access path, so onboarding can no longer present or persist the full set of eligible delegated lanes.

**Fix:** Preserve all delegated lanes per provider and emit one subscription-managed access path for each delegated descriptor, keeping direct API and local-runtime paths merged alongside them.
