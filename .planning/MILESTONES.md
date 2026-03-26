# Project Milestones: OpenRustClaw

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
