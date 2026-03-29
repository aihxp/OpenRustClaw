# Summary 168-01: GitHub Actions and release automation repair

The relevant GitHub Actions lanes are now repo-owned and locally reproducible. The E2E workflow has a real `workflow_dispatch` trigger, the CI workflow now runs a checked-in security-audit script instead of relying on an opaque action with no project policy, and the release-budget path uses the hardened runtime-budget script from the cleanup phase.

This phase did not pretend to fix GitHub authentication itself. The source-controlled workflow and script behavior is repaired; GitHub-side release creation still depends on valid `gh` or Actions credentials at runtime.
