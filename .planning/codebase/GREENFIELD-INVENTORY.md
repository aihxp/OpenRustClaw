# Greenfield Seam Inventory

**Created:** 2026-03-28
**Purpose:** Canonical ranked seam inventory for the brownfield-to-greenfield conversion after the proving-slice milestone.

## Baseline

- **Baseline type:** Ranked seam inventory
- **Current score:** `18/18` migrated seams
- **Current percentage:** `100%`
- **Interpretation:** The canonical derived score is `100%` because every ranked seam in the current inventory is now migrated.

This inventory starts after the proving-slice milestone and does **not** count:

- the initial `openrustclaw-app` shell creation
- contributor-default and governance-only posture updates
- broad milestone scaffolding work that did not migrate one concrete seam

## Inventory

| Rank | Area | Seam | Status | Phase | Notes |
| --- | --- | --- | --- | --- | --- |
| 1 | inspect | Self-hosted product-mode summary composition | migrated | 61 | Moved product-mode summary composition out of `inspect.rs`. |
| 2 | routes | Self-hosted product-mode route family | migrated | 62 | Moved the route family behind `openrustclaw-app`. |
| 3 | mobile | Mobile node operator report | migrated | 63 | Moved the operator report into the application lane. |
| 4 | skills | Compiled-skill overview lane | migrated | 64 | Moved compiled-skill overview behavior out of `skills.rs`. |
| 5 | inspect | Enterprise admin aggregation | migrated | 65 | Moved enterprise admin summary composition out of `inspect.rs`. |
| 6 | routes | Enterprise access write route family | migrated | 66 | Moved enterprise access write orchestration behind the application seam. |
| 7 | skills | Skill install/update/uninstall mutation lane | migrated | 67 | Moved the first mutation-heavy skills lane behind `openrustclaw-app`. |
| 8 | runtime | Runtime provider/model switch lane | migrated | 68 | Moved provider and model switching behind the application seam. |
| 9 | skills | Voice-plugin bind lifecycle lane | migrated | 69 | Moved voice-plugin binding rules and result shaping out of `skills.rs`. |
| 10 | runtime | Runtime vault mutation lane | migrated | 70 | Moved runtime vault set/delete mutation behind the application seam. |
| 11 | runtime | Runtime reload-planning lane | migrated | 71 | Moved reload-plan comparison and classification out of `runtime.rs`. |
| 12 | routes | Runtime vault route family | migrated | 72 | Moved `/control/runtime/vault` behind the application seam. |
| 13 | skills | Auth-plugin lifecycle lane | migrated | 74 | Moved auth-plugin binding validation, key derivation, and scope shaping out of `skills.rs`. |
| 14 | skills | Channel-extension and background workflow lifecycle lane | migrated | 77 | Moved background workflow scheduling and channel-extension binding behind `openrustclaw-app`. |
| 15 | runtime | Runtime upgrade planning lane | migrated | 75 | Moved upgrade-plan blocker detection and operator-step generation out of `runtime.rs`. |
| 16 | runtime | Runtime self-update and rollback planning lane | migrated | 75 | Moved self-update and rollback planning guidance out of `runtime.rs`. |
| 17 | control | Greenfield progress summary surface | migrated | 76 | Added a shipped runtime-maintenance summary surface that reports canonical conversion progress and the remaining queue. |
| 18 | routes | Runtime maintenance route family | migrated | 76 | Added a bounded runtime-maintenance route surface behind `openrustclaw-app`. |

## Phase Guidance

### Phase 73

- preserve this inventory as the ranked denominator for greenfield completion reporting
- align any shipped percentage surface to the canonical score derived from this inventory as follow-on seams migrate

## Post-Closure Decision

- **Decision:** retire the current ranked seam inventory at `18/18`
- **Rule:** future greenfield follow-on work must define an explicit new canonical inventory instead of silently extending this retired ledger
- **Why:** the current ranked queue was intentionally bounded; keeping the denominator fixed preserves truthful historical completion reporting
