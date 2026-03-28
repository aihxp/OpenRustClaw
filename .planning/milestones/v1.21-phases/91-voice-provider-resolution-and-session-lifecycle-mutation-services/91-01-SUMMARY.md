# Phase 91 Summary

Voice provider resolution and voice-session lifecycle mutation now compose through `openrustclaw-app::voice_runtime_lifecycle`. `voice_runtime.rs` continues to handle transcriber and synthesis I/O plus session-file persistence, but the provider and session state machine no longer live only in the command module.

## Evidence

- `crates/app/src/voice_runtime_lifecycle.rs`
- `crates/cli/src/commands/voice_runtime.rs`
- `cargo test -p openrustclaw-app voice_runtime_lifecycle -- --nocapture`
- `cargo check -p openrustclaw-cli --lib`
