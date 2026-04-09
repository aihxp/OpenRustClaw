---
phase: 46
requested_reviewers: [gemini, claude]
reviewers: [gemini, claude]
reviewed_at: 2026-04-09T17:57:54.008Z
plans_reviewed: [46-01-PLAN.md]
---
# Cross-AI Plan Review — Phase 46

## Gemini Review

Here is the cross-AI review for Plan 46-01: Repair Linux Release Workflow Dependencies.

### Summary
The plan correctly identifies the necessary missing dependencies (`libasound2-dev`, `libssl-dev:arm64`) to fix the Linux release builds in the GitHub Actions workflow. It sensibly proposes validating the workflow via `workflow_dispatch` to ensure it works on real runners. However, it glosses over the significant technical nuances of cross-compiling Rust projects that depend on C libraries (like OpenSSL and ALSA) on Ubuntu runners, and it misses the explicit constraint to handle ARM64 fallback if it continues to fail.

### Strengths
- **Directly targets the root cause:** Identifies the exact missing dependencies required for ALSA and OpenSSL.
- **Real-world validation:** Using `workflow_dispatch` via the `gh` CLI to test the workflow on actual GitHub runners is the best way to verify CI changes.
- **Maintainability:** Seeks to keep the workflow readable and updates admin helpers/docs.

### Concerns
- **Missing Architecture Setup (MEDIUM):** The plan mentions installing `libssl-dev:arm64`, but doesn't mention that the Ubuntu runner must first be configured to accept arm64 packages via `sudo dpkg --add-architecture arm64` and `sudo apt-get update`. Without this, the apt install step will fail.
- **Vague Cross-Compilation Environment (HIGH):** Cross-compiling C dependencies in Rust requires very specific environment variables. The plan vaguely mentions "pkg-config or OpenSSL environment", but if variables like `PKG_CONFIG_ALLOW_CROSS=1`, `PKG_CONFIG_SYSROOT_DIR`, `OPENSSL_DIR`, or the specific cross-compiler linker flags (`CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER`) are not strictly defined, the build will attempt to link x86_64 libraries and fail.
- **Missing Fallback Constraint (HIGH):** The context explicitly states: *"If the ARM64 Linux lane still fails after a truthful sysroot setup, the target must be re-scoped rather than silently kept as supported."* The plan does not include a step to handle this outcome.

### Suggestions
- **Detail the `apt` setup:** Explicitly include the `dpkg --add-architecture arm64` and `apt-get update` commands in the plan's steps for the ARM64 job.
- **Define Environment Variables:** Specify the exact environment variables that will be injected for the `aarch64-unknown-linux-gnu` target, such as `PKG_CONFIG_ALLOW_CROSS=1` and pointing `PKG_CONFIG_PATH` to the arm64 pkgconfig directories (e.g., `/usr/lib/aarch64-linux-gnu/pkgconfig`).
- **Add a Contingency Step:** Add a step: "If `workflow_dispatch` for ARM64 fails after sysroot setup, remove `aarch64-unknown-linux-gnu` from the release matrix and update documentation to reflect that it is currently unsupported."
- **Use `cross` as an alternative:** If native cross-compilation proves too brittle on the standard Ubuntu runners, consider using the `cross` tool (cargo-cross) in the workflow for the ARM64 target, as it handles sysroot and C-dependencies automatically via Docker containers.

### Risk Assessment
**MEDIUM**. The risk lies primarily in the execution of Step 2. Cross-compiling Rust projects with native C dependencies (OpenSSL, ALSA) on Debian/Ubuntu systems is notoriously finicky. If the environment variables and `dpkg` architectures are not aligned perfectly, the CI pipeline will continue to fail, blocking the release lane. Addressing the specific environment configurations in the plan will mitigate this risk.

---

## Claude Review

The review is complete above. The plan is sound with **LOW overall risk** — the key callout is ensuring `dpkg --add-architecture arm64` and runner image pinning are handled during execution.

---

## Consensus Summary

### Agreed Strengths
- Reviewers agreed on the phase direction and sequencing, but they emphasized different strengths.

### Agreed Concerns
- No exact shared concern wording emerged across reviewers; use the reviewer sections above for plan-specific concerns.

### Divergent Views
- Not all reviewers exposed a phase-level risk label; parsed labels: gemini=MEDIUM.
