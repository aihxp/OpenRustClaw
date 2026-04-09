# Phase 198 Context

## Title

Release and Operational Hardening Capture

## Goal

Preserve the shipped release-lane, migration, update-check, and local-build hardening as canonical operational baseline.

## Why This Phase Exists

Several release and runtime improvements were shipped directly but are not yet reflected in the planning deck. That creates avoidable rediscovery risk.

## Expected Outputs

- Truthful planning coverage for the public semver line through `1.4.9`
- Canonical record of startup update checking and OpenClaw migration support
- Canonical record of repo-local temp-dir mitigation and related runtime hardening
