//! Onboarding and first-run readiness integration tests.

use chrono::Utc;
use openrustclaw_cli::commands::doctor::{DiagnosticCheck, DiagnosticReport, DiagnosticStatus};
use openrustclaw_cli::commands::{channels, control, doctor, onboard};

use crate::common::init_test_tracing;

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
fn workspace_status_detects_existing_onboarding_state() {
    init_test_tracing();

    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "OPENAI_API_KEY=test\n").expect("write env");
    std::fs::create_dir_all(control::control_root_for(temp.path())).expect("create control dir");
    std::fs::create_dir_all(channels::channels_root_for(temp.path())).expect("create channels dir");

    let status = onboard::workspace_status(temp.path());
    assert!(status.env_present);
    assert!(status.control_registry_present);
    assert!(status.channels_registry_present);
}

#[test]
fn first_start_readiness_and_launch_gate_allow_ready_workspace() {
    init_test_tracing();

    let readiness = doctor::first_start_readiness(&sample_report(vec![
        DiagnosticCheck {
            id: "api_keys".to_string(),
            label: "provider API keys".to_string(),
            status: DiagnosticStatus::Ok,
            message: None,
        },
        DiagnosticCheck {
            id: "onboarding_state".to_string(),
            label: "onboarding-managed workspace state".to_string(),
            status: DiagnosticStatus::Ok,
            message: None,
        },
        DiagnosticCheck {
            id: "channel_readiness".to_string(),
            label: "enabled channel readiness probes".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No shipped channels are enabled in the effective config.".to_string()),
        },
    ]));

    assert!(readiness.ready);
    assert!(onboard::should_offer_assistant_launch(
        readiness.ready,
        true,
        Some("openrouter")
    ));
}

#[test]
fn first_start_readiness_and_launch_gate_block_broken_workspace() {
    init_test_tracing();

    let readiness = doctor::first_start_readiness(&sample_report(vec![
        DiagnosticCheck {
            id: "api_keys".to_string(),
            label: "provider API keys".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("No LLM provider API keys configured".to_string()),
        },
        DiagnosticCheck {
            id: "channel_readiness".to_string(),
            label: "enabled channel readiness probes".to_string(),
            status: DiagnosticStatus::Warning,
            message: Some("Channel readiness failures: slack: missing token".to_string()),
        },
    ]));

    assert!(!readiness.ready);
    assert_eq!(readiness.blocking_items.len(), 2);
    assert!(!onboard::should_offer_assistant_launch(
        readiness.ready,
        true,
        Some("openrouter")
    ));
}
