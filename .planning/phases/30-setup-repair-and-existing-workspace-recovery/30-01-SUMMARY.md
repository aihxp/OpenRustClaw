# Summary 30-01: Add Explicit Repair Entry to Onboarding

## Completed

- added a dedicated `Repair setup blockers or workspace drift` path when onboarding detects existing workspace state
- repair now prints a targeted step plan before rerunning setup work
- health-check-only and reset-with-backup remain separate choices instead of being folded into a vague recovery action

## Result

Existing workspaces can now re-enter setup through an explicit repair path instead of forcing operators to choose between a full rerun and manual workspace edits.
