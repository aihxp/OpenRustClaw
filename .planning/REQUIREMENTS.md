# Requirements: OpenRustClaw

**Defined:** 2026-03-27
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Repository Presence

- [x] **GHMD-01**: The public GitHub repo About description and linked entry metadata describe OpenRustClaw as a self-hosted open-source Rust-first assistant platform rather than the older hybrid framework framing.
- [x] **GHMD-02**: The repo's public entry surface, including README badges and linked resources, points at the current shipped documentation and release paths.

### Discovery and Topics

- [x] **DISC-01**: OpenRustClaw has a canonical GitHub topic or word-tag set that reflects the product's self-hosted, Rust-first, assistant, and enterprise-capable positioning.
- [x] **DISC-02**: The canonical GitHub topic or tag set is documented in-repo so future repo-admin updates stay consistent.

### Actions and Release Automation

- [x] **ACT-01**: Public GitHub Actions workflows referenced by badges, release tags, or operator docs execute against the current repo layout and shipped verification contract.
- [x] **ACT-02**: Stale, failing, or misleading GitHub Actions jobs are repaired, removed, or clearly downgraded to informational status.
- [x] **ACT-03**: Tag or release automation stays consistent with shipped milestone tags and binary publishing expectations.

### GitHub Admin Verification

- [x] **OPS-01**: The repo includes a repeatable admin sync path for About metadata, topics, and workflow health when authenticated GitHub access is available.
- [x] **OPS-02**: The milestone closes with evidence from both local verification and live GitHub surface checks.

## v2 Requirements

### Broader GitHub Product Operations

- **GHOP-01**: Add issue forms, pull request templates, and contributor workflow polish beyond the minimum repo-admin contract.
- **GHOP-02**: Add broader community or marketing surfaces such as Discussions, a project board, or a landing site integration.

## Out of Scope

| Feature | Reason |
|---------|--------|
| A full website or marketing redesign outside the repo surface | This milestone is about GitHub truthfulness and automation, not a separate marketing lane |
| Growth tactics such as stars, social campaigns, or SEO programs | Discovery must be truthful first; growth work is premature |
| Broad new product features unrelated to repo presence or GitHub automation | Keep the milestone tightly bounded to external repo presentation and CI health |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| GHMD-01 | Phase 41 | Complete |
| GHMD-02 | Phase 41 | Complete |
| DISC-01 | Phase 42 | Complete |
| DISC-02 | Phase 42 | Complete |
| ACT-01 | Phase 43 | Complete |
| ACT-02 | Phase 43 | Complete |
| ACT-03 | Phase 43 | Complete |
| OPS-01 | Phase 44 | Complete |
| OPS-02 | Phase 44 | Complete |

**Coverage:**
- v1 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-28 after completing phases 43-44*
