---
status: findings
depth: standard
files_reviewed: 7
files_reviewed_list:
  - crates/core/src/types.rs
  - crates/db/src/models.rs
  - crates/db/src/migrate.rs
  - crates/db/src/lib.rs
  - crates/db/src/learning_store.rs
  - crates/app/src/learning_review.rs
  - crates/app/src/lib.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 183 Code Review

Standard review of the Phase 183 learning-candidate queue, review, and promotion surfaces.

### WR-01: Promotion can leak a live lesson before durable promotion state is recorded

**File:** `crates/app/src/learning_review.rs:146-176`

**Issue:** `LearningReviewService::promote()` creates the autonomy lesson first and only then calls `mark_learning_candidate_promoted()`. If the candidate-state write fails after lesson creation succeeds, the runtime is left with an active lesson that has no durable promoted candidate lineage or promotion-history entry. That breaks the phase guarantee that promotion history exists before live guidance takes effect.

**Fix:** On promotion-state write failure, immediately compensate by deactivating the just-created lesson before returning the error, and cover the path with a regression test so failed promotions cannot bleed into live runtime guidance.
