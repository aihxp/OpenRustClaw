use openrustclaw_cli::commands::browser::{BrowserWorkflowRecord, list_workflow_history};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn append_record(
    workspace_root: &std::path::Path,
    record: &BrowserWorkflowRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = workspace_root
        .join(".claw")
        .join("browser")
        .join("workflow-history.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write;
    serde_json::to_writer(&mut file, record)?;
    file.write_all(b"\n")?;
    Ok(())
}

#[test]
fn browser_workflow_history_persists_and_filters_records() -> TestResult {
    let workspace = tempfile::tempdir()?;

    append_record(
        workspace.path(),
        &BrowserWorkflowRecord {
            id: "browser-1".to_string(),
            action: "read_page".to_string(),
            backend: "http_fetch".to_string(),
            session_id: None,
            status: "success".to_string(),
            final_url: "https://example.com/docs".to_string(),
            title: "Docs".to_string(),
            step_count: 1,
            artifact_path: ".claw/browser/read/docs.json".to_string(),
            created_at: "2026-03-26T20:00:00Z".to_string(),
            result_preview: Some(serde_json::json!({
                "status": 200,
                "link_count": 5
            })),
        },
    )?;
    append_record(
        workspace.path(),
        &BrowserWorkflowRecord {
            id: "browser-2".to_string(),
            action: "run_sequence".to_string(),
            backend: "native_cdp".to_string(),
            session_id: Some("session-77".to_string()),
            status: "success".to_string(),
            final_url: "https://example.com/app".to_string(),
            title: "Example App".to_string(),
            step_count: 4,
            artifact_path: ".claw/browser/sequences/app.json".to_string(),
            created_at: "2026-03-26T20:05:00Z".to_string(),
            result_preview: Some(serde_json::json!({
                "step_count": 4,
                "last_action": "screenshot"
            })),
        },
    )?;

    let report = list_workflow_history(workspace.path(), 10, Some("run_sequence"), None)?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].action, "run_sequence");
    assert_eq!(report.entries[0].backend, "native_cdp");
    assert_eq!(report.entries[0].session_id.as_deref(), Some("session-77"));
    assert_eq!(report.entries[0].step_count, 4);
    assert_eq!(
        report.entries[0]
            .result_preview
            .as_ref()
            .and_then(|value| value.get("last_action")),
        Some(&serde_json::json!("screenshot"))
    );

    Ok(())
}

#[test]
fn browser_workflow_history_filters_by_backend() -> TestResult {
    let workspace = tempfile::tempdir()?;

    append_record(
        workspace.path(),
        &BrowserWorkflowRecord {
            id: "browser-3".to_string(),
            action: "inspect".to_string(),
            backend: "agent_browser_cli".to_string(),
            session_id: Some("session-agent".to_string()),
            status: "success".to_string(),
            final_url: "https://example.com/ops".to_string(),
            title: "Ops".to_string(),
            step_count: 1,
            artifact_path: ".claw/browser/inspect/ops.json".to_string(),
            created_at: "2026-03-26T20:08:00Z".to_string(),
            result_preview: Some(serde_json::json!({"kind": "snapshot"})),
        },
    )?;
    append_record(
        workspace.path(),
        &BrowserWorkflowRecord {
            id: "browser-4".to_string(),
            action: "pdf".to_string(),
            backend: "native_cdp".to_string(),
            session_id: None,
            status: "success".to_string(),
            final_url: "https://example.com/report".to_string(),
            title: "Report".to_string(),
            step_count: 1,
            artifact_path: ".claw/browser/pdf/report.pdf".to_string(),
            created_at: "2026-03-26T20:09:00Z".to_string(),
            result_preview: Some(serde_json::json!({"bytes": 2048})),
        },
    )?;

    let report = list_workflow_history(workspace.path(), 10, None, Some("agent_browser_cli"))?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].backend, "agent_browser_cli");
    assert_eq!(report.entries[0].action, "inspect");

    Ok(())
}
