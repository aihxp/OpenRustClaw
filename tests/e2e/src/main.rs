//! E2E test runner binary for OpenRustClaw.
//!
//! This binary can be used to run E2E tests in a standalone mode,
//! useful for CI/CD pipelines or manual testing.
//!
//! Usage:
//!   cargo run --bin e2e
//!   E2E_LIVE=1 cargo run --bin e2e

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     OpenRustClaw E2E Test Runner                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check environment
    let live_mode = std::env::var("E2E_LIVE").is_ok_and(|v| v == "1" || v == "true");
    
    if live_mode {
        println!("🌐 Running in LIVE provider mode");
        println!("   (requires API keys for providers)");
        
        // Check for API keys
        let providers = vec!["OPENAI", "ANTHROPIC", "OLLAMA"];
        for provider in &providers {
            let key = format!("{}_API_KEY", provider);
            if std::env::var(&key).is_ok() {
                println!("   ✓ {} configured", provider);
            } else {
                println!("   ✗ {} not configured", provider);
            }
        }
    } else {
        println!("🔧 Running in MOCK mode (default)");
        println!("   Use E2E_LIVE=1 for live provider tests");
    }
    
    println!();
    println!("To run tests, use: cargo test --test e2e");
    println!();
    println!("Test suites:");
    println!("  - test_chat_workflow      : Chat and conversation flows");
    println!("  - test_memory_workflow    : Memory storage and retrieval");
    println!("  - test_scheduler_workflow : Job scheduling and execution");
    println!("  - test_security_workflow  : Authentication and security");
    println!("  - test_provider_fallback  : Provider resilience");
    println!();

    ExitCode::SUCCESS
}
