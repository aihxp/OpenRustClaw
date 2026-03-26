use openrustclaw_cli::commands::{assistant, inspect};
use openrustclaw_core::types::{Message, Platform, Session};
use openrustclaw_db::{SessionStatus, SqliteSessionStore, run_migrations};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn inspect_session_reports_resumed_assistant_continuity() -> TestResult {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    run_migrations(&pool).await?;
    let store = SqliteSessionStore::new(pool.clone());
    let workspace = tempdir()?;
    let route_key = "cli:assistant:alice:test";

    let mut session = Session::new_dm("alice", Platform::Cli);
    session.metadata = assistant::session_metadata(
        "cli",
        Some(route_key),
        Some(workspace.path()),
        serde_json::json!({"assistant_mode": "chat"}),
    );
    store
        .create_or_update(&session, Some(route_key), SessionStatus::Active)
        .await?;
    store
        .append_message(&session.id.to_string(), &Message::user("hello"))
        .await?;
    store
        .append_message(&session.id.to_string(), &Message::assistant("hi"))
        .await?;

    let report = inspect::inspect_session(&pool, &session.id.to_string(), 25).await?;
    let continuity = report.continuity.expect("continuity summary");

    assert!(continuity.assistant_managed);
    assert_eq!(continuity.assistant_surface.as_deref(), Some("cli"));
    assert_eq!(
        continuity.assistant_session_model.as_deref(),
        Some("persisted")
    );
    assert_eq!(continuity.route_key.as_deref(), Some(route_key));
    assert!(continuity.route_bound);
    assert!(continuity.likely_resumed);
    assert_eq!(continuity.history_messages, 2);
    assert_eq!(continuity.status_label, "resumed");
    assert!(continuity.detail.contains("matched by route key"));

    Ok(())
}

#[tokio::test]
async fn list_sessions_marks_non_assistant_sessions_as_generic() -> TestResult {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    run_migrations(&pool).await?;
    let store = SqliteSessionStore::new(pool.clone());

    let session = Session::new_dm("bob", Platform::Telegram);
    store
        .create_or_update(&session, None, SessionStatus::Active)
        .await?;
    store
        .append_message(&session.id.to_string(), &Message::user("plain session"))
        .await?;

    let entries = inspect::list_sessions(&pool, None, 10).await?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.session.session.id == session.id)
        .expect("listed session");

    assert!(!entry.continuity.assistant_managed);
    assert_eq!(entry.continuity.status_label, "generic");
    assert_eq!(entry.continuity.history_messages, 1);
    assert!(!entry.continuity.route_bound);
    assert!(entry.continuity.detail.contains("Generic session"));

    Ok(())
}
