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
- 🚧 **v1.12 Secure Node Connectivity and SSH Tunnel Revisit** — active

## Current Status

- Active milestone: **v1.12 Secure Node Connectivity and SSH Tunnel Revisit**
- Progress: 0 of 4 phases complete
- Most recent shipment: **v1.11 Crates.io and Docs.rs Publication Foundation**
- Current execution: **Phase 53 not started**
- Next step: `$gsd-plan-phase 53` or `$gsd-autonomous`

## Live Planning

### Phase Checklist

- [ ] **Phase 53: Node Identity and Topology Contract**
- [ ] **Phase 54: SSH Tunnel Bootstrap and Remote Connectivity Path**
- [ ] **Phase 55: Node Inspection, Recovery, and Operator Controls**
- [ ] **Phase 56: Node Docs, Onboarding, and Verification Exit**

### Phase 53: Node Identity and Topology Contract

**Goal:** Define one truthful contract for local runtime, distributed nodes, mobile nodes, and SSH-tunneled remote access so the product stops mixing transport, topology, and node terminology.

**Success criteria:**
- node roles and supported topology modes are explicitly defined
- the trust boundary between local runtime and remote connectivity is clear
- unsupported or future node behaviors are called out honestly

**Plans:** 0 plans complete

Plans:
- [ ] 53-01 Define the bounded node and topology contract

### Phase 54: Node-First Remote Connectivity and Permanent Tunnel Fallback

**Goal:** Make remote connectivity node-first while turning permanent SSH tunnel usage into an explicit advanced fallback path for self-hosted deployments.

**Success criteria:**
- the supported node-first path is documented or configured intentionally
- permanent tunnel fallback steps are documented or configured intentionally
- bootstrap and failover steps for remote connectivity are repeatable
- the node path and tunnel fallback both preserve the existing security and control boundary

**Plans:** 0 plans complete

Plans:
- [ ] 54-01 Implement the node-first path with permanent tunnel fallback

### Phase 55: Node Inspection, Recovery, and Operator Controls

**Goal:** Surface remote-node connectivity, failover, tunnel state, and recovery evidence through shipped inspection and operator surfaces.

**Success criteria:**
- operators can inspect remote connectivity from one coherent shipped surface
- failure modes distinguish auth, config, connectivity, node-path, and tunnel-fallback problems
- repair or reconnect guidance is preserved in runtime evidence or operator docs

**Plans:** 0 plans complete

Plans:
- [ ] 55-01 Surface node and tunnel health in operator controls

### Phase 56: Node Docs, Onboarding, and Verification Exit

**Goal:** Close the milestone by aligning onboarding, docs, and verification around the supported local and remote node connectivity contract.

**Success criteria:**
- setup guidance differentiates standard local deployment from advanced remote connectivity
- docs and control surfaces tell the same node and tunnel story
- milestone verification preserves the supported node and SSH tunnel contract

**Plans:** 0 plans complete

Plans:
- [ ] 56-01 Align docs, onboarding, and milestone verification
