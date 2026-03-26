use openrustclaw_cli::commands::security::posture_summary;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn security_posture_summary_surfaces_release_critical_fields() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let workspace_root = workspace.path();

    std::fs::create_dir_all(workspace_root.join("config"))?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/default.toml"),
        workspace_root.join("config/default.toml"),
    )?;
    std::fs::create_dir_all(workspace_root.join(".claw/control"))?;
    std::fs::write(
        workspace_root.join(".claw/control/runtime-vault.json"),
        "{}",
    )?;

    let summary = posture_summary(
        workspace_root.join("config/default.toml").to_str().unwrap(),
        workspace_root,
    )?;

    assert!(summary.require_auth);
    assert!(summary.origin_validation);
    assert!(summary.vault_present);
    assert_eq!(summary.issues_found, 0);
    assert!(summary.warnings_found >= 1);
    assert!(
        summary
            .checks
            .iter()
            .any(|entry| entry.area == "gateway_auth" && entry.status == "pass")
    );
    assert!(
        summary
            .checks
            .iter()
            .any(|entry| entry.area == "origin_validation" && entry.status == "pass")
    );
    assert!(
        summary
            .recommended_actions
            .iter()
            .any(|entry| entry.contains("security audit"))
    );

    Ok(())
}
