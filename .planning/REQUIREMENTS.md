# Requirements: OpenRustClaw

## Active Milestone: v1.11 Crates.io and Docs.rs Publication Foundation

**Goal:** Establish the first truthful public Rust package surface for OpenRustClaw through an explicit publishable crate set, crates.io-ready metadata, docs.rs-ready documentation, and a repeatable publication and verification loop.

## Requirements

### PUB-01 Public crate scope

OpenRustClaw must define the first publishable crate set explicitly, including which workspace crates are public publish targets now, which remain internal-only, and any required publish ordering between crates.

**Acceptance signals:**
- the workspace has one canonical publishability contract
- internal-only crates are marked or documented as non-publish targets
- the initial public crate set is small enough to be supportable and truthful

### PUB-02 Crates.io package metadata

The selected public crates must expose crates.io-ready metadata and packaging inputs, including accurate package descriptions, repository and documentation links, readme paths, license data, and any required keywords or categories.

**Acceptance signals:**
- selected crates pass packaging or publish dry-runs without metadata surprises
- package metadata matches the shipped product story and repo ownership
- packaging inputs do not depend on missing or private files

### PUB-03 Docs.rs documentation contract

The selected public crates must render truthfully on docs.rs, with a defined docs build contract and any needed `package.metadata.docs.rs` configuration or feature shaping.

**Acceptance signals:**
- docs.rs-targeted builds succeed locally or through the closest truthful equivalent
- the public crate docs have one intentional entry surface
- docs configuration is documented for future releases

### PUB-04 Publish and verification loop

Operators must have one documented and repeatable publish path for the first public crates.io release, covering credentials, publish order, dry-run verification, post-publish checks, and docs.rs follow-up.

**Acceptance signals:**
- the publish runbook covers both preflight and post-publish checks
- the first public release path is proven or left at a truthful blocker checkpoint with evidence
- milestone verification preserves the crates.io and docs.rs evidence bundle

## Most Recent Archive

- Last shipped milestone: `v1.10 Release Binaries Workflow Recovery`
- Archived requirements: `.planning/milestones/v1.10-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.10-VERIFICATIONS.md`

## Next Step

Start execution with `$gsd-plan-phase 49` or `$gsd-autonomous`.
