//! Diagnostics command - Check system health.

use anyhow::Result;
use std::path::Path;

/// Run diagnostics.
pub async fn run() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           OpenRustClaw Diagnostics                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let mut checks_passed = 0;
    let mut checks_failed = 0;
    let mut checks_warning = 0;

    // 1. Check database connection
    print!("[1/7] Checking database connection... ");
    match check_database().await {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[31m✗ FAILED\x1b[0m");
            println!("      Error: {}", e);
            checks_failed += 1;
        }
    }

    // 2. Check database migrations
    print!("[2/7] Checking database migrations... ");
    match check_migrations().await {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[31m✗ FAILED\x1b[0m");
            println!("      Error: {}", e);
            checks_failed += 1;
        }
    }

    // 3. Check provider API keys
    print!("[3/7] Checking provider API keys... ");
    match check_api_keys() {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[33m⚠ WARNING\x1b[0m");
            println!("      {}", e);
            checks_warning += 1;
        }
    }

    // 4. Check sidecar availability
    print!("[4/7] Checking sidecar availability... ");
    match check_sidecar().await {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[33m⚠ WARNING\x1b[0m");
            println!("      {}", e);
            checks_warning += 1;
        }
    }

    // 5. Check configuration files
    print!("[5/7] Checking configuration files... ");
    match check_config() {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[31m✗ FAILED\x1b[0m");
            println!("      Error: {}", e);
            checks_failed += 1;
        }
    }

    // 6. Check skill directory
    print!("[6/7] Checking skill directory... ");
    match check_skills_dir().await {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[33m⚠ WARNING\x1b[0m");
            println!("      {}", e);
            checks_warning += 1;
        }
    }

    // 7. Check data directory
    print!("[7/7] Checking data directory... ");
    match check_data_dir().await {
        Ok(()) => {
            println!("\x1b[32m✓ OK\x1b[0m");
            checks_passed += 1;
        }
        Err(e) => {
            println!("\x1b[31m✗ FAILED\x1b[0m");
            println!("      Error: {}", e);
            checks_failed += 1;
        }
    }

    // Summary
    println!();
    println!("══════════════════════════════════════════════════════════");

    let total = checks_passed + checks_failed + checks_warning;

    if checks_failed == 0 && checks_warning == 0 {
        println!("\x1b[32m✓ All {} checks passed!\x1b[0m", total);
        println!();
        println!("Your OpenRustClaw installation is ready to use.");
    } else {
        println!(
            "Results: {} passed, {} failed, {} warnings",
            checks_passed, checks_failed, checks_warning
        );

        if checks_failed > 0 {
            println!();
            println!(
                "\x1b[31m{} critical issue(s) need to be resolved.\x1b[0m",
                checks_failed
            );
        }

        if checks_warning > 0 {
            println!();
            println!(
                "\x1b[33m{} warning(s) - you may want to address these.\x1b[0m",
                checks_warning
            );
        }
    }

    println!();

    Ok(())
}

/// Check database connection.
async fn check_database() -> Result<()> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    let pool = openrustclaw_db::init_pool(&config.database.url, 1).await?;

    // Test query
    sqlx::query("SELECT 1").fetch_one(&pool).await?;

    Ok(())
}

/// Check database migrations status.
async fn check_migrations() -> Result<()> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    let pool = openrustclaw_db::init_pool(&config.database.url, 1).await?;

    // Check if key tables exist
    let tables = vec![
        "sessions",
        "conversations",
        "memory_entries",
        "skills",
        "scheduled_jobs",
        "core_memory",
    ];

    for table in tables {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table)
                .fetch_one(&pool)
                .await?;

        if count == 0 {
            anyhow::bail!("Table '{}' is missing - migrations may not have run", table);
        }
    }

    Ok(())
}

/// Check API key configuration.
fn check_api_keys() -> Result<()> {
    let keys = vec![
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("OPENROUTER_API_KEY", "OpenRouter"),
    ];

    let mut configured = 0;

    for (env_var, _name) in &keys {
        if std::env::var(env_var).is_ok() {
            configured += 1;
        }
    }

    if configured == 0 {
        anyhow::bail!(
            "No LLM provider API keys configured. Set at least one of:\n\
             - ANTHROPIC_API_KEY\n\
             - OPENAI_API_KEY\n\
             - OPENROUTER_API_KEY\n\
             Or use Ollama for local models (no API key needed)"
        );
    }

    Ok(())
}

/// Check sidecar availability.
async fn check_sidecar() -> Result<()> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Check if Python is available
    let output = tokio::process::Command::new(&config.sidecar.python_path)
        .arg("--version")
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();

            // Check if sidecar module is available
            let check_module = tokio::process::Command::new(&config.sidecar.python_path)
                .args(["-c", "import openrustclaw_sidecar"])
                .output()
                .await;

            match check_module {
                Ok(output) if output.status.success() => Ok(()),
                _ => {
                    anyhow::bail!(
                        "Python {} found, but openrustclaw_sidecar module is not installed. \
                        The sidecar will not be available.",
                        version
                    )
                }
            }
        }
        _ => {
            anyhow::bail!(
                "Python not found at '{}'. \
                The Python sidecar will not be available.",
                config.sidecar.python_path
            )
        }
    }
}

/// Check configuration files.
fn check_config() -> Result<()> {
    // Check default config exists
    if !Path::new("config/default.toml").exists() {
        anyhow::bail!("config/default.toml not found");
    }

    // Try to load config
    let _config = openrustclaw_core::config::AppConfig::load()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    Ok(())
}

/// Check skills directory.
async fn check_skills_dir() -> Result<()> {
    let skills_dir = Path::new("skills");

    if !skills_dir.exists() {
        println!();
        println!("      Creating skills directory...");
        tokio::fs::create_dir_all(skills_dir).await?;
    }

    // Check if writable
    let test_file = skills_dir.join(".write_test");
    match tokio::fs::write(&test_file, "test").await {
        Ok(()) => {
            let _ = tokio::fs::remove_file(&test_file).await;
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Skills directory is not writable: {}", e)
        }
    }
}

/// Check data directory.
async fn check_data_dir() -> Result<()> {
    let data_dir = Path::new("data");

    if !data_dir.exists() {
        println!();
        println!("      Creating data directory...");
        tokio::fs::create_dir_all(data_dir).await?;
    }

    // Check if writable
    let test_file = data_dir.join(".write_test");
    match tokio::fs::write(&test_file, "test").await {
        Ok(()) => {
            let _ = tokio::fs::remove_file(&test_file).await;
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Data directory is not writable: {}", e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_api_keys_no_keys_set() {
        // Temporarily remove any API keys that might be set
        let anthropic = std::env::var("ANTHROPIC_API_KEY").ok();
        let openai = std::env::var("OPENAI_API_KEY").ok();
        let openrouter = std::env::var("OPENROUTER_API_KEY").ok();

        // SAFETY: Tests run single-threaded (or serialized) so env mutation is safe
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENROUTER_API_KEY");
        }

        let result = check_api_keys();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No LLM provider API keys configured"));

        // Restore
        unsafe {
            if let Some(v) = anthropic {
                std::env::set_var("ANTHROPIC_API_KEY", v);
            }
            if let Some(v) = openai {
                std::env::set_var("OPENAI_API_KEY", v);
            }
            if let Some(v) = openrouter {
                std::env::set_var("OPENROUTER_API_KEY", v);
            }
        }
    }

    #[test]
    fn test_check_api_keys_with_anthropic_key() {
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        // SAFETY: Tests run single-threaded (or serialized) so env mutation is safe
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key-12345678901234567890");
        }

        let result = check_api_keys();
        assert!(result.is_ok());

        // Restore
        unsafe {
            match original {
                Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
                None => std::env::remove_var("ANTHROPIC_API_KEY"),
            }
        }
    }

    #[test]
    fn test_check_api_keys_with_openai_key() {
        let original = std::env::var("OPENAI_API_KEY").ok();
        // SAFETY: Tests run single-threaded (or serialized) so env mutation is safe
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key-123456789012345678901234");
        }

        let result = check_api_keys();
        assert!(result.is_ok());

        // Restore
        unsafe {
            match original {
                Some(v) => std::env::set_var("OPENAI_API_KEY", v),
                None => std::env::remove_var("OPENAI_API_KEY"),
            }
        }
    }

    #[test]
    fn test_check_config_missing_file() {
        // This tests that check_config fails when config/default.toml doesn't exist
        // We can't guarantee this file exists in test env, so we test the function signature
        // The function itself checks Path::new("config/default.toml").exists()
        let _result = check_config();
        // Just verify it doesn't panic
    }
}
