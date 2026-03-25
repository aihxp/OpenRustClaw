//! Fixture-driven integration tests for shipped operator workflows.

use std::fs;

use openrustclaw_cli::commands::media::{
    MediaExtractTextRequest, MediaInspectRequest, extract_text_with_config, inspect_data,
};
use openrustclaw_cli::commands::mobile::{
    DeviceCommandKind, MobileCommandDecisionRequest, MobileCommandDispatchRequest,
    MobilePairRequest, MobileUnpairRequest, approve_command_data, dispatch_command_data,
    list_command_data, list_pairing_data, pair_node_data, reject_command_data, unpair_node_data,
};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::types::{Message, Platform, Session};
use openrustclaw_db::{SessionStatus, SqliteSessionStore};
use serde_json::json;
use tempfile::tempdir;

use crate::common::{create_test_db_file, init_test_tracing};

#[tokio::test]
async fn media_text_fixture_workflow() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    let document_path = workspace.path().join("fixture-note.txt");
    fs::write(&document_path, "fixture media text\nsecond line\n").unwrap();

    let inspect = inspect_data(MediaInspectRequest {
        path: document_path.display().to_string(),
    })
    .await
    .unwrap();

    assert_eq!(inspect.media_kind, "document");
    assert!(inspect.text_extractable);
    assert_eq!(
        inspect.text_preview.as_deref(),
        Some("fixture media text\nsecond line\n")
    );

    let extracted = extract_text_with_config(
        &AppConfig::default(),
        workspace.path(),
        MediaExtractTextRequest {
            path: document_path.display().to_string(),
            provider: None,
            model: None,
            language: None,
            prompt: None,
            max_audio_bytes: None,
            timeout_secs: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(extracted.media_kind, "document");
    assert_eq!(extracted.extractor, "plain_text");
    assert_eq!(extracted.text, "fixture media text\nsecond line\n");
}

#[tokio::test]
async fn mobile_pairing_fixture_workflow() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    let manifest = pair_node_data(
        workspace.path(),
        MobilePairRequest {
            id: "fixture-node".to_string(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            auth_token_env: "PATH".to_string(),
            device_name: Some("Fixture Phone".to_string()),
            platform: Some("ios".to_string()),
            capabilities: vec!["mobile".to_string(), "notifications".to_string()],
            enabled: true,
            sync: None,
            notifications: None,
            metadata: json!({ "suite": "fixture" }),
        },
    )
    .unwrap();

    assert_eq!(manifest.node.id, "fixture-node");
    assert_eq!(manifest.node.device_name, "Fixture Phone");

    let removed = unpair_node_data(
        workspace.path(),
        "fixture-node",
        MobileUnpairRequest {
            requested_by: Some("fixture-suite".to_string()),
            reason: Some("fixture coverage".to_string()),
            remove_runtime_state: true,
        },
    )
    .unwrap();

    assert!(removed.removed_manifest);
    assert_eq!(removed.pairing.kind, "unpaired");

    let history = list_pairing_data(workspace.path(), Some("fixture-node"), Some(10)).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].kind, "unpaired");
    assert_eq!(history[1].kind, "paired");
}

#[tokio::test]
async fn mobile_command_approval_fixture_workflow() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    pair_node_data(
        workspace.path(),
        MobilePairRequest {
            id: "command-node".to_string(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            auth_token_env: "PATH".to_string(),
            device_name: Some("Command Fixture".to_string()),
            platform: Some("android".to_string()),
            capabilities: vec!["mobile".to_string(), "notifications".to_string()],
            enabled: true,
            sync: None,
            notifications: None,
            metadata: json!({}),
        },
    )
    .unwrap();

    let approved = dispatch_command_data(
        workspace.path(),
        MobileCommandDispatchRequest {
            node_id: "command-node".to_string(),
            command: DeviceCommandKind::PushNotification,
            payload: json!({
                "title": "Fixture push",
                "body": "Approved notification",
                "type": "system",
                "data": { "source": "fixture-suite" }
            }),
            approved_by: None,
            require_approval: Some(true),
        },
    )
    .await
    .unwrap();

    assert_eq!(approved.status, "pending_approval");

    let approved = approve_command_data(
        workspace.path(),
        &approved.id,
        MobileCommandDecisionRequest {
            decided_by: "fixture-operator".to_string(),
            reason: Some("approved during fixture run".to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(approved.status, "executed");
    assert_eq!(approved.command, DeviceCommandKind::PushNotification);
    assert_eq!(approved.result["transport"], "notification_preview");

    let rejected = dispatch_command_data(
        workspace.path(),
        MobileCommandDispatchRequest {
            node_id: "command-node".to_string(),
            command: DeviceCommandKind::PushNotification,
            payload: json!({
                "title": "Fixture reject",
                "body": "Rejected notification"
            }),
            approved_by: None,
            require_approval: Some(true),
        },
    )
    .await
    .unwrap();

    let rejected = reject_command_data(
        workspace.path(),
        &rejected.id,
        MobileCommandDecisionRequest {
            decided_by: "fixture-operator".to_string(),
            reason: Some("rejected during fixture run".to_string()),
        },
    )
    .unwrap();

    assert_eq!(rejected.status, "rejected");

    let commands = list_command_data(workspace.path(), Some("command-node"), Some(10)).unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].status, "rejected");
    assert_eq!(commands[1].status, "executed");
}

#[tokio::test]
async fn session_store_fixture_workflow() {
    init_test_tracing();

    let (pool, _temp_dir) = create_test_db_file().await;
    let store = SqliteSessionStore::new(pool);
    let session = Session::new_dm("fixture-user", Platform::WebChat);
    let session_id = session.id.to_string();

    store
        .create_or_update(
            &session,
            Some("webchat:fixture-user"),
            SessionStatus::Active,
        )
        .await
        .unwrap();

    store
        .append_message(&session_id, &Message::user("fixture request"))
        .await
        .unwrap();
    store
        .append_message(&session_id, &Message::assistant("fixture response"))
        .await
        .unwrap();

    let persisted = store.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(persisted.route_key.as_deref(), Some("webchat:fixture-user"));
    assert_eq!(persisted.status, SessionStatus::Active);

    let history = store.list_history(&session_id, 10).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "fixture request");
    assert_eq!(history[1].content, "fixture response");

    store
        .set_status(
            &session_id,
            SessionStatus::Archived,
            Some("fixture archived"),
        )
        .await
        .unwrap();

    let archived = store
        .list_sessions(Some(SessionStatus::Archived), 10)
        .await
        .unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].session.id.to_string(), session_id);
    assert_eq!(archived[0].status, SessionStatus::Archived);
    assert_eq!(
        archived[0].session.metadata["status_reason"],
        json!("fixture archived")
    );
}
