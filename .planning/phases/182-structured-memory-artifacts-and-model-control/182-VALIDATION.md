# Phase 182 Validation

## Goal-backward check

Phase 182 is complete only if all of the following are true:

1. User model, operator model, project memory, and archive summaries exist as distinct durable artifact classes.
2. Promotion into those artifacts happens only through explicit policy-gated flows with source lineage.
3. Only a bounded subset of structured artifacts reaches active context.
4. Operators can inspect, correct, deactivate, and remove artifacts through existing shipped surfaces.

## Required evidence

### Storage and policy
- tests prove durable artifact rows round-trip with kind, status, and lineage
- tests prove promotion is blocked when lineage is missing or summary content is empty
- tests prove promoting a new active artifact supersedes the prior active artifact for the same namespace/kind

### Projection
- tests prove only reserved typed keys are projected into core memory
- tests prove projection removes stale reserved keys after deactivation/removal
- tests prove prompt rendering still depends on core memory only

### Operator surfaces
- tests prove CLI/MCP listing returns typed artifact data
- tests prove correction updates the summary while preserving lineage
- tests prove deactivate and remove paths stop projecting the artifact

## Verification commands

- `cargo test -p openrustclaw-core --lib`
- `cargo test -p openrustclaw-memory model_artifacts -- --nocapture`
- `cargo test -p openrustclaw-db memory_store -- --nocapture`
- `cargo test -p openrustclaw-cli memory -- --nocapture`
- `cargo test -p openrustclaw-cli inspect -- --nocapture`
- `cargo test -p openrustclaw-cli mcp_server_memory_tools_persist_and_render -- --nocapture`
- `cargo check -p openrustclaw-cli --tests`

## Phase guardrails

- Do not promote learning candidates or lessons in this phase
- Do not generate skill proposals in this phase
- Do not widen runtime authority or God Mode in this phase
- Do not inject raw artifact dumps into the prompt
