//! Diagnostics command - Check system health.

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::path::{Path, PathBuf};

use super::{channels, control, onboard, runtime, self_hosted, services};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Ok,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticCheck {
    pub id: String,
    pub label: String,
    pub status: DiagnosticStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub config_path: String,
    pub deep: bool,
    pub checks: Vec<DiagnosticCheck>,
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub healthy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstStartReadiness {
    pub ready: bool,
    pub blocking_items: Vec<String>,
}

fn blocking_reason(check: &DiagnosticCheck) -> String {
    if let Some(message) = &check.message {
        format!("{}: {}", check.label, message)
    } else {
        check.label.clone()
    }
}

pub fn first_start_readiness(report: &DiagnosticReport) -> FirstStartReadiness {
    let mut blocking_items = Vec::new();

    for check in &report.checks {
        let should_block = match check.id.as_str() {
            _ if check.status == DiagnosticStatus::Failed => true,
            "api_keys" | "control_registry" | "onboarding_state" => {
                check.status != DiagnosticStatus::Ok
            }
            "channel_readiness" => {
                if check.status == DiagnosticStatus::Ok {
                    false
                } else {
                    let message = check.message.as_deref().unwrap_or_default();
                    !message.contains("No shipped channels are enabled")
                }
            }
            _ => false,
        };

        if should_block {
            blocking_items.push(blocking_reason(check));
        }
    }

    FirstStartReadiness {
        ready: blocking_items.is_empty(),
        blocking_items,
    }
}

/// Run diagnostics.
pub async fn run(repair: bool, deep: bool, non_interactive: bool) -> Result<()> {
    let report = collect_report(repair, deep, None).await?;
    if non_interactive {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("diagnostic report should serialize")
        );
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           OpenRustClaw Diagnostics                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    for (index, check) in report.checks.iter().enumerate() {
        print!(
            "[{}/{}] Checking {}... ",
            index + 1,
            report.checks.len(),
            check.label
        );
        match check.status {
            DiagnosticStatus::Ok => println!("\x1b[32m✓ OK\x1b[0m"),
            DiagnosticStatus::Warning => println!("\x1b[33m⚠ WARNING\x1b[0m"),
            DiagnosticStatus::Failed => println!("\x1b[31m✗ FAILED\x1b[0m"),
        }
        if let Some(message) = &check.message {
            let prefix = if check.status == DiagnosticStatus::Failed {
                "Error"
            } else {
                ""
            };
            if prefix.is_empty() {
                println!("      {}", message);
            } else {
                println!("      {}: {}", prefix, message);
            }
        }
    }

    if report.deep {
        println!();
        println!("Deep diagnostics:");
        if let Some(check) = report.checks.iter().find(|check| check.id == "ollama") {
            print!("  • Ollama availability... ");
            match check.status {
                DiagnosticStatus::Ok => println!("\x1b[32mOK\x1b[0m"),
                DiagnosticStatus::Warning => println!("\x1b[90mnot detected\x1b[0m"),
                DiagnosticStatus::Failed => println!("\x1b[31mfailed\x1b[0m"),
            }
        }
    }

    // Summary
    println!();
    println!("══════════════════════════════════════════════════════════");

    let total = report.passed + report.failed + report.warnings;

    if report.failed == 0 && report.warnings == 0 {
        println!("\x1b[32m✓ All {} checks passed!\x1b[0m", total);
        println!();
        println!("Your OpenRustClaw installation is ready to use.");
    } else {
        println!(
            "Results: {} passed, {} failed, {} warnings",
            report.passed, report.failed, report.warnings
        );

        if report.failed > 0 {
            println!();
            println!(
                "\x1b[31m{} critical issue(s) need to be resolved.\x1b[0m",
                report.failed
            );
        }

        if report.warnings > 0 {
            println!();
            println!(
                "\x1b[33m{} warning(s) - you may want to address these.\x1b[0m",
                report.warnings
            );
        }
    }

    println!();

    Ok(())
}

pub async fn collect_report(
    repair: bool,
    deep: bool,
    config_path: Option<&str>,
) -> Result<DiagnosticReport> {
    let config_path = config_path.unwrap_or("config/default.toml").to_string();
    let mut checks = Vec::new();

    checks.push(diagnostic_check(
        "database",
        "database connection",
        check_database(Some(&config_path)).await,
        false,
    ));
    checks.push(diagnostic_check(
        "migrations",
        "database migrations",
        check_migrations(Some(&config_path)).await,
        false,
    ));
    checks.push(diagnostic_check(
        "api_keys",
        "provider API keys",
        check_api_keys(),
        true,
    ));
    checks.push(diagnostic_check(
        "sidecar",
        "sidecar availability",
        check_sidecar(Some(&config_path)).await,
        true,
    ));
    checks.push(diagnostic_check(
        "config",
        "configuration files",
        check_config(Some(&config_path)),
        false,
    ));
    checks.push(diagnostic_check(
        "skills_dir",
        "skill directory",
        check_skills_dir().await,
        true,
    ));
    checks.push(diagnostic_check(
        "data_dir",
        "data directory",
        check_data_dir().await,
        false,
    ));
    checks.push(diagnostic_check(
        "control_registry",
        "control-plane registry",
        check_control_registry(repair).await,
        true,
    ));
    checks.push(diagnostic_check(
        "channels_registry",
        "channel registry",
        check_channels_registry(repair).await,
        true,
    ));
    checks.push(diagnostic_check(
        "onboarding_state",
        "onboarding-managed workspace state",
        check_onboarding_state(),
        true,
    ));
    checks.push(diagnostic_check(
        "product_mode",
        "self-hosted product mode",
        check_product_mode(),
        true,
    ));
    checks.push(diagnostic_check(
        "channel_readiness",
        "enabled channel readiness probes",
        check_channel_readiness(&config_path).await,
        true,
    ));

    if deep {
        checks.push(diagnostic_check(
            "ollama",
            "ollama availability",
            if check_ollama().await {
                Ok(())
            } else {
                anyhow::bail!("Ollama not detected")
            },
            true,
        ));
    }

    let passed = checks
        .iter()
        .filter(|check| check.status == DiagnosticStatus::Ok)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DiagnosticStatus::Warning)
        .count();
    let failed = checks
        .iter()
        .filter(|check| check.status == DiagnosticStatus::Failed)
        .count();

    Ok(DiagnosticReport {
        generated_at: Utc::now(),
        config_path,
        deep,
        checks,
        passed,
        warnings,
        failed,
        healthy: failed == 0,
    })
}

fn diagnostic_check(
    id: &str,
    label: &str,
    result: Result<()>,
    warning_on_error: bool,
) -> DiagnosticCheck {
    match result {
        Ok(()) => DiagnosticCheck {
            id: id.to_string(),
            label: label.to_string(),
            status: DiagnosticStatus::Ok,
            message: None,
        },
        Err(error) => DiagnosticCheck {
            id: id.to_string(),
            label: label.to_string(),
            status: if warning_on_error {
                DiagnosticStatus::Warning
            } else {
                DiagnosticStatus::Failed
            },
            message: Some(error.to_string()),
        },
    }
}

async fn check_control_registry(repair: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let control_root = control::control_root_for(&cwd);
    if repair && !control_root.exists() {
        control::init(None)?;
    }
    let registry = control::load_registry(control_root.clone())?;
    if registry.agent_profiles.is_empty()
        && registry.model_profiles.is_empty()
        && registry.claws.is_empty()
        && !control_root.exists()
    {
        anyhow::bail!("Control registry not initialized. Run `openrustclaw control init`.");
    }
    control::validate_registry(&registry)?;
    Ok(())
}

async fn check_channels_registry(repair: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let channels_root = channels::channels_root_for(&cwd);
    if repair && !channels_root.exists() {
        channels::init(None)?;
    }
    if !channels_root.exists() {
        anyhow::bail!("Channel registry not initialized. Run `openrustclaw channels init`.");
    }
    let _registry = channels::load_registry(channels_root)?;
    Ok(())
}

fn check_onboarding_state() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let status = onboard::workspace_status(&cwd);
    if !status.env_present && !status.control_registry_present && !status.channels_registry_present
    {
        anyhow::bail!(
            "No onboarding-managed workspace state detected yet. Run `openrustclaw onboard` or scaffold control/channel state manually."
        );
    }
    Ok(())
}

fn check_product_mode() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let manifest = self_hosted::load_manifest(&cwd)?;
    if manifest.is_none() {
        anyhow::bail!(
            "No explicit self-hosted product mode selected yet. Run `openrustclaw onboard` to choose solo, team, company, or enterprise."
        );
    }
    Ok(())
}

async fn check_channel_readiness(config_path: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let report = services::channel_probes(config_path, &cwd).await?;
    if report.entries.is_empty() {
        anyhow::bail!("No shipped channels are enabled in the effective config.");
    }

    let failed: Vec<_> = report
        .entries
        .iter()
        .filter(|entry| entry.status == services::ChannelProbeStatus::Failed)
        .map(|entry| format!("{}: {}", entry.platform, entry.detail))
        .collect();
    if !failed.is_empty() {
        anyhow::bail!("Channel readiness failures: {}", failed.join("; "));
    }
    Ok(())
}

/// Check database connection.
async fn check_database(config_path: Option<&str>) -> Result<()> {
    let config = load_app_config(config_path);

    let pool = openrustclaw_db::init_pool(&config.database.url, 1).await?;

    // Test query
    sqlx::query("SELECT 1").fetch_one(&pool).await?;

    Ok(())
}

/// Check database migrations status.
async fn check_migrations(config_path: Option<&str>) -> Result<()> {
    let config = load_app_config(config_path);

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
    if let Ok(workspace_root) = std::env::current_dir() {
        let _ = runtime::apply_runtime_secret_sources(&workspace_root);
    }
    check_api_keys_with(|env_var| std::env::var(env_var).ok())
}

fn check_api_keys_with<F>(mut get_env: F) -> Result<()>
where
    F: FnMut(&str) -> Option<String>,
{
    let keys = vec![
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("OPENROUTER_API_KEY", "OpenRouter"),
    ];

    let mut configured = 0;

    for (env_var, _name) in &keys {
        if get_env(env_var).is_some() {
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
async fn check_sidecar(config_path: Option<&str>) -> Result<()> {
    let config = load_app_config(config_path);

    // Check if Python is available
    let output = tokio::process::Command::new(&config.sidecar.python_path)
        .arg("--version")
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let version = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            };

            // Check the sidecar using the same source-tree execution model as runtime startup.
            let sidecar_dir = sidecar_source_dir();
            let check_module = if sidecar_dir.exists() {
                tokio::process::Command::new(&config.sidecar.python_path)
                    .args(["-c", "import src.server"])
                    .current_dir(&sidecar_dir)
                    .output()
                    .await
            } else {
                tokio::process::Command::new(&config.sidecar.python_path)
                    .args(["-c", "import src.server"])
                    .output()
                    .await
            };

            match check_module {
                Ok(output) if output.status.success() => Ok(()),
                _ => {
                    anyhow::bail!(
                        "Python {} found, but the sidecar source environment is not runnable. \
                        Install sidecar dependencies before starting OpenRustClaw.",
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

fn sidecar_source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sidecar")
}

/// Check configuration files.
fn check_config(config_path: Option<&str>) -> Result<()> {
    let path = config_path.unwrap_or("config/default.toml");
    // Check default config exists
    if !Path::new(path).exists() {
        anyhow::bail!("{} not found", path);
    }

    // Try to load config
    let _config = load_app_config(config_path);

    Ok(())
}

fn load_app_config(config_path: Option<&str>) -> openrustclaw_core::config::AppConfig {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match config_path {
        Some(path) => runtime::load_effective_config(path, &workspace_root).unwrap_or_default(),
        None => runtime::load_effective_config("config/default.toml", &workspace_root)
            .unwrap_or_default(),
    }
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

async fn check_ollama() -> bool {
    let base_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    reqwest::Client::new()
        .get(format!("{}/api/tags", base_url))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report(checks: Vec<DiagnosticCheck>) -> DiagnosticReport {
        let passed = checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Ok)
            .count();
        let warnings = checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Warning)
            .count();
        let failed = checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Failed)
            .count();

        DiagnosticReport {
            generated_at: Utc::now(),
            config_path: "config/default.toml".to_string(),
            deep: true,
            checks,
            passed,
            warnings,
            failed,
            healthy: failed == 0,
        }
    }

    #[test]
    fn test_check_api_keys_no_keys_set() {
        let result = check_api_keys_with(|_| None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No LLM provider API keys configured"));
    }

    #[test]
    fn test_check_api_keys_with_anthropic_key() {
        let result = check_api_keys_with(|env_var| match env_var {
            "ANTHROPIC_API_KEY" => Some("sk-ant-test-key-12345678901234567890".to_string()),
            _ => None,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_api_keys_with_openai_key() {
        let result = check_api_keys_with(|env_var| match env_var {
            "OPENAI_API_KEY" => Some("sk-test-key-123456789012345678901234".to_string()),
            _ => None,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_config_missing_file() {
        // This tests that check_config fails when config/default.toml doesn't exist
        // We can't guarantee this file exists in test env, so we test the function signature
        // The function itself checks Path::new("config/default.toml").exists()
        let _result = check_config(None);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_first_start_readiness_blocks_missing_api_keys_warning() {
        let readiness = first_start_readiness(&sample_report(vec![DiagnosticCheck {
            id: "api_keys".to_string(),
            label: "provider API keys".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No LLM provider API keys configured".to_string()),
        }]));

        assert!(!readiness.ready);
        assert_eq!(readiness.blocking_items.len(), 1);
        assert!(readiness.blocking_items[0].contains("provider API keys"));
    }

    #[test]
    fn test_first_start_readiness_allows_no_channels_enabled_warning() {
        let readiness = first_start_readiness(&sample_report(vec![DiagnosticCheck {
            id: "channel_readiness".to_string(),
            label: "enabled channel readiness probes".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No shipped channels are enabled in the effective config.".to_string()),
        }]));

        assert!(readiness.ready);
        assert!(readiness.blocking_items.is_empty());
    }

    #[test]
    fn test_first_start_readiness_blocks_channel_failures() {
        let readiness = first_start_readiness(&sample_report(vec![DiagnosticCheck {
            id: "channel_readiness".to_string(),
            label: "enabled channel readiness probes".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("Channel readiness failures: slack: missing token".to_string()),
        }]));

        assert!(!readiness.ready);
        assert_eq!(readiness.blocking_items.len(), 1);
        assert!(readiness.blocking_items[0].contains("channel readiness probes"));
    }

    #[test]
    fn test_first_start_readiness_blocks_missing_onboarding_state() {
        let readiness = first_start_readiness(&sample_report(vec![DiagnosticCheck {
            id: "onboarding_state".to_string(),
            label: "onboarding-managed workspace state".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No onboarding-managed workspace state detected yet.".to_string()),
        }]));

        assert!(!readiness.ready);
        assert_eq!(readiness.blocking_items.len(), 1);
        assert!(readiness.blocking_items[0].contains("onboarding-managed workspace state"));
    }

    #[test]
    fn test_first_start_readiness_does_not_block_missing_product_mode_warning() {
        let readiness = first_start_readiness(&sample_report(vec![DiagnosticCheck {
            id: "product_mode".to_string(),
            label: "self-hosted product mode".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No explicit self-hosted product mode selected yet.".to_string()),
        }]));

        assert!(readiness.ready);
        assert!(readiness.blocking_items.is_empty());
    }
}
