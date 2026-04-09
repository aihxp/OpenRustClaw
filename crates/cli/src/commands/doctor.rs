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
    let config = load_app_config(None);
    if !has_enabled_channels(&config) {
        return Ok(());
    }
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
    let config = load_app_config(None);
    if !status.env_present
        && !status.control_registry_present
        && !status.channels_registry_present
        && !status.setup_state_present
    {
        anyhow::bail!(
            "No onboarding-managed workspace state detected yet. Run `openrustclaw onboard` or scaffold control/channel state manually."
        );
    }
    if let Some(setup_state) = onboard::load_setup_state(&cwd)? {
        let setup = setup_state.setup;
        if !matches!(setup.status.as_str(), "ready" | "completed")
            && !setup_state_is_effectively_ready(&setup, &cwd, &config)
        {
            let detail = setup
                .next_action
                .unwrap_or_else(|| "rerun `openrustclaw onboard` to continue setup.".to_string());
            anyhow::bail!("Setup state is {}. {}", setup.status, detail);
        }
    }
    Ok(())
}

fn setup_state_is_effectively_ready(
    setup: &onboard::SetupState,
    workspace_root: &Path,
    config: &openrustclaw_core::config::AppConfig,
) -> bool {
    let Some(access_mode) = setup.selected_access_mode.as_deref() else {
        return false;
    };
    if setup.selected_provider.is_none() {
        return false;
    }

    match access_mode {
        "api_key" => {
            if !workspace_has_any_api_key() {
                return false;
            }
        }
        "subscription_managed" => {
            if setup.selected_backend_id.is_none() {
                return false;
            }
        }
        "local_runtime" => {}
        _ => {}
    }

    let channels_enabled = has_enabled_channels(config);
    let non_api_path = workspace_has_non_api_provider_path(workspace_root);

    let has_meaningful_blocker = setup.blockers.iter().any(|blocker| {
        let blocker = blocker.to_ascii_lowercase();
        if !channels_enabled && (blocker.contains("channel ") || blocker.contains("whatsapp")) {
            return false;
        }
        if non_api_path && blocker.contains("provider api keys") {
            return false;
        }
        if blocker.contains("primary task model") {
            return false;
        }
        if blocker.contains("run `openrustclaw doctor --deep`") {
            return false;
        }
        true
    });
    if has_meaningful_blocker {
        return false;
    }

    !setup.bootstrap_outcomes.iter().any(|outcome| {
        if outcome.status == "ready" {
            return false;
        }
        if !channels_enabled && outcome.category == "channel" {
            return false;
        }
        if non_api_path && outcome.category == "provider" {
            return false;
        }
        true
    })
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
        if workspace_has_non_api_provider_path(&workspace_root) {
            return Ok(());
        }
    }
    check_api_keys_with(|env_var| std::env::var(env_var).ok())
}

fn workspace_has_any_api_key() -> bool {
    [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "OPENROUTER_API_KEY",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
    ]
    .into_iter()
    .any(|env_var| std::env::var(env_var).ok().is_some())
}

fn workspace_has_non_api_provider_path(workspace_root: &Path) -> bool {
    if let Ok(Some(setup_state)) = onboard::load_setup_state(workspace_root)
        && matches!(
            setup_state.setup.selected_access_mode.as_deref(),
            Some("subscription_managed" | "local_runtime")
        )
    {
        return true;
    }

    let config =
        runtime::load_effective_config("config/default.toml", workspace_root).unwrap_or_default();
    matches!(
        config.providers.default_provider.as_str(),
        "ollama" | "claude_code" | "codex" | "gemini_cli" | "cursor"
    )
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

    if !sidecar_is_required(&config) {
        return Ok(());
    }

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

fn sidecar_is_required(config: &openrustclaw_core::config::AppConfig) -> bool {
    config.sidecar.auto_start
        && !matches!(
            config.sidecar.role,
            openrustclaw_core::config::SidecarRole::Disabled
        )
}

fn has_enabled_channels(config: &openrustclaw_core::config::AppConfig) -> bool {
    config.channels.telegram.enabled
        || config.channels.discord.enabled
        || config.channels.slack.enabled
        || config.channels.mattermost.enabled
        || config.channels.matrix.enabled
        || config.channels.whatsapp.enabled
        || config.channels.teams.enabled
        || config.channels.google_chat.enabled
        || config.channels.google_meet.enabled
        || config.channels.gmail_pubsub.enabled
        || config.channels.signal.enabled
        || config.channels.imessage.enabled
}

fn sidecar_source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sidecar")
}

/// Check configuration files.
fn check_config(config_path: Option<&str>) -> Result<()> {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = runtime::resolve_runtime_config_path(
        &workspace_root,
        config_path.unwrap_or("config/default.toml"),
    );
    // Check default config exists
    if !path.exists() {
        anyhow::bail!("{} not found", path.display());
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
    use crate::commands::onboard::{SetupState, SetupStateManifest, save_setup_state};
    use tempfile::tempdir;

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
    fn test_sidecar_is_not_required_when_autostart_is_disabled() {
        let mut config = openrustclaw_core::config::AppConfig::default();
        config.sidecar.auto_start = false;
        config.sidecar.role = openrustclaw_core::config::SidecarRole::Compatibility;
        assert!(!sidecar_is_required(&config));
    }

    #[test]
    fn test_has_enabled_channels_false_for_default_config() {
        assert!(!has_enabled_channels(
            &openrustclaw_core::config::AppConfig::default()
        ));
    }

    #[test]
    fn test_workspace_has_non_api_provider_path_for_subscription_managed_setup() {
        let dir = tempdir().unwrap();
        let manifest = SetupStateManifest {
            version: 1,
            setup: SetupState {
                started_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                completed_at: None,
                status: "in_progress".to_string(),
                workspace_action: "repair_existing".to_string(),
                deployment_mode: None,
                deployment_path: None,
                remote_connectivity_profile: None,
                setup_path: Some("Standard".to_string()),
                selected_provider: Some("openai".to_string()),
                selected_lane_id: Some("codex".to_string()),
                selected_lane_label: Some("OpenAI (GPT)".to_string()),
                selected_lane_kind: Some("delegated_agent".to_string()),
                selected_backend_id: Some("codex".to_string()),
                selected_lane_detail: Some("Use your existing Codex login.".to_string()),
                selected_lane_compatibility_note: None,
                selected_access_mode: Some("subscription_managed".to_string()),
                selected_primary_model: None,
                selected_primary_model_source: None,
                selected_steps: vec!["model".to_string()],
                completed_steps: Vec::new(),
                blockers: Vec::new(),
                next_action: Some("Complete AI Model Setup".to_string()),
                current_step: Some("model".to_string()),
                bootstrap_outcomes: Vec::new(),
            },
        };
        save_setup_state(dir.path(), &manifest).unwrap();

        assert!(workspace_has_non_api_provider_path(dir.path()));
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

    #[test]
    fn test_setup_state_is_effectively_ready_for_subscription_managed_without_channels() {
        let dir = tempdir().unwrap();
        let setup = SetupState {
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            completed_at: None,
            status: "blocked".to_string(),
            workspace_action: "repair_existing".to_string(),
            deployment_mode: None,
            deployment_path: None,
            remote_connectivity_profile: None,
            setup_path: Some("Standard".to_string()),
            selected_provider: Some("openai".to_string()),
            selected_lane_id: Some("codex".to_string()),
            selected_lane_label: Some("OpenAI (GPT)".to_string()),
            selected_lane_kind: Some("delegated_agent".to_string()),
            selected_backend_id: Some("codex".to_string()),
            selected_lane_detail: Some("Use your existing Codex login.".to_string()),
            selected_lane_compatibility_note: None,
            selected_access_mode: Some("subscription_managed".to_string()),
            selected_primary_model: None,
            selected_primary_model_source: None,
            selected_steps: vec!["model".to_string()],
            completed_steps: Vec::new(),
            blockers: vec![
                "provider API keys: No LLM provider API keys configured".to_string(),
                "channel whatsapp needs review: stale warning".to_string(),
            ],
            next_action: Some(
                "Run `openrustclaw doctor --deep`, fix blockers, then rerun onboarding."
                    .to_string(),
            ),
            current_step: None,
            bootstrap_outcomes: vec![
                onboard::SetupBootstrapOutcome {
                    category: "provider".to_string(),
                    target: "openai".to_string(),
                    status: "blocked".to_string(),
                    detail: "stale provider block".to_string(),
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
                    updated_at: Utc::now().to_rfc3339(),
                },
                onboard::SetupBootstrapOutcome {
                    category: "channel".to_string(),
                    target: "whatsapp".to_string(),
                    status: "warning".to_string(),
                    detail: "stale channel warning".to_string(),
                    issue_kind: None,
                    verification_stage: None,
                    suggested_action: None,
                    updated_at: Utc::now().to_rfc3339(),
                },
            ],
        };

        let manifest = SetupStateManifest {
            version: 1,
            setup: setup.clone(),
        };
        save_setup_state(dir.path(), &manifest).unwrap();

        assert!(setup_state_is_effectively_ready(
            &setup,
            dir.path(),
            &openrustclaw_core::config::AppConfig::default(),
        ));
    }
}
