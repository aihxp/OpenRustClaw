# OpenRustClaw

## Project Structure
Rust workspace with 14 crates in `crates/`. Python sidecar in `sidecar/`.

## Build
```
cargo build --workspace
```

## Test
```
cargo test --workspace
```

## Key Conventions
- Native provider SDKs only (reqwest for now, anthropic_rust/async-openai/openrouter_api planned) — never raw HTTP outside providers crate
- All DB access through crates/db — sqlx for general, libSQL for vectors, rusqlite for CLI
- Memory writes go through crates/memory/src/policies.rs (dedupe, confidence, TTL)
- No cron jobs — all scheduling via LangGraph workflows in sidecar/
- MCP tools defined in crates/mcp/server.rs
- Security: WebSocket origin validation on all connections; token auth enabled by default via gateway config
- 3-tier memory: Core (always loaded, ~500 tokens) → Recall (on-demand search) → Archive (consolidated)
- Recall-only memory: NEVER inject full memory files into system prompt

## Crate Dependency Order
core → db → memory, providers, mcp, observability, security → agent → gateway → channels → skills, scheduler → langbridge → cli

## Error Handling
- thiserror for library errors (crates/core/src/error.rs)
- anyhow for CLI/application errors

## Code Style
- Use tracing macros (info!, warn!, error!) — never println! outside CLI
- All async functions use tokio runtime
- Prefer Arc<dyn Trait> over generics for plugin boundaries
