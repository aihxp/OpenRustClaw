# Project Milestones: OpenRustClaw

## v1.14 Continued Greenfield Conversion (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 4 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.14-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Broadened the greenfield application lane from one proving slice into multiple shipped service seams across reports, routes, and compiled-skill overview behavior.
- Moved self-hosted product-mode summary composition and the `/control/self-hosted/product-mode` transition path behind `openrustclaw-app`.
- Migrated the mobile node operator summary report into the greenfield lane while preserving the shipped runtime and Control UI contract.
- Carved the first bounded `skills.rs` seam by centralizing compiled manifests, artifacts, executable-component discovery, and reference reading behind one shared application service.

---

## v1.13 Brownfield-to-Greenfield Transition (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 4 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.13-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Defined one canonical greenfield transition contract instead of leaving architectural cleanup as vague rewrite language.
- Added `openrustclaw-app` as the first real application-layer shell for future migration work.
- Migrated the setup handoff proving slice so report composition now runs through the new application lane while preserving the shipped runtime and Control UI contract.
- Locked contributor defaults so legacy command hubs are now explicitly compatibility surfaces unless a migration slice targets them.

---

## v1.12 Secure Node Connectivity and SSH Tunnel Revisit (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 4 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.12-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Defined one bounded node and topology contract across local runtime, distributed and mobile nodes, SSH tunnel fallback, and reverse-proxy last-resort recovery.
- Persisted the chosen remote-connectivity profile in onboarding and setup state instead of leaving remote access as transient tunnel guidance.
- Surfaced the saved primary remote path and fallback order in the shipped `Setup Handoff` operator surface.
- Aligned installation, quickstart, deployment docs, and milestone verification around the same node-first remote-connectivity story.

---

## v1.11 Crates.io and Docs.rs Publication Foundation (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 4 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.11-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Selected `openrustclaw-core` as the first public crate boundary instead of over-claiming the whole workspace.
- Normalized crates.io metadata and added an explicit docs.rs build contract plus a real crate-level documentation entry surface.
- Added a repeatable crates.io readiness script and operator runbook for future package releases.
- Published `openrustclaw-core v0.1.0` to crates.io and confirmed docs.rs availability.

---

## v1.10 Release Binaries Workflow Recovery (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 4 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.10-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Captured the live failing tag contract from public GitHub evidence instead of guessing from stale release assumptions.
- Repaired the supported Linux and macOS release matrix on GitHub-hosted runners and aligned the admin helper around that contract.
- Proved the tagged publish path on `v1.10-rc1`, which now exposes tarball and checksum assets for all four supported targets.
- Closed the milestone with one repeatable operator verification loop for branch dry-runs, tag validation, and archived release evidence.

---

## v1.9 GitHub Repository Presence and Actions Recovery (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 8 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.9-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Replaced the stale public GitHub repo framing with the current self-hosted Rust-first assistant product story.
- Made the canonical GitHub topic contract explicit and re-verifiable through the repo-admin workflow.
- Repaired the shipped-surface GitHub workflow contract so the latest `main` CI and E2E runs now pass on the live repo surface.
- Added one repeatable repo-admin loop for both metadata sync and GitHub Actions health verification.

---

## v1.8 Clean Codebase (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 8 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.8-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Created a canonical cleanup inventory and no-touch boundary contract for brownfield refactors.
- Aligned CI to the current canonical planning docs and made sidecar local Python state explicit non-canonical repo surface.
- Extracted the control-auth and enterprise-access middleware slice from `start.rs` into `crates/cli/src/commands/start/auth.rs` with focused tests.
- Added a rerunnable repo-hygiene guardrail script and wired it into CI alongside the cleanup verification bundle.

---

## v1.7 Documentation Convergence and OpenClaw-Inspired Docs Rewrite (Shipped: 2026-03-27)

**Phases completed:** 5 phases, 10 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.7-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Defined the canonical documentation ownership contract across README, root planning docs, mdBook mirrors, and milestone artifacts.
- Rewrote the README and mdBook entry surfaces so OpenRustClaw now reads like one coherent self-hosted product.
- Aligned installation, quickstart, and first-agent around the shipped onboarding, repair, and setup-handoff lifecycle.
- Rewrote operator and planning docs into a tighter, truthful runbook set and locked the result in with a standing docs audit and maintenance workflow.

---

## v1.6 Proper Onboarding and Setup (Shipped: 2026-03-28)

**Phases completed:** 4 phases, 10 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.6-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Added a durable setup-state contract with resumable Standard, Advanced, and Custom onboarding paths.
- Made onboarding validate provider, runtime, and channel bootstrap through shipped health and probe surfaces instead of assuming config writes equal readiness.
- Added an explicit repair path for existing workspaces that derives targeted rerun steps from setup state and doctor diagnostics.
- Closed the loop with a shared setup handoff contract across onboarding, `/control/setup/handoff`, Control UI, and the setup docs.

---

## v1.5 Self-Hosted Product Modes and Lifecycle Packaging (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 8 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.5-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Added a first-class self-hosted product-mode contract for `solo`, `team`, `company`, and `enterprise` deployments.
- Made onboarding choose and persist the deployment path explicitly instead of leaving it implied by later configuration drift.
- Added explicit upgrade and downgrade transitions with durable receipts and retained-state warnings.
- Aligned the README, installation path, quickstart, first-agent guide, and shipped dashboard wording around the same self-hosted open-source product story.

---

## v1.4 Enterprise Governance and Operator-Gated Full Autonomy (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 12 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.4-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Deepened enterprise governance with explicit per-scope approval-chain and separation-of-duties rules.
- Expanded enterprise audit retention and review so governance, supervision, and operator evidence are exportable and reviewable from one typed surface.
- Added an explicit operator-gated full-autonomy lane with bounded budgets, baseline restoration, durable lifecycle events, and a kill switch.
- Closed the milestone with a shipped Control UI surface for inspecting and controlling the stronger autonomy lane from the same enterprise admin workflow.

---

## v1.3 Enterprise Expansion and Supervised Autonomy Foundations (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 12 plans, 0 tasks
**Verification archive:** `.planning/milestones/v1.3-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Added the first-class enterprise operator registry, bootstrap flow, and typed access summary.
- Expanded enterprise policy into a unified typed control surface with durable audit-export bundles.
- Added explicit supervised-autonomy escalation, rollback, and intervention evidence for active orchestration runs.
- Closed the milestone with a shipped enterprise admin loop in Control UI backed by a typed enterprise admin summary.

---

## v1.2 Deeper OpenClaw Surface Parity (Shipped: 2026-03-27)

**Phases completed:** 5 phases, 15 plans, 28 tasks
**Verification archive:** `.planning/milestones/v1.2-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Added a durable browser workflow history ledger plus shipped runtime and dashboard inspection for richer bounded browser runs.
- Expanded orchestration parity with typed supervision reports and Control UI visibility into delegated tasks, worker outcomes, and live attention signals.
- Added a typed per-node mobile operator report so approval pressure, sync conflicts, and recent node activity are visible from one runtime summary.
- Replaced the remaining priority raw dashboard panes with typed Control UI renderers across voice, talk, extension, bounded voice-call, and mobile sub-detail surfaces.
- Added a typed voice operator report and a report-driven top-level voice Control UI surface over persisted voice sessions, talk receipts, and bounded voice-call evidence.

---

## v1.1 Lifecycle Integrity and Enterprise Foundations (Shipped: 2026-03-26)

**Phases completed:** 3 phases, 9 plans, 23 tasks
**Verification archive:** `.planning/milestones/v1.1-VERIFICATIONS.md`
**Verification debt:** none

**Key accomplishments:**

- Hardened the core lifecycle contract so milestone bootstrap and phase completion now respect real verification state instead of optimistic filesystem assumptions.
- Made verification readiness debt visible across the milestone so operators can see missing or stale verification before they attempt lifecycle operations.
- Closed the manual gap by making direct verification scaffolds schema-compatible and documenting the hard verification gate where phase execution hands off to completion.
- Made milestone completion preserve verification evidence as a first-class archive artifact instead of leaving it implicit in live phase directories.
- Aligned audit, archive, and cleanup guidance around the same milestone verification archive artifact the CLI now generates.
- Updated the live project brief to reflect the real v1.1 lifecycle state and closed the phase with archive-focused verification evidence.
- Built the typed enterprise foundations summary so approval-sensitive actions now have one operator-facing runtime surface instead of scattered raw evidence.
- Exposed the enterprise baseline in Control UI so operators can answer approval-and-audit questions from the shipped dashboard instead of jumping between raw endpoints.
- Aligned the operator docs and verification record with the shipped enterprise baseline so the next milestone can build on a truthful contract.

---

## v1.0 Rust OpenClaw MVP (Shipped: 2026-03-26)

**Delivered:** A trustworthy Rust-first OpenClaw-style MVP across onboarding, assistant continuity, memory policy, tools/coding evidence, communications, runtime ops, security posture, and release exit.

**Phases completed:** 7 phases, 21 plans, 53 tasks

**Key accomplishments:**

- Hardened the onboarding post-check so the wizard no longer offers the persisted assistant when first-start prerequisites are still missing.
- Added workflow-level onboarding regression coverage so first-run launch and existing-workspace detection are no longer guarded only by helper-level unit tests.
- Aligned the repo entrypoint, installation guide, and quickstart so they all describe the same doctor-backed first-run path.
- Added a typed assistant continuity contract so resumed session trust is visible in both control inspection and the CLI.
- Surfaced the assistant continuity contract directly in Control UI so resumed session trust is visible without decoding raw JSON.
- Closed the phase by aligning quickstart and README language with the shipped continuity contract in CLI and Control UI.
- Locked the default assistant memory lane behind an explicit policy gate so it no longer persists arbitrary conversation details.
- Made assistant memory-write decisions visible from shipped operator surfaces instead of hiding them in SQLite metadata.
- Closed the memory phase by aligning docs and end-to-end verification with the stricter assistant write contract.
- Added a durable execution ledger so recent tool and MCP activity is now inspectable from shipped control surfaces instead of disappearing into logs and traces.
- Made the Cursor coding lane auditable by persisting per-run artifacts for command, edit, and other coding-tool executions under the workspace control plane.
- Closed the Phase 4 trust loop by making recent coding artifacts inspectable from the shipped control plane and documenting how operators review the retained evidence.
- Started the communications phase by making recent Gmail ingress activity durable and visible from shipped operator surfaces instead of raw webhook logs.
- Hardened the voice lane by turning persisted session receipts into operator-readable recent outcomes instead of leaving voice diagnostics buried in raw JSON blobs.
- Closed the communications phase by documenting the actual operator inspection loop and adding one combined regression test that covers both recent email and recent voice diagnostics.
- Added one coherent operator-ops summary so deploy, restart, and recovery decisions no longer require hopping between unrelated runtime endpoints.
- Aligned README, installation, and production docs around the actual Rust runtime maintenance path instead of leaving restart and recovery scattered across command references.
- Closed Phase 6 with an integration test that proves the runtime operator summary still exposes health, lock, and recovery guidance from realistic workspace state.
- Added a typed security posture summary so operators can inspect release-critical security defaults from shipped control surfaces instead of relying on CLI-only audit output.
- Turned the MVP closeout into an explicit operator release checklist tied to the real runtime, security, and observability surfaces.
- Locked the MVP release exit behind an automated verification bundle that exercises security posture, metrics exposure, origin protection, and runtime budgets.

**Known gap:**

- The archived v1.0 audit records missing per-phase `VERIFICATION.md` artifacts as lifecycle debt even though the release gate passed.

---
