use openrustclaw_cli::commands::inspect::{
    append_tool_execution_record, new_tool_execution_record, tool_execution_history,
};
use openrustclaw_cursor::tools::load_execution_artifacts;
use openrustclaw_cursor::{CursorConfig, ToolContext, ToolRegistry};
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn coding_artifacts_and_tool_ledger_share_auditable_evidence() -> TestResult {
    let workspace = tempfile::tempdir()?;
    let repo = workspace.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["config", "user.name", "OpenRustClaw Tests"])
        .current_dir(repo)
        .status()?;

    tokio::fs::write(repo.join("hello.txt"), "Hello, World!\n").await?;
    Command::new("git")
        .args(["add", "hello.txt"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["commit", "-m", "baseline"])
        .current_dir(repo)
        .status()?;

    let registry = ToolRegistry::new(ToolContext::new(
        repo.to_path_buf(),
        CursorConfig::default(),
    ));
    let result = registry
        .execute(
            "edit_file",
            serde_json::json!({
                "path": "hello.txt",
                "old_text": "World",
                "new_text": "Rust",
            }),
        )
        .await?;

    let artifacts = load_execution_artifacts(repo, 5)?;
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(artifact.tool_name, "edit_file");
    assert!(artifact.success);
    assert!(std::path::Path::new(&artifact.artifact_path).exists());
    assert_eq!(
        artifact.target_path.as_deref(),
        Some(repo.join("hello.txt").to_string_lossy().as_ref())
    );
    assert!(
        artifact
            .diff_preview
            .as_deref()
            .is_some_and(|preview| preview.contains("+Hello, Rust!")),
        "expected diff preview to contain edited line",
    );

    let ledger_record = new_tool_execution_record(
        "edit_file",
        "cursor_tool",
        "success",
        "artifact_captured",
        12,
        None,
        Some(artifact.artifact_path.clone()),
        Some(serde_json::json!({ "path": "hello.txt" })),
        Some(result),
    );
    append_tool_execution_record(repo, &ledger_record)?;

    let report = tool_execution_history(repo, 5, Some("cursor_tool"), Some("success"), None)?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].tool_name, "edit_file");
    assert_eq!(
        report.entries[0].artifact_path.as_deref(),
        Some(artifact.artifact_path.as_str())
    );

    Ok(())
}
