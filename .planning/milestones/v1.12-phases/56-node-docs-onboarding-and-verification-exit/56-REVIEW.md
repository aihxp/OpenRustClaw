---
status: findings
depth: standard
files_reviewed: 3
files_reviewed_list:
  - docs/src/getting-started/installation.md
  - docs/src/deployment/remote-connectivity.md
  - docs/src/deployment/production.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 56 Retroactive Code Review

Reviewed the node-docs verification exit against the current onboarding and deployment docs.

### WR-01: The installation guide still advertises the old fallback order

**File:** `docs/src/getting-started/installation.md`

**Issue:** The installation guide still says the saved remote-connectivity profile uses node-first,
then SSH tunnel, then reverse proxy. The canonical remote-connectivity guide now puts Tailscale
tailnet between node-first and SSH tunnel. That breaks the phase `56` goal of keeping onboarding and
deployment docs aligned on one operator story.

**Fix:** Update the installation guide so the saved-profile explanation uses the same node-first,
Tailscale, SSH, reverse-proxy order as the canonical deployment guide.
