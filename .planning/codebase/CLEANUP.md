# Codebase Cleanup Contract

**Created:** 2026-03-27
**Purpose:** Canonical cleanup inventory for `v1.8 Clean Codebase`

## How To Use This File

Use this file before deleting, moving, or splitting code. It identifies:

- priority cleanup targets for this milestone
- no-touch or extra-care boundaries where behavior regressions are likely
- stale or drifting repo surfaces that should be aligned before more feature work lands

This file is planning-facing and complements:

- `.planning/codebase/CONCERNS.md`
- `.planning/codebase/STRUCTURE.md`
- `.planning/ROADMAP.md`
- `docs/documentation-contract.md`

## Inventory

### Priority Structural Hotspots

| Surface | Current Signal | Why It Matters | Target Phase |
|---------|----------------|----------------|--------------|
| `crates/cli/src/commands/start.rs` | ~14k lines | Control-plane, runtime, security, browser, mobile, enterprise, and UI routes are concentrated in one file | Phase 39 |
| `crates/cli/src/commands/skills.rs` | ~5.5k lines | Large mixed skills/control surface that is costly to reason about safely | follow-up debt after Phase 39 |
| `crates/channels/src/discord.rs` | ~3k lines | Large protocol adapter with high regression risk | follow-up debt |
| `crates/channels/src/teams.rs` | ~2.5k lines | Large protocol adapter with high regression risk | follow-up debt |

### Repo Drift Targets

| Surface | Current Signal | Cleanup Intent | Target Phase |
|---------|----------------|----------------|--------------|
| `.github/workflows/ci.yml` parity inventory job | still references deleted `docs/parity-matrix.md` and `docs/parity-positioning.md` | align CI to current canonical docs | Phase 38 |
| `sidecar/.venv`, `sidecar/.pytest_cache`, `sidecar/**/__pycache__` | local/generated paths inside repo tree | keep out of the canonical repo surface; do not depend on them for source truth | Phase 38 |
| root planning and codebase docs | concerns are documented, but cleanup targets were not previously prioritized in one place | preserve this contract as the shared cleanup index | Phase 37 |

### Canonical Surfaces

These are the current source-of-truth surfaces that cleanup work must preserve:

- runtime and control behavior described by `README.md`, `docs/roadmap.md`, `docs/feature-matrix.md`, `docs/surface-matrix.md`, and `docs/product-positioning.md`
- shipped operator surfaces under `crates/cli/src/commands/`
- enterprise, autonomy, setup, and docs contracts already archived through `v1.7`

## No-Touch Or Extra-Care Boundaries

### No-Touch Without Explicit Replacement

- authenticated control-plane behavior
- enterprise approval and governance enforcement
- setup, onboarding, and repair lifecycle contracts
- tagged or archived milestone artifacts

### Extra-Care Refactor Areas

- `start.rs` auth and middleware paths
- sidecar bridge and workflow compatibility contracts
- CI and docs references that enforce shipped-surface truth

## Phase Targets

### Phase 37

- write the cleanup inventory
- define no-touch boundaries
- choose the first concrete cleanup slice

### Phase 38

- remove or isolate known drift in CI and repo-hygiene surfaces
- make local/generated sidecar paths explicit non-canonical artifacts

### Phase 39

- split a cohesive slice out of `start.rs`
- preserve existing middleware behavior and tests

### Phase 40

- add or tighten targeted verification for cleanup-sensitive behavior
- record what debt remains after the bounded refactor

## Deferred Cleanup Debt

- broader `skills.rs` decomposition
- channel-adapter decomposition for Discord and Teams
- wider contract testing between Rust runtime and optional sidecar

---
*Update this file when cleanup targets are completed or reprioritized.*
