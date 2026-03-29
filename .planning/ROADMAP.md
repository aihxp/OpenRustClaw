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
- 🚧 **v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I** — active. Baseline: native-delivery roadmap `2/8` shipped milestones, or `25%`; target after shipment: `3/8`, or about `38%`

## Current Status

- Active milestone: **v1.27 Native Delivery Layer: CLI Core Dispatch and Operator Commands I**
- Progress: **0 of 4 phases complete**
- Most recent shipment: **v1.26 Native Delivery Layer: Control, MCP, and Gateway Delivery**
- Greenfield conversion baseline: **historical `18/18` ranked seam ledger complete and retired**
- Full-conversion roadmap progress: **`6/6` milestones shipped, or `100%`**
- Native-delivery roadmap progress: **`2/8` milestones shipped, or `25%`**
- Current execution: **Phase 113 ready for planning**
- Next step: `$gsd-plan-phase 113` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 113: Native CLI Dispatch Layer**
- [ ] **Phase 114: Assistant, Chat, Session, and Inspect Native Delivery**
- [ ] **Phase 115: Control and Runtime Native CLI Delivery**
- [ ] **Phase 116: CLI Boundary Separation and Compatibility Shim Plan**

### Current Queue Rule

The original ranked greenfield seam inventory remains closed at `18/18` and retired. The follow-on adapter-only roadmap in `.planning/codebase/GREENFIELD-FULL-CONVERSION.md` is also complete at `6/6`. The native-delivery program proceeds under `.planning/codebase/NATIVE-DELIVERY-LAYER-ROADMAP.md`, which currently stands at `2/8` shipped milestones and measures progress by native delivery-layer and legacy-retirement milestones instead of extending either completed denominator.

### Phase 113: Native CLI Dispatch Layer

**Goal:** Define the new top-level CLI dispatch layer over app ports so `main.rs` stops being the permanent routing owner for the product path.

**Depends on:** Phase 112
**Requirements:** `NDL-09`

**Success criteria:**
1. the roadmap defines the target native CLI dispatch ownership explicitly
2. the dispatch path is described in terms of app-port invocation rather than command-module cross-calls
3. the milestone leaves a compatibility-preserving path for the existing `openrustclaw` binary while breaking the `main.rs` monopoly

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 114: Assistant, Chat, Session, and Inspect Native Delivery

**Goal:** Define the first core operator CLI delivery family over app ports for assistant, chat, session, and inspect entrypoints.

**Depends on:** Phase 113
**Requirements:** `NDL-10`

**Success criteria:**
1. the roadmap defines the native delivery ownership for assistant, chat, session, and inspect flows
2. the entrypoint plan separates CLI parsing and rendering from app-use orchestration
3. the affected flows no longer depend conceptually on command-to-command routing inside the legacy tree

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 115: Control and Runtime Native CLI Delivery

**Goal:** Define the native CLI delivery path for control and runtime entrypoints so those operator flows stop defaulting to legacy command hubs.

**Depends on:** Phase 114
**Requirements:** `NDL-11`

**Success criteria:**
1. the roadmap defines native CLI ownership for control and runtime entrypoints explicitly
2. the control and runtime command path is routed through app ports instead of command-local orchestration
3. the milestone preserves a compatibility story for any still-live legacy entrypoints

**Plans:** 0/0 plans complete

Plans:
- none yet

### Phase 116: CLI Boundary Separation and Compatibility Shim Plan

**Goal:** Make parsing, rendering, app invocation boundaries, and the temporary compatibility shim plan explicit enough to implement the native CLI slice safely.

**Depends on:** Phase 115
**Requirements:** `NDL-12`

**Success criteria:**
1. the roadmap separates parsing, rendering, and app invocation responsibilities explicitly
2. the compatibility shim rule for any still-live legacy CLI paths is concrete and bounded
3. the live planning surface leaves the native-delivery roadmap at `3/8`, or about `38%`, only if the CLI replacement slice is explicit end to end

**Plans:** 0/0 plans complete

Plans:
- none yet
