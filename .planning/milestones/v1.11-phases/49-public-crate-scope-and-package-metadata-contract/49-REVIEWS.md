---
phase: 49
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T18:03:59.692Z
plans_reviewed: [49-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 49

## Gemini Review

# Review: Plan 49-01 - Lock the First Public Crate Boundary

## Summary
The plan is pragmatic, tightly scoped, and correctly targets `openrustclaw-core` as the initial candidate for crates.io publication. By focusing on a single crate that lacks internal path dependencies, the plan minimizes complexity and risk. It properly identifies the need to update stale repository URLs and missing metadata, and provides standard Cargo commands for verification. Overall, it is a solid foundation for establishing a public Rust package presence.

## Strengths
- **Tight Scoping:** Limiting the initial publication to just `openrustclaw-core` significantly reduces the risk of accidental internal API leakage.
- **Clear Identification of Gaps:** The plan (supported by the context) correctly identifies specific missing metadata fields like `documentation`, `homepage`, and the stale `repository` URL.
- **Standard Verification:** Utilizing `cargo metadata` coupled with `jq`, and `cargo package` for dry-runs are idiomatic and effective ways to verify crate packaging state.

## Concerns
- **MEDIUM - Missing Explicit Prevention:** The plan mentions keeping non-target crates out of scope, but it does not explicitly state the mechanism. Relying on manual exclusion during `cargo publish` is error-prone. 
- **LOW - Redundant Step:** Step 1 ("Audit the workspace manifests...") is slightly redundant since the context already establishes `openrustclaw-core` as the chosen target, but it's harmless as a final sanity check.
- **LOW - Incomplete Dry-Run:** The verification step uses `cargo package --no-verify`. While this is fine for quickly checking if the tarball can be assembled, it skips building the packaged source. If the crate is missing a file in its `include` directive, `--no-verify` will mask the issue.
- **LOW - Discoverability Metadata:** The verification `jq` script checks for `keywords` and `categories`, but the plan steps don't explicitly mention adding them. These are critical for a successful crates.io launch.

## Suggestions
- **Enforce Internal Boundaries:** Add a specific step to set `publish = false` in the `Cargo.toml` of all workspace crates *except* `openrustclaw-core`. This acts as a hard guardrail against accidental publication of internal binaries or unready SDKs.
- **Enhance Discoverability:** Explicitly include a step to define relevant `categories` (matching crates.io's strict list) and `keywords` in the `openrustclaw-core` manifest.
- **Strengthen Verification:** Change the final verification to run `cargo package -p openrustclaw-core` (removing the `--no-verify` flag) to ensure the crate actually compiles cleanly from the generated package source.
- **Verify License:** Ensure the `license` or `license-file` field is explicitly set and that the `LICENSE` file is included in the package payload.

## Risk Assessment
**Risk Level: LOW**

**Justification:** The scope of this plan is strictly limited to metadata updates and packaging configuration. It does not alter runtime behavior, architectural boundaries, or external APIs. The primary risk is publishing a crate with incomplete metadata, which is easily mitigated by the suggested additions. Enforcing `publish = false` on the rest of the workspace will eliminate the only moderate risk of accidental publication.

---

## Claude Review

# Cross-AI Review: Phase 49 — Public Crate Scope and Package Metadata Contract

## Plan 49-01: Lock the First Public Crate Boundary

### Summary

A straightforward, well-scoped plan that selects `openrustclaw-core` as the first public crate, normalizes its metadata, and verifies packaging readiness. The scope is appropriately narrow for a first publication wave.

### Strengths

- Correctly identifies `openrustclaw-core` as the right first target (no internal path dependencies)
- Includes concrete verification commands
- Explicitly keeps non-target crates out of scope
- Addresses the stale repository URL (`openrustclaw/openrustclaw` → `aihxp/OpenRustClaw`)

### Concerns

- **MEDIUM** — No mention of `publish = false` for non-target crates. Without this, an accidental `cargo publish --workspace` could attempt to publish internal crates.
- **MEDIUM** — No discussion of semver strategy. The crate version in `Cargo.toml` needs to be intentional before first publish — once `0.x.y` or `1.x.y` hits crates.io, the version is consumed permanently.
- **LOW** — The `--allow-dirty` and `--no-verify` flags in verification skip build checks. The final publish should not use these flags; the plan should note this distinction.
- **LOW** — No mention of checking that `openrustclaw-core`'s `README.md` content is appropriate for a public crates.io listing (it renders on the crate page).
- **LOW** — Missing `license-file` vs `license` field consideration. The plan says normalize metadata but doesn't specify which license field form to use.

### Suggestions

- Add `publish = false` to workspace-level `Cargo.toml` defaults or to each non-target crate manifest to prevent accidental publication.
- State the intended version number explicitly and confirm it hasn't been squatted on crates.io.
- Separate "dry-run verification" (with `--allow-dirty`) from "publish-ready verification" (clean git state, full `--verify`).
- Review `openrustclaw-core/README.md` content for public suitability as a sub-step.

### Risk Assessment

**LOW** — The plan is minimal and achievable. The main risk is procedural (accidental publish of internal crates), not architectural. The concerns above are all addressable within the same phase without scope expansion.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: claude=LOW.
