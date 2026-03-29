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
- 🚧 **v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery** — active. Baseline: native-delivery roadmap `1/8` shipped milestones, or about `13%`; target after shipment: `2/8`, or `25%`

## Current Status

- Active milestone: **v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.25 Native Delivery Layer: Port Contracts and Legacy Inventory**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`6/6` milestones shipped, or `100%`**
- Native-delivery roadmap progress: **`1/8` milestones shipped, or about `13%`**
- Current execution: **Phase 109 ready for planning**
- Next step: `$gsd-plan-phase 109` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 109: Native Control HTTP Delivery Layer**
- [ ] **Phase 110: Native MCP Server Delivery Layer**
- [ ] **Phase 111: Gateway Bootstrap Split and Start.rs Retirement Slice**
- [ ] **Phase 112: Control UI Serving and Native Delivery Alignment**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. The follow-on adapter-only roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` is also complete at `6/6`. Future architecture work now proceeds under `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`, which currently stands at `1/8` shipped milestones and measures progress by native delivery-layer and legacy-retirement milestones instead of extending either completed denominator.

### Phase 109: Native Control HTTP Delivery Layer

**Goal:** Replace the remaining `start.rs` control-route bootstrap monopoly with a native control HTTP delivery layer that calls app ports directly.

**Depends on:** Phase 108
**Requirements:** `NDL-05`

**Success criteria:**
1. the roadmap defines the target native control HTTP entrypoints and route ownership explicitly
2. the control delivery plan uses app ports directly instead of routing future work back through `start.rs`
3. the migration slice is narrow enough to ship incrementally without breaking the existing control contract

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 110: Native MCP Server Delivery Layer

**Goal:** Define the native MCP server delivery path that will replace the legacy MCP bootstrap and tool registration ownership in `start.rs`.

**Depends on:** Phase 109
**Requirements:** `NDL-06`

**Success criteria:**
1. the roadmap defines the target MCP delivery ownership in `openrustclaw-mcp` or its successor delivery layer
2. the tool-catalog, invocation, and compiled-skill exposure path is described in terms of app ports rather than legacy command helpers
3. the roadmap makes the MCP replacement path concrete enough to implement without rediscovering transport boundaries

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 111: Gateway Bootstrap Split and Start.rs Retirement Slice

**Goal:** Split the gateway or server bootstrap path out of `start.rs` so native delivery can own startup without depending on the legacy command hotspot.

**Depends on:** Phase 110
**Requirements:** `NDL-07`

**Success criteria:**
1. the roadmap defines how gateway startup moves out of `start.rs`
2. bootstrap ownership for control, websocket, and related server paths is explicit
3. the milestone leaves a real retirement slice for `start.rs` instead of only describing future cleanup

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 112: Control UI Serving and Native Delivery Alignment

**Goal:** Align Control UI serving and wiring with the native gateway-delivery path so UI transport stops depending on the legacy bootstrap contract.

**Depends on:** Phase 111
**Requirements:** `NDL-08`

**Success criteria:**
1. the control UI serving and wiring path is mapped to the native gateway delivery layer
2. the roadmap preserves compatibility for the shipped UI while reducing legacy ownership
3. the live planning surface leaves the native-delivery roadmap at `2/8`, or `25%`, only if the gateway and UI alignment story is explicit

**Plans:** 0/0 plans complete

Plans:
- none yet
