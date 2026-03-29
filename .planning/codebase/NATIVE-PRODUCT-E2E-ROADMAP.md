# Native Product E2E Roadmap

**Created:** 2026-03-29
**Purpose:** Canonical follow-on roadmap for verifying the shipped product end to end and applying repairs through greenfield-native ownership when failures appear.
**Status:** Complete at `1/1` shipped milestones, or `100%`
**Baselines preserved:** historical greenfield seam ledger closed at `18/18`; adapter-only full-conversion roadmap closed at `6/6`; native-delivery planning roadmap closed at `8/8`; native-delivery implementation roadmap closed at `6/6`

## What "Continue Greenfield Conversion" Means Now

The architecture and implementation denominators are closed. The next truthful queue is not more architectural completion. It is product-reality verification: run the shipped product end to end, identify what actually fails, and force any fixes back through greenfield-native ownership instead of using the surviving brownfield command surfaces as the default repair layer.

This roadmap measures:

- real end-to-end product verification instead of compile-only or architecture-only confidence
- explicit failure ownership across app, native delivery, infrastructure, and bounded legacy exceptions
- greenfield-first repair application when defects are found
- truthful revalidation and operator-facing exit reporting after repairs land

## Milestone Sequence

### v1.39 Native Product E2E Verification and Greenfield Repairs (Shipped: 2026-03-29)

Primary target: run the product end to end, classify failures by ownership, apply any required greenfield-first repairs, and close with truthful revalidation.

- end-to-end verification across CLI, control, gateway, MCP, runtime-host, and operator paths
- failure ownership and evidence matrix over the current codebase
- greenfield-first repair path for defects found during verification
- revalidation plus operator-facing exit report after repair

Result: the shipped E2E and integration verification matrix passed cleanly, no repair-triggering product failures were found, and the roadmap closes truthfully at `1/1` without inventing cleanup work as a fake repair.

## Exit Criteria

OpenRustClaw should only claim native-product E2E completion for this queue when all of the following are true:

- the milestone has executed a real end-to-end verification matrix over the shipped product paths
- defects found during verification are classified by ownership instead of hand-waved as generic breakage
- repairs, where needed, land through app-lane or native-delivery ownership first
- the final report states clearly what works, what was repaired, and what remains bounded

## Companion Documents

- `.planning/codebase/GREENFIELD-INVENTORY.md` — retired historical `18/18` seam ledger
- `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` — completed adapter-only roadmap
- `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md` — completed native-delivery planning roadmap
- `.planning/codebase/NATIVE-DELIVERY-IMPLEMENTATION-ROADMAP.md` — completed native-delivery implementation roadmap
- `.planning/ROADMAP.md` — active milestone phases
- `.planning/PROJECT.md` — project-level milestone context and decisions
