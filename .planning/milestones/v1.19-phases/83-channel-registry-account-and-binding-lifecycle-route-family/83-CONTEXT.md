# Phase 83: Channel Registry Account and Binding Lifecycle Route Family - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

## Phase Boundary

Move channel registry account and binding lifecycle orchestration out of `start.rs` so approval, activation, binding, and registry mutation flows no longer live inside the same route hub.

## Decisions

- The extraction should focus on mutation and lifecycle flows only, leaving the read-only registry listing and detail handlers in place for now.
- `start.rs` should remain the HTTP adapter for channel registry payloads and status-code mapping.
- The new application service should own path-id validation, bind-request default shaping, and stable mutation-result composition.

## Existing Code Insights

- `start.rs` still owns create, update, delete, approve, block, activation, bind, update binding, and delete binding orchestration inline.
- The route family already shares common patterns: root lookup, channel-manifest mutation, registry reload, and stable result JSON.
- The seam is well bounded because the lifecycle path can move without reopening route preview, message routing, or read-only registry inspection.
