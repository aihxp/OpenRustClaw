use chrono::Utc;
use openrustclaw_cli::commands::runtime::{
    RuntimeArtifactHealth, RuntimeHealthProviderEntry, RuntimeHealthReport,
    runtime_health_path_for, runtime_lock_path_for, runtime_operator_ops_summary,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn runtime_operator_ops_summary_covers_runtime_health_and_recovery_state() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let workspace_root = workspace.path();
    let now = Utc::now();

    std::fs::create_dir_all(workspace_root.join("config"))?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/default.toml"),
        workspace_root.join("config/default.toml"),
    )?;
    std::fs::create_dir_all(workspace_root.join(".claw/control"))?;

    std::fs::write(
        runtime_health_path_for(workspace_root),
        serde_json::to_vec_pretty(&RuntimeHealthReport {
            generated_at: now.to_rfc3339(),
            config_path: "config/default.toml".to_string(),
            default_provider: "anthropic".to_string(),
            fallback_chain: vec!["openai".to_string()],
            control_plane_provider: Some("anthropic".to_string()),
            control_plane_fallback_chain: vec!["openai".to_string()],
            recommended_control_plane_provider: Some("anthropic".to_string()),
            startup_fallback_valid: true,
            degraded_control_plane_mode: false,
            failover_recommendations: Vec::new(),
            operator_warnings: vec!["rotation window pending".to_string()],
            providers: vec![RuntimeHealthProviderEntry {
                provider: "anthropic".to_string(),
                role: "primary".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                configured: true,
                healthy: true,
                issue_kind: None,
                recommendation: Some("configured task/runtime primary".to_string()),
                issue: None,
                model_available: Some(true),
                limit_snapshot: None,
            }],
            artifacts: RuntimeArtifactHealth {
                artifact_count: 0,
                persona_artifact_count: 0,
                registry_path: workspace_root
                    .join(".claw/artifacts/registry.json")
                    .display()
                    .to_string(),
                model_family: "anthropic".to_string(),
                included_for_default_model: 0,
            },
        })?,
    )?;

    std::fs::write(
        runtime_lock_path_for(workspace_root),
        serde_json::to_vec_pretty(&serde_json::json!({
            "acquired_at": now.to_rfc3339(),
            "process_id": 999_999u32,
            "gateway_addr": "127.0.0.1:18789",
            "config_path": "config/default.toml"
        }))?,
    )?;

    let summary = runtime_operator_ops_summary(
        workspace_root.join("config/default.toml").to_str().unwrap(),
        workspace_root,
        "127.0.0.1:18789",
        None,
        false,
    )
    .await?;

    assert_eq!(summary.runtime_health.default_provider, "anthropic");
    assert_eq!(summary.beacon.gateway_addr, "127.0.0.1:18789");
    assert!(summary.lock_status.present);
    assert!(summary.lock_status.stale);
    assert!(
        summary
            .recommended_actions
            .iter()
            .any(|entry| entry.contains("openrustclaw runtime backup"))
    );
    assert!(
        summary
            .recommended_actions
            .iter()
            .any(|entry| entry.contains("openrustclaw runtime rollback-plan"))
    );
    assert!(
        summary
            .recommended_actions
            .iter()
            .any(|entry| entry.contains("rotation window pending"))
    );

    Ok(())
}
