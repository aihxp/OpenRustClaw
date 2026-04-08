use std::sync::Arc;

use openrustclaw_agent::MemoryStoreTool;
use openrustclaw_cli::commands::inspect;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_db::SqliteMemoryStore;
use serde_json::json;

use crate::common::create_test_db;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn memory_store_allows_explicit_request_and_persists_policy_metadata() -> TestResult {
    let pool = create_test_db().await;
    let store = Arc::new(SqliteMemoryStore::new(pool));
    let tool = MemoryStoreTool::new(store.clone());
    let ctx = ToolContext {
        session_id: uuid::Uuid::new_v4().to_string(),
        user_id: "user_123".to_string(),
        workspace_path: None,
    };

    let output = tool
        .execute(
            json!({
                "content": "User asked me to remember that their preferred language is Rust.",
                "basis": "explicit_user_request",
                "reason": "The user explicitly asked me to remember it.",
                "memory_type": "semantic"
            }),
            &ctx,
        )
        .await?;

    assert!(!output.is_error);
    assert!(output.content.contains("Memory stored successfully"));

    let entries = store.list_recent(Some("user_123"), 10).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].metadata["assistant_write_policy"]["basis"],
        "explicit_user_request"
    );
    assert_eq!(
        entries[0].metadata["assistant_write_policy"]["declared_reason"],
        "The user explicitly asked me to remember it."
    );

    Ok(())
}

#[tokio::test]
async fn memory_timeline_exposes_assistant_write_policy_metadata() -> TestResult {
    let pool = create_test_db().await;
    let store = Arc::new(SqliteMemoryStore::new(pool.clone()));
    let tool = MemoryStoreTool::new(store.clone());
    let ctx = ToolContext {
        session_id: uuid::Uuid::new_v4().to_string(),
        user_id: "user_789".to_string(),
        workspace_path: None,
    };

    tool.execute(
        json!({
            "content": "User prefers concise Rust answers for this repository.",
            "basis": "durable_user_fact",
            "reason": "Stable user preference for this project workspace.",
            "memory_type": "semantic"
        }),
        &ctx,
    )
    .await?;

    let report = inspect::memory_timeline(&store, &pool, Some("user_789"), 5).await?;
    assert_eq!(report.entries.len(), 1);
    assert_eq!(
        report.entries[0].metadata["assistant_write_policy"]["basis"],
        "durable_user_fact"
    );
    assert_eq!(
        report.entries[0].metadata["assistant_write_policy"]["declared_reason"],
        "Stable user preference for this project workspace."
    );

    Ok(())
}

#[tokio::test]
async fn memory_store_blocks_agent_inference_and_persists_nothing() -> TestResult {
    let pool = create_test_db().await;
    let store = Arc::new(SqliteMemoryStore::new(pool));
    let tool = MemoryStoreTool::new(store.clone());
    let ctx = ToolContext {
        session_id: uuid::Uuid::new_v4().to_string(),
        user_id: "user_456".to_string(),
        workspace_path: None,
    };

    let output = tool
        .execute(
            json!({
                "content": "User might be impatient with long answers.",
                "basis": "agent_inference",
                "reason": "Inferred from tone."
            }),
            &ctx,
        )
        .await?;

    assert!(!output.is_error);
    assert!(output.content.contains("Memory not stored"));

    let entries = store.list_recent(Some("user_456"), 10).await?;
    assert!(entries.is_empty());

    Ok(())
}
