---
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
---

# Phase 183 Code Review Fix

Applied a manual fix for the Phase 183 review finding.

## Outcome

- `WR-01` was fixed in `crates/app/src/learning_review.rs`.

## Fix Summary

- `LearningReviewService::promote()` now compensates when durable promotion state fails after lesson creation by deactivating the just-created lesson before returning the error.
- Added a regression test that forces promotion persistence failure and verifies the lesson is deactivated instead of bleeding into live runtime guidance.
- This keeps candidate promotion and active-lesson state aligned even when the second write in the promotion flow fails.
