# Phase 172: Canonical Surface and Overlap Map - Context

**Gathered:** 2026-03-29
**Status:** Complete

## Phase Boundary

Define which Markdown files are canonical, which are mirrors, and which are archive-only or internal so later cleanup work can delete or normalize files safely.

## Key Decision

Keep the root `docs/*.md` planning-facing pages as canonical, and keep `docs/src/planning/*.md` as thin mdBook mirrors instead of letting both copies drift into competing sources of truth.
