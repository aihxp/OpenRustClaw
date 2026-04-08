# Requirements: OpenRustClaw v1.46 Milestone Audit, Deck Convergence, and Release Alignment

**Defined:** 2026-04-08
**Core Value:** Ship a trustworthy Rust-native assistant platform that can do real work end-to-end, not just demo isolated features.

## v1 Requirements

### Milestone and Archive Audit

- [ ] **AUD-01**: The shipped milestone stack is reviewed against its preserved audit and verification artifacts instead of milestone memory.
- [ ] **AUD-02**: Any real residual blocker, release mismatch, or documentation drift is preserved once in canonical planning docs instead of remaining implicit.

### Canonical Deck Convergence

- [ ] **DOC-01**: The live planning deck reflects `v1.45` as shipped and `v1.46` as the active audit/release milestone with archived `v1.45` roadmap and requirements snapshots preserved.
- [ ] **DOC-02**: The canonical docs deck describes the shipped delegated-agent fabric, Cursor delegated-backend support, routing console, trusted remote backends, and guided first-task journey truthfully.
- [ ] **DOC-03**: The release docs explain the difference between planning milestone tags and the public semver release line so README, docs, and GitHub metadata do not contradict one another.

### Public Release Alignment

- [ ] **REL-01**: GitHub contains a truthful `v1.45` milestone release/tag as an archive marker for the shipped delegated-agent fabric milestone.
- [ ] **REL-02**: The public semver lane for `openrustclaw-core 1.4.1` is verified and either published successfully or stopped at a truthful credential or platform blocker with evidence.
- [ ] **REL-03**: GitHub `Latest` release metadata points at the public semver release line rather than the planning milestone line.
- [ ] **REL-04**: Release evidence records the verification commands used, the final GitHub release URLs, and the crates.io/docs.rs state.

## v2 Requirements

### Release Automation Hardening

- **REL-05**: The repo can generate milestone-archive release notes and semver release notes from separate templates so the two public surfaces stop drifting.
- **REL-06**: Release evidence is emitted automatically into a single planning archive file after a successful publish.

### Docs Deck Automation

- **DOC-04**: Canonical docs freshness can be linted so stale shipped-surface claims fail CI before release.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Broad net-new product features unrelated to audit, documentation convergence, or release alignment | This milestone is for truthfulness and release integrity, not breadth expansion |
| Expanding crates.io publication beyond `openrustclaw-core` | The public crate lane remains intentionally narrow until a new publish contract exists |
| Rewriting every historical milestone summary or audit file | The goal is to preserve truthful archives and repair current canonical surfaces, not flatten history |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUD-01 | Phase 196 | Planned |
| AUD-02 | Phase 196 | Planned |
| DOC-01 | Phase 197 | Planned |
| DOC-02 | Phase 197 | Planned |
| DOC-03 | Phase 197 | Planned |
| REL-01 | Phase 198 | Planned |
| REL-02 | Phase 198 | Planned |
| REL-03 | Phase 198 | Planned |
| REL-04 | Phase 199 | Planned |

**Coverage:**
- v1 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-04-08*
*Last updated: 2026-04-08 after milestone initialization*
