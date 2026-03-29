# Roadmap: OpenRustClaw

## Milestones

- ✅ **v1.0 Rust OpenClaw MVP** — shipped 2026-03-26. Archive: `.planning/milestones/v1.0-ROADMAP.md`
- ✅ **v1.1 Lifecycle Integrity and Enterprise Foundations** — shipped 2026-03-26. Archive: `.planning/milestones/v1.1-ROADMAP.md`
- ✅ **v1.2 Deeper OpenClaw Surface Parity** — shipped 2026-03-27. Archive: `.planning/milestones/v1.2-ROADMAP.md`
- ✅ **v1.3 Enterprise Expansion and Supervised Autonomy Foundations** — shipped 2026-03-27. Archive: `.planning/milestones/v1.3-ROADMAP.md`
- ✅ **v1.4 Enterprise Governance and Operator-Gated Full Autonomy** — shipped 2026-03-27. Archive: `.planning/milestones/v1.4-ROADMAP.md`
- ✅ **v1.5 Self-Hosted Product Modes and Lifecycle Packaging** — shipped 2026-03-27. Archive: `.planning/milestones/v1.5-ROADMAP.md`
- ✅ **v1.6 Proper Onboarding and Setup** — shipped 2026-03-28. Archive: `.planning/milestones/v1.6-ROADMAP.md`
- ✅ **v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite** — shipped 2026-03-27. Archive: `.planning/milestones/v1.7-ROADMAP.md`
- ✅ **v1.8 Clean Codebase** — shipped 2026-03-27. Archive: `.planning/milestones/v1.8-ROADMAP.md`
- ✅ **v1.9 GitHub Repository Presence and Actions Recovery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.9-ROADMAP.md`
- ✅ **v1.10 Release Binaries Workflow Recovery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.10-ROADMAP.md`
- ✅ **v1.11 Crates.io and Docs.rs Publication Foundation** — shipped 2026-03-28. Archive: `.planning/milestones/v1.11-ROADMAP.md`
- ✅ **v1.12 Secure Node Connectivity and SSH Tunnel Revisit** — shipped 2026-03-28. Archive: `.planning/milestones/v1.12-ROADMAP.md`
- ✅ **v1.13 Brownfield-to-Greenfield Transition** — shipped 2026-03-28. Archive: `.planning/milestones/v1.13-ROADMAP.md`
- ✅ **v1.14 Continued Greenfield Conversion** — shipped 2026-03-28. Archive: `.planning/milestones/v1.14-ROADMAP.md`
- ✅ **v1.15 Deeper Greenfield Conversion** — shipped 2026-03-28. Archive: `.planning/milestones/v1.15-ROADMAP.md`
- ✅ **v1.16 Greenfield Conversion: Skills and Runtime Hotspots** — shipped 2026-03-28. Archive: `.planning/milestones/v1.16-ROADMAP.md`
- ✅ **v1.17 Greenfield Conversion: Completion Metrics and Remaining Hotspots** — shipped 2026-03-28. Archive: `.planning/milestones/v1.17-ROADMAP.md`
- ✅ **v1.18 Greenfield Conversion: Final Ranked Seam and 100% Completion Path** — shipped 2026-03-28. Archive: `.planning/milestones/v1.18-ROADMAP.md`
- ✅ **v1.19 Full Greenfield Conversion: Control Plane Route Families I** — shipped 2026-03-28. Archive: `.planning/milestones/v1.19-ROADMAP.md`
- ✅ **v1.20 Full Greenfield Conversion: Control Plane Route Families II** — shipped 2026-03-28. Archive: `.planning/milestones/v1.20-ROADMAP.md`
- ✅ **v1.21 Full Greenfield Conversion: Mobile and Voice Runtime Services** — shipped 2026-03-28. Archive: `.planning/milestones/v1.21-ROADMAP.md`
- ✅ **v1.22 Full Greenfield Conversion: Orchestration and Browser Services** — shipped 2026-03-28. Archive: `.planning/milestones/v1.22-ROADMAP.md`
- ✅ **v1.23 Full Greenfield Conversion: Setup and Secondary Command Surfaces** — shipped 2026-03-28. Archive: `.planning/milestones/v1.23-ROADMAP.md`
- ✅ **v1.24 Full Greenfield Conversion: Adapter-Only Exit and Enforcement** — shipped 2026-03-28. Archive: `.planning/milestones/v1.24-ROADMAP.md`
- ✅ **v1.25 Native Delivery Layer: Port Contracts and Legacy Inventory** — shipped 2026-03-28. Archive: `.planning/milestones/v1.25-ROADMAP.md`
- ✅ **v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery** — shipped 2026-03-28. Archive: `.planning/milestones/v1.26-ROADMAP.md`
- ✅ **v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I** — shipped 2026-03-28. Archive: `.planning/milestones/v1.27-ROADMAP.md`
- 🚧 **v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows** — active. Baseline: native-delivery roadmap `3/8` shipped milestones, or about `38%`; target after shipment: `4/8`, or `50%`

## Current Status

- Active milestone: **v1.28 Native Delivery Layer: CLI Operator Commands II and UI-Adjacent Flows**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`6/6` milestones shipped, or `100%`**
- Native-delivery roadmap progress: **`3/8` milestones shipped, or about `38%`**
- Current execution: **Phase 117 ready for planning**
- Next step: `$gsd-plan-phase 117` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 117: Large Operator Command Family Native Delivery**
- [ ] **Phase 118: Secondary Operator and Utility Native Delivery**
- [ ] **Phase 119: CLI Family Dependency Removal**
- [ ] **Phase 120: UI-Adjacent Delivery Alignment**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. The follow-on adapter-only roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` is also complete at `6/6`. The native-delivery program proceeds under `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`, which currently stands at `3/8` shipped milestones and measures progress by native delivery-layer and legacy-retirement milestones instead of extending either completed denominator.

### Phase 117: Large Operator Command Family Native Delivery

**Goal:** Define the native CLI delivery path for the remaining large operator-facing command families such as browser, orchestration, mobile, voice runtime, onboarding, skills, and self-hosted flows.

**Depends on:** Phase 116
**Requirements:** `NDL-13`

**Success criteria:**
1. the roadmap defines native delivery ownership for the remaining large operator command families explicitly
2. those families are mapped to app ports instead of legacy command-to-command orchestration
3. the milestone leaves an incremental implementation path instead of bundling every operator surface into one vague rewrite

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 118: Secondary Operator and Utility Native Delivery

**Goal:** Define the native CLI delivery path for channels, services, schedule, tools, media, memory, and adjacent secondary utility command families.

**Depends on:** Phase 117
**Requirements:** `NDL-14`

**Success criteria:**
1. the roadmap defines native delivery ownership for the remaining secondary operator and utility families
2. the families are grouped around app-port or delivery concerns rather than legacy file layout
3. the roadmap makes the second CLI operator slice concrete enough to implement without rediscovering ownership boundaries

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 119: CLI Family Dependency Removal

**Goal:** Remove command-to-command orchestration dependencies conceptually from the remaining operator CLI families so native delivery modules can own their flows directly.

**Depends on:** Phase 118
**Requirements:** `NDL-15`

**Success criteria:**
1. the roadmap states which command-to-command dependencies must be removed or avoided
2. future native delivery modules are described as independent entrypoint families over app ports
3. the milestone preserves a compatibility story without re-creating legacy internal routing

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 120: UI-Adjacent Delivery Alignment

**Goal:** Align UI-adjacent operator entrypoints to native CLI and gateway delivery paths so those surfaces stop depending conceptually on legacy command ownership.

**Depends on:** Phase 119
**Requirements:** `NDL-16`

**Success criteria:**
1. the roadmap aligns UI-adjacent operator surfaces with native entrypoints explicitly
2. the alignment preserves compatibility for shipped operator surfaces while reducing legacy ownership
3. the live planning surface leaves the native-delivery roadmap at `4/8`, or `50%`, only if the second CLI slice is explicit end to end

**Plans:** 0/0 plans complete

Plans:
- none yet
